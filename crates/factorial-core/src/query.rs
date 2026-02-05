//! Read-only query API for inspecting simulation state.
//!
//! Provides snapshot types that aggregate engine state into convenient views
//! for rendering, UI, and FFI consumers. All types are owned copies -- no
//! references into internal engine storage.

use crate::fixed::Fixed64;
use crate::id::{BuildingTypeId, EdgeId, NodeId};
use crate::item::ItemStack;
use crate::processor::ProcessorState;

// ---------------------------------------------------------------------------
// Node snapshot
// ---------------------------------------------------------------------------

/// An aggregated, read-only view of a single node in the production graph.
///
/// Contains a copy of the processor state, progress fraction, and inventory
/// summaries. Suitable for passing across FFI boundaries or to rendering code.
#[derive(Debug, Clone)]
pub struct NodeSnapshot {
    /// The node's ID in the production graph.
    pub id: NodeId,
    /// The building type this node was created from.
    pub building_type: BuildingTypeId,
    /// Current processor state (Idle, Working, Stalled).
    pub processor_state: ProcessorState,
    /// Progress as a 0..1 fraction. 0 when idle or stalled.
    pub progress: Fixed64,
    /// Summary of items in the input inventory.
    pub input_contents: Vec<ItemStack>,
    /// Summary of items in the output inventory.
    pub output_contents: Vec<ItemStack>,
    /// Edges feeding into this node.
    pub input_edges: Vec<EdgeId>,
    /// Edges leaving this node.
    pub output_edges: Vec<EdgeId>,
}

// ---------------------------------------------------------------------------
// Transport snapshot
// ---------------------------------------------------------------------------

/// An aggregated, read-only view of a single transport edge.
///
/// Provides utilization (how full the transport is), items currently in
/// transit, and endpoint information.
#[derive(Debug, Clone)]
pub struct TransportSnapshot {
    /// The edge's ID in the production graph.
    pub id: EdgeId,
    /// Source node.
    pub from: NodeId,
    /// Destination node.
    pub to: NodeId,
    /// Utilization as a 0..1 fraction (how full the transport is).
    pub utilization: Fixed64,
    /// Total items currently in transit within this transport.
    pub items_in_transit: u32,
}
