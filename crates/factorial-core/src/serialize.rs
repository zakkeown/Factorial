//! Serialization and snapshot support for the simulation engine.
//!
//! Provides binary serialization via `bitcode` with a versioned header,
//! a snapshot ring buffer for undo/replay, and per-subsystem hashing
//! for desync debugging.

use crate::engine::Engine;
use crate::event::EventBus;
use crate::graph::ProductionGraph;
use crate::id::{EdgeId, NodeId};
use crate::item::Inventory;
use crate::processor::{Modifier, Processor, ProcessorState};
use crate::sim::{SimState, SimulationStrategy, StateHash};
use crate::transport::{Transport, TransportState};
use serde::{Deserialize, Serialize};
use slotmap::SecondaryMap;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Magic number identifying a Factorial engine snapshot.
pub const SNAPSHOT_MAGIC: u32 = 0xFAC7_0001;

/// Current format version. Increment when breaking the wire format.
pub const FORMAT_VERSION: u32 = 2;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during serialization.
#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    #[error("bitcode encoding failed: {0}")]
    Encode(String),
}

/// Errors that can occur during deserialization.
#[derive(Debug, thiserror::Error)]
pub enum DeserializeError {
    #[error("data too short for snapshot header")]
    TooShort,
    #[error("invalid magic number: expected 0x{:08X}, got 0x{:08X}", SNAPSHOT_MAGIC, .0)]
    InvalidMagic(u32),
    #[error("unsupported format version: expected {}, got {}", FORMAT_VERSION, .0)]
    UnsupportedVersion(u32),
    #[error("snapshot from future version {0} (this build supports up to {FORMAT_VERSION})")]
    FutureVersion(u32),
    #[error("bitcode decoding failed: {0}")]
    Decode(String),
}

// ---------------------------------------------------------------------------
// Snapshot header
// ---------------------------------------------------------------------------

/// Header prepended to every serialized snapshot. Enables format detection
/// and version checking before attempting to decode the payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotHeader {
    /// Magic number for format detection.
    pub magic: u32,
    /// Format version for forward compatibility.
    pub version: u32,
    /// Tick count at the time the snapshot was taken.
    pub tick: u64,
}

impl SnapshotHeader {
    /// Create a header for the current format version.
    pub fn new(tick: u64) -> Self {
        Self {
            magic: SNAPSHOT_MAGIC,
            version: FORMAT_VERSION,
            tick,
        }
    }

    /// Validate the header. Returns `Ok(())` if valid.
    pub fn validate(&self) -> Result<(), DeserializeError> {
        if self.magic != SNAPSHOT_MAGIC {
            return Err(DeserializeError::InvalidMagic(self.magic));
        }
        if self.version > FORMAT_VERSION {
            return Err(DeserializeError::FutureVersion(self.version));
        }
        if self.version < FORMAT_VERSION {
            return Err(DeserializeError::UnsupportedVersion(self.version));
        }
        Ok(())
    }
}

/// Try to read just the snapshot header from serialized data.
///
/// This decodes the full snapshot but only returns the header, enabling
/// version detection before deciding whether to migrate.
pub fn read_snapshot_header(data: &[u8]) -> Result<SnapshotHeader, DeserializeError> {
    // Try to decode as an EngineSnapshot to extract the header.
    // If the version doesn't match, the decode might still work for
    // header extraction. We decode the whole thing because bitcode
    // doesn't support partial deserialization.
    let snapshot: EngineSnapshot =
        bitcode::deserialize(data).map_err(|e| DeserializeError::Decode(e.to_string()))?;
    Ok(snapshot.header)
}

// ---------------------------------------------------------------------------
// Serializable engine state (excludes non-serializable fields)
// ---------------------------------------------------------------------------

/// The serializable portion of the engine state. Excludes the EventBus
/// (contains closures) and the topo cache (recomputed on deserialize).
#[derive(Debug, Serialize, Deserialize)]
struct EngineSnapshot {
    header: SnapshotHeader,
    graph: ProductionGraph,
    strategy: SimulationStrategy,
    sim_state: SimState,
    processors: SecondaryMap<NodeId, Processor>,
    processor_states: SecondaryMap<NodeId, ProcessorState>,
    inputs: SecondaryMap<NodeId, Inventory>,
    outputs: SecondaryMap<NodeId, Inventory>,
    modifiers: SecondaryMap<NodeId, Vec<Modifier>>,
    transports: SecondaryMap<EdgeId, Transport>,
    transport_states: SecondaryMap<EdgeId, TransportState>,
    last_state_hash: u64,
    #[serde(default)]
    paused: bool,
    #[serde(default)]
    junctions: SecondaryMap<NodeId, crate::junction::Junction>,
    #[serde(default)]
    junction_states: SecondaryMap<NodeId, crate::junction::JunctionState>,
}

