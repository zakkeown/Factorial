use crate::id::*;
use serde::{Deserialize, Serialize};
use slotmap::{SecondaryMap, SlotMap};
use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during graph operations.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("cycle detected in production graph")]
    CycleDetected,
    #[error("node not found: {0:?}")]
    NodeNotFound(NodeId),
    #[error("edge not found: {0:?}")]
    EdgeNotFound(EdgeId),
}

// ---------------------------------------------------------------------------
// Core data structures
// ---------------------------------------------------------------------------

/// Adjacency lists for a single node, tracking incoming and outgoing edges.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct NodeAdjacency {
    /// Edges whose destination is this node.
    inputs: Vec<EdgeId>,
    /// Edges whose source is this node.
    outputs: Vec<EdgeId>,
}

/// Per-node data stored in the production graph.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeData {
    /// The building template this node was created from.
    pub building_type: BuildingTypeId,
}

/// Per-edge data stored in the production graph.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeData {
    /// Source node.
    pub from: NodeId,
    /// Destination node.
    pub to: NodeId,
    /// Optional item type filter. When set, only this item type flows on this edge.
    #[serde(default)]
    pub item_filter: Option<ItemTypeId>,
}

// ---------------------------------------------------------------------------
// Queued mutations
// ---------------------------------------------------------------------------

/// A mutation to be applied during the next `apply_mutations` call.
#[derive(Debug)]
enum Mutation {
    AddNode {
        building_type: BuildingTypeId,
        pending_id: PendingNodeId,
    },
    RemoveNode {
        node: NodeId,
    },
    Connect {
        from: NodeId,
        to: NodeId,
        pending_id: PendingEdgeId,
    },
    ConnectFiltered {
        from: NodeId,
        to: NodeId,
        pending_id: PendingEdgeId,
        item_filter: Option<ItemTypeId>,
    },
    Disconnect {
        edge: EdgeId,
    },
}

/// Result of applying queued mutations. Maps pending IDs to real IDs.
#[derive(Debug, Default)]
pub struct MutationResult {
    /// Maps each `PendingNodeId` counter to the real `NodeId` it was assigned.
    pub added_nodes: Vec<(PendingNodeId, NodeId)>,
    /// Maps each `PendingEdgeId` counter to the real `EdgeId` it was assigned.
    pub added_edges: Vec<(PendingEdgeId, EdgeId)>,
}

impl MutationResult {
    /// Look up the real `NodeId` for a pending node.
    pub fn resolve_node(&self, pending: PendingNodeId) -> Option<NodeId> {
        self.added_nodes
            .iter()
            .find(|(p, _)| *p == pending)
            .map(|(_, id)| *id)
    }

    /// Look up the real `EdgeId` for a pending edge.
    pub fn resolve_edge(&self, pending: PendingEdgeId) -> Option<EdgeId> {
        self.added_edges
            .iter()
            .find(|(p, _)| *p == pending)
            .map(|(_, id)| *id)
    }
}

// ---------------------------------------------------------------------------
// ProductionGraph
// ---------------------------------------------------------------------------

/// The production graph: nodes (buildings), edges (transport links), with
/// cached topological ordering and a queued mutation system.
///
/// Adjacency is stored in a `SecondaryMap` keyed by `NodeId`, following the
/// same SoA pattern used by `ComponentStorage`. This guarantees key
/// synchronization with the primary `nodes` SlotMap.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProductionGraph {
    nodes: SlotMap<NodeId, NodeData>,
    edges: SlotMap<EdgeId, EdgeData>,
    adjacency: SecondaryMap<NodeId, NodeAdjacency>,

    /// Cached topological order (strict, errors on cycles).
    /// Recomputed lazily when `dirty` is true.
    #[serde(skip)]
    topo_cache: Vec<NodeId>,
    /// Whether the cached topological order needs recomputation.
    /// Defaults to `true` on deserialize so the cache is recomputed.
    #[serde(skip, default = "default_dirty")]
    dirty: bool,

    /// Cached feedback-aware topological order (tolerates cycles).
    #[serde(skip)]
    feedback_order_cache: Vec<NodeId>,
    /// Cached back-edges from the feedback-aware ordering.
    #[serde(skip)]
    back_edge_cache: Vec<EdgeId>,
    /// Whether the feedback cache needs recomputation.
    #[serde(skip, default = "default_dirty")]
    feedback_dirty: bool,

    /// Queued mutations to be applied atomically.
    #[serde(skip)]
    mutations: Vec<Mutation>,
    /// Counter for generating unique `PendingNodeId` values.
    next_pending_node: u64,
    /// Counter for generating unique `PendingEdgeId` values.
    next_pending_edge: u64,
}

/// Default for dirty flag on deserialize -- always `true` so topo cache is recomputed.
fn default_dirty() -> bool {
    true
}

impl Clone for ProductionGraph {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            adjacency: self.adjacency.clone(),
            topo_cache: Vec::new(), // Cache will be recomputed.
            dirty: true,            // Force recomputation.
            feedback_order_cache: Vec::new(),
            back_edge_cache: Vec::new(),
            feedback_dirty: true,
            mutations: Vec::new(), // Don't clone queued mutations.
            next_pending_node: self.next_pending_node,
            next_pending_edge: self.next_pending_edge,
        }
    }
}

