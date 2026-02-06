//! Spatial grid module for building placement, adjacency, and area queries.
//!
//! Provides a 2D grid-based spatial index that maps grid positions to
//! production graph nodes, supporting multi-tile buildings, adjacency
//! queries, and area searches.

use factorial_core::id::NodeId;
use serde::{Deserialize, Serialize};
use slotmap::{Key, SecondaryMap};
use std::collections::BTreeMap;

pub mod blueprint;
pub use blueprint::BlueprintCommitError;
#[cfg(feature = "blueprint-io")]
pub use blueprint::BlueprintIoError;
pub use blueprint::{
    Blueprint, BlueprintCommitResult, BlueprintConnection, BlueprintEntry, BlueprintEntryId,
    BlueprintError, BlueprintNodeRef, BlueprintUndoRecord,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A position on the 2D grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

impl GridPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Manhattan distance to another position.
    pub fn manhattan_distance(&self, other: &GridPosition) -> u32 {
        (self.x - other.x).unsigned_abs() + (self.y - other.y).unsigned_abs()
    }

    /// Chebyshev (chessboard) distance to another position.
    pub fn chebyshev_distance(&self, other: &GridPosition) -> u32 {
        (self.x - other.x)
            .unsigned_abs()
            .max((self.y - other.y).unsigned_abs())
    }
}

/// The footprint (size) of a building on the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildingFootprint {
    pub width: u32,
    pub height: u32,
}

impl BuildingFootprint {
    /// A 1x1 building.
    pub fn single() -> Self {
        Self {
            width: 1,
            height: 1,
        }
    }

    /// Return a new footprint rotated by the given rotation.
    /// For 90/270 degrees, width and height are swapped.
    pub fn rotated(&self, rotation: Rotation) -> Self {
        match rotation {
            Rotation::None | Rotation::Cw180 => *self,
            Rotation::Cw90 | Rotation::Cw270 => Self {
                width: self.height,
                height: self.width,
            },
        }
    }

    /// Iterate over all tiles occupied by this footprint at the given origin.
    /// Origin is the top-left corner.
    pub fn tiles(&self, origin: GridPosition) -> impl Iterator<Item = GridPosition> {
        let w = self.width as i32;
        let h = self.height as i32;
        let ox = origin.x;
        let oy = origin.y;
        (0..h).flat_map(move |dy| (0..w).map(move |dx| GridPosition::new(ox + dx, oy + dy)))
    }
}

/// Rotation applied to a building or blueprint entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Rotation {
    /// No rotation.
    #[default]
    None,
    /// 90 degrees clockwise.
    Cw90,
    /// 180 degrees.
    Cw180,
    /// 270 degrees clockwise (90 degrees counter-clockwise).
    Cw270,
}

impl Rotation {
    /// All four rotation values.
    pub fn all() -> [Rotation; 4] {
        [
            Rotation::None,
            Rotation::Cw90,
            Rotation::Cw180,
            Rotation::Cw270,
        ]
    }

    /// Rotate 90 degrees clockwise.
    pub fn rotate_cw(self) -> Self {
        match self {
            Rotation::None => Rotation::Cw90,
            Rotation::Cw90 => Rotation::Cw180,
            Rotation::Cw180 => Rotation::Cw270,
            Rotation::Cw270 => Rotation::None,
        }
    }

    /// Rotate 90 degrees counter-clockwise.
    pub fn rotate_ccw(self) -> Self {
        match self {
            Rotation::None => Rotation::Cw270,
            Rotation::Cw90 => Rotation::None,
            Rotation::Cw180 => Rotation::Cw90,
            Rotation::Cw270 => Rotation::Cw180,
        }
    }
}

/// Cardinal directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    /// All four cardinal directions.
    pub fn all() -> [Direction; 4] {
        [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
    }

    /// Offset for this direction.
    pub fn offset(&self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::East => (1, 0),
            Direction::South => (0, 1),
            Direction::West => (-1, 0),
        }
    }
}

/// Errors from spatial operations.
#[derive(Debug, thiserror::Error)]
pub enum SpatialError {
    #[error("position is occupied")]
    Occupied,
    #[error("node is not placed on the grid")]
    NotPlaced,
    #[error("node is already placed on the grid")]
    AlreadyPlaced,
}

