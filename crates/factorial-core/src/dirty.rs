use crate::id::{EdgeId, NodeId};
use std::collections::BTreeSet;

/// Tracks which nodes and edges have been modified since the last clean point.
///
/// Used by the engine to skip unnecessary work (re-sorting, serialization, etc.)
/// when nothing has changed. Call [`mark_clean`](DirtyTracker::mark_clean) at the
/// end of a tick to reset all flags.
#[derive(Debug, Clone, Default)]
pub struct DirtyTracker {
    dirty_nodes: BTreeSet<NodeId>,
    dirty_edges: BTreeSet<EdgeId>,
    graph_dirty: bool,
    any_dirty: bool,
    dirty_partitions: [bool; 5],
}

impl DirtyTracker {
    pub const PARTITION_GRAPH: usize = 0;
    pub const PARTITION_PROCESSORS: usize = 1;
    pub const PARTITION_INVENTORIES: usize = 2;
    pub const PARTITION_TRANSPORTS: usize = 3;
    pub const PARTITION_JUNCTIONS: usize = 4;
    pub const PARTITION_COUNT: usize = 5;

    /// Create a new tracker with nothing dirty.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_partition(&mut self, idx: usize) {
        self.dirty_partitions[idx] = true;
    }

    pub fn dirty_partitions(&self) -> &[bool; 5] {
        &self.dirty_partitions
    }

    pub fn any_partition_dirty(&self) -> bool {
        self.dirty_partitions.iter().any(|&d| d)
    }

    pub fn clear_partitions(&mut self) {
        self.dirty_partitions = [false; 5];
    }

    pub fn mark_all_partitions(&mut self) {
        self.dirty_partitions = [true; 5];
    }

    /// Mark a single node as dirty (e.g. processor or inventory changed).
    pub fn mark_node(&mut self, node: NodeId) {
        self.dirty_nodes.insert(node);
        self.any_dirty = true;
    }

    /// Mark a single edge as dirty (e.g. transport state changed).
    pub fn mark_edge(&mut self, edge: EdgeId) {
        self.dirty_edges.insert(edge);
        self.any_dirty = true;
    }

    /// Mark the graph topology as dirty (node/edge added or removed).
    pub fn mark_graph(&mut self) {
        self.graph_dirty = true;
        self.any_dirty = true;
    }

    /// Returns `true` if anything has been marked dirty since the last clean.
    pub fn is_dirty(&self) -> bool {
        self.any_dirty
    }

    /// Returns `true` if the given node has been marked dirty.
    pub fn is_node_dirty(&self, node: NodeId) -> bool {
        self.dirty_nodes.contains(&node)
    }

    /// Returns `true` if the given edge has been marked dirty.
    pub fn is_edge_dirty(&self, edge: EdgeId) -> bool {
        self.dirty_edges.contains(&edge)
    }

    /// Returns `true` if the graph topology has been marked dirty.
    pub fn is_graph_dirty(&self) -> bool {
        self.graph_dirty
    }

    /// Returns the set of all dirty node IDs.
    pub fn dirty_nodes(&self) -> &BTreeSet<NodeId> {
        &self.dirty_nodes
    }

    /// Returns the set of all dirty edge IDs.
    pub fn dirty_edges(&self) -> &BTreeSet<EdgeId> {
        &self.dirty_edges
    }

    /// Reset all dirty flags, marking everything as clean.
    pub fn mark_clean(&mut self) {
        self.dirty_nodes.clear();
        self.dirty_edges.clear();
        self.graph_dirty = false;
        self.any_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    // Helper: create a SlotMap and insert keys for NodeId.
    fn make_node_ids(count: usize) -> (SlotMap<NodeId, ()>, Vec<NodeId>) {
        let mut sm: SlotMap<NodeId, ()> = SlotMap::with_key();
        let ids: Vec<NodeId> = (0..count).map(|_| sm.insert(())).collect();
        (sm, ids)
    }

    // Helper: create a SlotMap and insert keys for EdgeId.
    fn make_edge_ids(count: usize) -> (SlotMap<EdgeId, ()>, Vec<EdgeId>) {
        let mut sm: SlotMap<EdgeId, ()> = SlotMap::with_key();
        let ids: Vec<EdgeId> = (0..count).map(|_| sm.insert(())).collect();
        (sm, ids)
    }

    #[test]
    fn tracker_initially_clean() {
        let tracker = DirtyTracker::new();
        assert!(!tracker.is_dirty());
        assert!(!tracker.is_graph_dirty());
        assert!(tracker.dirty_nodes().is_empty());
        assert!(tracker.dirty_edges().is_empty());
    }

    #[test]
    fn mark_node_makes_dirty() {
        let (_sm, ids) = make_node_ids(1);
        let mut tracker = DirtyTracker::new();

        tracker.mark_node(ids[0]);

        assert!(tracker.is_dirty());
        assert!(tracker.is_node_dirty(ids[0]));
    }

    #[test]
    fn mark_edge_makes_dirty() {
        let (_sm, ids) = make_edge_ids(1);
        let mut tracker = DirtyTracker::new();

        tracker.mark_edge(ids[0]);

        assert!(tracker.is_dirty());
        assert!(tracker.is_edge_dirty(ids[0]));
    }

    #[test]
    fn mark_graph_makes_dirty() {
        let mut tracker = DirtyTracker::new();

        tracker.mark_graph();

        assert!(tracker.is_dirty());
        assert!(tracker.is_graph_dirty());
    }

    #[test]
    fn mark_clean_resets_all() {
        let (_nsm, nids) = make_node_ids(2);
        let (_esm, eids) = make_edge_ids(1);
        let mut tracker = DirtyTracker::new();

        tracker.mark_node(nids[0]);
        tracker.mark_node(nids[1]);
        tracker.mark_edge(eids[0]);
        tracker.mark_graph();
        assert!(tracker.is_dirty());

        tracker.mark_clean();

        assert!(!tracker.is_dirty());
        assert!(!tracker.is_node_dirty(nids[0]));
        assert!(!tracker.is_node_dirty(nids[1]));
        assert!(!tracker.is_edge_dirty(eids[0]));
        assert!(!tracker.is_graph_dirty());
        assert!(tracker.dirty_nodes().is_empty());
        assert!(tracker.dirty_edges().is_empty());
    }

    #[test]
    fn dirty_nodes_set_correct() {
        let (_sm, ids) = make_node_ids(3);
        let mut tracker = DirtyTracker::new();

        tracker.mark_node(ids[0]);
        tracker.mark_node(ids[2]);

        let dirty = tracker.dirty_nodes();
        assert_eq!(dirty.len(), 2);
        assert!(dirty.contains(&ids[0]));
        assert!(dirty.contains(&ids[2]));
        assert!(!dirty.contains(&ids[1]));
    }

    #[test]
    fn dirty_edges_set_correct() {
        let (_sm, ids) = make_edge_ids(3);
        let mut tracker = DirtyTracker::new();

        tracker.mark_edge(ids[0]);
        tracker.mark_edge(ids[1]);
        tracker.mark_edge(ids[2]);

        let dirty = tracker.dirty_edges();
        assert_eq!(dirty.len(), 3);
        assert!(dirty.contains(&ids[0]));
        assert!(dirty.contains(&ids[1]));
        assert!(dirty.contains(&ids[2]));
    }

    #[test]
    fn duplicate_marks_idempotent() {
        let (_sm, ids) = make_node_ids(1);
        let mut tracker = DirtyTracker::new();

        tracker.mark_node(ids[0]);
        tracker.mark_node(ids[0]);

        assert_eq!(tracker.dirty_nodes().len(), 1);
        assert!(tracker.is_node_dirty(ids[0]));
    }

    #[test]
    fn engine_set_processor_marks_dirty() {
        // Simulates what the engine would do when set_processor is called:
        // it marks the affected node as dirty in the tracker.
        let (_sm, ids) = make_node_ids(1);
        let mut tracker = DirtyTracker::new();

        // Simulate: engine.set_processor(node, new_processor) internally does:
        tracker.mark_node(ids[0]);

        assert!(tracker.is_dirty());
        assert!(tracker.is_node_dirty(ids[0]));
    }

    #[test]
    fn engine_serialize_if_dirty_none_when_clean() {
        // Simulates the engine's "serialize only if dirty" logic:
        // when the tracker is clean, serialization is skipped.
        let tracker = DirtyTracker::new();

        // Engine would do: if tracker.is_dirty() { serialize() } else { None }
        let should_serialize = tracker.is_dirty();

        assert!(!should_serialize, "clean tracker should skip serialization");
    }

    #[test]
    fn partition_initially_clean() {
        let tracker = DirtyTracker::new();
        assert!(!tracker.any_partition_dirty());
        for &p in tracker.dirty_partitions() {
            assert!(!p);
        }
    }

    #[test]
    fn mark_partition_makes_dirty() {
        let mut tracker = DirtyTracker::new();
        tracker.mark_partition(DirtyTracker::PARTITION_PROCESSORS);
        assert!(tracker.any_partition_dirty());
        assert!(tracker.dirty_partitions()[DirtyTracker::PARTITION_PROCESSORS]);
        assert!(!tracker.dirty_partitions()[DirtyTracker::PARTITION_GRAPH]);
    }

    #[test]
    fn mark_clean_does_not_clear_partitions() {
        let mut tracker = DirtyTracker::new();
        tracker.mark_partition(DirtyTracker::PARTITION_INVENTORIES);
        tracker.mark_node(make_node_ids(1).1[0]);
        tracker.mark_clean();
        // Per-tick dirty state is cleared...
        assert!(!tracker.is_dirty());
        // ...but partitions survive.
        assert!(tracker.dirty_partitions()[DirtyTracker::PARTITION_INVENTORIES]);
    }

    #[test]
    fn clear_partitions_resets_all() {
        let mut tracker = DirtyTracker::new();
        tracker.mark_all_partitions();
        assert!(tracker.any_partition_dirty());
        tracker.clear_partitions();
        assert!(!tracker.any_partition_dirty());
    }

    #[test]
    fn mark_all_partitions_sets_all() {
        let mut tracker = DirtyTracker::new();
        tracker.mark_all_partitions();
        for &p in tracker.dirty_partitions() {
            assert!(p);
        }
    }

    #[test]
    fn partition_accumulates_across_mark_clean_cycles() {
        let mut tracker = DirtyTracker::new();
        tracker.mark_partition(DirtyTracker::PARTITION_GRAPH);
        tracker.mark_clean();
        tracker.mark_partition(DirtyTracker::PARTITION_TRANSPORTS);
        tracker.mark_clean();
        // Both partitions should still be dirty.
        assert!(tracker.dirty_partitions()[DirtyTracker::PARTITION_GRAPH]);
        assert!(tracker.dirty_partitions()[DirtyTracker::PARTITION_TRANSPORTS]);
    }
}
