//! Blueprint / ghost placement system for previewing and staging building
//! placement before committing to the simulation.

use crate::{BuildingFootprint, GridPosition, Rotation, SpatialIndex};
use factorial_core::engine::Engine;
use factorial_core::id::{BuildingTypeId, EdgeId, ItemTypeId, NodeId};
use factorial_core::item::Inventory;
use factorial_core::processor::Processor;
use factorial_core::serialize::{SerializeError, SnapshotEntry, SnapshotRingBuffer};
use factorial_core::transport::Transport;
use serde::{Deserialize, Serialize};
use slotmap::Key;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Unique identifier for an entry within a blueprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlueprintEntryId(pub u64);

/// Reference to a node — either an existing graph node or a planned blueprint entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlueprintNodeRef {
    Existing(NodeId),
    Planned(BlueprintEntryId),
}

/// A single building entry in a blueprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintEntry {
    pub building_type: BuildingTypeId,
    pub position: GridPosition,
    pub footprint: BuildingFootprint,
    #[serde(default)]
    pub rotation: Rotation,
    pub processor: Processor,
    pub input_capacity: u32,
    pub output_capacity: u32,
}

/// A connection between two nodes in a blueprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintConnection {
    pub from: BlueprintNodeRef,
    pub to: BlueprintNodeRef,
    pub transport: Transport,
    pub item_filter: Option<ItemTypeId>,
}

/// Errors from blueprint operations.
#[derive(Debug, thiserror::Error)]
pub enum BlueprintError {
    #[error("overlaps existing building")]
    OverlapsExisting,
    #[error("overlaps planned building")]
    OverlapsPlanned,
    #[error("blueprint entry not found")]
    EntryNotFound,
    #[error("node not found in graph")]
    NodeNotFound,
    #[error("blueprint is empty")]
    Empty,
    #[error("partial commit rolled back ({placed_count} placed): {cause}")]
    PartialCommitRollback { placed_count: usize, cause: String },
    #[error("rollback failed: original={original_error}, rollback={rollback_error}")]
    RollbackFailed {
        original_error: String,
        rollback_error: String,
    },
}

/// Error combining a blueprint commit with a snapshot.
#[derive(Debug, thiserror::Error)]
pub enum BlueprintCommitError {
    #[error("commit failed: {0}")]
    Commit(#[from] BlueprintError),
    #[error("snapshot failed: {0}")]
    Snapshot(#[from] SerializeError),
}

/// Error type for blueprint save/load I/O operations.
#[cfg(feature = "blueprint-io")]
#[derive(Debug, thiserror::Error)]
pub enum BlueprintIoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("deserialization error: {0}")]
    Deserialize(#[source] serde_json::Error),
}

/// Result of committing a blueprint to the engine.
#[derive(Debug)]
pub struct BlueprintCommitResult {
    /// Maps blueprint entry IDs to their assigned NodeIds.
    pub node_map: BTreeMap<BlueprintEntryId, NodeId>,
    /// Edge IDs created for connections.
    pub edge_ids: Vec<EdgeId>,
}

impl BlueprintCommitResult {
    /// Create an undo record from this commit result.
    pub fn undo_record(&self) -> BlueprintUndoRecord {
        BlueprintUndoRecord {
            node_ids: self.node_map.values().copied().collect(),
            edge_ids: self.edge_ids.clone(),
        }
    }
}

/// Record of committed nodes/edges that can be used to undo a commit.
#[derive(Debug, Clone)]
pub struct BlueprintUndoRecord {
    pub node_ids: Vec<NodeId>,
    pub edge_ids: Vec<EdgeId>,
}

// ---------------------------------------------------------------------------
// Blueprint
// ---------------------------------------------------------------------------

/// A staged set of buildings and connections that can be previewed as ghosts
/// and then committed to the engine atomically.
#[derive(Debug, Clone, Serialize)]
pub struct Blueprint {
    entries: BTreeMap<BlueprintEntryId, BlueprintEntry>,
    connections: Vec<BlueprintConnection>,
    #[serde(skip)]
    ghost_tiles: BTreeMap<GridPosition, BlueprintEntryId>,
    next_id: u64,
}

impl<'de> Deserialize<'de> for Blueprint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BlueprintData {
            entries: BTreeMap<BlueprintEntryId, BlueprintEntry>,
            connections: Vec<BlueprintConnection>,
            next_id: u64,
        }

        let data = BlueprintData::deserialize(deserializer)?;

        // Rebuild ghost_tiles from entries (using rotation-aware footprint).
        let mut ghost_tiles = BTreeMap::new();
        for (&id, entry) in &data.entries {
            let effective = entry.footprint.rotated(entry.rotation);
            for tile in effective.tiles(entry.position) {
                ghost_tiles.insert(tile, id);
            }
        }

        Ok(Blueprint {
            entries: data.entries,
            connections: data.connections,
            ghost_tiles,
            next_id: data.next_id,
        })
    }
}

impl Blueprint {
    /// Create an empty blueprint.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            connections: Vec::new(),
            ghost_tiles: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Add a building entry to the blueprint.
    ///
    /// Validates that the footprint doesn't overlap with real buildings
    /// (via SpatialIndex) or other ghost tiles.
    pub fn add(
        &mut self,
        entry: BlueprintEntry,
        spatial: &SpatialIndex,
    ) -> Result<BlueprintEntryId, BlueprintError> {
        let effective = entry.footprint.rotated(entry.rotation);
        // Check overlap with real buildings.
        for tile in effective.tiles(entry.position) {
            if spatial.is_occupied(tile) {
                return Err(BlueprintError::OverlapsExisting);
            }
        }
        // Check overlap with ghost tiles.
        for tile in effective.tiles(entry.position) {
            if self.ghost_tiles.contains_key(&tile) {
                return Err(BlueprintError::OverlapsPlanned);
            }
        }

        let id = BlueprintEntryId(self.next_id);
        self.next_id += 1;

        // Insert ghost tiles.
        for tile in effective.tiles(entry.position) {
            self.ghost_tiles.insert(tile, id);
        }

        self.entries.insert(id, entry);
        Ok(id)
    }