// ---------------------------------------------------------------------------
// SpatialIndex
// ---------------------------------------------------------------------------

/// A spatial index mapping grid positions to production graph nodes.
///
/// Maintains a bidirectional mapping:
/// - `tiles`: position -> node (which node occupies each tile)
/// - `positions`: node -> origin position
/// - `footprints`: node -> building footprint
#[derive(Debug, Default)]
pub struct SpatialIndex {
    tiles: BTreeMap<GridPosition, NodeId>,
    positions: SecondaryMap<NodeId, GridPosition>,
    footprints: SecondaryMap<NodeId, BuildingFootprint>,
}

impl SpatialIndex {
    pub fn new() -> Self {
        Self::default()
    }

    // -- Placement --

    /// Place a building on the grid. Origin is the top-left corner.
    pub fn place(
        &mut self,
        node: NodeId,
        position: GridPosition,
        footprint: BuildingFootprint,
    ) -> Result<(), SpatialError> {
        // Check not already placed
        if self.positions.contains_key(node) {
            return Err(SpatialError::AlreadyPlaced);
        }

        // Check all tiles are free
        for tile in footprint.tiles(position) {
            if self.tiles.contains_key(&tile) {
                return Err(SpatialError::Occupied);
            }
        }

        // Place all tiles
        for tile in footprint.tiles(position) {
            self.tiles.insert(tile, node);
        }
        self.positions.insert(node, position);
        self.footprints.insert(node, footprint);

        Ok(())
    }

    /// Remove a building from the grid. Returns its origin position.
    pub fn remove(&mut self, node: NodeId) -> Result<GridPosition, SpatialError> {
        let position = *self.positions.get(node).ok_or(SpatialError::NotPlaced)?;
        let footprint = *self.footprints.get(node).ok_or(SpatialError::NotPlaced)?;

        for tile in footprint.tiles(position) {
            self.tiles.remove(&tile);
        }
        self.positions.remove(node);
        self.footprints.remove(node);

        Ok(position)
    }

    /// Check if a building can be placed at the given position.
    pub fn can_place(&self, position: GridPosition, footprint: BuildingFootprint) -> bool {
        footprint
            .tiles(position)
            .all(|tile| !self.tiles.contains_key(&tile))
    }

    // -- Point queries --

    /// Get the node at a grid position.
    pub fn node_at(&self, pos: GridPosition) -> Option<NodeId> {
        self.tiles.get(&pos).copied()
    }

    /// Get the origin position of a placed node.
    pub fn get_position(&self, node: NodeId) -> Option<GridPosition> {
        self.positions.get(node).copied()
    }

    /// Get the footprint of a placed node.
    pub fn get_footprint(&self, node: NodeId) -> Option<BuildingFootprint> {
        self.footprints.get(node).copied()
    }

    /// Check if a position is occupied.
    pub fn is_occupied(&self, pos: GridPosition) -> bool {
        self.tiles.contains_key(&pos)
    }

    // -- Area queries --

    /// Find all unique nodes within an axis-aligned rectangle (inclusive).
    pub fn nodes_in_rect(&self, min: GridPosition, max: GridPosition) -> Vec<NodeId> {
        let mut result_set = std::collections::BTreeSet::new();
        let mut result = Vec::new();

        for (&pos, &node) in self.tiles.range(min..=max) {
            if pos.y >= min.y && pos.y <= max.y && pos.x >= min.x && pos.x <= max.x {
                let key = node.data().as_ffi();
                if result_set.insert(key) {
                    result.push(node);
                }
            }
        }
        result
    }

    /// Find all unique nodes within a Manhattan distance radius.
    pub fn nodes_in_radius(&self, center: GridPosition, radius: u32) -> Vec<NodeId> {
        // Use bounding box + Manhattan distance filter
        let r = radius as i32;
        let min = GridPosition::new(center.x - r, center.y - r);
        let max = GridPosition::new(center.x + r, center.y + r);

        let mut result_set = std::collections::BTreeSet::new();
        let mut result = Vec::new();

        for (&pos, &node) in self.tiles.range(min..=max) {
            if pos.y >= min.y
                && pos.y <= max.y
                && pos.x >= min.x
                && pos.x <= max.x
                && center.manhattan_distance(&pos) <= radius
            {
                let key = node.data().as_ffi();
                if result_set.insert(key) {
                    result.push(node);
                }
            }
        }
        result
    }