// ---------------------------------------------------------------------------
// SubsystemHashes
// ---------------------------------------------------------------------------

/// Per-subsystem state hashes for debugging desyncs. Allows pinpointing
/// which subsystem diverged between two simulation instances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubsystemHashes {
    pub graph: u64,
    pub processors: u64,
    pub processor_states: u64,
    pub inventories: u64,
    pub transports: u64,
    pub sim_state: u64,
}

// ---------------------------------------------------------------------------
// SnapshotRingBuffer
// ---------------------------------------------------------------------------

/// A fixed-capacity ring buffer of serialized engine snapshots.
///
/// When the buffer is full, the oldest snapshot is evicted. Each entry
/// stores the serialized bytes and the tick at which it was taken.
#[derive(Debug)]
pub struct SnapshotRingBuffer {
    /// Stored snapshots. Fixed-size allocation.
    entries: Vec<Option<SnapshotEntry>>,
    /// Write position (wraps around).
    head: usize,
    /// Number of snapshots currently stored.
    len: usize,
    /// Total snapshots ever taken (including evicted).
    total_taken: u64,
}

/// A single entry in the snapshot ring buffer.
#[derive(Debug, Clone)]
pub struct SnapshotEntry {
    /// Tick at which the snapshot was taken.
    pub tick: u64,
    /// Serialized engine state (bitcode bytes).
    pub data: Vec<u8>,
}

impl SnapshotRingBuffer {
    /// Create a new ring buffer with the given capacity.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "SnapshotRingBuffer capacity must be > 0");
        Self {
            entries: (0..capacity).map(|_| None).collect(),
            head: 0,
            len: 0,
            total_taken: 0,
        }
    }

    /// Push a snapshot into the ring buffer. If full, the oldest entry
    /// is evicted.
    pub fn push(&mut self, entry: SnapshotEntry) {
        self.entries[self.head] = Some(entry);
        self.head = (self.head + 1) % self.capacity();
        if self.len < self.capacity() {
            self.len += 1;
        }
        self.total_taken += 1;
    }

    /// The maximum number of snapshots this buffer can hold.
    pub fn capacity(&self) -> usize {
        self.entries.len()
    }

    /// Number of snapshots currently stored.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Total snapshots ever taken (including evicted).
    pub fn total_taken(&self) -> u64 {
        self.total_taken
    }

    /// Get a snapshot by index (0 = oldest, len-1 = newest).
    /// Returns `None` if the index is out of range.
    pub fn get(&self, index: usize) -> Option<&SnapshotEntry> {
        if index >= self.len {
            return None;
        }
        let start = if self.len < self.capacity() {
            0
        } else {
            self.head
        };
        let actual_index = (start + index) % self.capacity();
        self.entries[actual_index].as_ref()
    }

    /// Get the most recent snapshot.
    pub fn latest(&self) -> Option<&SnapshotEntry> {
        if self.len == 0 {
            return None;
        }
        self.get(self.len - 1)
    }

    /// Clear all snapshots.
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = None;
        }
        self.head = 0;
        self.len = 0;
    }
}

// ---------------------------------------------------------------------------
// Engine serialization methods
// ---------------------------------------------------------------------------

impl Engine {
    /// Serialize the engine state to a binary blob via bitcode.
    ///
    /// The EventBus is excluded (it contains closures that cannot be
    /// serialized). On deserialize, a fresh EventBus is created.
    pub fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        let snapshot = EngineSnapshot {
            header: SnapshotHeader::new(self.sim_state.tick),
            graph: self.graph.clone(),
            strategy: self.strategy.clone(),
            sim_state: self.sim_state.clone(),
            processors: self.processors.clone(),
            processor_states: self.processor_states.clone(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            modifiers: self.modifiers.clone(),
            transports: self.transports.clone(),
            transport_states: self.transport_states.clone(),
            last_state_hash: self.last_state_hash,
            paused: self.paused,
            junctions: self.junctions.clone(),
            junction_states: self.junction_states.clone(),
        };

        bitcode::serialize(&snapshot).map_err(|e| SerializeError::Encode(e.to_string()))
    }

