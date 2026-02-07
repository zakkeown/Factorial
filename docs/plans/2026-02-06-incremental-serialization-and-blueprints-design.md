# Incremental Serialization & Blueprint System Design

**Date:** 2026-02-06
**Status:** Draft
**Scope:** 2 features across 2 crates (factorial-core, factorial-spatial)

These are the final two features with 2+ persona demand from the stress test findings. Both are independent and can be implemented in parallel.

---

## Feature 1: Incremental / Dirty-State Serialization

### Problem

`Engine::serialize()` clones all 14 fields (9 full `SecondaryMap`s) into an `EngineSnapshot` and bitcode-serializes the entire thing every time. At 100k nodes with complex inventories, this is expensive. Mobile autosave, background persistence, and frequent snapshots (undo ring buffer) all pay this full cost unnecessarily.

The `DirtyTracker` already knows which nodes and edges changed, but this information is discarded at the end of every tick and never used by the serialization path.

### Design: Partitioned Snapshots

Split engine state into 5 independently serializable **partitions**. On each save, only re-serialize partitions that contain dirty state. Store a `PartitionedSnapshot` as a collection of partition blobs.

#### Partition Layout

```
Partition 0 - Graph:       graph topology + sim_state + strategy
Partition 1 - Processors:  processors + processor_states + modifiers
Partition 2 - Inventories: inputs + outputs
Partition 3 - Transports:  transports + transport_states
Partition 4 - Junctions:   junctions + junction_states
```

**Rationale:** Partition 2 (inventories) changes every tick as items move. Partitions 0, 1, 3, 4 change only on configuration (new buildings, recipe swaps, belt changes). During steady-state autosave, typically only 1 of 5 partitions needs re-serialization.

#### New Types

In `crates/factorial-core/src/serialize.rs`:

```rust
/// Identifies which state partition was modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Partition {
    Graph = 0,
    Processors = 1,
    Inventories = 2,
    Transports = 3,
    Junctions = 4,
}

/// A snapshot stored as independent partition blobs.
/// Each partition is independently bitcode-serialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionedSnapshot {
    pub header: SnapshotHeader,
    /// One serialized blob per partition. Index by Partition enum ordinal.
    pub partitions: [Vec<u8>; 5],
    pub last_state_hash: u64,
    pub paused: bool,
}
```

Individual partition payload structs (private, used for ser/de):

```rust
#[derive(Serialize, Deserialize)]
struct GraphPartition {
    graph: ProductionGraph,
    strategy: SimulationStrategy,
    sim_state: SimState,
}

#[derive(Serialize, Deserialize)]
struct ProcessorPartition {
    processors: SecondaryMap<NodeId, Processor>,
    processor_states: SecondaryMap<NodeId, ProcessorState>,
    modifiers: SecondaryMap<NodeId, Vec<Modifier>>,
}

#[derive(Serialize, Deserialize)]
struct InventoryPartition {
    inputs: SecondaryMap<NodeId, Inventory>,
    outputs: SecondaryMap<NodeId, Inventory>,
}

#[derive(Serialize, Deserialize)]
struct TransportPartition {
    transports: SecondaryMap<EdgeId, Transport>,
    transport_states: SecondaryMap<EdgeId, TransportState>,
}

#[derive(Serialize, Deserialize)]
struct JunctionPartition {
    junctions: SecondaryMap<NodeId, Junction>,
    junction_states: SecondaryMap<NodeId, JunctionState>,
}
```

#### DirtyTracker Extension

In `crates/factorial-core/src/dirty.rs`, add partition-level tracking:

```rust
pub struct DirtyTracker {
    dirty_nodes: BTreeSet<NodeId>,
    dirty_edges: BTreeSet<EdgeId>,
    graph_dirty: bool,
    any_dirty: bool,
    // NEW: partition-level dirty flags, NOT cleared by mark_clean()
    dirty_partitions: BTreeSet<Partition>,
}
```

