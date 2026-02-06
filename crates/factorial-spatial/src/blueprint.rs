//! Blueprint / ghost placement system for previewing and staging building
//! placement before committing to the simulation.

use crate::{BuildingFootprint, GridPosition, SpatialIndex};
use factorial_core::engine::Engine;
use factorial_core::id::{BuildingTypeId, EdgeId, ItemTypeId, NodeId};
use factorial_core::item::Inventory;
use factorial_core::processor::Processor;
use factorial_core::transport::Transport;
use serde::{Deserialize, Serialize};
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
}

/// Result of committing a blueprint to the engine.
#[derive(Debug)]
pub struct BlueprintCommitResult {
    /// Maps blueprint entry IDs to their assigned NodeIds.
    pub node_map: BTreeMap<BlueprintEntryId, NodeId>,
    /// Edge IDs created for connections.
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

        // Rebuild ghost_tiles from entries.
        let mut ghost_tiles = BTreeMap::new();
        for (&id, entry) in &data.entries {
            for tile in entry.footprint.tiles(entry.position) {
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
        // Check overlap with real buildings.
        for tile in entry.footprint.tiles(entry.position) {
            if spatial.is_occupied(tile) {
                return Err(BlueprintError::OverlapsExisting);
            }
        }
        // Check overlap with ghost tiles.
        for tile in entry.footprint.tiles(entry.position) {
            if self.ghost_tiles.contains_key(&tile) {
                return Err(BlueprintError::OverlapsPlanned);
            }
        }

        let id = BlueprintEntryId(self.next_id);
        self.next_id += 1;

        // Insert ghost tiles.
        for tile in entry.footprint.tiles(entry.position) {
            self.ghost_tiles.insert(tile, id);
        }

        self.entries.insert(id, entry);
        Ok(id)
    }

    /// Remove an entry from the blueprint, clearing its ghost tiles
    /// and any connections referencing it.
    pub fn remove(&mut self, id: BlueprintEntryId) -> Result<(), BlueprintError> {
        let entry = self.entries.remove(&id).ok_or(BlueprintError::EntryNotFound)?;

        // Remove ghost tiles.
        for tile in entry.footprint.tiles(entry.position) {
            self.ghost_tiles.remove(&tile);
        }

        // Remove connections that reference this entry.
        let planned_ref = BlueprintNodeRef::Planned(id);
        self.connections.retain(|c| c.from != planned_ref && c.to != planned_ref);

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
        let footprint = entry.footprint;

        // Remove old ghost tiles.
        for tile in footprint.tiles(old_pos) {
            self.ghost_tiles.remove(&tile);
        }

        // Check overlap with real buildings.
        for tile in footprint.tiles(new_pos) {
            if spatial.is_occupied(tile) {
                // Rollback: re-insert old ghost tiles.
                for tile in footprint.tiles(old_pos) {
                    self.ghost_tiles.insert(tile, id);
                }
                return Err(BlueprintError::OverlapsExisting);
            }
        }
        // Check overlap with other ghost tiles.
        for tile in footprint.tiles(new_pos) {
            if self.ghost_tiles.contains_key(&tile) {
                // Rollback.
                for tile in footprint.tiles(old_pos) {
                    self.ghost_tiles.insert(tile, id);
                }
                return Err(BlueprintError::OverlapsPlanned);
            }
        }

        // Insert new ghost tiles.
        for tile in footprint.tiles(new_pos) {
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
        for (_id, entry) in &self.entries {
            for tile in entry.footprint.tiles(entry.position) {
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
    pub fn commit(
        self,
        engine: &mut Engine,
        spatial: &mut SpatialIndex,
    ) -> Result<BlueprintCommitResult, BlueprintError> {
        if self.entries.is_empty() {
            return Err(BlueprintError::Empty);
        }

        // Validate no overlaps with real buildings.
        for (_id, entry) in &self.entries {
            for tile in entry.footprint.tiles(entry.position) {
                if spatial.is_occupied(tile) {
                    return Err(BlueprintError::OverlapsExisting);
                }
            }
        }

        // Validate Existing node refs.
        for conn in &self.connections {
            if let BlueprintNodeRef::Existing(node_id) = conn.from {
                if !engine.graph.contains_node(node_id) {
                    return Err(BlueprintError::NodeNotFound);
                }
            }
            if let BlueprintNodeRef::Existing(node_id) = conn.to {
                if !engine.graph.contains_node(node_id) {
                    return Err(BlueprintError::NodeNotFound);
                }
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
            let node_id = mutation_result.resolve_node(*pending)
                .ok_or(BlueprintError::NodeNotFound)?;
            node_map.insert(id, node_id);
        }

        // Place in spatial index and set up processors/inventories.
        for (&id, entry) in &self.entries {
            let node_id = node_map[&id];
            spatial.place(node_id, entry.position, entry.footprint)
                .map_err(|_| BlueprintError::OverlapsExisting)?;
            engine.set_processor(node_id, entry.processor.clone());
            engine.set_input_inventory(node_id, Inventory::new(1, 1, entry.input_capacity));
            engine.set_output_inventory(node_id, Inventory::new(1, 1, entry.output_capacity));
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

            let pending = engine.graph.queue_connect_filtered(from_node, to_node, conn.item_filter);
            pending_edges.push((pending, conn.transport.clone()));
        }

        // Apply edge mutations.
        let edge_result = engine.graph.apply_mutations();
        let mut edge_ids = Vec::new();
        for (pending, transport) in pending_edges {
            let edge_id = edge_result.resolve_edge(pending)
                .ok_or(BlueprintError::NodeNotFound)?;
            engine.set_transport(edge_id, transport);
            edge_ids.push(edge_id);
        }

        Ok(BlueprintCommitResult { node_map, edge_ids })
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
    use factorial_core::sim::SimulationStrategy;
    use factorial_core::test_utils::{building, iron, make_flow_transport, make_source};

    fn make_entry(pos: GridPosition) -> BlueprintEntry {
        BlueprintEntry {
            building_type: building(),
            position: pos,
            footprint: BuildingFootprint::single(),
            processor: make_source(iron(), 1.0),
            input_capacity: 100,
            output_capacity: 100,
        }
    }

    fn make_entry_with_footprint(pos: GridPosition, footprint: BuildingFootprint) -> BlueprintEntry {
        BlueprintEntry {
            building_type: building(),
            position: pos,
            footprint,
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

        let id = bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();

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
        spatial.place(node, GridPosition::new(5, 5), BuildingFootprint::single()).unwrap();

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

        bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();
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

        let fp = BuildingFootprint { width: 3, height: 2 };
        let id = bp.add(make_entry_with_footprint(GridPosition::new(0, 0), fp), &spatial).unwrap();

        // Should occupy 6 tiles.
        for x in 0..3 {
            for y in 0..2 {
                assert!(bp.is_ghost_at(GridPosition::new(x, y)), "ghost at ({x},{y})");
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

        let id = bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();
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

        let id1 = bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();
        let id2 = bp.add(make_entry(GridPosition::new(1, 0)), &spatial).unwrap();
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

        let id = bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();
        bp.move_entry(id, GridPosition::new(5, 5), &spatial).unwrap();

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
        spatial.place(node, GridPosition::new(5, 5), BuildingFootprint::single()).unwrap();

        let id = bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();
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

        let bp_id = bp.add(make_entry(GridPosition::new(3, 3)), &spatial).unwrap();
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

        let id1 = bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();
        let id2 = bp.add(make_entry(GridPosition::new(1, 0)), &spatial).unwrap();
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
        spatial.place(existing_node, GridPosition::new(0, 0), BuildingFootprint::single()).unwrap();

        let mut bp = Blueprint::new();
        let bp_id = bp.add(make_entry(GridPosition::new(1, 0)), &spatial).unwrap();
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
        bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();

        // Now place a real building at (0,0), making the blueprint stale.
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        spatial.place(node, GridPosition::new(0, 0), BuildingFootprint::single()).unwrap();

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
        spatial.place(node, GridPosition::new(0, 0), BuildingFootprint::single()).unwrap();

        // Ghost at (1,0).
        bp.add(make_entry(GridPosition::new(1, 0)), &spatial).unwrap();

        // Can't place at (0,0) (real) or (1,0) (ghost).
        assert!(!bp.can_place(GridPosition::new(0, 0), BuildingFootprint::single(), &spatial));
        assert!(!bp.can_place(GridPosition::new(1, 0), BuildingFootprint::single(), &spatial));
        // Can place at (2,0).
        assert!(bp.can_place(GridPosition::new(2, 0), BuildingFootprint::single(), &spatial));
    }

    // -----------------------------------------------------------------------
    // Test 15: clear_resets_all
    // -----------------------------------------------------------------------
    #[test]
    fn clear_resets_all() {
        let (_engine, spatial) = setup_engine_and_spatial();
        let mut bp = Blueprint::new();

        let id1 = bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();
        let id2 = bp.add(make_entry(GridPosition::new(1, 0)), &spatial).unwrap();
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

        let id1 = bp.add(make_entry(GridPosition::new(0, 0)), &spatial).unwrap();
        let id2 = bp.add(make_entry(GridPosition::new(1, 0)), &spatial).unwrap();
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
}