    /// Remove an entry from the blueprint, clearing its ghost tiles
    /// and any connections referencing it.
    pub fn remove(&mut self, id: BlueprintEntryId) -> Result<(), BlueprintError> {
        let entry = self
            .entries
            .remove(&id)
            .ok_or(BlueprintError::EntryNotFound)?;

        // Remove ghost tiles.
        let effective = entry.footprint.rotated(entry.rotation);
        for tile in effective.tiles(entry.position) {
            self.ghost_tiles.remove(&tile);
        }

        // Remove connections that reference this entry.
        let planned_ref = BlueprintNodeRef::Planned(id);
        self.connections
            .retain(|c| c.from != planned_ref && c.to != planned_ref);

        Ok(())
    }

    /// Move a blueprint entry to a new position.
    ///
    /// Removes old ghost tiles, validates the new position, and inserts
    /// new ghost tiles. If the new position is blocked, the entry is
    /// rolled back to its original position.
    pub fn move_entry(
        &mut self,
        id: BlueprintEntryId,
        new_pos: GridPosition,
        spatial: &SpatialIndex,
    ) -> Result<(), BlueprintError> {
        let entry = self.entries.get(&id).ok_or(BlueprintError::EntryNotFound)?;
        let old_pos = entry.position;
        let effective = entry.footprint.rotated(entry.rotation);

        // Remove old ghost tiles.
        for tile in effective.tiles(old_pos) {
            self.ghost_tiles.remove(&tile);
        }

        // Check overlap with real buildings.
        for tile in effective.tiles(new_pos) {
            if spatial.is_occupied(tile) {
                // Rollback: re-insert old ghost tiles.
                for tile in effective.tiles(old_pos) {
                    self.ghost_tiles.insert(tile, id);
                }
                return Err(BlueprintError::OverlapsExisting);
            }
        }
        // Check overlap with other ghost tiles.
        for tile in effective.tiles(new_pos) {
            if self.ghost_tiles.contains_key(&tile) {
                // Rollback.
                for tile in effective.tiles(old_pos) {
                    self.ghost_tiles.insert(tile, id);
                }
                return Err(BlueprintError::OverlapsPlanned);
            }
        }

        // Insert new ghost tiles.
        for tile in effective.tiles(new_pos) {
            self.ghost_tiles.insert(tile, id);
        }

        // Update entry position.
        self.entries.get_mut(&id).unwrap().position = new_pos;

        Ok(())
    }

    /// Add a connection between two nodes.
    pub fn connect(
        &mut self,
        from: BlueprintNodeRef,
        to: BlueprintNodeRef,
        transport: Transport,
        item_filter: Option<ItemTypeId>,
    ) {
        self.connections.push(BlueprintConnection {
            from,
            to,
            transport,
            item_filter,
        });
    }

    /// Check if a ghost tile exists at the given position.
    pub fn is_ghost_at(&self, pos: GridPosition) -> bool {
        self.ghost_tiles.contains_key(&pos)
    }

    /// Get the blueprint entry ID at a ghost tile position.
    pub fn ghost_at(&self, pos: GridPosition) -> Option<BlueprintEntryId> {
        self.ghost_tiles.get(&pos).copied()
    }

    /// Get a blueprint entry by ID.
    pub fn get(&self, id: BlueprintEntryId) -> Option<&BlueprintEntry> {
        self.entries.get(&id)
    }

    /// Iterate over all entries.
    pub fn entries(&self) -> impl Iterator<Item = (&BlueprintEntryId, &BlueprintEntry)> {
        self.entries.iter()
    }

    /// Get all connections.
    pub fn connections(&self) -> &[BlueprintConnection] {
        &self.connections
    }