    // -- Adjacency --

    /// Find 4-directional neighbors (unique nodes adjacent to any edge tile).
    pub fn neighbors_4(&self, node: NodeId) -> Vec<(Direction, NodeId)> {
        let Some(&origin) = self.positions.get(node) else {
            return Vec::new();
        };
        let Some(&footprint) = self.footprints.get(node) else {
            return Vec::new();
        };

        let own_tiles: std::collections::BTreeSet<GridPosition> = footprint.tiles(origin).collect();
        let mut result = Vec::new();
        let mut seen = std::collections::BTreeSet::new();

        for dir in Direction::all() {
            let (dx, dy) = dir.offset();
            for tile in &own_tiles {
                let neighbor_pos = GridPosition::new(tile.x + dx, tile.y + dy);
                if own_tiles.contains(&neighbor_pos) {
                    continue; // skip own tiles
                }
                if let Some(&neighbor_node) = self.tiles.get(&neighbor_pos) {
                    let key = (dir as u8, neighbor_node.data().as_ffi());
                    if seen.insert(key) {
                        result.push((dir, neighbor_node));
                    }
                }
            }
        }
        result
    }

    /// Find 8-directional neighbors (including diagonals).
    pub fn neighbors_8(&self, node: NodeId) -> Vec<NodeId> {
        let Some(&origin) = self.positions.get(node) else {
            return Vec::new();
        };
        let Some(&footprint) = self.footprints.get(node) else {
            return Vec::new();
        };

        let own_tiles: std::collections::BTreeSet<GridPosition> = footprint.tiles(origin).collect();
        let mut result_set = std::collections::BTreeSet::new();
        let mut result = Vec::new();

        let offsets = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];