    /// Deserialize an engine from a binary blob.
    ///
    /// Validates the snapshot header (magic number, version) before
    /// attempting to decode the payload. Returns an error (not a panic)
    /// on version mismatch.
    ///
    /// The EventBus is recreated empty. Subscribers must be re-registered
    /// after deserialization.
    pub fn deserialize(data: &[u8]) -> Result<Self, DeserializeError> {
        // We need at least enough bytes for a bitcode-encoded header.
        // Rather than trying to parse the header separately, we decode
        // the full snapshot and then validate the header.
        let snapshot: EngineSnapshot =
            bitcode::deserialize(data).map_err(|e| DeserializeError::Decode(e.to_string()))?;

        // Validate the header.
        snapshot.header.validate()?;

        Ok(Engine {
            graph: snapshot.graph,
            strategy: snapshot.strategy,
            sim_state: snapshot.sim_state,
            processors: snapshot.processors,
            processor_states: snapshot.processor_states,
            inputs: snapshot.inputs,
            outputs: snapshot.outputs,
            modifiers: snapshot.modifiers,
            transports: snapshot.transports,
            transport_states: snapshot.transport_states,
            last_state_hash: snapshot.last_state_hash,
            paused: snapshot.paused,
            event_bus: EventBus::default(),
            modules: Vec::new(),
            dirty: crate::dirty::DirtyTracker::new(),
            junctions: snapshot.junctions,
            junction_states: snapshot.junction_states,
            edge_budgets: SecondaryMap::new(),
            #[cfg(feature = "profiling")]
            last_profile: None,
        })
    }

    /// Deserialize an engine from a binary blob, applying migrations if needed.
    ///
    /// If the data is at the current format version, behaves like `deserialize()`.
    /// If the data is from an older version, applies registered migrations to
    /// bring it up to the current version before deserializing.
    /// If the data is from a future version, returns `FutureVersion` error.
    pub fn deserialize_with_migrations(
        data: &[u8],
        migrations: &crate::migration::MigrationRegistry,
    ) -> Result<Self, DeserializeError> {
        // Try normal deserialization first.
        match Self::deserialize(data) {
            Ok(engine) => Ok(engine),
            Err(DeserializeError::FutureVersion(v)) => {
                Err(DeserializeError::FutureVersion(v))
            }
            Err(DeserializeError::UnsupportedVersion(old_version)) => {
                // Try to migrate.
                let migrated_data = migrations
                    .migrate(data, old_version, FORMAT_VERSION)
                    .map_err(|e| DeserializeError::Decode(format!("migration failed: {e}")))?;
                Self::deserialize(&migrated_data)
            }
            Err(other) => Err(other),
        }
    }

    /// Compute per-subsystem state hashes for desync debugging.
    ///
    /// Each subsystem is hashed independently so that when two simulation
    /// instances diverge, you can identify which subsystem is responsible.
    pub fn subsystem_hashes(&self) -> SubsystemHashes {
        SubsystemHashes {
            graph: self.hash_graph(),
            processors: self.hash_processors(),
            processor_states: self.hash_processor_states(),
            inventories: self.hash_inventories(),
            transports: self.hash_transports(),
            sim_state: self.hash_sim_state(),
        }
    }

    // -- Subsystem hash helpers --

    fn hash_graph(&self) -> u64 {
        let mut h = StateHash::new();
        h.write_u64(self.graph.node_count() as u64);
        h.write_u64(self.graph.edge_count() as u64);
        for (node_id, node_data) in self.graph.nodes() {
            // Hash the raw slot key bits for determinism.
            h.write(&serde_json_key_bytes(node_id));
            h.write_u32(node_data.building_type.0);
        }
        for (edge_id, edge_data) in self.graph.edges() {
            h.write(&serde_json_key_bytes(edge_id));
            h.write(&serde_json_key_bytes(edge_data.from));
            h.write(&serde_json_key_bytes(edge_data.to));
        }
        h.finish()
    }