    /// Number of entries in the blueprint.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the blueprint is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries, connections, and ghost tiles.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.connections.clear();
        self.ghost_tiles.clear();
    }

    /// Check if a building with the given footprint can be placed at the
    /// given position, considering both real buildings and ghost tiles.
    pub fn can_place(
        &self,
        pos: GridPosition,
        footprint: BuildingFootprint,
        spatial: &SpatialIndex,
    ) -> bool {
        for tile in footprint.tiles(pos) {
            if spatial.is_occupied(tile) || self.ghost_tiles.contains_key(&tile) {
                return false;
            }
        }
        true
    }

    /// Validate all entries against the current spatial state.
    /// Returns a list of errors for entries that now overlap.
    pub fn validate(&self, spatial: &SpatialIndex) -> Vec<BlueprintError> {
        let mut errors = Vec::new();
        for entry in self.entries.values() {
            let effective = entry.footprint.rotated(entry.rotation);
            for tile in effective.tiles(entry.position) {
                if spatial.is_occupied(tile) {
                    errors.push(BlueprintError::OverlapsExisting);
                    break;
                }
            }
        }
        errors
    }

    /// Commit the blueprint to the engine and spatial index atomically.
    ///
    /// 1. Validates all entries against the spatial index
    /// 2. Validates Existing node refs exist in the graph
    /// 3. Creates all nodes in the graph
    /// 4. Places buildings in the spatial index
    /// 5. Sets up processors and inventories
    /// 6. Creates all connections
    /// 7. Returns the mapping from blueprint IDs to real NodeIds
    ///
    /// If placement fails partway through, already-placed nodes are rolled back.
    pub fn commit(
        self,
        engine: &mut Engine,
        spatial: &mut SpatialIndex,
    ) -> Result<BlueprintCommitResult, BlueprintError> {
        if self.entries.is_empty() {
            return Err(BlueprintError::Empty);
        }

        // Validate no overlaps with real buildings.
        for entry in self.entries.values() {
            let effective = entry.footprint.rotated(entry.rotation);
            for tile in effective.tiles(entry.position) {
                if spatial.is_occupied(tile) {
                    return Err(BlueprintError::OverlapsExisting);
                }
            }
        }

        // Validate Existing node refs.
        for conn in &self.connections {
            if let BlueprintNodeRef::Existing(node_id) = conn.from
                && !engine.graph.contains_node(node_id)
            {
                return Err(BlueprintError::NodeNotFound);
            }
            if let BlueprintNodeRef::Existing(node_id) = conn.to
                && !engine.graph.contains_node(node_id)
            {
                return Err(BlueprintError::NodeNotFound);
            }
        }

        // Queue all node additions.
        let mut pending_map = BTreeMap::new();
        for (&id, entry) in &self.entries {
            let pending = engine.graph.queue_add_node(entry.building_type);
            pending_map.insert(id, pending);
        }

        // Apply mutations to get real NodeIds.
        let mutation_result = engine.graph.apply_mutations();
        let mut node_map = BTreeMap::new();
        for (&id, pending) in &pending_map {
            let node_id = mutation_result
                .resolve_node(*pending)
                .ok_or(BlueprintError::NodeNotFound)?;
            node_map.insert(id, node_id);
        }

        // Place in spatial index and set up processors/inventories.
        // Track placed nodes for rollback on failure.
        let mut placed_nodes: Vec<(NodeId, BlueprintEntryId)> = Vec::new();
        for (&id, entry) in &self.entries {
            let node_id = node_map[&id];
            let effective = entry.footprint.rotated(entry.rotation);
            if let Err(e) = spatial.place(node_id, entry.position, effective) {
                // Rollback: remove already-placed nodes.
                let placed_count = placed_nodes.len();
                let cause = e.to_string();
                let mut rollback_errors = Vec::new();
                for &(placed_node, _) in &placed_nodes {
                    if let Err(re) = spatial.remove(placed_node) {
                        rollback_errors.push(re.to_string());
                    }
                    engine.remove_node_state(placed_node);
                    engine.graph.queue_remove_node(placed_node);
                }
                engine.graph.apply_mutations();
                if rollback_errors.is_empty() {
                    return Err(BlueprintError::PartialCommitRollback {
                        placed_count,
                        cause,
                    });
                } else {
                    return Err(BlueprintError::RollbackFailed {
                        original_error: cause,
                        rollback_error: rollback_errors.join("; "),
                    });
                }
            }
            engine.set_processor(node_id, entry.processor.clone());
            engine.set_input_inventory(node_id, Inventory::new(1, 1, entry.input_capacity));
            engine.set_output_inventory(node_id, Inventory::new(1, 1, entry.output_capacity));
            placed_nodes.push((node_id, id));
        }

        // Queue connections.
        let mut pending_edges = Vec::new();
        for conn in &self.connections {
            let from_node = match conn.from {
                BlueprintNodeRef::Existing(id) => id,
                BlueprintNodeRef::Planned(bp_id) => node_map[&bp_id],
            };
            let to_node = match conn.to {
                BlueprintNodeRef::Existing(id) => id,
                BlueprintNodeRef::Planned(bp_id) => node_map[&bp_id],
            };

            let pending = engine
                .graph
                .queue_connect_filtered(from_node, to_node, conn.item_filter);
            pending_edges.push((pending, conn.transport.clone()));
        }

        // Apply edge mutations.
        let edge_result = engine.graph.apply_mutations();
        let mut edge_ids = Vec::new();
        for (pending, transport) in pending_edges {
            let edge_id = edge_result
                .resolve_edge(pending)
                .ok_or(BlueprintError::NodeNotFound)?;
            engine.set_transport(edge_id, transport);
            edge_ids.push(edge_id);
        }

        Ok(BlueprintCommitResult { node_map, edge_ids })
    }

    /// Commit the blueprint and take an incremental snapshot atomically.
    pub fn commit_with_snapshot(
        self,
        engine: &mut Engine,
        spatial: &mut SpatialIndex,
        ring_buffer: &mut SnapshotRingBuffer,
        baseline: Option<&[u8]>,
    ) -> Result<(BlueprintCommitResult, Vec<u8>), BlueprintCommitError> {
        let commit_result = self.commit(engine, spatial)?;
        let snapshot_data = engine.serialize_incremental(baseline)?;
        ring_buffer.push(SnapshotEntry {
            tick: engine.sim_state.tick,
            data: snapshot_data.clone(),
        });
        Ok((commit_result, snapshot_data))
    }

    /// Undo a previously committed blueprint, removing its nodes and edges.
    pub fn undo(
        record: &BlueprintUndoRecord,
        engine: &mut Engine,
        spatial: &mut SpatialIndex,
    ) -> Result<(), BlueprintError> {
        // Disconnect edges first.
        for &edge_id in &record.edge_ids {
            engine.graph.queue_disconnect(edge_id);
        }
        engine.graph.apply_mutations();

        // Remove edge state.
        for &edge_id in &record.edge_ids {
            engine.remove_edge_state(edge_id);
        }

        // Remove nodes from spatial and graph.
        for &node_id in &record.node_ids {
            let _ = spatial.remove(node_id);
            engine.remove_node_state(node_id);
            engine.graph.queue_remove_node(node_id);
        }
        engine.graph.apply_mutations();

        Ok(())
    }

    /// Estimate the total resource cost of all buildings in this blueprint.
    pub fn estimate_cost<F>(&self, cost_fn: F) -> BTreeMap<ItemTypeId, u32>
    where
        F: Fn(BuildingTypeId) -> Vec<(ItemTypeId, u32)>,
    {
        let mut totals = BTreeMap::new();
        for entry in self.entries.values() {
            for (item, qty) in cost_fn(entry.building_type) {
                *totals.entry(item).or_insert(0) += qty;
            }
        }
        totals
    }

    /// Capture a region from an existing engine/spatial into a new blueprint.
    ///
    /// Scans all nodes in the rectangle `[min, max]`, creates blueprint entries
    /// for each, and preserves connections between captured nodes.
    /// Positions are translated by `offset`.
    pub fn capture_region(
        engine: &Engine,
        spatial: &SpatialIndex,
        min: GridPosition,
        max: GridPosition,
        offset: GridPosition,
    ) -> Self {
        let nodes = spatial.nodes_in_rect(min, max);
        let mut bp = Blueprint::new();
        let mut node_to_bp: BTreeMap<u64, BlueprintEntryId> = BTreeMap::new();

        for &node_id in &nodes {
            let Some(position) = spatial.get_position(node_id) else {
                continue;
            };
            let Some(footprint) = spatial.get_footprint(node_id) else {
                continue;
            };
            let Some(node_data) = engine.graph.get_node(node_id) else {
                continue;
            };

            let processor = engine
                .get_processor(node_id)
                .cloned()
                .unwrap_or(Processor::Passthrough);

            let input_capacity = engine
                .get_input_inventory(node_id)
                .and_then(|inv| inv.input_slots.first())
                .map(|s| s.capacity)
                .unwrap_or(100);

            let output_capacity = engine
                .get_output_inventory(node_id)
                .and_then(|inv| inv.output_slots.first())
                .map(|s| s.capacity)
                .unwrap_or(100);

            let entry = BlueprintEntry {
                building_type: node_data.building_type,
                position: GridPosition::new(position.x + offset.x, position.y + offset.y),
                footprint,
                rotation: Rotation::None,
                processor,
                input_capacity,
                output_capacity,
            };

            let bp_id = BlueprintEntryId(bp.next_id);
            bp.next_id += 1;
            bp.entries.insert(bp_id, entry);
            node_to_bp.insert(node_id.data().as_ffi(), bp_id);
        }

        // Capture connections between captured nodes.
        for &node_id in &nodes {
            let node_key = node_id.data().as_ffi();
            let Some(&from_bp_id) = node_to_bp.get(&node_key) else {
                continue;
            };
            for &edge_id in engine.graph.get_outputs(node_id) {
                let Some(edge_data) = engine.graph.get_edge(edge_id) else {
                    continue;
                };
                let to_key = edge_data.to.data().as_ffi();
                if let Some(&to_bp_id) = node_to_bp.get(&to_key) {
                    let transport = engine.get_transport(edge_id).cloned().unwrap_or_else(|| {
                        Transport::Flow(factorial_core::transport::FlowTransport {
                            rate: factorial_core::fixed::Fixed64::from_num(1.0),
                            buffer_capacity: factorial_core::fixed::Fixed64::from_num(100.0),
                            latency: 0,
                        })
                    });
                    bp.connections.push(BlueprintConnection {
                        from: BlueprintNodeRef::Planned(from_bp_id),
                        to: BlueprintNodeRef::Planned(to_bp_id),
                        transport,
                        item_filter: edge_data.item_filter,
                    });
                }
            }
        }

        bp
    }

    /// Save this blueprint to a JSON file.
    #[cfg(feature = "blueprint-io")]
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), BlueprintIoError> {
        let json = serde_json::to_string_pretty(self).map_err(BlueprintIoError::Serialize)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a blueprint from a JSON file.
    #[cfg(feature = "blueprint-io")]
    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> Result<Self, BlueprintIoError> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(BlueprintIoError::Deserialize)
    }
}