impl Default for ProductionGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductionGraph {
    /// Create a new, empty production graph.
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            edges: SlotMap::with_key(),
            adjacency: SecondaryMap::new(),
            topo_cache: Vec::new(),
            dirty: true,
            feedback_order_cache: Vec::new(),
            back_edge_cache: Vec::new(),
            feedback_dirty: true,
            mutations: Vec::new(),
            next_pending_node: 0,
            next_pending_edge: 0,
        }
    }

    /// Invalidate all topological order caches.
    fn invalidate_caches(&mut self) {
        self.dirty = true;
        self.feedback_dirty = true;
    }

    // -----------------------------------------------------------------------
    // Immediate (non-queued) mutations — used internally by apply_mutations
    // -----------------------------------------------------------------------

    /// Add a node immediately. Returns the assigned `NodeId`.
    fn add_node_immediate(&mut self, building_type: BuildingTypeId) -> NodeId {
        let node_id = self.nodes.insert(NodeData { building_type });
        self.adjacency.insert(node_id, NodeAdjacency::default());
        self.invalidate_caches();
        node_id
    }

    /// Remove a node immediately. Also removes all connected edges.
    fn remove_node_immediate(&mut self, node: NodeId) {
        // Collect edges to remove (both inputs and outputs).
        let edges_to_remove: Vec<EdgeId> = if let Some(adj) = self.adjacency.get(node) {
            adj.inputs
                .iter()
                .chain(adj.outputs.iter())
                .copied()
                .collect()
        } else {
            return;
        };

        // Remove each connected edge.
        for edge_id in edges_to_remove {
            self.disconnect_immediate(edge_id);
        }

        self.nodes.remove(node);
        self.adjacency.remove(node);
        self.invalidate_caches();
    }

    /// Connect two nodes immediately. Returns the assigned `EdgeId`.
    fn connect_immediate(&mut self, from: NodeId, to: NodeId) -> EdgeId {
        self.connect_immediate_filtered(from, to, None)
    }

    /// Connect two nodes immediately with an optional item filter. Returns the assigned `EdgeId`.
    fn connect_immediate_filtered(
        &mut self,
        from: NodeId,
        to: NodeId,
        item_filter: Option<ItemTypeId>,
    ) -> EdgeId {
        let edge_id = self.edges.insert(EdgeData {
            from,
            to,
            item_filter,
        });

        if let Some(adj) = self.adjacency.get_mut(from) {
            adj.outputs.push(edge_id);
        }
        if let Some(adj) = self.adjacency.get_mut(to) {
            adj.inputs.push(edge_id);
        }

        self.invalidate_caches();
        edge_id
    }

    /// Disconnect (remove) an edge immediately.
    fn disconnect_immediate(&mut self, edge: EdgeId) {
        if let Some(edge_data) = self.edges.remove(edge) {
            // Remove from source's output list.
            if let Some(adj) = self.adjacency.get_mut(edge_data.from) {
                adj.outputs.retain(|&e| e != edge);
            }
            // Remove from destination's input list.
            if let Some(adj) = self.adjacency.get_mut(edge_data.to) {
                adj.inputs.retain(|&e| e != edge);
            }
            self.invalidate_caches();
        }
    }

    // -----------------------------------------------------------------------
    // Queued mutations — used by game code during simulation
    // -----------------------------------------------------------------------

    /// Queue a node to be added. Returns a `PendingNodeId` that can be
    /// resolved to a real `NodeId` after `apply_mutations`.
    ///
    /// # Examples
    ///
    /// ```
    /// use factorial_core::graph::ProductionGraph;
    /// use factorial_core::id::BuildingTypeId;
    ///
    /// let mut graph = ProductionGraph::new();
    /// let pending = graph.queue_add_node(BuildingTypeId(1));
    /// let result = graph.apply_mutations();
    /// let node_id = result.resolve_node(pending).unwrap();
    /// ```
    pub fn queue_add_node(&mut self, building_type: BuildingTypeId) -> PendingNodeId {
        let pending = PendingNodeId(self.next_pending_node);
        self.next_pending_node += 1;
        self.mutations.push(Mutation::AddNode {
            building_type,
            pending_id: pending,
        });
        pending
    }

    /// Queue a node for removal.
    pub fn queue_remove_node(&mut self, node: NodeId) {
        self.mutations.push(Mutation::RemoveNode { node });
    }

    /// Queue an edge connecting two existing nodes. Returns a `PendingEdgeId`.
    ///
    /// # Examples
    ///
    /// ```
    /// use factorial_core::graph::ProductionGraph;
    /// use factorial_core::id::BuildingTypeId;
    ///
    /// let mut graph = ProductionGraph::new();
    /// let p1 = graph.queue_add_node(BuildingTypeId(1));
    /// let p2 = graph.queue_add_node(BuildingTypeId(2));
    /// let result = graph.apply_mutations();
    /// let n1 = result.resolve_node(p1).unwrap();
    /// let n2 = result.resolve_node(p2).unwrap();
    ///
    /// let pending_edge = graph.queue_connect(n1, n2);
    /// let result = graph.apply_mutations();
    /// let edge_id = result.resolve_edge(pending_edge).unwrap();
    /// ```
    pub fn queue_connect(&mut self, from: NodeId, to: NodeId) -> PendingEdgeId {
        let pending = PendingEdgeId(self.next_pending_edge);
        self.next_pending_edge += 1;
        self.mutations.push(Mutation::Connect {
            from,
            to,
            pending_id: pending,
        });
        pending
    }

    /// Queue an edge connecting two existing nodes with an optional item type
    /// filter. Returns a `PendingEdgeId`.
    pub fn queue_connect_filtered(
        &mut self,
        from: NodeId,
        to: NodeId,
        item_filter: Option<ItemTypeId>,
    ) -> PendingEdgeId {
        let pending = PendingEdgeId(self.next_pending_edge);
        self.next_pending_edge += 1;
        self.mutations.push(Mutation::ConnectFiltered {
            from,
            to,
            pending_id: pending,
            item_filter,
        });
        pending
    }

    /// Queue an edge for removal.
    pub fn queue_disconnect(&mut self, edge: EdgeId) {
        self.mutations.push(Mutation::Disconnect { edge });
    }

    /// Apply all queued mutations atomically. Returns a `MutationResult`
    /// mapping pending IDs to their real IDs.
    pub fn apply_mutations(&mut self) -> MutationResult {
        let mutations = std::mem::take(&mut self.mutations);
        let mut result = MutationResult::default();

        for mutation in mutations {
            match mutation {
                Mutation::AddNode {
                    building_type,
                    pending_id,
                } => {
                    let node_id = self.add_node_immediate(building_type);
                    result.added_nodes.push((pending_id, node_id));
                }
                Mutation::RemoveNode { node } => {
                    self.remove_node_immediate(node);
                }
                Mutation::Connect {
                    from,
                    to,
                    pending_id,
                } => {
                    let edge_id = self.connect_immediate(from, to);
                    result.added_edges.push((pending_id, edge_id));
                }
                Mutation::ConnectFiltered {
                    from,
                    to,
                    pending_id,
                    item_filter,
                } => {
                    let edge_id = self.connect_immediate_filtered(from, to, item_filter);
                    result.added_edges.push((pending_id, edge_id));
                }
                Mutation::Disconnect { edge } => {
                    self.disconnect_immediate(edge);
                }
            }
        }

        result
    }

    /// Returns true if there are queued mutations waiting to be applied.
    pub fn has_pending_mutations(&self) -> bool {
        !self.mutations.is_empty()
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Get the node data for a given node ID.
    pub fn get_node(&self, node: NodeId) -> Option<&NodeData> {
        self.nodes.get(node)
    }

    /// Get the edge data for a given edge ID.
    pub fn get_edge(&self, edge: EdgeId) -> Option<&EdgeData> {
        self.edges.get(edge)
    }

    /// Get the edges coming into a node (inputs).
    pub fn get_inputs(&self, node: NodeId) -> &[EdgeId] {
        self.adjacency
            .get(node)
            .map(|adj| adj.inputs.as_slice())
            .unwrap_or(&[])
    }

    /// Get the edges going out of a node (outputs).
    pub fn get_outputs(&self, node: NodeId) -> &[EdgeId] {
        self.adjacency
            .get(node)
            .map(|adj| adj.outputs.as_slice())
            .unwrap_or(&[])
    }

    /// Total number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns true if the node exists in the graph.
    pub fn contains_node(&self, node: NodeId) -> bool {
        self.nodes.contains_key(node)
    }

    /// Returns true if the edge exists in the graph.
    pub fn contains_edge(&self, edge: EdgeId) -> bool {
        self.edges.contains_key(edge)
    }

    /// Iterate over all node IDs and their data.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &NodeData)> {
        self.nodes.iter()
    }

    /// Iterate over all edge IDs and their data.
    pub fn edges(&self) -> impl Iterator<Item = (EdgeId, &EdgeData)> {
        self.edges.iter()
    }

    // -----------------------------------------------------------------------
    // Topo cache borrowing helpers
    // -----------------------------------------------------------------------

    /// Temporarily take ownership of the strict topo cache.
    /// The cache Vec is left empty; call `restore_topo_cache` to put it back.
    pub(crate) fn take_topo_cache(&mut self) -> Vec<NodeId> {
        std::mem::take(&mut self.topo_cache)
    }

    /// Restore the strict topo cache after a `take_topo_cache` call.
    pub(crate) fn restore_topo_cache(&mut self, cache: Vec<NodeId>) {
        self.topo_cache = cache;
    }

    /// Temporarily take ownership of the feedback-order cache.
    /// The cache Vec is left empty; call `restore_feedback_cache` to put it back.
    pub(crate) fn take_feedback_cache(&mut self) -> Vec<NodeId> {
        std::mem::take(&mut self.feedback_order_cache)
    }

    /// Restore the feedback-order cache after a `take_feedback_cache` call.
    pub(crate) fn restore_feedback_cache(&mut self, cache: Vec<NodeId>) {
        self.feedback_order_cache = cache;
    }

    // -----------------------------------------------------------------------
    // Topological sort (Kahn's algorithm)
    // -----------------------------------------------------------------------

    /// Get the cached topological order, recomputing if dirty.
    /// Returns an error if the graph contains a cycle.
    pub fn topological_order(&mut self) -> Result<&[NodeId], GraphError> {
        if self.dirty {
            self.recompute_topological_order()?;
            self.dirty = false;
        }
        Ok(&self.topo_cache)
    }

    /// Returns a processing order even when cycles exist.
    ///
    /// Runs Kahn's algorithm. If some nodes remain (they form cycles), they are
    /// appended to the order (sorted by `NodeId` for determinism). Back-edges
    /// -- edges where `to` appears before `from` in the final order -- are
    /// returned separately.  Callers can use the back-edge set to understand
    /// which connections carry a one-tick delay.
    ///
    /// The result is cached and only recomputed when the graph is mutated.
    pub fn topological_order_with_feedback(&mut self) -> (&[NodeId], &[EdgeId]) {
        if self.feedback_dirty {
            self.recompute_feedback_order();
            self.feedback_dirty = false;
        }
        (&self.feedback_order_cache, &self.back_edge_cache)
    }

    /// Recompute the feedback-aware topological order and back-edges.
    fn recompute_feedback_order(&mut self) {
        let node_count = self.nodes.len();

        // Compute in-degree for each node.
        let mut in_degree: SecondaryMap<NodeId, usize> = SecondaryMap::new();
        for (nid, _) in &self.nodes {
            in_degree.insert(nid, 0);
        }
        for (_, edge) in &self.edges {
            if let Some(deg) = in_degree.get_mut(edge.to) {
                *deg += 1;
            }
        }

        // Seed the queue with all zero-in-degree nodes.
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        for (nid, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(nid);
            }
        }

        let mut order: Vec<NodeId> = Vec::with_capacity(node_count);

        while let Some(node) = queue.pop_front() {
            order.push(node);

            // For each outgoing edge, decrement the destination's in-degree.
            let destinations: Vec<NodeId> = self
                .adjacency
                .get(node)
                .map(|adj| {
                    adj.outputs
                        .iter()
                        .filter_map(|&eid| self.edges.get(eid).map(|e| e.to))
                        .collect()
                })
                .unwrap_or_default();

            for dest in destinations {
                if let Some(deg) = in_degree.get_mut(dest) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dest);
                    }
                }
            }
        }

        // If there are remaining nodes, they are in cycles. Append them in
        // deterministic order (by NodeId slot-map key).
        if order.len() < node_count {
            let in_order: std::collections::HashSet<NodeId> = order.iter().copied().collect();
            let mut cycle_nodes: Vec<NodeId> = self
                .nodes
                .keys()
                .filter(|nid| !in_order.contains(nid))
                .collect();
            cycle_nodes.sort();
            order.extend(cycle_nodes);
        }

        // Build a position map: node -> index in `order`.
        let mut position: SecondaryMap<NodeId, usize> = SecondaryMap::new();
        for (idx, &nid) in order.iter().enumerate() {
            position.insert(nid, idx);
        }

        // Back-edges: any edge where `to` appears before `from` in the order.
        let back_edges: Vec<EdgeId> = self
            .edges
            .iter()
            .filter_map(|(eid, edge)| {
                let from_pos = position.get(edge.from).copied().unwrap_or(0);
                let to_pos = position.get(edge.to).copied().unwrap_or(0);
                if to_pos <= from_pos { Some(eid) } else { None }
            })
            .collect();

        self.feedback_order_cache = order;
        self.back_edge_cache = back_edges;
    }

    /// Returns nodes grouped by topological level, plus back-edges.
    ///
    /// Modified Kahn's algorithm that tracks depth: when a node's in-degree
    /// hits 0, its depth = max(predecessor depths) + 1. Nodes within each
    /// level are sorted by `NodeId` for determinism.
    ///
    /// Cycle nodes are placed in a final catch-all level. Back-edges are
    /// edges where `to` appears at an equal or earlier level than `from`.
    pub fn topological_order_by_level(&self) -> (Vec<Vec<NodeId>>, Vec<EdgeId>) {
        let node_count = self.nodes.len();

        // Compute in-degree and track depth.
        let mut in_degree: SecondaryMap<NodeId, usize> = SecondaryMap::new();
        let mut depth: SecondaryMap<NodeId, usize> = SecondaryMap::new();
        for (nid, _) in &self.nodes {
            in_degree.insert(nid, 0);
            depth.insert(nid, 0);
        }
        for (_, edge) in &self.edges {
            if let Some(deg) = in_degree.get_mut(edge.to) {
                *deg += 1;
            }
        }

        // Seed queue with zero-in-degree nodes (level 0).
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        for (nid, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(nid);
            }
        }

        let mut processed = 0usize;
        let mut max_depth = 0usize;

        while let Some(node) = queue.pop_front() {
            processed += 1;
            let node_depth = depth.get(node).copied().unwrap_or(0);
            if node_depth > max_depth {
                max_depth = node_depth;
            }

            // For each outgoing edge, update destination depth and decrement in-degree.
            let destinations: Vec<NodeId> = self
                .adjacency
                .get(node)
                .map(|adj| {
                    adj.outputs
                        .iter()
                        .filter_map(|&eid| self.edges.get(eid).map(|e| e.to))
                        .collect()
                })
                .unwrap_or_default();

            for dest in destinations {
                // Update dest depth to be at least node_depth + 1.
                if let Some(d) = depth.get_mut(dest) {
                    *d = (*d).max(node_depth + 1);
                }
                if let Some(deg) = in_degree.get_mut(dest) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dest);
                    }
                }
            }
        }

        // Group nodes by depth level.
        let mut levels: Vec<Vec<NodeId>> = vec![Vec::new(); max_depth + 1];
        let mut in_order: std::collections::HashSet<NodeId> =
            std::collections::HashSet::with_capacity(processed);

        for (nid, _) in &self.nodes {
            if let Some(&deg) = in_degree.get(nid)
                && deg == 0
            {
                // This node was processed by Kahn's.
                let d = depth.get(nid).copied().unwrap_or(0);
                levels[d].push(nid);
                in_order.insert(nid);
            }
        }

        // Cycle nodes: any node not processed goes in a final level.
        if processed < node_count {
            let mut cycle_nodes: Vec<NodeId> = self
                .nodes
                .keys()
                .filter(|nid| !in_order.contains(nid))
                .collect();
            cycle_nodes.sort();
            levels.push(cycle_nodes);
        }

        // Sort within each level for determinism.
        for level in &mut levels {
            level.sort();
        }

        // Remove empty levels.
        levels.retain(|l| !l.is_empty());

        // Build position map for back-edge detection.
        let mut level_of: SecondaryMap<NodeId, usize> = SecondaryMap::new();
        for (lvl_idx, level) in levels.iter().enumerate() {
            for &nid in level {
                level_of.insert(nid, lvl_idx);
            }
        }

        // Back-edges: edges where to is at same or earlier level than from.
        let back_edges: Vec<EdgeId> = self
            .edges
            .iter()
            .filter_map(|(eid, edge)| {
                let from_lvl = level_of.get(edge.from).copied().unwrap_or(0);
                let to_lvl = level_of.get(edge.to).copied().unwrap_or(0);
                if to_lvl <= from_lvl { Some(eid) } else { None }
            })
            .collect();

        (levels, back_edges)
    }

    /// Recompute the topological order using Kahn's algorithm.
    ///
    /// Uses a `SecondaryMap<NodeId, usize>` for in-degree tracking, giving
    /// O(V+E) complexity. No `HashMap` in the simulation hot path.
    fn recompute_topological_order(&mut self) -> Result<(), GraphError> {
        let node_count = self.nodes.len();

        // Compute in-degree for each node.
        let mut in_degree: SecondaryMap<NodeId, usize> = SecondaryMap::new();
        for (nid, _) in &self.nodes {
            in_degree.insert(nid, 0);
        }
        for (_, edge) in &self.edges {
            if let Some(deg) = in_degree.get_mut(edge.to) {
                *deg += 1;
            }
        }

        // Seed the queue with all zero-in-degree nodes.
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        for (nid, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(nid);
            }
        }

        let mut order: Vec<NodeId> = Vec::with_capacity(node_count);

        while let Some(node) = queue.pop_front() {
            order.push(node);

            // For each outgoing edge, decrement the destination's in-degree.
            // We collect destinations first to avoid borrowing conflicts.
            let destinations: Vec<NodeId> = self
                .adjacency
                .get(node)
                .map(|adj| {
                    adj.outputs
                        .iter()
                        .filter_map(|&eid| self.edges.get(eid).map(|e| e.to))
                        .collect()
                })
                .unwrap_or_default();

            for dest in destinations {
                if let Some(deg) = in_degree.get_mut(dest) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dest);
                    }
                }
            }
        }

        if order.len() != node_count {
            return Err(GraphError::CycleDetected);
        }

        self.topo_cache = order;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a graph and add nodes via queued mutations, then apply.
    /// Returns (graph, vec of real NodeIds).
    fn make_graph_with_nodes(count: usize) -> (ProductionGraph, Vec<NodeId>) {
        let mut graph = ProductionGraph::new();
        let building = BuildingTypeId(0);

        let pending: Vec<PendingNodeId> =
            (0..count).map(|_| graph.queue_add_node(building)).collect();

        let result = graph.apply_mutations();

        let node_ids: Vec<NodeId> = pending
            .iter()
            .map(|p| result.resolve_node(*p).unwrap())
            .collect();

        (graph, node_ids)
    }

    // -----------------------------------------------------------------------
    // Test 1: Add/remove nodes
    // -----------------------------------------------------------------------
    #[test]
    fn add_and_remove_nodes() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        assert_eq!(graph.node_count(), 3);

        // Verify nodes exist.
        for &n in &nodes {
            assert!(graph.contains_node(n));
        }

        // Remove the middle node via queued mutation.
        graph.queue_remove_node(nodes[1]);
        graph.apply_mutations();

        assert_eq!(graph.node_count(), 2);
        assert!(graph.contains_node(nodes[0]));
        assert!(!graph.contains_node(nodes[1]));
        assert!(graph.contains_node(nodes[2]));
    }

    // -----------------------------------------------------------------------
    // Test 2: Connect edges
    // -----------------------------------------------------------------------
    #[test]
    fn connect_edges() {
        let (mut graph, nodes) = make_graph_with_nodes(2);

        let pending_edge = graph.queue_connect(nodes[0], nodes[1]);
        let result = graph.apply_mutations();

        let edge_id = result.resolve_edge(pending_edge).unwrap();
        assert_eq!(graph.edge_count(), 1);

        let edge_data = graph.get_edge(edge_id).unwrap();
        assert_eq!(edge_data.from, nodes[0]);
        assert_eq!(edge_data.to, nodes[1]);
    }

    // -----------------------------------------------------------------------
    // Test 3: Topological sort correctness — linear chain A->B->C
    // -----------------------------------------------------------------------
    #[test]
    fn topological_sort_linear_chain() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];

        // A->B, B->C
        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.apply_mutations();

        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], a);
        assert_eq!(order[1], b);
        assert_eq!(order[2], c);
    }

    // -----------------------------------------------------------------------
    // Test 4: Topological sort with diamond — A->B, A->C, B->D, C->D
    // -----------------------------------------------------------------------
    #[test]
    fn topological_sort_diamond() {
        let (mut graph, nodes) = make_graph_with_nodes(4);
        let [a, b, c, d] = [nodes[0], nodes[1], nodes[2], nodes[3]];

        graph.queue_connect(a, b);
        graph.queue_connect(a, c);
        graph.queue_connect(b, d);
        graph.queue_connect(c, d);
        graph.apply_mutations();

        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 4);

        // A must come first.
        assert_eq!(order[0], a);
        // D must come last.
        assert_eq!(order[3], d);

        // B and C must both come after A and before D (order between them
        // is not specified by the algorithm, just that both precede D).
        let b_pos = order.iter().position(|&n| n == b).unwrap();
        let c_pos = order.iter().position(|&n| n == c).unwrap();
        assert!(b_pos > 0 && b_pos < 3);
        assert!(c_pos > 0 && c_pos < 3);
    }

    // -----------------------------------------------------------------------
    // Test 5: Queued mutations — verify pending state
    // -----------------------------------------------------------------------
    #[test]
    fn queued_mutations_pending_state() {
        let mut graph = ProductionGraph::new();
        let building = BuildingTypeId(0);

        // Queue a node but don't apply yet.
        let pending = graph.queue_add_node(building);
        assert!(graph.has_pending_mutations());
        assert_eq!(graph.node_count(), 0, "node should not exist before apply");

        // Now apply.
        let result = graph.apply_mutations();
        assert_eq!(graph.node_count(), 1, "node should exist after apply");
        assert!(!graph.has_pending_mutations());

        // Verify the pending ID resolves.
        assert!(result.resolve_node(pending).is_some());
    }

    // -----------------------------------------------------------------------
    // Test 6: Cycle detection — A->B->C->A
    // -----------------------------------------------------------------------
    #[test]
    fn cycle_detection() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];

        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.queue_connect(c, a);
        graph.apply_mutations();

        let result = graph.topological_order();
        assert!(result.is_err());
        assert!(matches!(result, Err(GraphError::CycleDetected)));
    }

    // -----------------------------------------------------------------------
    // Test 7: Adjacent node queries — get_inputs / get_outputs
    // -----------------------------------------------------------------------
    #[test]
    fn adjacency_queries() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];

        // A->B, C->B (B has two inputs)
        let pe1 = graph.queue_connect(a, b);
        let pe2 = graph.queue_connect(c, b);
        let result = graph.apply_mutations();

        let e1 = result.resolve_edge(pe1).unwrap();
        let e2 = result.resolve_edge(pe2).unwrap();

        // B should have two inputs.
        let b_inputs = graph.get_inputs(b);
        assert_eq!(b_inputs.len(), 2);
        assert!(b_inputs.contains(&e1));
        assert!(b_inputs.contains(&e2));

        // B should have no outputs.
        assert_eq!(graph.get_outputs(b).len(), 0);

        // A should have one output (to B).
        let a_outputs = graph.get_outputs(a);
        assert_eq!(a_outputs.len(), 1);
        assert!(a_outputs.contains(&e1));

        // A should have no inputs.
        assert_eq!(graph.get_inputs(a).len(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 8: Dirty flag — topo order recomputed after mutation
    // -----------------------------------------------------------------------
    #[test]
    fn dirty_flag_recomputation() {
        let (mut graph, nodes) = make_graph_with_nodes(2);
        let [a, b] = [nodes[0], nodes[1]];

        // A->B
        graph.queue_connect(a, b);
        graph.apply_mutations();

        // First call computes topo order.
        let order = graph.topological_order().unwrap();
        assert_eq!(order, &[a, b]);

        // Add a new node C and edge B->C.
        let pending_c = graph.queue_add_node(BuildingTypeId(0));
        let result = graph.apply_mutations();
        let c = result.resolve_node(pending_c).unwrap();

        graph.queue_connect(b, c);
        graph.apply_mutations();

        // Topo order should now include C.
        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], a);
        assert_eq!(order[1], b);
        assert_eq!(order[2], c);
    }

    // -----------------------------------------------------------------------
    // Test 9: Remove node cleans up edges
    // -----------------------------------------------------------------------
    #[test]
    fn remove_node_cleans_edges() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];

        // A->B->C
        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.apply_mutations();

        assert_eq!(graph.edge_count(), 2);

        // Remove B — should also remove both edges.
        graph.queue_remove_node(b);
        graph.apply_mutations();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 0);

        // A and C should have no adjacency.
        assert_eq!(graph.get_outputs(a).len(), 0);
        assert_eq!(graph.get_inputs(c).len(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 10: Disconnect edge
    // -----------------------------------------------------------------------
    #[test]
    fn disconnect_edge() {
        let (mut graph, nodes) = make_graph_with_nodes(2);
        let [a, b] = [nodes[0], nodes[1]];

        let pending_edge = graph.queue_connect(a, b);
        let result = graph.apply_mutations();
        let edge_id = result.resolve_edge(pending_edge).unwrap();

        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.get_outputs(a).len(), 1);
        assert_eq!(graph.get_inputs(b).len(), 1);

        // Disconnect the edge.
        graph.queue_disconnect(edge_id);
        graph.apply_mutations();

        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.contains_edge(edge_id));
        assert_eq!(graph.get_outputs(a).len(), 0);
        assert_eq!(graph.get_inputs(b).len(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 11: Self-loop detected as cycle
    // -----------------------------------------------------------------------
    #[test]
    fn self_loop_detected_as_cycle() {
        let (mut graph, nodes) = make_graph_with_nodes(1);
        let a = nodes[0];
        graph.queue_connect(a, a);
        graph.apply_mutations();
        let result = graph.topological_order();
        assert!(result.is_err());
        assert!(matches!(result, Err(GraphError::CycleDetected)));
    }

    // -----------------------------------------------------------------------
    // Test 12: Duplicate edges allowed
    // -----------------------------------------------------------------------
    #[test]
    fn duplicate_edges_allowed() {
        let (mut graph, nodes) = make_graph_with_nodes(2);
        let [a, b] = [nodes[0], nodes[1]];
        let pe1 = graph.queue_connect(a, b);
        let pe2 = graph.queue_connect(a, b);
        let result = graph.apply_mutations();
        let e1 = result.resolve_edge(pe1).unwrap();
        let e2 = result.resolve_edge(pe2).unwrap();
        assert_ne!(e1, e2);
        assert_eq!(graph.edge_count(), 2);
        assert_eq!(graph.get_outputs(a).len(), 2);
        assert_eq!(graph.get_inputs(b).len(), 2);
        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], a);
        assert_eq!(order[1], b);
    }

    // -----------------------------------------------------------------------
    // Test 13: Remove nonexistent node — no panic
    // -----------------------------------------------------------------------
    #[test]
    fn remove_nonexistent_node_no_panic() {
        let (mut graph, nodes) = make_graph_with_nodes(1);
        let real_node = nodes[0];
        graph.queue_remove_node(real_node);
        graph.apply_mutations();
        assert_eq!(graph.node_count(), 0);
        graph.queue_remove_node(real_node);
        graph.apply_mutations();
        assert_eq!(graph.node_count(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 14: Disconnect nonexistent edge — no panic
    // -----------------------------------------------------------------------
    #[test]
    fn disconnect_nonexistent_edge_no_panic() {
        let (mut graph, nodes) = make_graph_with_nodes(2);
        let [a, b] = [nodes[0], nodes[1]];
        let pe = graph.queue_connect(a, b);
        let result = graph.apply_mutations();
        let edge = result.resolve_edge(pe).unwrap();
        graph.queue_disconnect(edge);
        graph.apply_mutations();
        assert_eq!(graph.edge_count(), 0);
        graph.queue_disconnect(edge);
        graph.apply_mutations();
        assert_eq!(graph.edge_count(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 15: Remove node with inbound and outbound edges
    // -----------------------------------------------------------------------
    #[test]
    fn remove_node_with_inbound_and_outbound_edges() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];
        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.apply_mutations();
        assert_eq!(graph.edge_count(), 2);
        graph.queue_remove_node(b);
        graph.apply_mutations();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 0);
        assert_eq!(graph.get_outputs(a).len(), 0);
        assert_eq!(graph.get_inputs(c).len(), 0);
        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Test 16: Queued mutations don't affect topo until applied
    // -----------------------------------------------------------------------
    #[test]
    fn queued_mutations_dont_affect_topo_until_applied() {
        let (mut graph, nodes) = make_graph_with_nodes(2);
        let [a, b] = [nodes[0], nodes[1]];
        graph.queue_connect(a, b);
        graph.apply_mutations();
        let order = graph.topological_order().unwrap();
        assert_eq!(order, &[a, b]);
        let _pending_c = graph.queue_add_node(BuildingTypeId(0));
        assert!(graph.has_pending_mutations());
        let order = graph.topological_order().unwrap();
        assert_eq!(order, &[a, b]);
        assert_eq!(graph.node_count(), 2);
    }

    // -----------------------------------------------------------------------
    // Error path tests
    // -----------------------------------------------------------------------

    #[test]
    fn graph_error_display_messages() {
        let (_graph, nodes) = make_graph_with_nodes(1);
        let node = nodes[0];
        let err = GraphError::NodeNotFound(node);
        let msg = format!("{err}");
        assert!(msg.contains("node not found"), "got: {msg}");

        let mut sm: slotmap::SlotMap<EdgeId, ()> = slotmap::SlotMap::with_key();
        let edge = sm.insert(());
        let err = GraphError::EdgeNotFound(edge);
        let msg = format!("{err}");
        assert!(msg.contains("edge not found"), "got: {msg}");

        let err = GraphError::CycleDetected;
        let msg = format!("{err}");
        assert!(msg.contains("cycle"), "got: {msg}");
    }

    #[test]
    fn empty_graph_topological_order() {
        let mut graph = ProductionGraph::new();
        let order = graph.topological_order().unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn topological_order_with_feedback_acyclic() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];
        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.apply_mutations();

        let (order, back_edges) = graph.topological_order_with_feedback();
        assert_eq!(order.len(), 3);
        assert!(
            back_edges.is_empty(),
            "acyclic graph should have no back edges"
        );
    }

    #[test]
    fn topological_order_with_feedback_cyclic() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];
        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.queue_connect(c, a);
        graph.apply_mutations();

        let (order, back_edges) = graph.topological_order_with_feedback();
        assert_eq!(
            order.len(),
            3,
            "all nodes should appear in order even with cycle"
        );
        assert!(
            !back_edges.is_empty(),
            "cyclic graph should have back edges"
        );
    }

    #[test]
    fn connect_filtered_edge_preserves_filter() {
        let (mut graph, nodes) = make_graph_with_nodes(2);
        let [a, b] = [nodes[0], nodes[1]];
        let iron = ItemTypeId(0);
        let pe = graph.queue_connect_filtered(a, b, Some(iron));
        let result = graph.apply_mutations();
        let edge = result.resolve_edge(pe).unwrap();
        let edge_data = graph.get_edge(edge).unwrap();
        assert_eq!(edge_data.item_filter, Some(iron));
    }

    #[test]
    fn connect_filtered_edge_none_filter() {
        let (mut graph, nodes) = make_graph_with_nodes(2);
        let [a, b] = [nodes[0], nodes[1]];
        let pe = graph.queue_connect_filtered(a, b, None);
        let result = graph.apply_mutations();
        let edge = result.resolve_edge(pe).unwrap();
        let edge_data = graph.get_edge(edge).unwrap();
        assert_eq!(edge_data.item_filter, None);
    }

    // ===================================================================
    // Mutation-testing targeted tests
    // ===================================================================

    // Kill: line 335 "replace += with *=" in queue_connect_filtered
    // The pending edge counter must increment by 1, not multiply.
    #[test]
    fn connect_filtered_increments_pending_edge_id() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];

        // Queue two filtered connections; their pending IDs should differ by exactly 1.
        let pe1 = graph.queue_connect_filtered(a, b, None);
        let pe2 = graph.queue_connect_filtered(b, c, None);
        assert_eq!(pe2.0 - pe1.0, 1, "pending edge IDs must be sequential");

        let result = graph.apply_mutations();
        let e1 = result.resolve_edge(pe1).unwrap();
        let e2 = result.resolve_edge(pe2).unwrap();
        assert_ne!(e1, e2);
    }

    // Kill: line 521 "replace -= with +=" / "/=" in topological_order_with_feedback
    // and line 531 "replace < with <=" boundary.
    // The in-degree decrement is critical for Kahn's algorithm correctness.
    #[test]
    fn topological_order_with_feedback_respects_in_degree() {
        // Build: A->B, A->C, B->D, C->D (diamond).
        // D has in-degree 2. Both B and C must be processed before D appears.
        let (mut graph, nodes) = make_graph_with_nodes(4);
        let [a, b, c, d] = [nodes[0], nodes[1], nodes[2], nodes[3]];

        graph.queue_connect(a, b);
        graph.queue_connect(a, c);
        graph.queue_connect(b, d);
        graph.queue_connect(c, d);
        graph.apply_mutations();

        let (order, back_edges) = graph.topological_order_with_feedback();
        assert_eq!(order.len(), 4);
        assert!(
            back_edges.is_empty(),
            "acyclic diamond should have no back edges"
        );

        // A must be first (only node with in-degree 0).
        assert_eq!(order[0], a);
        // D must be last (in-degree 2, only reachable after B and C).
        assert_eq!(order[3], d);

        // B and C positions must both be between A and D.
        let b_pos = order.iter().position(|&n| n == b).unwrap();
        let c_pos = order.iter().position(|&n| n == c).unwrap();
        assert!(b_pos > 0 && b_pos < 3);
        assert!(c_pos > 0 && c_pos < 3);
    }

    // Kill: line 145 "replace default_dirty -> bool with false"
    // The default_dirty function returns true so that deserialized graphs
    // recompute their topo cache.
    #[test]
    fn deserialized_graph_recomputes_topo() {
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];
        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.apply_mutations();

        // Compute topo order first so cache is populated.
        let order = graph.topological_order().unwrap();
        assert_eq!(order, &[a, b, c]);

        // Round-trip via serialization.
        let bytes = bitcode::serialize(&graph).expect("serialize graph");
        let mut deserialized: ProductionGraph =
            bitcode::deserialize(&bytes).expect("deserialize graph");

        // The deserialized graph must still produce a correct topo order
        // (dirty flag should force recomputation).
        let order = deserialized.topological_order().unwrap();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], a);
        assert_eq!(order[2], c);
    }

    // -----------------------------------------------------------------------
    // topological_order_by_level tests
    // -----------------------------------------------------------------------

    #[test]
    fn topo_by_level_linear_chain() {
        // A -> B -> C: each level has exactly 1 node.
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];
        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.apply_mutations();

        let (levels, back_edges) = graph.topological_order_by_level();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec![a]);
        assert_eq!(levels[1], vec![b]);
        assert_eq!(levels[2], vec![c]);
        assert!(back_edges.is_empty());
    }

    #[test]
    fn topo_by_level_diamond() {
        // A -> B, A -> C, B -> D, C -> D
        // Levels: [[A], [B, C], [D]]
        let (mut graph, nodes) = make_graph_with_nodes(4);
        let [a, b, c, d] = [nodes[0], nodes[1], nodes[2], nodes[3]];
        graph.queue_connect(a, b);
        graph.queue_connect(a, c);
        graph.queue_connect(b, d);
        graph.queue_connect(c, d);
        graph.apply_mutations();

        let (levels, back_edges) = graph.topological_order_by_level();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec![a]);
        // B and C are at level 1 (sorted by NodeId).
        assert_eq!(levels[1].len(), 2);
        assert!(levels[1].contains(&b));
        assert!(levels[1].contains(&c));
        assert_eq!(levels[2], vec![d]);
        assert!(back_edges.is_empty());
    }

    #[test]
    fn topo_by_level_wide_fan_out() {
        // A -> B, A -> C, A -> D: 1 source at level 0, 3 consumers at level 1.
        let (mut graph, nodes) = make_graph_with_nodes(4);
        let [a, b, c, d] = [nodes[0], nodes[1], nodes[2], nodes[3]];
        graph.queue_connect(a, b);
        graph.queue_connect(a, c);
        graph.queue_connect(a, d);
        graph.apply_mutations();

        let (levels, back_edges) = graph.topological_order_by_level();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0], vec![a]);
        assert_eq!(levels[1].len(), 3);
        assert!(back_edges.is_empty());
    }

    #[test]
    fn topo_by_level_disconnected_components() {
        // A -> B, C -> D (two disconnected pairs).
        // All roots (A, C) at level 0.
        let (mut graph, nodes) = make_graph_with_nodes(4);
        let [a, b, c, d] = [nodes[0], nodes[1], nodes[2], nodes[3]];
        graph.queue_connect(a, b);
        graph.queue_connect(c, d);
        graph.apply_mutations();

        let (levels, back_edges) = graph.topological_order_by_level();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 2); // A and C.
        assert!(levels[0].contains(&a));
        assert!(levels[0].contains(&c));
        assert_eq!(levels[1].len(), 2); // B and D.
        assert!(levels[1].contains(&b));
        assert!(levels[1].contains(&d));
        assert!(back_edges.is_empty());
    }

    #[test]
    fn topo_by_level_cycle() {
        // A -> B -> C -> A (cycle). Cycle nodes in final level.
        let (mut graph, nodes) = make_graph_with_nodes(3);
        let [a, b, c] = [nodes[0], nodes[1], nodes[2]];
        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.queue_connect(c, a);
        graph.apply_mutations();

        let (levels, back_edges) = graph.topological_order_by_level();
        // All nodes are in a cycle, so they should be in the catch-all level.
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 3);
        assert!(!back_edges.is_empty());
    }

    #[test]
    fn topo_by_level_mixed_cycle_and_acyclic() {
        // A -> B -> C -> B (cycle on B-C), A -> D (acyclic).
        let (mut graph, nodes) = make_graph_with_nodes(4);
        let [a, b, c, d] = [nodes[0], nodes[1], nodes[2], nodes[3]];
        graph.queue_connect(a, b);
        graph.queue_connect(b, c);
        graph.queue_connect(c, b); // cycle
        graph.queue_connect(a, d);
        graph.apply_mutations();

        let (levels, back_edges) = graph.topological_order_by_level();
        // A should be at level 0, D at level 1.
        // B and C are in the cycle catch-all level.
        let total_nodes: usize = levels.iter().map(|l| l.len()).sum();
        assert_eq!(total_nodes, 4);
        // A must be in the earliest level.
        assert!(levels[0].contains(&a));
        // D should be processed (not in cycle).
        let d_level = levels.iter().position(|l| l.contains(&d));
        assert!(d_level.is_some());
        assert!(!back_edges.is_empty());
    }
}
