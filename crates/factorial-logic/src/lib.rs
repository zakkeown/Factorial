//! Logic/Circuit Networks Module for the Factorial engine.
//!
//! Models wire-based signal networks where buildings share typed signals
//! (ItemTypeId -> Fixed64). Supports constant combinators, inventory readers,
//! arithmetic/decider combinators, and per-node circuit control conditions.
//!
//! Signal propagation uses a one-tick delay on combinator outputs to prevent
//! infinite feedback loops and ensure deterministic evaluation order.

pub mod combinator;
pub mod condition;

use std::collections::BTreeMap;

use factorial_core::fixed::{Fixed64, Ticks};
use factorial_core::id::{ItemTypeId, NodeId};
use serde::{Deserialize, Serialize};

use combinator::{ArithmeticCombinator, DeciderCombinator};
use condition::{CircuitControl, InventoryReader};

// ---------------------------------------------------------------------------
// Signal set
// ---------------------------------------------------------------------------

/// A set of signals: item type -> value. Sparse -- only non-zero signals stored.
pub type SignalSet = BTreeMap<ItemTypeId, Fixed64>;

// ---------------------------------------------------------------------------
// Wire network types
// ---------------------------------------------------------------------------

/// Identifies a wire network. Cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WireNetworkId(pub u32);

/// Wire color -- buildings can connect to one red and one green network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WireColor {
    Red,
    Green,
}

/// A wire network containing member nodes and their merged signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireNetwork {
    pub id: WireNetworkId,
    pub color: WireColor,
    pub members: Vec<NodeId>,
    /// Merged signal set, recomputed each tick.
    pub signals: SignalSet,
}

impl WireNetwork {
    pub fn new(id: WireNetworkId, color: WireColor) -> Self {
        Self {
            id,
            color,
            members: Vec::new(),
            signals: SignalSet::new(),
        }
    }

    pub fn add_member(&mut self, node: NodeId) {
        if !self.members.contains(&node) {
            self.members.push(node);
        }
    }

    pub fn remove_member(&mut self, node: NodeId) {
        self.members.retain(|n| *n != node);
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events emitted by the logic module on state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicEvent {
    /// A node's circuit control transitioned from inactive to active.
    CircuitActivated { node: NodeId, tick: Ticks },
    /// A node's circuit control transitioned from active to inactive.
    CircuitDeactivated { node: NodeId, tick: Ticks },
    /// A wire network's merged signals changed from last tick.
    NetworkSignalsChanged { network: WireNetworkId, tick: Ticks },
}

// ---------------------------------------------------------------------------
// Constant combinator
// ---------------------------------------------------------------------------

/// Outputs a fixed set of signals every tick when enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantCombinator {
    pub signals: SignalSet,
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Logic module
// ---------------------------------------------------------------------------

/// Manages all wire networks, signal sources, and circuit controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicModule {
    // Topology
    pub networks: BTreeMap<WireNetworkId, WireNetwork>,

    // Per-node signal sources
    pub constants: BTreeMap<NodeId, ConstantCombinator>,
    pub inventory_readers: BTreeMap<NodeId, InventoryReader>,
    pub arithmetic_combinators: BTreeMap<NodeId, ArithmeticCombinator>,
    pub decider_combinators: BTreeMap<NodeId, DeciderCombinator>,

    // Per-node signal consumption
    pub circuit_controls: BTreeMap<NodeId, CircuitControl>,

    // Internal: combinator outputs from last tick (for one-tick delay)
    combinator_outputs: BTreeMap<NodeId, SignalSet>,

    // Internal: previous network signals (for change detection)
    prev_signals: BTreeMap<WireNetworkId, SignalSet>,

    next_network_id: u32,
}

impl Default for LogicModule {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicModule {
    pub fn new() -> Self {
        Self {
            networks: BTreeMap::new(),
            constants: BTreeMap::new(),
            inventory_readers: BTreeMap::new(),
            arithmetic_combinators: BTreeMap::new(),
            decider_combinators: BTreeMap::new(),
            circuit_controls: BTreeMap::new(),
            combinator_outputs: BTreeMap::new(),
            prev_signals: BTreeMap::new(),
            next_network_id: 0,
        }
    }

    // --- Network management ---

    pub fn create_network(&mut self, color: WireColor) -> WireNetworkId {
        let id = WireNetworkId(self.next_network_id);
        self.next_network_id += 1;
        self.networks.insert(id, WireNetwork::new(id, color));
        id
    }

    pub fn remove_network(&mut self, id: WireNetworkId) {
        self.networks.remove(&id);
        self.prev_signals.remove(&id);
    }

    pub fn add_to_network(&mut self, network: WireNetworkId, node: NodeId) {
        if let Some(net) = self.networks.get_mut(&network) {
            net.add_member(node);
        }
    }

    pub fn remove_from_network(&mut self, network: WireNetworkId, node: NodeId) {
        if let Some(net) = self.networks.get_mut(&network) {
            net.remove_member(node);
        }
    }

    // --- Queries ---

    pub fn network_signals(&self, network: WireNetworkId) -> Option<&SignalSet> {
        self.networks.get(&network).map(|n| &n.signals)
    }

    pub fn is_active(&self, node: NodeId) -> Option<bool> {
        self.circuit_controls.get(&node).map(|c| c.active)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node_ids(count: usize) -> Vec<NodeId> {
        let mut sm = slotmap::SlotMap::<NodeId, ()>::with_key();
        (0..count).map(|_| sm.insert(())).collect()
    }

    #[test]
    fn wire_network_creation_and_membership() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(2);
        let net = module.create_network(WireColor::Red);

        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[1]);

        let network = module.networks.get(&net).unwrap();
        assert_eq!(network.color, WireColor::Red);
        assert_eq!(network.members.len(), 2);
        assert!(network.members.contains(&nodes[0]));
        assert!(network.members.contains(&nodes[1]));
    }

    #[test]
    fn network_ids_are_unique() {
        let mut module = LogicModule::new();
        let a = module.create_network(WireColor::Red);
        let b = module.create_network(WireColor::Green);
        let c = module.create_network(WireColor::Red);
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn remove_from_network() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(2);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[1]);

        module.remove_from_network(net, nodes[0]);

        let network = module.networks.get(&net).unwrap();
        assert_eq!(network.members.len(), 1);
        assert!(!network.members.contains(&nodes[0]));
        assert!(network.members.contains(&nodes[1]));
    }

    #[test]
    fn remove_network() {
        let mut module = LogicModule::new();
        let net = module.create_network(WireColor::Red);
        assert!(module.networks.contains_key(&net));
        module.remove_network(net);
        assert!(!module.networks.contains_key(&net));
    }

    #[test]
    fn no_duplicate_members() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[0]); // duplicate
        let network = module.networks.get(&net).unwrap();
        assert_eq!(network.members.len(), 1);
    }

    #[test]
    fn empty_network_has_no_signals() {
        let module = LogicModule::new();
        assert!(module.network_signals(WireNetworkId(999)).is_none());
    }
}