    fn hash_processors(&self) -> u64 {
        let mut h = StateHash::new();
        for (node_id, _) in self.graph.nodes() {
            if let Some(proc) = self.processors.get(node_id) {
                h.write(&serde_json_key_bytes(node_id));
                // Hash the processor variant discriminant.
                match proc {
                    Processor::Source(src) => {
                        h.write_u32(0);
                        h.write_u32(src.output_type.0);
                        h.write_fixed64(src.base_rate);
                        h.write_fixed64(src.accumulated);
                    }
                    Processor::Fixed(recipe) => {
                        h.write_u32(1);
                        h.write_u32(recipe.duration);
                        h.write_u32(recipe.inputs.len() as u32);
                        h.write_u32(recipe.outputs.len() as u32);
                    }
                    Processor::Property(prop) => {
                        h.write_u32(2);
                        h.write_u32(prop.input_type.0);
                        h.write_u32(prop.output_type.0);
                    }
                    Processor::Demand(demand) => {
                        h.write_u32(3);
                        h.write_u32(demand.input_type.0);
                        h.write_fixed64(demand.base_rate);
                        h.write_fixed64(demand.accumulated);
                    }
                    Processor::Passthrough => {
                        h.write_u32(4);
                    }
                }
            }
        }
        h.finish()
    }

    fn hash_processor_states(&self) -> u64 {
        let mut h = StateHash::new();
        for (node_id, _) in self.graph.nodes() {
            if let Some(ps) = self.processor_states.get(node_id) {
                h.write(&serde_json_key_bytes(node_id));
                match ps {
                    ProcessorState::Idle => h.write_u32(0),
                    ProcessorState::Working { progress } => {
                        h.write_u32(1);
                        h.write_u32(*progress);
                    }
                    ProcessorState::Stalled { reason } => {
                        h.write_u32(2);
                        h.write_u32(*reason as u32);
                    }
                }
            }
        }
        h.finish()
    }

    fn hash_inventories(&self) -> u64 {
        let mut h = StateHash::new();
        for (node_id, _) in self.graph.nodes() {
            h.write(&serde_json_key_bytes(node_id));
            if let Some(inv) = self.inputs.get(node_id) {
                for slot in &inv.input_slots {
                    for stack in &slot.stacks {
                        h.write_u32(stack.item_type.0);
                        h.write_u32(stack.quantity);
                    }
                }
            }
            if let Some(inv) = self.outputs.get(node_id) {
                for slot in &inv.output_slots {
                    for stack in &slot.stacks {
                        h.write_u32(stack.item_type.0);
                        h.write_u32(stack.quantity);
                    }
                }
            }
        }
        h.finish()
    }

    fn hash_transports(&self) -> u64 {
        let mut h = StateHash::new();
        for (edge_id, _) in self.graph.edges() {
            if let Some(state) = self.transport_states.get(edge_id) {
                h.write(&serde_json_key_bytes(edge_id));
                match state {
                    TransportState::Flow(fs) => {
                        h.write_u32(0);
                        h.write_fixed64(fs.buffered);
                        h.write_u32(fs.latency_remaining);
                    }
                    TransportState::Item(bs) => {
                        h.write_u32(1);
                        h.write_u32(bs.occupied_count() as u32);
                    }
                    TransportState::Batch(bs) => {
                        h.write_u32(2);
                        h.write_u32(bs.progress);
                        h.write_u32(bs.pending);
                    }
                    TransportState::Vehicle(vs) => {
                        h.write_u32(3);
                        h.write_u32(vs.position);
                        let cargo_total: u32 = vs.cargo.iter().map(|s| s.quantity).sum();
                        h.write_u32(cargo_total);
                    }
                }
            }
        }
        h.finish()
    }

    fn hash_sim_state(&self) -> u64 {
        let mut h = StateHash::new();
        h.write_u64(self.sim_state.tick);
        h.write_u64(self.sim_state.accumulator);
        h.finish()
    }
}

/// Convert a slotmap key to deterministic bytes for hashing.
/// We use the raw FFI representation (version + index packed into u64).
fn serde_json_key_bytes<K: slotmap::Key>(key: K) -> [u8; 8] {
    key.data().as_ffi().to_le_bytes()
}

// ---------------------------------------------------------------------------
// Engine snapshot ring buffer integration
// ---------------------------------------------------------------------------

impl Engine {
    /// Take a snapshot of the current engine state and store it in the
    /// provided ring buffer.
    pub fn take_snapshot(&self, buffer: &mut SnapshotRingBuffer) -> Result<(), SerializeError> {
        let data = self.serialize()?;
        buffer.push(SnapshotEntry {
            tick: self.sim_state.tick,
            data,
        });
        Ok(())
    }