        for tile in &own_tiles {
            for (dx, dy) in offsets {
                let neighbor_pos = GridPosition::new(tile.x + dx, tile.y + dy);
                if own_tiles.contains(&neighbor_pos) {
                    continue;
                }
                if let Some(&neighbor_node) = self.tiles.get(&neighbor_pos) {
                    let key = neighbor_node.data().as_ffi();
                    if result_set.insert(key) {
                        result.push(neighbor_node);
                    }
                }
            }
        }
        result
    }

    /// Find the neighbor in a specific direction, if any.
    pub fn neighbor_in_direction(&self, node: NodeId, dir: Direction) -> Option<NodeId> {
        let origin = self.positions.get(node)?;
        let footprint = self.footprints.get(node)?;
        let (dx, dy) = dir.offset();

        let own_tiles: std::collections::BTreeSet<GridPosition> =
            footprint.tiles(*origin).collect();

        for tile in &own_tiles {
            let neighbor_pos = GridPosition::new(tile.x + dx, tile.y + dy);
            if own_tiles.contains(&neighbor_pos) {
                continue;
            }
            if let Some(&neighbor_node) = self.tiles.get(&neighbor_pos) {
                return Some(neighbor_node);
            }
        }
        None
    }

    // -- Stats --

    /// Number of unique nodes placed on the grid.
    pub fn node_count(&self) -> usize {
        self.positions.len()
    }

    /// Total number of occupied tiles.
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    // Helper to create NodeIds (need a SlotMap since NodeId is a slotmap key)
    fn make_nodes(count: usize) -> (SlotMap<NodeId, ()>, Vec<NodeId>) {
        let mut sm = SlotMap::with_key();
        let ids: Vec<NodeId> = (0..count).map(|_| sm.insert(())).collect();
        (sm, ids)
    }

    // -----------------------------------------------------------------------
    // GridPosition tests
    // -----------------------------------------------------------------------

    #[test]
    fn grid_position_new() {
        let pos = GridPosition::new(3, -7);
        assert_eq!(pos.x, 3);
        assert_eq!(pos.y, -7);
    }

    #[test]
    fn grid_position_manhattan_distance() {
        let a = GridPosition::new(0, 0);
        let b = GridPosition::new(3, 4);
        assert_eq!(a.manhattan_distance(&b), 7);

        // With negatives
        let c = GridPosition::new(-2, 5);
        let d = GridPosition::new(3, -1);
        assert_eq!(c.manhattan_distance(&d), 11);

        // Same position
        assert_eq!(a.manhattan_distance(&a), 0);
    }

    #[test]
    fn grid_position_chebyshev_distance() {
        let a = GridPosition::new(0, 0);
        let b = GridPosition::new(3, 4);
        assert_eq!(a.chebyshev_distance(&b), 4);

        let c = GridPosition::new(-2, 5);
        let d = GridPosition::new(3, -1);
        assert_eq!(c.chebyshev_distance(&d), 6);

        // Same position
        assert_eq!(a.chebyshev_distance(&a), 0);
    }

    // -----------------------------------------------------------------------
    // BuildingFootprint tests
    // -----------------------------------------------------------------------

    #[test]
    fn footprint_single() {
        let fp = BuildingFootprint::single();
        assert_eq!(fp.width, 1);
        assert_eq!(fp.height, 1);

        let tiles: Vec<_> = fp.tiles(GridPosition::new(5, 10)).collect();
        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0], GridPosition::new(5, 10));
    }

    #[test]
    fn footprint_tiles_iteration() {
        let fp = BuildingFootprint {
            width: 2,
            height: 3,
        };
        let origin = GridPosition::new(10, 20);
        let tiles: Vec<_> = fp.tiles(origin).collect();

        assert_eq!(tiles.len(), 6);
        // Row 0 (y=20)
        assert!(tiles.contains(&GridPosition::new(10, 20)));
        assert!(tiles.contains(&GridPosition::new(11, 20)));
        // Row 1 (y=21)
        assert!(tiles.contains(&GridPosition::new(10, 21)));
        assert!(tiles.contains(&GridPosition::new(11, 21)));
        // Row 2 (y=22)
        assert!(tiles.contains(&GridPosition::new(10, 22)));
        assert!(tiles.contains(&GridPosition::new(11, 22)));
    }

    // -----------------------------------------------------------------------
    // Placement tests
    // -----------------------------------------------------------------------

    #[test]
    fn place_1x1() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();
        let pos = GridPosition::new(0, 0);

        index
            .place(ids[0], pos, BuildingFootprint::single())
            .unwrap();

        assert_eq!(index.node_at(pos), Some(ids[0]));
    }

    #[test]
    fn place_2x2() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();
        let fp = BuildingFootprint {
            width: 2,
            height: 2,
        };
        let origin = GridPosition::new(5, 5);

        index.place(ids[0], origin, fp).unwrap();

        // All 4 tiles should be occupied by the same node
        assert_eq!(index.node_at(GridPosition::new(5, 5)), Some(ids[0]));
        assert_eq!(index.node_at(GridPosition::new(6, 5)), Some(ids[0]));
        assert_eq!(index.node_at(GridPosition::new(5, 6)), Some(ids[0]));
        assert_eq!(index.node_at(GridPosition::new(6, 6)), Some(ids[0]));

        // Adjacent tile is empty
        assert_eq!(index.node_at(GridPosition::new(7, 5)), None);
    }

    #[test]
    fn place_occupied_error() {
        let (_sm, ids) = make_nodes(2);
        let mut index = SpatialIndex::new();
        let pos = GridPosition::new(0, 0);

        index
            .place(ids[0], pos, BuildingFootprint::single())
            .unwrap();

        let result = index.place(ids[1], pos, BuildingFootprint::single());
        assert!(matches!(result, Err(SpatialError::Occupied)));
    }

    #[test]
    fn place_partial_overlap_error() {
        let (_sm, ids) = make_nodes(2);
        let mut index = SpatialIndex::new();

        // Place 1x1 at (1, 1)
        index
            .place(ids[0], GridPosition::new(1, 1), BuildingFootprint::single())
            .unwrap();

        // Try to place 2x2 at (0, 0) -- its (1,1) tile overlaps
        let fp = BuildingFootprint {
            width: 2,
            height: 2,
        };
        let result = index.place(ids[1], GridPosition::new(0, 0), fp);
        assert!(matches!(result, Err(SpatialError::Occupied)));
    }

    #[test]
    fn remove_and_reuse() {
        let (_sm, ids) = make_nodes(2);
        let mut index = SpatialIndex::new();
        let pos = GridPosition::new(3, 3);
        let fp = BuildingFootprint {
            width: 2,
            height: 2,
        };

        index.place(ids[0], pos, fp).unwrap();
        let removed_pos = index.remove(ids[0]).unwrap();
        assert_eq!(removed_pos, pos);

        // All tiles should be free now
        assert!(!index.is_occupied(GridPosition::new(3, 3)));
        assert!(!index.is_occupied(GridPosition::new(4, 3)));
        assert!(!index.is_occupied(GridPosition::new(3, 4)));
        assert!(!index.is_occupied(GridPosition::new(4, 4)));

        // Can place another building in the same spot
        index.place(ids[1], pos, fp).unwrap();
        assert_eq!(index.node_at(pos), Some(ids[1]));
    }

    #[test]
    fn can_place_checks_all_tiles() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();
        let fp = BuildingFootprint {
            width: 2,
            height: 2,
        };

        // Empty grid: can place anywhere
        assert!(index.can_place(GridPosition::new(0, 0), fp));

        // Place at (0,0)
        index.place(ids[0], GridPosition::new(0, 0), fp).unwrap();

        // Can't overlap
        assert!(!index.can_place(GridPosition::new(0, 0), fp));
        assert!(!index.can_place(GridPosition::new(1, 0), fp)); // partial overlap at (1,0)
        assert!(!index.can_place(GridPosition::new(0, 1), fp)); // partial overlap at (0,1)
        assert!(!index.can_place(GridPosition::new(1, 1), fp)); // partial overlap at (1,1)

        // Can place adjacent (no overlap)
        assert!(index.can_place(GridPosition::new(2, 0), fp));
        assert!(index.can_place(GridPosition::new(0, 2), fp));
    }

    // -----------------------------------------------------------------------
    // Point query tests
    // -----------------------------------------------------------------------

    #[test]
    fn node_at_placed() {
        let (_sm, ids) = make_nodes(2);
        let mut index = SpatialIndex::new();

        index
            .place(ids[0], GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();
        index
            .place(ids[1], GridPosition::new(1, 0), BuildingFootprint::single())
            .unwrap();

        assert_eq!(index.node_at(GridPosition::new(0, 0)), Some(ids[0]));
        assert_eq!(index.node_at(GridPosition::new(1, 0)), Some(ids[1]));
    }

    #[test]
    fn node_at_empty() {
        let index = SpatialIndex::new();
        assert_eq!(index.node_at(GridPosition::new(0, 0)), None);
        assert_eq!(index.node_at(GridPosition::new(999, -999)), None);
    }

    #[test]
    fn get_position_and_footprint() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();
        let pos = GridPosition::new(7, 3);
        let fp = BuildingFootprint {
            width: 3,
            height: 2,
        };

        index.place(ids[0], pos, fp).unwrap();

        assert_eq!(index.get_position(ids[0]), Some(pos));
        assert_eq!(index.get_footprint(ids[0]), Some(fp));
    }

    #[test]
    fn is_occupied() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();

        assert!(!index.is_occupied(GridPosition::new(0, 0)));

        index
            .place(ids[0], GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();

        assert!(index.is_occupied(GridPosition::new(0, 0)));
        assert!(!index.is_occupied(GridPosition::new(1, 0)));
    }

    // -----------------------------------------------------------------------
    // Area query tests
    // -----------------------------------------------------------------------

    #[test]
    fn rect_includes_nodes_in_range() {
        let (_sm, ids) = make_nodes(3);
        let mut index = SpatialIndex::new();

        index
            .place(ids[0], GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();
        index
            .place(ids[1], GridPosition::new(2, 2), BuildingFootprint::single())
            .unwrap();
        index
            .place(ids[2], GridPosition::new(5, 5), BuildingFootprint::single())
            .unwrap();

        let result = index.nodes_in_rect(GridPosition::new(0, 0), GridPosition::new(3, 3));

        assert!(result.contains(&ids[0]));
        assert!(result.contains(&ids[1]));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn rect_excludes_nodes_outside() {
        let (_sm, ids) = make_nodes(2);
        let mut index = SpatialIndex::new();

        index
            .place(ids[0], GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();
        index
            .place(
                ids[1],
                GridPosition::new(10, 10),
                BuildingFootprint::single(),
            )
            .unwrap();

        let result = index.nodes_in_rect(GridPosition::new(0, 0), GridPosition::new(5, 5));

        assert!(result.contains(&ids[0]));
        assert!(!result.contains(&ids[1]));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn rect_deduplicates_multi_tile() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();
        let fp = BuildingFootprint {
            width: 2,
            height: 2,
        };

        index.place(ids[0], GridPosition::new(0, 0), fp).unwrap();

        // Rect covers all 4 tiles of the 2x2 building
        let result = index.nodes_in_rect(GridPosition::new(0, 0), GridPosition::new(1, 1));

        // Should only appear once
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ids[0]);
    }

    #[test]
    fn radius_includes_excludes() {
        let (_sm, ids) = make_nodes(3);
        let mut index = SpatialIndex::new();

        // Center at (5, 5)
        index
            .place(ids[0], GridPosition::new(5, 5), BuildingFootprint::single())
            .unwrap();
        // Manhattan distance 2 from center
        index
            .place(ids[1], GridPosition::new(6, 6), BuildingFootprint::single())
            .unwrap();
        // Manhattan distance 6 from center -- outside radius 3
        index
            .place(ids[2], GridPosition::new(8, 8), BuildingFootprint::single())
            .unwrap();

        let result = index.nodes_in_radius(GridPosition::new(5, 5), 3);

        assert!(result.contains(&ids[0]));
        assert!(result.contains(&ids[1]));
        assert!(!result.contains(&ids[2]));
        assert_eq!(result.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Adjacency tests
    // -----------------------------------------------------------------------

    #[test]
    fn neighbors_4_single_tile() {
        let (_sm, ids) = make_nodes(5);
        let mut index = SpatialIndex::new();

        // Center
        index
            .place(ids[0], GridPosition::new(5, 5), BuildingFootprint::single())
            .unwrap();
        // North
        index
            .place(ids[1], GridPosition::new(5, 4), BuildingFootprint::single())
            .unwrap();
        // East
        index
            .place(ids[2], GridPosition::new(6, 5), BuildingFootprint::single())
            .unwrap();
        // South
        index
            .place(ids[3], GridPosition::new(5, 6), BuildingFootprint::single())
            .unwrap();
        // West
        index
            .place(ids[4], GridPosition::new(4, 5), BuildingFootprint::single())
            .unwrap();

        let neighbors = index.neighbors_4(ids[0]);
        assert_eq!(neighbors.len(), 4);

        let neighbor_nodes: Vec<NodeId> = neighbors.iter().map(|(_, n)| *n).collect();
        assert!(neighbor_nodes.contains(&ids[1]));
        assert!(neighbor_nodes.contains(&ids[2]));
        assert!(neighbor_nodes.contains(&ids[3]));
        assert!(neighbor_nodes.contains(&ids[4]));

        // Check directions
        assert!(neighbors.contains(&(Direction::North, ids[1])));
        assert!(neighbors.contains(&(Direction::East, ids[2])));
        assert!(neighbors.contains(&(Direction::South, ids[3])));
        assert!(neighbors.contains(&(Direction::West, ids[4])));
    }

    #[test]
    fn neighbors_4_multi_tile() {
        let (_sm, ids) = make_nodes(5);
        let mut index = SpatialIndex::new();
        let fp = BuildingFootprint {
            width: 2,
            height: 2,
        };

        // 2x2 building at (5,5), occupying (5,5),(6,5),(5,6),(6,6)
        index.place(ids[0], GridPosition::new(5, 5), fp).unwrap();

        // North neighbor at (5,4) -- touches top edge
        index
            .place(ids[1], GridPosition::new(5, 4), BuildingFootprint::single())
            .unwrap();
        // East neighbor at (7,5) -- touches right edge
        index
            .place(ids[2], GridPosition::new(7, 5), BuildingFootprint::single())
            .unwrap();
        // South neighbor at (6,7) -- touches bottom edge
        index
            .place(ids[3], GridPosition::new(6, 7), BuildingFootprint::single())
            .unwrap();
        // West neighbor at (4,6) -- touches left edge
        index
            .place(ids[4], GridPosition::new(4, 6), BuildingFootprint::single())
            .unwrap();

        let neighbors = index.neighbors_4(ids[0]);

        let neighbor_nodes: Vec<NodeId> = neighbors.iter().map(|(_, n)| *n).collect();
        assert!(neighbor_nodes.contains(&ids[1])); // North
        assert!(neighbor_nodes.contains(&ids[2])); // East
        assert!(neighbor_nodes.contains(&ids[3])); // South
        assert!(neighbor_nodes.contains(&ids[4])); // West
    }

    #[test]
    fn neighbors_4_empty() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();

        // Single isolated building
        index
            .place(ids[0], GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();

        let neighbors = index.neighbors_4(ids[0]);
        assert!(neighbors.is_empty());
    }

    #[test]
    fn neighbors_8_with_diagonals() {
        let (_sm, ids) = make_nodes(3);
        let mut index = SpatialIndex::new();

        // Center
        index
            .place(ids[0], GridPosition::new(5, 5), BuildingFootprint::single())
            .unwrap();
        // Diagonal (NE)
        index
            .place(ids[1], GridPosition::new(6, 4), BuildingFootprint::single())
            .unwrap();
        // Cardinal (South)
        index
            .place(ids[2], GridPosition::new(5, 6), BuildingFootprint::single())
            .unwrap();

        let neighbors = index.neighbors_8(ids[0]);

        assert!(neighbors.contains(&ids[1])); // diagonal
        assert!(neighbors.contains(&ids[2])); // cardinal
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn neighbor_in_direction() {
        let (_sm, ids) = make_nodes(2);
        let mut index = SpatialIndex::new();

        index
            .place(ids[0], GridPosition::new(5, 5), BuildingFootprint::single())
            .unwrap();
        index
            .place(ids[1], GridPosition::new(6, 5), BuildingFootprint::single())
            .unwrap();

        assert_eq!(
            index.neighbor_in_direction(ids[0], Direction::East),
            Some(ids[1])
        );
        assert_eq!(index.neighbor_in_direction(ids[0], Direction::West), None);
        assert_eq!(index.neighbor_in_direction(ids[0], Direction::North), None);
        assert_eq!(index.neighbor_in_direction(ids[0], Direction::South), None);
    }

    #[test]
    fn already_placed_error() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();

        index
            .place(ids[0], GridPosition::new(0, 0), BuildingFootprint::single())
            .unwrap();

        // Try to place same node again at different position
        let result = index.place(ids[0], GridPosition::new(5, 5), BuildingFootprint::single());
        assert!(matches!(result, Err(SpatialError::AlreadyPlaced)));
    }

    // -----------------------------------------------------------------------
    // Statistics tests
    // -----------------------------------------------------------------------

    #[test]
    fn node_count_tile_count_after_place() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();
        let fp = BuildingFootprint {
            width: 2,
            height: 2,
        };

        index.place(ids[0], GridPosition::new(0, 0), fp).unwrap();

        assert_eq!(index.node_count(), 1);
        assert_eq!(index.tile_count(), 4);
    }

    #[test]
    fn counts_after_remove() {
        let (_sm, ids) = make_nodes(1);
        let mut index = SpatialIndex::new();
        let fp = BuildingFootprint {
            width: 2,
            height: 2,
        };

        index.place(ids[0], GridPosition::new(0, 0), fp).unwrap();
        index.remove(ids[0]).unwrap();

        assert_eq!(index.node_count(), 0);
        assert_eq!(index.tile_count(), 0);
    }

    #[test]
    fn empty_index_stats() {
        let index = SpatialIndex::new();
        assert_eq!(index.node_count(), 0);
        assert_eq!(index.tile_count(), 0);
    }
}