**Inference rules** (applied automatically when existing mark methods are called):
- `mark_node()` → sets `Processors` and `Inventories` dirty (conservative: we don't know if it was a processor change or inventory change)
- `mark_edge()` → sets `Transports` dirty
- `mark_graph()` → sets `Graph` dirty

**Key design decision:** `dirty_partitions` is NOT cleared by `mark_clean()`. It accumulates across ticks and is only cleared when the game calls `clear_dirty_partitions()` after a successful incremental save. This decouples save frequency from tick frequency.

New methods:

```rust
impl DirtyTracker {
    /// Which partitions have been modified since last save.
    pub fn dirty_partitions(&self) -> &BTreeSet<Partition> {
        &self.dirty_partitions
    }

    /// Clear partition dirty flags after a successful save.
    pub fn clear_dirty_partitions(&mut self) {
        self.dirty_partitions.clear();
    }

    /// Mark all partitions dirty (used for first save or full snapshot).
    pub fn mark_all_partitions_dirty(&mut self) {
        self.dirty_partitions.insert(Partition::Graph);
        self.dirty_partitions.insert(Partition::Processors);
        self.dirty_partitions.insert(Partition::Inventories);
        self.dirty_partitions.insert(Partition::Transports);
        self.dirty_partitions.insert(Partition::Junctions);
    }
}
```

#### Engine API

New methods on `Engine` in `crates/factorial-core/src/engine.rs`:

```rust
impl Engine {
    /// Serialize into a partitioned snapshot. All partitions are serialized.
    /// Use `serialize_incremental` for selective re-serialization.
    pub fn serialize_partitioned(&self) -> Result<PartitionedSnapshot, SerializeError>

    /// Serialize only dirty partitions, reusing clean partition blobs
    /// from the baseline. If no baseline is provided, serializes everything.
    ///
    /// After a successful call, clears the partition dirty flags.
    pub fn serialize_incremental(
        &mut self,
        baseline: Option<&PartitionedSnapshot>,
    ) -> Result<PartitionedSnapshot, SerializeError>

    /// Deserialize a PartitionedSnapshot back to an Engine.
    pub fn deserialize_partitioned(
        snapshot: &PartitionedSnapshot,
    ) -> Result<Self, DeserializeError>

    /// Convert a legacy full snapshot to partitioned format.
    pub fn partitioned_from_bytes(data: &[u8]) -> Result<PartitionedSnapshot, DeserializeError>
}
```

#### Usage Pattern

```rust
// Game initialization: first save creates full baseline
let mut baseline = engine.serialize_partitioned()?;

// Game loop
loop {
    engine.step();

    if should_autosave() {
        // Only re-serializes partitions that changed since last save
        baseline = engine.serialize_incremental(Some(&baseline))?;
        write_to_disk(&bitcode::serialize(&baseline)?);
    }
}

// Loading
let snapshot: PartitionedSnapshot = bitcode::deserialize(&read_from_disk()?)?;
let engine = Engine::deserialize_partitioned(&snapshot)?;
```

#### Backwards Compatibility

- `Engine::serialize()` and `Engine::deserialize()` remain unchanged
- `PartitionedSnapshot` uses a different magic number (`0xFAC7_0002`) to distinguish from legacy format
- `Engine::partitioned_from_bytes()` can upgrade legacy snapshots to partitioned format
- The `SnapshotRingBuffer` gains a `push_partitioned()` method that stores `PartitionedSnapshot` alongside the existing `push()` for full snapshots

#### Performance Expectations

At steady state (no building/config changes, just item movement):
- **Before:** Serialize all 5 partitions every save
- **After:** Serialize only Partition 2 (inventories) — roughly 1/5th the work

When configuration changes (new building, recipe swap):
- 2-3 partitions dirty — still a meaningful reduction vs full serialization

#### Files Modified

| File | Changes |
|------|---------|
| `crates/factorial-core/src/dirty.rs` | Add `dirty_partitions` field, inference rules, new methods |
| `crates/factorial-core/src/serialize.rs` | Add `Partition`, `PartitionedSnapshot`, partition payload structs, serialize/deserialize methods |
| `crates/factorial-core/src/engine.rs` | Add `serialize_partitioned()`, `serialize_incremental()`, `deserialize_partitioned()` |

#### Tests

1. `partitioned_round_trip` — serialize partitioned, deserialize, verify identical state
2. `incremental_only_reserialized_dirty_partitions` — modify only inventories, verify only Partition 2 blob changes
3. `incremental_with_no_baseline_serializes_all` — first save without baseline works
4. `partition_dirty_flags_accumulate_across_ticks` — dirty flags persist until save
5. `partition_dirty_flags_cleared_after_save` — flags reset after `serialize_incremental`
6. `legacy_to_partitioned_upgrade` — convert old format snapshot to partitioned
7. `partitioned_snapshot_ring_buffer` — ring buffer stores partitioned snapshots

---

## Feature 2: Blueprint / Ghost Placement System

### Problem

Games with build UIs need to preview where buildings will go before committing. Players drag-place multiple buildings, see visual feedback on validity, then confirm the whole batch. Currently there's no way to tentatively place buildings and validate them against each other and existing buildings.

### Design: Blueprint as a Lightweight Overlay

A `Blueprint` is a collection of planned buildings and connections that haven't been committed to the engine. It validates against both the real `SpatialIndex` and its own internal ghost tiles.

#### Location

New file: `crates/factorial-spatial/src/blueprint.rs`, re-exported from `lib.rs`.

The `commit()` method takes `&mut Engine` + `&mut SpatialIndex` as parameters, keeping the dependency direction clean. `factorial-spatial` already depends on `factorial-core` for `NodeId`, `EdgeId`, etc.

#### Core Types

```rust
/// Opaque ID for a planned building within a blueprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlueprintEntryId(u64);

/// Reference to either an existing node or a planned one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintNodeRef {
    /// An already-committed node in the engine.
    Existing(NodeId),
    /// A planned node within this blueprint.
    Planned(BlueprintEntryId),
}

/// A planned building placement.
#[derive(Debug, Clone)]
pub struct BlueprintEntry {
    pub building_type: BuildingTypeId,
    pub position: GridPosition,
    pub footprint: BuildingFootprint,
    pub processor: Processor,
    pub input_capacity: u32,
    pub output_capacity: u32,
}

/// A planned connection between nodes (existing or planned).
#[derive(Debug, Clone)]
pub struct BlueprintConnection {
    pub from: BlueprintNodeRef,
    pub to: BlueprintNodeRef,
    pub transport: Transport,
    pub item_filter: Option<ItemTypeId>,
}

/// Error types for blueprint operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlueprintError {
    /// Position overlaps with an existing building on the grid.
    OverlapsExisting { position: GridPosition },
    /// Position overlaps with another planned building in this blueprint.
    OverlapsPlanned { position: GridPosition, other: BlueprintEntryId },
    /// Referenced blueprint entry does not exist.
    EntryNotFound(BlueprintEntryId),
    /// Referenced existing node does not exist in the engine.
    NodeNotFound(NodeId),
    /// Blueprint is empty (nothing to commit).
    Empty,
}

/// Result of committing a blueprint to the engine.
#[derive(Debug)]
pub struct BlueprintCommitResult {
    /// Maps each blueprint entry to its real NodeId after commit.
    pub node_map: BTreeMap<BlueprintEntryId, NodeId>,
    /// Maps each connection index to its real EdgeId.
    pub edge_ids: Vec<EdgeId>,
}
```

#### Blueprint Struct & API

```rust
pub struct Blueprint {
    entries: BTreeMap<BlueprintEntryId, BlueprintEntry>,
    connections: Vec<BlueprintConnection>,
    /// Ghost tile occupancy: position → entry that occupies it.
    ghost_tiles: BTreeMap<GridPosition, BlueprintEntryId>,
    next_id: u64,
}

impl Blueprint {
    /// Create an empty blueprint.
    pub fn new() -> Self

    /// Add a planned building. Validates against:
    /// 1. Real buildings in the SpatialIndex
    /// 2. Other planned buildings in this blueprint
    /// Returns the entry ID on success.
    pub fn add(
        &mut self,
        entry: BlueprintEntry,
        spatial: &SpatialIndex,
    ) -> Result<BlueprintEntryId, BlueprintError>

    /// Remove a planned building and its ghost tiles.
    pub fn remove(&mut self, id: BlueprintEntryId) -> Result<(), BlueprintError>

    /// Move a planned building to a new position.
    /// Re-validates against spatial index and other entries.
    pub fn move_entry(
        &mut self,
        id: BlueprintEntryId,
        new_position: GridPosition,
        spatial: &SpatialIndex,
    ) -> Result<(), BlueprintError>

    /// Add a planned connection between two node references.
    pub fn connect(
        &mut self,
        from: BlueprintNodeRef,
        to: BlueprintNodeRef,
        transport: Transport,
        item_filter: Option<ItemTypeId>,
    )

    /// Check if a grid position is occupied by a ghost building.
    pub fn is_ghost_at(&self, pos: &GridPosition) -> bool

    /// Get the entry ID at a ghost position.
    pub fn ghost_at(&self, pos: &GridPosition) -> Option<BlueprintEntryId>

    /// Get a planned entry by ID.
    pub fn get(&self, id: BlueprintEntryId) -> Option<&BlueprintEntry>

    /// Iterate all planned entries.
    pub fn entries(&self) -> impl Iterator<Item = (BlueprintEntryId, &BlueprintEntry)>

    /// Iterate all planned connections.
    pub fn connections(&self) -> &[BlueprintConnection]

    /// Number of planned buildings.
    pub fn len(&self) -> usize

    /// Whether the blueprint has any entries.
    pub fn is_empty(&self) -> bool

    /// Validate the entire blueprint against current spatial state.
    /// Returns all errors found (not just the first).
    pub fn validate(&self, spatial: &SpatialIndex) -> Vec<BlueprintError>

    /// Check if a single position + footprint can be placed,
    /// considering both real buildings and ghost buildings.
    pub fn can_place(
        &self,
        position: &GridPosition,
        footprint: &BuildingFootprint,
        spatial: &SpatialIndex,
    ) -> bool

    /// Commit the entire blueprint to the engine and spatial index.
    ///
    /// 1. Validates all entries against current spatial state
    /// 2. Queues graph mutations for all nodes
    /// 3. Applies mutations and resolves real NodeIds
    /// 4. Places all buildings on the spatial index
    /// 5. Sets up processors, inventories, transports
    /// 6. Queues and applies edge mutations
    ///
    /// On success, the blueprint is consumed and all entries become
    /// real engine nodes. On failure, nothing is modified.
    pub fn commit(
        self,
        engine: &mut Engine,
        spatial: &mut SpatialIndex,
    ) -> Result<BlueprintCommitResult, BlueprintError>

    /// Clear all entries and connections.
    pub fn clear(&mut self)
}
```

#### Commit Implementation Detail

The commit is **atomic** — either all buildings are placed or none are. Implementation:

1. **Pre-validate:** Call `validate(spatial)`. If any errors, return early.
2. **Queue all node additions:**
   ```rust
   let mut pending_map: BTreeMap<BlueprintEntryId, PendingNodeId> = BTreeMap::new();
   for (id, entry) in &self.entries {
       let pending = engine.graph.queue_add_node(entry.building_type);
       pending_map.insert(id, pending);
   }
   let result = engine.graph.apply_mutations();
   ```
3. **Resolve and configure each node:**
   ```rust
   let mut node_map: BTreeMap<BlueprintEntryId, NodeId> = BTreeMap::new();
   for (entry_id, pending_id) in &pending_map {
       let node_id = result.resolve_node(*pending_id).unwrap();
       let entry = &self.entries[entry_id];

       spatial.place(node_id, entry.position, entry.footprint)?;
       engine.set_processor(node_id, entry.processor.clone());
       engine.set_input_inventory(node_id, simple_inventory(entry.input_capacity));
       engine.set_output_inventory(node_id, simple_inventory(entry.output_capacity));

       node_map.insert(*entry_id, node_id);
   }
   ```
4. **Queue and apply connections:**
   ```rust
   let mut edge_ids = Vec::new();
   for conn in &self.connections {
       let from = resolve_ref(conn.from, &node_map)?;
       let to = resolve_ref(conn.to, &node_map)?;
       let pending = match conn.item_filter {
           Some(filter) => engine.graph.queue_connect_filtered(from, to, Some(filter)),
           None => engine.graph.queue_connect(from, to),
       };
       let result = engine.graph.apply_mutations();
       let edge_id = result.resolve_edge(pending).unwrap();
       engine.set_transport(edge_id, conn.transport.clone());
       edge_ids.push(edge_id);
   }
   ```
5. **Return result:**
   ```rust
   Ok(BlueprintCommitResult { node_map, edge_ids })
   ```

#### Serialization

`Blueprint` derives `Serialize` and `Deserialize` so in-progress blueprints can be saved/loaded with the game state. This is separate from the engine snapshot — the game stores the blueprint alongside the engine save.

#### Usage Pattern

```rust
// Player starts placing buildings
let mut blueprint = Blueprint::new();

// Player clicks to place a furnace
let entry = BlueprintEntry {
    building_type: furnace_type,
    position: GridPosition { x: 5, y: 3 },
    footprint: BuildingFootprint { width: 2, height: 2 },
    processor: make_recipe(vec![(iron_ore, 1)], vec![(iron_ingot, 1)], 3),
    input_capacity: 100,
    output_capacity: 100,
};
let furnace_id = blueprint.add(entry, &spatial)?;

// Player places a belt endpoint
let belt_entry = BlueprintEntry { /* ... */ };
let belt_id = blueprint.add(belt_entry, &spatial)?;

// Player connects them
blueprint.connect(
    BlueprintNodeRef::Planned(furnace_id),
    BlueprintNodeRef::Planned(belt_id),
    make_flow_transport(5.0),
    None,
);

// Render ghosts using blueprint.entries() for positions/footprints
// Show red/green validity using blueprint.validate(&spatial)

// Player confirms
let result = blueprint.commit(engine, &mut spatial)?;
let real_furnace = result.node_map[&furnace_id];
```

#### Files Modified

| File | Changes |
|------|---------|
| `crates/factorial-spatial/src/blueprint.rs` | New file: Blueprint struct and all types |
| `crates/factorial-spatial/src/lib.rs` | Re-export `blueprint` module |
| `crates/factorial-spatial/Cargo.toml` | Add dependency on `factorial-core` (for Engine, Processor, etc.) |

#### Tests

1. `blueprint_add_and_validate` — add entries, validate against spatial index
2. `blueprint_overlap_detection` — entries can't overlap each other or existing buildings
3. `blueprint_move_entry` — move an entry, re-validates position
4. `blueprint_commit_creates_real_nodes` — commit produces real NodeIds and EdgeIds
5. `blueprint_commit_is_atomic` — if validation fails, nothing is modified
6. `blueprint_with_connections` — connections between planned nodes resolve correctly
7. `blueprint_mixed_refs` — connections from existing nodes to planned nodes
8. `blueprint_can_place_checks_ghosts` — can_place considers both real and ghost tiles
9. `blueprint_serialization_round_trip` — serialize/deserialize an in-progress blueprint
10. `blueprint_clear` — clear removes all entries and ghost tiles

---

## Implementation Order

Both features are independent. Suggested order:

```
1. Incremental Serialization (factorial-core only)
   ├── 1a. Extend DirtyTracker with partition flags
   ├── 1b. Add partition payload structs to serialize.rs
   ├── 1c. Implement serialize_partitioned / deserialize_partitioned
   ├── 1d. Implement serialize_incremental
   └── 1e. Tests

2. Blueprint System (factorial-spatial, depends on factorial-core types)
   ├── 2a. Core types and Blueprint struct
   ├── 2b. add / remove / move_entry / validate
   ├── 2c. commit implementation
   ├── 2d. Serialization support
   └── 2e. Tests
```

Estimated scope: ~400 lines for incremental serialization, ~500 lines for blueprints + tests for both.