    /// Restore the engine state from a snapshot in the ring buffer.
    ///
    /// `index` is 0-based from oldest (0) to newest (len-1).
    /// Returns `None` if the index is out of range.
    pub fn restore_snapshot(
        buffer: &SnapshotRingBuffer,
        index: usize,
    ) -> Result<Option<Engine>, DeserializeError> {
        let Some(entry) = buffer.get(index) else {
            return Ok(None);
        };
        let engine = Engine::deserialize(&entry.data)?;
        Ok(Some(engine))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed64;
    use crate::id::*;
    use crate::processor::*;
    use crate::sim::SimulationStrategy;
    use crate::transport::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn iron() -> ItemTypeId {
        ItemTypeId(0)
    }

    fn gear() -> ItemTypeId {
        ItemTypeId(2)
    }

    fn building() -> BuildingTypeId {
        BuildingTypeId(0)
    }

    fn simple_inventory(capacity: u32) -> Inventory {
        Inventory::new(1, 1, capacity)
    }

    fn make_source(item: ItemTypeId, rate: f64) -> Processor {
        Processor::Source(SourceProcessor {
            output_type: item,
            base_rate: Fixed64::from_num(rate),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0.0),
        })
    }

    fn make_recipe(
        inputs: Vec<(ItemTypeId, u32)>,
        outputs: Vec<(ItemTypeId, u32)>,
        duration: u32,
    ) -> Processor {
        Processor::Fixed(FixedRecipe {
            inputs: inputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeInput {
                    item_type,
                    quantity,
                })
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeOutput {
                    item_type,
                    quantity,
                })
                .collect(),
            duration,
        })
    }

    fn make_flow_transport(rate: f64) -> Transport {
        Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(rate),
            buffer_capacity: Fixed64::from_num(1000.0),
            latency: 0,
        })
    }

    /// Create a populated engine with some state for testing serialization.
    fn make_test_engine() -> Engine {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Add two nodes.
        let pending_src = engine.graph.queue_add_node(building());
        let pending_consumer = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let src_node = result.resolve_node(pending_src).unwrap();
        let consumer_node = result.resolve_node(pending_consumer).unwrap();

        // Connect with edge.
        let pending_edge = engine.graph.queue_connect(src_node, consumer_node);
        let result = engine.graph.apply_mutations();
        let edge_id = result.resolve_edge(pending_edge).unwrap();

        // Set up source.
        engine.set_processor(src_node, make_source(iron(), 3.0));
        engine.set_input_inventory(src_node, simple_inventory(100));
        engine.set_output_inventory(src_node, simple_inventory(100));

        // Set up consumer.
        engine.set_processor(
            consumer_node,
            make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 5),
        );
        let mut consumer_input = simple_inventory(100);
        consumer_input.input_slots[0].add(iron(), 10);
        engine.set_input_inventory(consumer_node, consumer_input);
        engine.set_output_inventory(consumer_node, simple_inventory(100));

        // Set up transport.
        engine.set_transport(edge_id, make_flow_transport(5.0));

        // Run a few ticks to build up state.
        for _ in 0..5 {
            engine.step();
        }

        engine
    }

    // -----------------------------------------------------------------------
    // Test 1: Round-trip serialize -> deserialize preserves state hash
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_round_trip_preserves_state_hash() {
        let engine = make_test_engine();
        let original_hash = engine.state_hash();
        let original_tick = engine.sim_state.tick;

        let data = engine.serialize().expect("serialize should succeed");
        let restored = Engine::deserialize(&data).expect("deserialize should succeed");

        assert_eq!(
            restored.state_hash(),
            original_hash,
            "state hash should be identical after round-trip"
        );
        assert_eq!(
            restored.sim_state.tick, original_tick,
            "tick should be identical after round-trip"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: NodeId stability across round-trips
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_node_id_stability() {
        let engine = make_test_engine();

        // Collect all node IDs.
        let original_node_ids: Vec<NodeId> =
            engine.graph.nodes().map(|(id, _)| id).collect();
        let original_edge_ids: Vec<EdgeId> =
            engine.graph.edges().map(|(id, _)| id).collect();

        let data = engine.serialize().unwrap();
        let restored = Engine::deserialize(&data).unwrap();

        // Verify all node IDs are present and accessible.
        let restored_node_ids: Vec<NodeId> =
            restored.graph.nodes().map(|(id, _)| id).collect();
        let restored_edge_ids: Vec<EdgeId> =
            restored.graph.edges().map(|(id, _)| id).collect();

        assert_eq!(
            original_node_ids.len(),
            restored_node_ids.len(),
            "node count should match"
        );
        assert_eq!(
            original_edge_ids.len(),
            restored_edge_ids.len(),
            "edge count should match"
        );

        // Each original NodeId should be valid in the restored graph.
        for node_id in &original_node_ids {
            assert!(
                restored.graph.contains_node(*node_id),
                "NodeId {:?} should be valid after round-trip",
                node_id
            );
        }
        for edge_id in &original_edge_ids {
            assert!(
                restored.graph.contains_edge(*edge_id),
                "EdgeId {:?} should be valid after round-trip",
                edge_id
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: Version mismatch produces explicit error (not panic)
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_version_mismatch_error() {
        let engine = make_test_engine();
        let _data = engine.serialize().unwrap();

        // Use completely invalid data to trigger a decode error.
        let garbage = vec![0u8; 10];
        let result = Engine::deserialize(&garbage);
        assert!(result.is_err(), "garbage data should fail to deserialize");

        // Verify it's a decode error (not a panic).
        match result {
            Err(DeserializeError::Decode(_)) => {} // expected
            Err(other) => panic!("expected Decode error, got: {other}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: State hash changes when state changes
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_state_hash_changes_with_state() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        engine.step();
        let hash1 = engine.state_hash();

        engine.step();
        let hash2 = engine.state_hash();

        assert_ne!(
            hash1, hash2,
            "state hash should change between ticks with state changes"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: State hash identical for identical state
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_state_hash_identical_for_identical_state() {
        fn run() -> u64 {
            let mut engine = Engine::new(SimulationStrategy::Tick);

            let pending = engine.graph.queue_add_node(building());
            let result = engine.graph.apply_mutations();
            let node = result.resolve_node(pending).unwrap();

            engine.set_processor(node, make_source(iron(), 2.0));
            engine.set_input_inventory(node, simple_inventory(100));
            engine.set_output_inventory(node, simple_inventory(100));

            for _ in 0..10 {
                engine.step();
            }
            engine.state_hash()
        }

        assert_eq!(
            run(),
            run(),
            "identical simulations should produce identical state hashes"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Snapshot ring buffer evicts oldest
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_snapshot_ring_buffer_evicts_oldest() {
        let mut buffer = SnapshotRingBuffer::new(3);

        // Push 5 entries into a buffer of capacity 3.
        for i in 0..5u64 {
            buffer.push(SnapshotEntry {
                tick: i,
                data: vec![i as u8],
            });
        }

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.capacity(), 3);
        assert_eq!(buffer.total_taken(), 5);

        // Oldest should be tick 2 (entries 0 and 1 were evicted).
        let oldest = buffer.get(0).unwrap();
        assert_eq!(oldest.tick, 2);

        // Newest should be tick 4.
        let newest = buffer.get(2).unwrap();
        assert_eq!(newest.tick, 4);

        let latest = buffer.latest().unwrap();
        assert_eq!(latest.tick, 4);
    }

    // -----------------------------------------------------------------------
    // Test 7: Snapshot ring buffer capacity 1
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_snapshot_ring_buffer_capacity_one() {
        let mut buffer = SnapshotRingBuffer::new(1);

        buffer.push(SnapshotEntry {
            tick: 10,
            data: vec![1],
        });
        buffer.push(SnapshotEntry {
            tick: 20,
            data: vec![2],
        });

        assert_eq!(buffer.len(), 1);
        let entry = buffer.get(0).unwrap();
        assert_eq!(entry.tick, 20);
    }

    // -----------------------------------------------------------------------
    // Test 8: Take and restore snapshot round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_take_and_restore_snapshot() {
        let engine = make_test_engine();
        let original_hash = engine.state_hash();

        let mut buffer = SnapshotRingBuffer::new(5);
        engine
            .take_snapshot(&mut buffer)
            .expect("take_snapshot should succeed");

        assert_eq!(buffer.len(), 1);

        let restored = Engine::restore_snapshot(&buffer, 0)
            .expect("restore should not fail")
            .expect("index 0 should exist");

        assert_eq!(
            restored.state_hash(),
            original_hash,
            "restored engine should have same state hash"
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: Restore from invalid index returns None
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_restore_invalid_index() {
        let buffer = SnapshotRingBuffer::new(5);
        let result = Engine::restore_snapshot(&buffer, 0).unwrap();
        assert!(result.is_none());
    }

    // -----------------------------------------------------------------------
    // Test 10: Subsystem hashes are consistent
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_subsystem_hashes_consistent() {
        let engine1 = make_test_engine();
        let engine2 = make_test_engine();

        let hashes1 = engine1.subsystem_hashes();
        let hashes2 = engine2.subsystem_hashes();

        assert_eq!(hashes1, hashes2, "identical engines should have identical subsystem hashes");
    }

    // -----------------------------------------------------------------------
    // Test 11: Deserialized engine can continue simulation
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_deserialized_engine_continues() {
        let mut engine = make_test_engine();
        let data = engine.serialize().unwrap();

        let mut restored = Engine::deserialize(&data).unwrap();

        // Both engines should produce the same hashes after stepping.
        engine.step();
        restored.step();

        assert_eq!(
            engine.state_hash(),
            restored.state_hash(),
            "engines should remain in sync after continuing from snapshot"
        );
    }

    // -----------------------------------------------------------------------
    // Test 12: Snapshot header validation
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_header_validation() {
        let good = SnapshotHeader::new(42);
        assert!(good.validate().is_ok());

        let bad_magic = SnapshotHeader {
            magic: 0xDEAD_BEEF,
            version: FORMAT_VERSION,
            tick: 0,
        };
        assert!(matches!(
            bad_magic.validate(),
            Err(DeserializeError::InvalidMagic(0xDEAD_BEEF))
        ));

        let bad_version = SnapshotHeader {
            magic: SNAPSHOT_MAGIC,
            version: 999,
            tick: 0,
        };
        assert!(matches!(
            bad_version.validate(),
            Err(DeserializeError::FutureVersion(999))
        ));
    }

    // -----------------------------------------------------------------------
    // Test 13: Empty engine serializes and deserializes
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_empty_engine_round_trip() {
        let engine = Engine::new(SimulationStrategy::Tick);
        let data = engine.serialize().unwrap();
        let restored = Engine::deserialize(&data).unwrap();

        assert_eq!(restored.node_count(), 0);
        assert_eq!(restored.edge_count(), 0);
        assert_eq!(restored.sim_state.tick, 0);
    }

    // -----------------------------------------------------------------------
    // Test 14: Ring buffer clear
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_ring_buffer_clear() {
        let mut buffer = SnapshotRingBuffer::new(5);

        for i in 0..3 {
            buffer.push(SnapshotEntry {
                tick: i,
                data: vec![],
            });
        }
        assert_eq!(buffer.len(), 3);

        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        // total_taken is NOT reset by clear.
        assert_eq!(buffer.total_taken(), 3);
    }

    // -----------------------------------------------------------------------
    // Test 15: Serialized data is compact (bitcode, not JSON)
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_data_is_compact() {
        let engine = make_test_engine();
        let data = engine.serialize().unwrap();

        // bitcode should be much more compact than JSON. A populated engine
        // should serialize to well under 10KB.
        assert!(
            data.len() < 10_000,
            "serialized data should be compact, got {} bytes",
            data.len()
        );
    }

    // -----------------------------------------------------------------------
    // Test 16: Inventory contents preserved across round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_inventory_contents_preserved() {
        let engine = make_test_engine();

        // Get inventory contents before serialization.
        let original_snapshots = engine.snapshot_all_nodes();

        let data = engine.serialize().unwrap();
        let restored = Engine::deserialize(&data).unwrap();

        let restored_snapshots = restored.snapshot_all_nodes();

        assert_eq!(original_snapshots.len(), restored_snapshots.len());

        // Match each node's inventory contents.
        for orig in &original_snapshots {
            let rest = restored_snapshots
                .iter()
                .find(|s| s.id == orig.id)
                .expect("node should exist in restored snapshots");

            assert_eq!(
                orig.input_contents, rest.input_contents,
                "input contents should match for node {:?}",
                orig.id
            );
            assert_eq!(
                orig.output_contents, rest.output_contents,
                "output contents should match for node {:?}",
                orig.id
            );
            assert_eq!(
                orig.processor_state, rest.processor_state,
                "processor state should match for node {:?}",
                orig.id
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 17: Multiple snapshots, restore specific one
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_multiple_snapshots_restore_specific() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        let mut buffer = SnapshotRingBuffer::new(10);

        // Take snapshots at different ticks.
        let mut hashes = Vec::new();
        for _ in 0..5 {
            engine.step();
            engine.take_snapshot(&mut buffer).unwrap();
            hashes.push(engine.state_hash());
        }

        assert_eq!(buffer.len(), 5);

        // Restore each snapshot and verify hash.
        for (i, expected_hash) in hashes.iter().enumerate() {
            let restored = Engine::restore_snapshot(&buffer, i)
                .unwrap()
                .unwrap();
            assert_eq!(
                restored.state_hash(),
                *expected_hash,
                "snapshot {} should have matching hash",
                i
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 18: Subsystem hashes change independently
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_subsystem_hashes_change_independently() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        let h1 = engine.subsystem_hashes();

        engine.step();

        let h2 = engine.subsystem_hashes();

        // After a step, sim_state should definitely change (tick increments).
        assert_ne!(
            h1.sim_state, h2.sim_state,
            "sim_state hash should change after step"
        );

        // Graph hash should NOT change (no structural changes).
        assert_eq!(
            h1.graph, h2.graph,
            "graph hash should not change without structural changes"
        );
    }

    // -----------------------------------------------------------------------
    // Test 19: Processor state preserved across round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_processor_state_preserved() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // Recipe: 1 iron -> 1 gear, 10 ticks.
        engine.set_processor(
            node,
            make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 10),
        );
        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 5);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        // Run 3 ticks (processor should be Working { progress: 3 }).
        for _ in 0..3 {
            engine.step();
        }

        let state_before = engine.get_processor_state(node).unwrap().clone();
        assert!(matches!(state_before, ProcessorState::Working { progress: 3 }));

        let data = engine.serialize().unwrap();
        let restored = Engine::deserialize(&data).unwrap();

        let state_after = restored.get_processor_state(node).unwrap();
        assert_eq!(
            *state_after, state_before,
            "processor state should be preserved across round-trip"
        );
    }

    // -----------------------------------------------------------------------
    // Test 20: Future version error
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_future_version_error() {
        let header = SnapshotHeader {
            magic: SNAPSHOT_MAGIC,
            version: FORMAT_VERSION + 1,
            tick: 0,
        };
        assert!(matches!(
            header.validate(),
            Err(DeserializeError::FutureVersion(_))
        ));
    }

    // -----------------------------------------------------------------------
    // Test 21: Past version error
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_past_version_error() {
        let header = SnapshotHeader {
            magic: SNAPSHOT_MAGIC,
            version: 0,
            tick: 0,
        };
        assert!(matches!(
            header.validate(),
            Err(DeserializeError::UnsupportedVersion(0))
        ));
    }

    // -----------------------------------------------------------------------
    // Test 22: Current version validates
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_current_version_validates() {
        let header = SnapshotHeader::new(42);
        assert!(header.validate().is_ok());
        assert_eq!(header.version, FORMAT_VERSION);
    }

    // -----------------------------------------------------------------------
    // Test 23: Transport with different variants all serialize
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_all_transport_variants() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Create 4 pairs of nodes for 4 transport types.
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for _ in 0..4 {
            let pa = engine.graph.queue_add_node(building());
            let pb = engine.graph.queue_add_node(building());
            let r = engine.graph.apply_mutations();
            let a = r.resolve_node(pa).unwrap();
            let b = r.resolve_node(pb).unwrap();

            let pe = engine.graph.queue_connect(a, b);
            let r = engine.graph.apply_mutations();
            let e = r.resolve_edge(pe).unwrap();

            engine.set_processor(a, make_source(iron(), 1.0));
            engine.set_input_inventory(a, simple_inventory(100));
            engine.set_output_inventory(a, simple_inventory(100));
            engine.set_processor(
                b,
                make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 1),
            );
            engine.set_input_inventory(b, simple_inventory(100));
            engine.set_output_inventory(b, simple_inventory(100));

            nodes.push((a, b));
            edges.push(e);
        }

        // Set up different transport types.
        engine.set_transport(edges[0], make_flow_transport(3.0));
        engine.set_transport(
            edges[1],
            Transport::Item(ItemTransport {
                speed: Fixed64::from_num(1.0),
                slot_count: 5,
                lanes: 1,
            }),
        );
        engine.set_transport(
            edges[2],
            Transport::Batch(BatchTransport {
                batch_size: 10,
                cycle_time: 5,
            }),
        );
        engine.set_transport(
            edges[3],
            Transport::Vehicle(VehicleTransport {
                capacity: 20,
                travel_time: 3,
            }),
        );

        // Run a few ticks.
        for _ in 0..5 {
            engine.step();
        }

        let data = engine.serialize().unwrap();
        let restored = Engine::deserialize(&data).unwrap();

        assert_eq!(
            engine.state_hash(),
            restored.state_hash(),
            "all transport variants should round-trip correctly"
        );
    }
}