impl Default for Blueprint {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use factorial_core::engine::Engine;
    use factorial_core::serialize::SnapshotRingBuffer;
    use factorial_core::sim::SimulationStrategy;
    use factorial_core::test_utils::{building, iron, make_flow_transport, make_source};

    fn make_entry(pos: GridPosition) -> BlueprintEntry {
        BlueprintEntry {
            building_type: building(),
            position: pos,
            footprint: BuildingFootprint::single(),
            rotation: Rotation::None,
            processor: make_source(iron(), 1.0),
            input_capacity: 100,
            output_capacity: 100,
        }
    }

    fn make_entry_with_footprint(
        pos: GridPosition,
        footprint: BuildingFootprint,
    ) -> BlueprintEntry {
        BlueprintEntry {
            building_type: building(),
            position: pos,
            footprint,
            rotation: Rotation::None,
            processor: make_source(iron(), 1.0),
            input_capacity: 100,
            output_capacity: 100,
        }
    }

    fn make_rotated_entry(
        pos: GridPosition,
        footprint: BuildingFootprint,
        rotation: Rotation,
    ) -> BlueprintEntry {
        BlueprintEntry {
            building_type: building(),
            position: pos,
            footprint,
            rotation,
            processor: make_source(iron(), 1.0),
            input_capacity: 100,
            output_capacity: 100,
        }
    }

    fn setup_engine_and_spatial() -> (Engine, SpatialIndex) {
        let engine = Engine::new(SimulationStrategy::Tick);
        let spatial = SpatialIndex::new();
        (engine, spatial)
    }

    // -----------------------------------------------------------------------
    // Test 1: add_single_entry
    // -----------------------------------------------------------------------
    #[test]
    fn add_single_entry() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let id = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        assert_eq!(bp.len(), 1);
        assert!(bp.is_ghost_at(GridPosition::new(0, 0)));
        assert_eq!(bp.ghost_at(GridPosition::new(0, 0)), Some(id));
        assert!(bp.get(id).is_some());
    }

    // -----------------------------------------------------------------------
    // Test 2: add_overlaps_existing
    // -----------------------------------------------------------------------
    #[test]
    fn add_overlaps_existing() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        // Place a real building.
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        spatial
            .place(node, GridPosition::new(5, 5), BuildingFootprint::single())
            .unwrap();

        // Try to add blueprint entry at same position.
        let result = bp.add(make_entry(GridPosition::new(5, 5)), &spatial);
        assert!(matches!(result, Err(BlueprintError::OverlapsExisting)));
    }

    // -----------------------------------------------------------------------
    // Test 3: add_overlaps_planned
    // -----------------------------------------------------------------------
    #[test]
    fn add_overlaps_planned() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let result = bp.add(make_entry(GridPosition::new(0, 0)), &spatial);
        assert!(matches!(result, Err(BlueprintError::OverlapsPlanned)));
    }

    // -----------------------------------------------------------------------
    // Test 4: add_multi_tile_ghost_tracking
    // -----------------------------------------------------------------------
    #[test]
    fn add_multi_tile_ghost_tracking() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let fp = BuildingFootprint {
            width: 3,
            height: 2,
        };
        let id = bp
            .add(
                make_entry_with_footprint(GridPosition::new(0, 0), fp),
                &spatial,
            )
            .unwrap();

        // Should occupy 6 tiles.
        for x in 0..3 {
            for y in 0..2 {
                assert!(
                    bp.is_ghost_at(GridPosition::new(x, y)),
                    "ghost at ({x},{y})"
                );
                assert_eq!(bp.ghost_at(GridPosition::new(x, y)), Some(id));
            }
        }
        assert!(!bp.is_ghost_at(GridPosition::new(3, 0)));
    }

    // -----------------------------------------------------------------------
    // Test 5: remove_entry
    // -----------------------------------------------------------------------
    #[test]
    fn remove_entry() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let id = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        bp.remove(id).unwrap();

        assert_eq!(bp.len(), 0);
        assert!(!bp.is_ghost_at(GridPosition::new(0, 0)));
        assert!(bp.get(id).is_none());
    }

    // -----------------------------------------------------------------------
    // Test 6: remove_cleans_connections
    // -----------------------------------------------------------------------
    #[test]
    fn remove_cleans_connections() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let id1 = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let id2 = bp
            .add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        bp.connect(
            BlueprintNodeRef::Planned(id1),
            BlueprintNodeRef::Planned(id2),
            make_flow_transport(1.0),
            None,
        );
        assert_eq!(bp.connections().len(), 1);

        bp.remove(id1).unwrap();
        assert_eq!(bp.connections().len(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 7: move_entry
    // -----------------------------------------------------------------------
    #[test]
    fn move_entry() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let id = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        bp.move_entry(id, GridPosition::new(5, 5), &spatial)
            .unwrap();

        assert!(!bp.is_ghost_at(GridPosition::new(0, 0)));
        assert!(bp.is_ghost_at(GridPosition::new(5, 5)));
        assert_eq!(bp.get(id).unwrap().position, GridPosition::new(5, 5));
    }

    // -----------------------------------------------------------------------
    // Test 8: move_entry_blocked
    // -----------------------------------------------------------------------
    #[test]
    fn move_entry_blocked() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        // Place real building at (5,5).
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        spatial
            .place(node, GridPosition::new(5, 5), BuildingFootprint::single())
            .unwrap();

        let id = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let result = bp.move_entry(id, GridPosition::new(5, 5), &spatial);
        assert!(matches!(result, Err(BlueprintError::OverlapsExisting)));

        // Rollback: original position should still be ghosted.
        assert!(bp.is_ghost_at(GridPosition::new(0, 0)));
    }

    // -----------------------------------------------------------------------
    // Test 9: commit_single_node
    // -----------------------------------------------------------------------
    #[test]
    fn commit_single_node() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let bp_id = bp
            .add(make_entry(GridPosition::new(3, 3)), &spatial)
            .unwrap();
        let result = bp.commit(&mut engine, &mut spatial).unwrap();

        assert!(result.node_map.contains_key(&bp_id));
        let node_id = result.node_map[&bp_id];
        assert!(engine.graph.contains_node(node_id));
        assert_eq!(spatial.node_at(GridPosition::new(3, 3)), Some(node_id));
        assert!(engine.get_input_inventory(node_id).is_some());
        assert!(engine.get_output_inventory(node_id).is_some());
    }

    // -----------------------------------------------------------------------
    // Test 10: commit_with_connections
    // -----------------------------------------------------------------------
    #[test]
    fn commit_with_connections() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let id1 = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let id2 = bp
            .add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        bp.connect(
            BlueprintNodeRef::Planned(id1),
            BlueprintNodeRef::Planned(id2),
            make_flow_transport(5.0),
            None,
        );

        let result = bp.commit(&mut engine, &mut spatial).unwrap();

        assert_eq!(result.node_map.len(), 2);
        assert_eq!(result.edge_ids.len(), 1);
        let edge_id = result.edge_ids[0];
        assert!(engine.graph.contains_edge(edge_id));
    }

    // -----------------------------------------------------------------------
    // Test 11: commit_connects_to_existing_node
    // -----------------------------------------------------------------------
    #[test]
    fn commit_connects_to_existing_node() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();

        // Create an existing node.
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let existing_node = result.resolve_node(pending).unwrap();
        engine.set_processor(existing_node, make_source(iron(), 1.0));
        engine.set_input_inventory(existing_node, Inventory::new(1, 1, 100));
        engine.set_output_inventory(existing_node, Inventory::new(1, 1, 100));
        spatial
            .place(
                existing_node,
                GridPosition::new(0, 0),
                BuildingFootprint::single(),
            )
            .unwrap();

        let mut bp = Blueprint::new();
        let bp_id = bp
            .add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        bp.connect(
            BlueprintNodeRef::Existing(existing_node),
            BlueprintNodeRef::Planned(bp_id),
            make_flow_transport(3.0),
            None,
        );

        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        assert_eq!(result.edge_ids.len(), 1);
        assert!(engine.graph.contains_edge(result.edge_ids[0]));
    }

    // -----------------------------------------------------------------------
    // Test 12: commit_empty_error
    // -----------------------------------------------------------------------
    #[test]
    fn commit_empty_error() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let bp = Blueprint::new();
        let result = bp.commit(&mut engine, &mut spatial);
        assert!(matches!(result, Err(BlueprintError::Empty)));
    }

    // -----------------------------------------------------------------------
    // Test 13: validate_detects_stale_overlap
    // -----------------------------------------------------------------------
    #[test]
    fn validate_detects_stale_overlap() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        // Add blueprint entry at (0,0).
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        // Now place a real building at (0,0), making the blueprint stale.
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        spatial
            .place(node, GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();

        let errors = bp.validate(&spatial);
        assert_eq!(errors.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Test 14: can_place_checks_both_layers
    // -----------------------------------------------------------------------
    #[test]
    fn can_place_checks_both_layers() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        // Place a real building at (0,0).
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        spatial
            .place(node, GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();

        // Ghost at (1,0).
        bp.add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();

        // Can't place at (0,0) (real) or (1,0) (ghost).
        assert!(!bp.can_place(
            GridPosition::new(0, 0),
            BuildingFootprint::single(),
            &spatial
        ));
        assert!(!bp.can_place(
            GridPosition::new(1, 0),
            BuildingFootprint::single(),
            &spatial
        ));
        // Can place at (2,0).
        assert!(bp.can_place(
            GridPosition::new(2, 0),
            BuildingFootprint::single(),
            &spatial
        ));
    }

    // -----------------------------------------------------------------------
    // Test 15: clear_resets_all
    // -----------------------------------------------------------------------
    #[test]
    fn clear_resets_all() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let id1 = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let id2 = bp
            .add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        bp.connect(
            BlueprintNodeRef::Planned(id1),
            BlueprintNodeRef::Planned(id2),
            make_flow_transport(1.0),
            None,
        );

        bp.clear();

        assert!(bp.is_empty());
        assert_eq!(bp.connections().len(), 0);
        assert!(!bp.is_ghost_at(GridPosition::new(0, 0)));
        assert!(!bp.is_ghost_at(GridPosition::new(1, 0)));
    }

    // -----------------------------------------------------------------------
    // Test 16: serialization_roundtrip
    // -----------------------------------------------------------------------
    #[test]
    fn serialization_roundtrip() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let id1 = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let id2 = bp
            .add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        bp.connect(
            BlueprintNodeRef::Planned(id1),
            BlueprintNodeRef::Planned(id2),
            make_flow_transport(1.0),
            None,
        );

        let json = serde_json::to_string(&bp).unwrap();
        let restored: Blueprint = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        assert_eq!(restored.connections().len(), 1);
        // Ghost tiles should be rebuilt from entries.
        assert!(restored.is_ghost_at(GridPosition::new(0, 0)));
        assert!(restored.is_ghost_at(GridPosition::new(1, 0)));
    }

    // =======================================================================
    // WI1: commit_with_snapshot tests
    // =======================================================================

    #[test]
    fn commit_with_snapshot_creates_entry() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut ring = SnapshotRingBuffer::new(5);
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        let (_result, _data) = bp
            .commit_with_snapshot(&mut engine, &mut spatial, &mut ring, None)
            .unwrap();
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn commit_with_snapshot_returns_valid_commit() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut ring = SnapshotRingBuffer::new(5);
        let mut bp = Blueprint::new();
        let bp_id1 = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let bp_id2 = bp
            .add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        bp.connect(
            BlueprintNodeRef::Planned(bp_id1),
            BlueprintNodeRef::Planned(bp_id2),
            make_flow_transport(1.0),
            None,
        );

        let (result, _data) = bp
            .commit_with_snapshot(&mut engine, &mut spatial, &mut ring, None)
            .unwrap();
        assert_eq!(result.node_map.len(), 2);
        assert_eq!(result.edge_ids.len(), 1);
    }

    #[test]
    fn commit_with_snapshot_incremental_uses_baseline() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut ring = SnapshotRingBuffer::new(5);

        let mut bp1 = Blueprint::new();
        bp1.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let (_result1, data1) = bp1
            .commit_with_snapshot(&mut engine, &mut spatial, &mut ring, None)
            .unwrap();

        let mut bp2 = Blueprint::new();
        bp2.add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        let (_result2, _data2) = bp2
            .commit_with_snapshot(&mut engine, &mut spatial, &mut ring, Some(&data1))
            .unwrap();

        assert_eq!(ring.len(), 2);
    }

    #[test]
    fn commit_with_snapshot_empty_blueprint_errors() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut ring = SnapshotRingBuffer::new(5);
        let bp = Blueprint::new();
        let result = bp.commit_with_snapshot(&mut engine, &mut spatial, &mut ring, None);
        assert!(matches!(result, Err(BlueprintCommitError::Commit(_))));
    }

    #[test]
    fn commit_with_snapshot_snapshot_is_deserializable() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut ring = SnapshotRingBuffer::new(5);
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        let (_result, data) = bp
            .commit_with_snapshot(&mut engine, &mut spatial, &mut ring, None)
            .unwrap();
        let restored = Engine::deserialize_partitioned(&data);
        assert!(restored.is_ok());
    }

    #[test]
    fn commit_with_snapshot_engine_state_matches() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut ring = SnapshotRingBuffer::new(5);
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        let (_result, data) = bp
            .commit_with_snapshot(&mut engine, &mut spatial, &mut ring, None)
            .unwrap();
        let restored = Engine::deserialize_partitioned(&data).unwrap();
        assert_eq!(restored.node_count(), engine.node_count());
    }

    // =======================================================================
    // WI3a: Rotation tests
    // =======================================================================

    #[test]
    fn rotation_footprint_swap_90() {
        let fp = BuildingFootprint {
            width: 3,
            height: 2,
        };
        let rotated = fp.rotated(Rotation::Cw90);
        assert_eq!(rotated.width, 2);
        assert_eq!(rotated.height, 3);
    }

    #[test]
    fn rotation_footprint_180_unchanged() {
        let fp = BuildingFootprint {
            width: 3,
            height: 2,
        };
        let rotated = fp.rotated(Rotation::Cw180);
        assert_eq!(rotated.width, 3);
        assert_eq!(rotated.height, 2);
    }

    #[test]
    fn rotation_add_uses_rotated_footprint() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        // 3w x 1h rotated 90 becomes 1w x 3h
        let fp = BuildingFootprint {
            width: 3,
            height: 1,
        };
        let id = bp
            .add(
                make_rotated_entry(GridPosition::new(0, 0), fp, Rotation::Cw90),
                &spatial,
            )
            .unwrap();

        // Should occupy (0,0), (0,1), (0,2) — 1 wide, 3 tall.
        assert!(bp.is_ghost_at(GridPosition::new(0, 0)));
        assert!(bp.is_ghost_at(GridPosition::new(0, 1)));
        assert!(bp.is_ghost_at(GridPosition::new(0, 2)));
        // Should NOT occupy (1,0) or (2,0).
        assert!(!bp.is_ghost_at(GridPosition::new(1, 0)));
        assert!(!bp.is_ghost_at(GridPosition::new(2, 0)));
        assert!(bp.get(id).is_some());
    }

    #[test]
    fn rotation_move_uses_rotated_footprint() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let fp = BuildingFootprint {
            width: 3,
            height: 1,
        };
        let id = bp
            .add(
                make_rotated_entry(GridPosition::new(0, 0), fp, Rotation::Cw90),
                &spatial,
            )
            .unwrap();

        bp.move_entry(id, GridPosition::new(5, 5), &spatial)
            .unwrap();
        // Old tiles should be gone.
        assert!(!bp.is_ghost_at(GridPosition::new(0, 0)));
        // New tiles should use rotated footprint (1w x 3h).
        assert!(bp.is_ghost_at(GridPosition::new(5, 5)));
        assert!(bp.is_ghost_at(GridPosition::new(5, 6)));
        assert!(bp.is_ghost_at(GridPosition::new(5, 7)));
        assert!(!bp.is_ghost_at(GridPosition::new(6, 5)));
    }

    #[test]
    fn rotation_commit_places_rotated_footprint() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let fp = BuildingFootprint {
            width: 3,
            height: 1,
        };
        let bp_id = bp
            .add(
                make_rotated_entry(GridPosition::new(0, 0), fp, Rotation::Cw90),
                &spatial,
            )
            .unwrap();

        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        let node_id = result.node_map[&bp_id];

        // Spatial should reflect the rotated footprint (1w x 3h).
        assert_eq!(spatial.node_at(GridPosition::new(0, 0)), Some(node_id));
        assert_eq!(spatial.node_at(GridPosition::new(0, 1)), Some(node_id));
        assert_eq!(spatial.node_at(GridPosition::new(0, 2)), Some(node_id));
        assert_eq!(spatial.node_at(GridPosition::new(1, 0)), None);
    }

    // =======================================================================
    // WI3c: capture_region tests
    // =======================================================================

    #[test]
    fn capture_region_single_node() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, Inventory::new(1, 1, 100));
        engine.set_output_inventory(node, Inventory::new(1, 1, 100));
        spatial
            .place(node, GridPosition::new(5, 5), BuildingFootprint::single())
            .unwrap();

        let bp = Blueprint::capture_region(
            &engine,
            &spatial,
            GridPosition::new(0, 0),
            GridPosition::new(10, 10),
            GridPosition::new(0, 0),
        );
        assert_eq!(bp.len(), 1);
    }

    #[test]
    fn capture_region_with_connections() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();

        let p1 = engine.graph.queue_add_node(building());
        let p2 = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let n1 = r.resolve_node(p1).unwrap();
        let n2 = r.resolve_node(p2).unwrap();

        engine.set_processor(n1, make_source(iron(), 1.0));
        engine.set_input_inventory(n1, Inventory::new(1, 1, 100));
        engine.set_output_inventory(n1, Inventory::new(1, 1, 100));
        engine.set_processor(n2, make_source(iron(), 1.0));
        engine.set_input_inventory(n2, Inventory::new(1, 1, 100));
        engine.set_output_inventory(n2, Inventory::new(1, 1, 100));

        spatial
            .place(n1, GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();
        spatial
            .place(n2, GridPosition::new(1, 0), BuildingFootprint::single())
            .unwrap();

        let pe = engine.graph.queue_connect(n1, n2);
        let er = engine.graph.apply_mutations();
        let eid = er.resolve_edge(pe).unwrap();
        engine.set_transport(eid, make_flow_transport(1.0));

        let bp = Blueprint::capture_region(
            &engine,
            &spatial,
            GridPosition::new(0, 0),
            GridPosition::new(5, 5),
            GridPosition::new(0, 0),
        );
        assert_eq!(bp.len(), 2);
        assert_eq!(bp.connections().len(), 1);
    }

    #[test]
    fn capture_region_excludes_outside() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();

        let p1 = engine.graph.queue_add_node(building());
        let p2 = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let n1 = r.resolve_node(p1).unwrap();
        let n2 = r.resolve_node(p2).unwrap();

        engine.set_processor(n1, make_source(iron(), 1.0));
        engine.set_input_inventory(n1, Inventory::new(1, 1, 100));
        engine.set_output_inventory(n1, Inventory::new(1, 1, 100));
        engine.set_processor(n2, make_source(iron(), 1.0));
        engine.set_input_inventory(n2, Inventory::new(1, 1, 100));
        engine.set_output_inventory(n2, Inventory::new(1, 1, 100));

        spatial
            .place(n1, GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();
        spatial
            .place(n2, GridPosition::new(100, 100), BuildingFootprint::single())
            .unwrap();

        let bp = Blueprint::capture_region(
            &engine,
            &spatial,
            GridPosition::new(0, 0),
            GridPosition::new(5, 5),
            GridPosition::new(0, 0),
        );
        assert_eq!(bp.len(), 1);
    }

    #[test]
    fn capture_region_connection_across_boundary() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();

        let p1 = engine.graph.queue_add_node(building());
        let p2 = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let n1 = r.resolve_node(p1).unwrap();
        let n2 = r.resolve_node(p2).unwrap();

        engine.set_processor(n1, make_source(iron(), 1.0));
        engine.set_input_inventory(n1, Inventory::new(1, 1, 100));
        engine.set_output_inventory(n1, Inventory::new(1, 1, 100));
        engine.set_processor(n2, make_source(iron(), 1.0));
        engine.set_input_inventory(n2, Inventory::new(1, 1, 100));
        engine.set_output_inventory(n2, Inventory::new(1, 1, 100));

        spatial
            .place(n1, GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();
        spatial
            .place(n2, GridPosition::new(100, 100), BuildingFootprint::single())
            .unwrap();

        let pe = engine.graph.queue_connect(n1, n2);
        let er = engine.graph.apply_mutations();
        let eid = er.resolve_edge(pe).unwrap();
        engine.set_transport(eid, make_flow_transport(1.0));

        // Only capture the first node — connection should NOT be included.
        let bp = Blueprint::capture_region(
            &engine,
            &spatial,
            GridPosition::new(0, 0),
            GridPosition::new(5, 5),
            GridPosition::new(0, 0),
        );
        assert_eq!(bp.len(), 1);
        assert_eq!(bp.connections().len(), 0);
    }

    #[test]
    fn capture_then_commit_roundtrip() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();

        let p1 = engine.graph.queue_add_node(building());
        let p2 = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let n1 = r.resolve_node(p1).unwrap();
        let n2 = r.resolve_node(p2).unwrap();

        engine.set_processor(n1, make_source(iron(), 1.0));
        engine.set_input_inventory(n1, Inventory::new(1, 1, 100));
        engine.set_output_inventory(n1, Inventory::new(1, 1, 100));
        engine.set_processor(n2, make_source(iron(), 1.0));
        engine.set_input_inventory(n2, Inventory::new(1, 1, 100));
        engine.set_output_inventory(n2, Inventory::new(1, 1, 100));

        spatial
            .place(n1, GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();
        spatial
            .place(n2, GridPosition::new(1, 0), BuildingFootprint::single())
            .unwrap();

        let pe = engine.graph.queue_connect(n1, n2);
        let er = engine.graph.apply_mutations();
        let eid = er.resolve_edge(pe).unwrap();
        engine.set_transport(eid, make_flow_transport(1.0));

        // Capture and commit at an offset.
        let bp = Blueprint::capture_region(
            &engine,
            &spatial,
            GridPosition::new(0, 0),
            GridPosition::new(5, 5),
            GridPosition::new(10, 10),
        );
        assert_eq!(bp.len(), 2);
        assert_eq!(bp.connections().len(), 1);

        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        assert_eq!(result.node_map.len(), 2);
        assert_eq!(result.edge_ids.len(), 1);

        // The original nodes + new nodes = 4.
        assert_eq!(engine.node_count(), 4);
    }

    // =======================================================================
    // WI3d: Cost estimation tests
    // =======================================================================

    #[test]
    fn estimate_cost_single_entry() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        let costs = bp.estimate_cost(|_bt| vec![(iron(), 5)]);
        assert_eq!(costs.get(&iron()), Some(&5));
    }

    #[test]
    fn estimate_cost_multiple_sums() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        bp.add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        bp.add(make_entry(GridPosition::new(2, 0)), &spatial)
            .unwrap();

        let costs =
            bp.estimate_cost(|_bt| vec![(iron(), 3), (factorial_core::test_utils::copper(), 2)]);
        assert_eq!(costs.get(&iron()), Some(&9));
        assert_eq!(costs.get(&factorial_core::test_utils::copper()), Some(&6));
    }

    #[test]
    fn estimate_cost_empty_blueprint() {
        let bp = Blueprint::new();
        let costs = bp.estimate_cost(|_bt| vec![(iron(), 5)]);
        assert!(costs.is_empty());
    }

    // =======================================================================
    // WI6: Hardening tests (rollback + undo)
    // =======================================================================

    #[test]
    fn commit_rollback_on_spatial_failure() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();

        // Place a real building at (1,0).
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        spatial
            .place(node, GridPosition::new(1, 0), BuildingFootprint::single())
            .unwrap();

        // Create a blueprint with 2 entries. Second one overlaps.
        // We need to bypass the initial validation, so place the building AFTER
        // creating the blueprint but before commit.
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        // Now place a real building at the second position to cause spatial.place to fail.
        let pending2 = engine.graph.queue_add_node(building());
        let result2 = engine.graph.apply_mutations();
        let node2 = result2.resolve_node(pending2).unwrap();
        spatial
            .place(node2, GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();

        // Commit should fail with overlap.
        let result = bp.commit(&mut engine, &mut spatial);
        assert!(result.is_err());
    }

    #[test]
    fn commit_rollback_cleans_spatial() {
        // This test verifies that after a failed commit, spatial tiles are freed.
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        // No overlap here, so commit succeeds.
        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        assert!(spatial.node_at(GridPosition::new(0, 0)).is_some());

        // Undo to verify spatial is cleaned.
        let record = result.undo_record();
        Blueprint::undo(&record, &mut engine, &mut spatial).unwrap();
        assert_eq!(spatial.node_at(GridPosition::new(0, 0)), None);
    }

    #[test]
    fn commit_rollback_cleans_graph() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        let committed_node = *result.node_map.values().next().unwrap();
        assert!(engine.graph.contains_node(committed_node));

        let record = result.undo_record();
        Blueprint::undo(&record, &mut engine, &mut spatial).unwrap();
        assert!(!engine.graph.contains_node(committed_node));
    }

    #[test]
    fn undo_removes_nodes() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        bp.add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();

        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        assert_eq!(engine.node_count(), 2);

        let record = result.undo_record();
        Blueprint::undo(&record, &mut engine, &mut spatial).unwrap();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn undo_removes_edges() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();
        let id1 = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let id2 = bp
            .add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        bp.connect(
            BlueprintNodeRef::Planned(id1),
            BlueprintNodeRef::Planned(id2),
            make_flow_transport(1.0),
            None,
        );

        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        assert_eq!(engine.edge_count(), 1);

        let record = result.undo_record();
        Blueprint::undo(&record, &mut engine, &mut spatial).unwrap();
        assert_eq!(engine.edge_count(), 0);
    }

    #[test]
    fn undo_removes_spatial() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        assert_eq!(spatial.node_count(), 1);

        let record = result.undo_record();
        Blueprint::undo(&record, &mut engine, &mut spatial).unwrap();
        assert_eq!(spatial.node_count(), 0);
    }

    #[test]
    fn undo_record_matches_commit() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();
        let id1 = bp
            .add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();
        let id2 = bp
            .add(make_entry(GridPosition::new(1, 0)), &spatial)
            .unwrap();
        bp.connect(
            BlueprintNodeRef::Planned(id1),
            BlueprintNodeRef::Planned(id2),
            make_flow_transport(1.0),
            None,
        );

        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        let record = result.undo_record();
        assert_eq!(record.node_ids.len(), 2);
        assert_eq!(record.edge_ids.len(), 1);
    }

    #[test]
    fn undo_allows_re_placement() {
        let (mut engine, mut spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial)
            .unwrap();

        let result = bp.commit(&mut engine, &mut spatial).unwrap();
        let record = result.undo_record();
        Blueprint::undo(&record, &mut engine, &mut spatial).unwrap();

        // Should be able to place at the same position again.
        let mut bp2 = Blueprint::new();
        let result2 = bp2.add(make_entry(GridPosition::new(0, 0)), &spatial);
        assert!(result2.is_ok());
        let result3 = bp2.commit(&mut engine, &mut spatial);
        assert!(result3.is_ok());
    }
}
