//! Power Networks Module for the Factorial engine.
//!
//! Models power production, consumption, and storage across independent
//! networks. Each tick the module balances supply and demand per network,
//! computes a satisfaction ratio (0..1 as [`Fixed64`]), and emits events
//! on state transitions (brownout/restored).
//!
//! # Design
//!
//! - Buildings are assigned to power networks via [`NodeId`].
//! - Each network tracks its own producers, consumers, and storage nodes.
//! - Per-node power specs are stored in the module (not in the core ECS).
//! - Satisfaction ratio affects building performance (applied externally).
//! - Events fire only on *transitions*, not every tick.

use std::collections::HashMap;

use factorial_core::fixed::{Fixed64, Ticks};
use factorial_core::id::NodeId;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Network identifier
// ---------------------------------------------------------------------------

/// Identifies a power network. Cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PowerNetworkId(pub u32);

// ---------------------------------------------------------------------------
// Per-node power specs
// ---------------------------------------------------------------------------

/// A node that produces power.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerProducer {
    /// Power output in watts (Fixed64).
    pub capacity: Fixed64,
}

/// A node that consumes power.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerConsumer {
    /// Power demand in watts (Fixed64).
    pub demand: Fixed64,
}

/// A node that stores power (battery, accumulator).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerStorage {
    /// Maximum charge capacity in joules (Fixed64).
    pub capacity: Fixed64,
    /// Current charge in joules (Fixed64). Clamped to [0, capacity].
    pub charge: Fixed64,
    /// Maximum charge/discharge rate per tick in watts (Fixed64).
    pub charge_rate: Fixed64,
}

// ---------------------------------------------------------------------------
// Power network
// ---------------------------------------------------------------------------

/// A single power network containing producers, consumers, and storage nodes.
///
/// The satisfaction ratio indicates how well demand is met:
/// - 1.0: all consumers fully powered
/// - 0.0: no power available
/// - Between: partial power (buildings operate at reduced efficiency)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerNetwork {
    /// Network identifier.
    pub id: PowerNetworkId,
    /// Producer node IDs (contiguous for cache-friendly iteration).
    pub producers: Vec<NodeId>,
    /// Consumer node IDs (contiguous for cache-friendly iteration).
    pub consumers: Vec<NodeId>,
    /// Storage node IDs (contiguous for cache-friendly iteration).
    pub storage: Vec<NodeId>,
    /// Current satisfaction ratio: 0.0 to 1.0 (Fixed64).
    pub satisfaction: Fixed64,
    /// Whether this network was in brownout state last tick.
    /// Used to detect transitions for event emission.
    pub was_brownout: bool,
}

impl PowerNetwork {
    /// Create a new empty power network.
    pub fn new(id: PowerNetworkId) -> Self {
        Self {
            id,
            producers: Vec::new(),
            consumers: Vec::new(),
            storage: Vec::new(),
            satisfaction: Fixed64::from_num(1),
            was_brownout: false,
        }
    }

    /// Add a producer node to this network.
    pub fn add_producer(&mut self, node: NodeId) {
        if !self.producers.contains(&node) {
            self.producers.push(node);
        }
    }

    /// Add a consumer node to this network.
    pub fn add_consumer(&mut self, node: NodeId) {
        if !self.consumers.contains(&node) {
            self.consumers.push(node);
        }
    }

    /// Add a storage node to this network.
    pub fn add_storage(&mut self, node: NodeId) {
        if !self.storage.contains(&node) {
            self.storage.push(node);
        }
    }

    /// Remove a producer node from this network.
    pub fn remove_producer(&mut self, node: NodeId) {
        self.producers.retain(|n| *n != node);
    }

    /// Remove a consumer node from this network.
    pub fn remove_consumer(&mut self, node: NodeId) {
        self.consumers.retain(|n| *n != node);
    }

    /// Remove a storage node from this network.
    pub fn remove_storage(&mut self, node: NodeId) {
        self.storage.retain(|n| *n != node);
    }

    /// Remove a node from any role in this network.
    pub fn remove_node(&mut self, node: NodeId) {
        self.remove_producer(node);
        self.remove_consumer(node);
        self.remove_storage(node);
    }
}

// ---------------------------------------------------------------------------
// Power events
// ---------------------------------------------------------------------------

/// Events emitted by the power module on state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerEvent {
    /// Emitted when a network transitions from satisfied to brownout.
    PowerGridBrownout {
        network_id: PowerNetworkId,
        /// The deficit: demand - (production + storage discharge).
        deficit: Fixed64,
        tick: Ticks,
    },
    /// Emitted when a network transitions from brownout to fully satisfied.
    PowerGridRestored {
        network_id: PowerNetworkId,
        tick: Ticks,
    },
}

// ---------------------------------------------------------------------------
// Power module
// ---------------------------------------------------------------------------

/// Manages all power networks and per-node power specifications.
///
/// The module is the top-level API for the power system. It owns both the
/// network topology and the per-node specs (producers, consumers, storage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerModule {
    /// All power networks, keyed by network ID.
    pub networks: HashMap<PowerNetworkId, PowerNetwork>,
    /// Per-node producer specs.
    pub producers: HashMap<NodeId, PowerProducer>,
    /// Per-node consumer specs.
    pub consumers: HashMap<NodeId, PowerConsumer>,
    /// Per-node storage specs (mutable charge state).
    pub storage: HashMap<NodeId, PowerStorage>,
    /// Next network ID to assign.
    next_network_id: u32,
}

impl Default for PowerModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerModule {
    /// Create a new empty power module.
    pub fn new() -> Self {
        Self {
            networks: HashMap::new(),
            producers: HashMap::new(),
            consumers: HashMap::new(),
            storage: HashMap::new(),
            next_network_id: 0,
        }
    }

    /// Create a new power network and return its ID.
    pub fn create_network(&mut self) -> PowerNetworkId {
        let id = PowerNetworkId(self.next_network_id);
        self.next_network_id += 1;
        self.networks.insert(id, PowerNetwork::new(id));
        id
    }

    /// Get a reference to a network by ID.
    pub fn network(&self, id: PowerNetworkId) -> Option<&PowerNetwork> {
        self.networks.get(&id)
    }

    /// Get a mutable reference to a network by ID.
    pub fn network_mut(&mut self, id: PowerNetworkId) -> Option<&mut PowerNetwork> {
        self.networks.get_mut(&id)
    }

    /// Remove a power network entirely.
    pub fn remove_network(&mut self, id: PowerNetworkId) {
        self.networks.remove(&id);
    }

    /// Register a producer node and add it to a network.
    pub fn add_producer(
        &mut self,
        network_id: PowerNetworkId,
        node: NodeId,
        producer: PowerProducer,
    ) {
        self.producers.insert(node, producer);
        if let Some(network) = self.networks.get_mut(&network_id) {
            network.add_producer(node);
        }
    }

    /// Register a consumer node and add it to a network.
    pub fn add_consumer(
        &mut self,
        network_id: PowerNetworkId,
        node: NodeId,
        consumer: PowerConsumer,
    ) {
        self.consumers.insert(node, consumer);
        if let Some(network) = self.networks.get_mut(&network_id) {
            network.add_consumer(node);
        }
    }

    /// Register a storage node and add it to a network.
    pub fn add_storage(
        &mut self,
        network_id: PowerNetworkId,
        node: NodeId,
        storage: PowerStorage,
    ) {
        self.storage.insert(node, storage);
        if let Some(network) = self.networks.get_mut(&network_id) {
            network.add_storage(node);
        }
    }

    /// Remove a node from the power system entirely (all networks and specs).
    pub fn remove_node(&mut self, node: NodeId) {
        self.producers.remove(&node);
        self.consumers.remove(&node);
        self.storage.remove(&node);
        for network in self.networks.values_mut() {
            network.remove_node(node);
        }
    }

    /// Get the satisfaction ratio for a network.
    pub fn satisfaction(&self, network_id: PowerNetworkId) -> Option<Fixed64> {
        self.networks.get(&network_id).map(|n| n.satisfaction)
    }

    /// Advance all power networks by one tick.
    ///
    /// For each network:
    /// 1. Sum total production from all producer nodes.
    /// 2. Sum total demand from all consumer nodes.
    /// 3. If production >= demand: satisfaction = 1.0, charge storage with excess.
    /// 4. If production < demand: discharge storage to cover deficit.
    ///    - If storage covers it: satisfaction = 1.0.
    ///    - Otherwise: satisfaction = (production + discharged) / demand.
    /// 5. Emit brownout/restored events on state transitions.
    ///
    /// Returns a list of events emitted this tick.
    pub fn tick(&mut self, current_tick: Ticks) -> Vec<PowerEvent> {
        let mut events = Vec::new();
        let zero = Fixed64::from_num(0);
        let one = Fixed64::from_num(1);

        // We need to iterate networks while also accessing producer/consumer/storage maps.
        // Collect network IDs to iterate, then process each.
        let network_ids: Vec<PowerNetworkId> = self.networks.keys().copied().collect();

        for net_id in network_ids {
            let network = self.networks.get(&net_id).unwrap();

            // Step 1: Sum total production.
            let total_production: Fixed64 = network
                .producers
                .iter()
                .filter_map(|node_id| self.producers.get(node_id))
                .map(|p| p.capacity)
                .fold(zero, |acc, val| acc + val);

            // Step 2: Sum total demand.
            let total_demand: Fixed64 = network
                .consumers
                .iter()
                .filter_map(|node_id| self.consumers.get(node_id))
                .map(|c| c.demand)
                .fold(zero, |acc, val| acc + val);

            // Collect storage node IDs for this network so we can mutate storage.
            let storage_nodes: Vec<NodeId> = network.storage.clone();
            let was_brownout = network.was_brownout;

            // Step 3 & 4: Balance production vs demand with storage.
            let satisfaction;
            let mut deficit = zero;

            if total_demand == zero {
                // No demand: fully satisfied. Charge storage with all production.
                satisfaction = one;
                let mut excess = total_production;
                for node_id in &storage_nodes {
                    if excess <= zero {
                        break;
                    }
                    if let Some(s) = self.storage.get_mut(node_id) {
                        let headroom = s.capacity - s.charge;
                        let can_charge = excess.min(s.charge_rate).min(headroom);
                        if can_charge > zero {
                            s.charge += can_charge;
                            excess -= can_charge;
                        }
                    }
                }
            } else if total_production >= total_demand {
                // Surplus: fully satisfied, charge storage with excess.
                satisfaction = one;
                let mut excess = total_production - total_demand;
                for node_id in &storage_nodes {
                    if excess <= zero {
                        break;
                    }
                    if let Some(s) = self.storage.get_mut(node_id) {
                        let headroom = s.capacity - s.charge;
                        let can_charge = excess.min(s.charge_rate).min(headroom);
                        if can_charge > zero {
                            s.charge += can_charge;
                            excess -= can_charge;
                        }
                    }
                }
            } else {
                // Deficit: try to cover with storage.
                let mut remaining_deficit = total_demand - total_production;
                for node_id in &storage_nodes {
                    if remaining_deficit <= zero {
                        break;
                    }
                    if let Some(s) = self.storage.get_mut(node_id) {
                        let can_discharge = remaining_deficit.min(s.charge_rate).min(s.charge);
                        if can_discharge > zero {
                            s.charge -= can_discharge;
                            remaining_deficit -= can_discharge;
                        }
                    }
                }

                if remaining_deficit <= zero {
                    // Storage covered the deficit.
                    satisfaction = one;
                } else {
                    // Partial satisfaction.
                    let supplied = total_demand - remaining_deficit;
                    // satisfaction = supplied / total_demand, clamped to [0, 1].
                    satisfaction = if total_demand > zero {
                        let ratio = supplied / total_demand;
                        if ratio > one { one } else if ratio < zero { zero } else { ratio }
                    } else {
                        one
                    };
                    deficit = remaining_deficit;
                }
            }

            // Update network state.
            let network = self.networks.get_mut(&net_id).unwrap();
            network.satisfaction = satisfaction;

            let is_brownout = satisfaction < one;

            // Step 5: Emit events on state transitions only.
            if is_brownout && !was_brownout {
                network.was_brownout = true;
                events.push(PowerEvent::PowerGridBrownout {
                    network_id: net_id,
                    deficit,
                    tick: current_tick,
                });
            } else if !is_brownout && was_brownout {
                network.was_brownout = false;
                events.push(PowerEvent::PowerGridRestored {
                    network_id: net_id,
                    tick: current_tick,
                });
            }
        }

        events
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn fixed(v: f64) -> Fixed64 {
        Fixed64::from_num(v)
    }

    fn make_node_ids(count: usize) -> Vec<NodeId> {
        let mut sm = SlotMap::<NodeId, ()>::with_key();
        (0..count).map(|_| sm.insert(())).collect()
    }

    fn make_node_id() -> NodeId {
        make_node_ids(1).into_iter().next().unwrap()
    }

    // -----------------------------------------------------------------------
    // Test 1: Balanced network — satisfaction equals exactly 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn balanced_network_satisfaction_is_one() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(2);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(100.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });

        let events = module.tick(1);

        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty(), "no events on balanced network");
    }

    // -----------------------------------------------------------------------
    // Test 2: Over-powered network — satisfaction is 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn overpowered_network_satisfaction_is_one() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(2);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(200.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(50.0) });

        let events = module.tick(1);

        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 3: Under-powered network — satisfaction < 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn underpowered_network_satisfaction_below_one() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(2);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(50.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });

        let events = module.tick(1);

        let satisfaction = module.satisfaction(net).unwrap();
        // 50 / 100 = 0.5
        assert_eq!(satisfaction, fixed(0.5));
        assert!(satisfaction < Fixed64::from_num(1));

        // Should emit brownout event.
        assert_eq!(events.len(), 1);
        match &events[0] {
            PowerEvent::PowerGridBrownout { network_id, deficit, tick } => {
                assert_eq!(*network_id, net);
                assert_eq!(*deficit, fixed(50.0));
                assert_eq!(*tick, 1);
            }
            _ => panic!("expected PowerGridBrownout"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: Zero demand — satisfaction is 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn zero_demand_satisfaction_is_one() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let node = make_node_id();

        module.add_producer(net, node, PowerProducer { capacity: fixed(100.0) });
        // No consumers.

        let events = module.tick(1);

        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 5: Empty network — satisfaction is 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn empty_network_satisfaction_is_one() {
        let mut module = PowerModule::new();
        let net = module.create_network();

        let events = module.tick(1);

        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 6: Storage charges with excess production
    // -----------------------------------------------------------------------
    #[test]
    fn storage_charges_with_excess() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(150.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });
        module.add_storage(
            net,
            nodes[2],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(0.0),
                charge_rate: fixed(100.0),
            },
        );

        let events = module.tick(1);

        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty());

        // Storage should have charged by 50 (excess = 150 - 100 = 50, rate allows up to 100).
        let storage = module.storage.get(&nodes[2]).unwrap();
        assert_eq!(storage.charge, fixed(50.0));
    }

    // -----------------------------------------------------------------------
    // Test 7: Storage discharges during deficit
    // -----------------------------------------------------------------------
    #[test]
    fn storage_discharges_during_deficit() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(50.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });
        module.add_storage(
            net,
            nodes[2],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(500.0),
                charge_rate: fixed(100.0),
            },
        );

        let events = module.tick(1);

        // Storage should cover the 50-unit deficit.
        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty());

        // Storage should have discharged by 50.
        let storage = module.storage.get(&nodes[2]).unwrap();
        assert_eq!(storage.charge, fixed(450.0));
    }

    // -----------------------------------------------------------------------
    // Test 8: Storage partially covers deficit — brownout
    // -----------------------------------------------------------------------
    #[test]
    fn storage_partially_covers_deficit_brownout() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(30.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });
        module.add_storage(
            net,
            nodes[2],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(20.0),
                charge_rate: fixed(100.0),
            },
        );

        let events = module.tick(1);

        // Production=30, storage discharges 20, total supplied=50, demand=100.
        // Satisfaction = 50/100 = 0.5.
        let satisfaction = module.satisfaction(net).unwrap();
        assert_eq!(satisfaction, fixed(0.5));

        // Storage should be fully drained.
        let storage = module.storage.get(&nodes[2]).unwrap();
        assert_eq!(storage.charge, fixed(0.0));

        // Brownout event emitted.
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PowerEvent::PowerGridBrownout { .. }));
    }

    // -----------------------------------------------------------------------
    // Test 9: Brownout event fires only on transition, not every tick
    // -----------------------------------------------------------------------
    #[test]
    fn brownout_event_fires_only_on_transition() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(2);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(50.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });

        // Tick 1: transition to brownout -> event.
        let events = module.tick(1);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PowerEvent::PowerGridBrownout { .. }));

        // Tick 2: still in brownout -> NO event.
        let events = module.tick(2);
        assert!(events.is_empty(), "no event when already in brownout");

        // Tick 3: still in brownout -> NO event.
        let events = module.tick(3);
        assert!(events.is_empty(), "no event when still in brownout");
    }

    // -----------------------------------------------------------------------
    // Test 10: Restored event fires on recovery
    // -----------------------------------------------------------------------
    #[test]
    fn restored_event_fires_on_recovery() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        // Start underpowered.
        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(50.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });

        // Tick 1: brownout.
        let events = module.tick(1);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PowerEvent::PowerGridBrownout { .. }));

        // Add another producer to meet demand.
        module.add_producer(net, nodes[2], PowerProducer { capacity: fixed(50.0) });

        // Tick 2: restored.
        let events = module.tick(2);
        assert_eq!(events.len(), 1);
        match &events[0] {
            PowerEvent::PowerGridRestored { network_id, tick } => {
                assert_eq!(*network_id, net);
                assert_eq!(*tick, 2);
            }
            _ => panic!("expected PowerGridRestored"),
        }

        // Tick 3: still satisfied -> no event.
        let events = module.tick(3);
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 11: Multiple networks are independent
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_networks_independent() {
        let mut module = PowerModule::new();
        let net_a = module.create_network();
        let net_b = module.create_network();
        let nodes = make_node_ids(4);

        // Network A: balanced.
        module.add_producer(net_a, nodes[0], PowerProducer { capacity: fixed(100.0) });
        module.add_consumer(net_a, nodes[1], PowerConsumer { demand: fixed(100.0) });

        // Network B: underpowered.
        module.add_producer(net_b, nodes[2], PowerProducer { capacity: fixed(25.0) });
        module.add_consumer(net_b, nodes[3], PowerConsumer { demand: fixed(100.0) });

        let events = module.tick(1);

        // A is satisfied.
        assert_eq!(module.satisfaction(net_a), Some(Fixed64::from_num(1)));

        // B is not.
        let sat_b = module.satisfaction(net_b).unwrap();
        assert_eq!(sat_b, fixed(0.25));

        // Only one brownout event (for network B).
        assert_eq!(events.len(), 1);
        match &events[0] {
            PowerEvent::PowerGridBrownout { network_id, .. } => {
                assert_eq!(*network_id, net_b);
            }
            _ => panic!("expected PowerGridBrownout for network B"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 12: Storage charge rate is respected
    // -----------------------------------------------------------------------
    #[test]
    fn storage_charge_rate_limits_charging() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(2);

        // 200 excess production, but storage can only charge at 30/tick.
        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(200.0) });
        module.add_storage(
            net,
            nodes[1],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(0.0),
                charge_rate: fixed(30.0),
            },
        );

        module.tick(1);

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.charge, fixed(30.0));
    }

    // -----------------------------------------------------------------------
    // Test 13: Storage discharge rate is respected
    // -----------------------------------------------------------------------
    #[test]
    fn storage_discharge_rate_limits_discharge() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(2);

        // 100 demand, 0 production, storage has plenty but rate-limited to 40/tick.
        module.add_consumer(net, nodes[0], PowerConsumer { demand: fixed(100.0) });
        module.add_storage(
            net,
            nodes[1],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(500.0),
                charge_rate: fixed(40.0),
            },
        );

        let events = module.tick(1);

        // Can only discharge 40, so satisfaction = 40/100 = 0.4.
        let satisfaction = module.satisfaction(net).unwrap();
        assert_eq!(satisfaction, fixed(0.4));

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.charge, fixed(460.0));

        // Brownout event.
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PowerEvent::PowerGridBrownout { .. }));
    }

    // -----------------------------------------------------------------------
    // Test 14: Storage capacity is respected (no overcharge)
    // -----------------------------------------------------------------------
    #[test]
    fn storage_does_not_overcharge() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(2);

        // 100 excess, storage almost full.
        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(100.0) });
        module.add_storage(
            net,
            nodes[1],
            PowerStorage {
                capacity: fixed(50.0),
                charge: fixed(45.0),
                charge_rate: fixed(100.0),
            },
        );

        module.tick(1);

        // Should charge at most 5 (headroom = 50 - 45).
        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.charge, fixed(50.0));
    }

    // -----------------------------------------------------------------------
    // Test 15: Storage does not discharge below zero
    // -----------------------------------------------------------------------
    #[test]
    fn storage_does_not_discharge_below_zero() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(2);

        module.add_consumer(net, nodes[0], PowerConsumer { demand: fixed(100.0) });
        module.add_storage(
            net,
            nodes[1],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(10.0),
                charge_rate: fixed(200.0),
            },
        );

        module.tick(1);

        // Only 10 units available, demand 100. Supplied = 10, satisfaction = 10/100.
        let satisfaction = module.satisfaction(net).unwrap();
        let expected = Fixed64::from_num(10) / Fixed64::from_num(100);
        assert_eq!(satisfaction, expected);

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.charge, fixed(0.0));
    }

    // -----------------------------------------------------------------------
    // Test 16: Multiple producers sum correctly
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_producers_sum() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(4);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(30.0) });
        module.add_producer(net, nodes[1], PowerProducer { capacity: fixed(40.0) });
        module.add_producer(net, nodes[2], PowerProducer { capacity: fixed(30.0) });
        module.add_consumer(net, nodes[3], PowerConsumer { demand: fixed(100.0) });

        let events = module.tick(1);

        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 17: Multiple consumers sum correctly
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_consumers_sum() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(4);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(100.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(40.0) });
        module.add_consumer(net, nodes[2], PowerConsumer { demand: fixed(30.0) });
        module.add_consumer(net, nodes[3], PowerConsumer { demand: fixed(30.0) });

        let events = module.tick(1);

        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 18: Multiple storage nodes used in order
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_storage_nodes_charge() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        // 100 excess, two storage nodes each with rate 60.
        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(100.0) });
        module.add_storage(
            net,
            nodes[1],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(0.0),
                charge_rate: fixed(60.0),
            },
        );
        module.add_storage(
            net,
            nodes[2],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(0.0),
                charge_rate: fixed(60.0),
            },
        );

        module.tick(1);

        // First storage gets min(100, 60) = 60. Remaining excess = 40.
        // Second storage gets min(40, 60) = 40.
        let s1 = module.storage.get(&nodes[1]).unwrap();
        let s2 = module.storage.get(&nodes[2]).unwrap();
        assert_eq!(s1.charge, fixed(60.0));
        assert_eq!(s2.charge, fixed(40.0));
    }

    // -----------------------------------------------------------------------
    // Test 19: Remove node clears from network and specs
    // -----------------------------------------------------------------------
    #[test]
    fn remove_node_clears_everything() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(100.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(50.0) });
        module.add_storage(
            net,
            nodes[2],
            PowerStorage {
                capacity: fixed(100.0),
                charge: fixed(50.0),
                charge_rate: fixed(10.0),
            },
        );

        // Remove consumer.
        module.remove_node(nodes[1]);

        assert!(!module.consumers.contains_key(&nodes[1]));
        let network = module.network(net).unwrap();
        assert!(!network.consumers.contains(&nodes[1]));

        // Remove producer.
        module.remove_node(nodes[0]);
        assert!(!module.producers.contains_key(&nodes[0]));
        let network = module.network(net).unwrap();
        assert!(!network.producers.contains(&nodes[0]));

        // Remove storage.
        module.remove_node(nodes[2]);
        assert!(!module.storage.contains_key(&nodes[2]));
        let network = module.network(net).unwrap();
        assert!(!network.storage.contains(&nodes[2]));
    }

    // -----------------------------------------------------------------------
    // Test 20: Brownout deficit value is accurate
    // -----------------------------------------------------------------------
    #[test]
    fn brownout_deficit_value_is_accurate() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        // Production=30, storage discharge=20, demand=100, deficit=50.
        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(30.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });
        module.add_storage(
            net,
            nodes[2],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(20.0),
                charge_rate: fixed(100.0),
            },
        );

        let events = module.tick(1);

        assert_eq!(events.len(), 1);
        match &events[0] {
            PowerEvent::PowerGridBrownout { deficit, .. } => {
                assert_eq!(*deficit, fixed(50.0));
            }
            _ => panic!("expected PowerGridBrownout"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 21: Full cycle — brownout then restored
    // -----------------------------------------------------------------------
    #[test]
    fn full_cycle_brownout_then_restored() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(50.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });

        // Tick 1: brownout.
        let events = module.tick(1);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PowerEvent::PowerGridBrownout { .. }));
        assert_eq!(module.satisfaction(net).unwrap(), fixed(0.5));

        // Tick 2: still brownout, no event.
        let events = module.tick(2);
        assert!(events.is_empty());

        // Add producer to meet demand.
        module.add_producer(net, nodes[2], PowerProducer { capacity: fixed(50.0) });

        // Tick 3: restored.
        let events = module.tick(3);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PowerEvent::PowerGridRestored { .. }));
        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));

        // Tick 4: still satisfied, no event.
        let events = module.tick(4);
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 22: Storage absorbs excess then releases during deficit
    // -----------------------------------------------------------------------
    #[test]
    fn storage_absorbs_then_releases() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(150.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(100.0) });
        module.add_storage(
            net,
            nodes[2],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(0.0),
                charge_rate: fixed(200.0),
            },
        );

        // Tick 1: excess 50 charges storage.
        module.tick(1);
        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert_eq!(module.storage.get(&nodes[2]).unwrap().charge, fixed(50.0));

        // Tick 2: excess 50 more.
        module.tick(2);
        assert_eq!(module.storage.get(&nodes[2]).unwrap().charge, fixed(100.0));

        // Now reduce production to create deficit.
        module.producers.get_mut(&nodes[0]).unwrap().capacity = fixed(60.0);

        // Tick 3: deficit of 40, storage covers it.
        module.tick(3);
        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert_eq!(module.storage.get(&nodes[2]).unwrap().charge, fixed(60.0));

        // Tick 4: deficit of 40 again.
        module.tick(4);
        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert_eq!(module.storage.get(&nodes[2]).unwrap().charge, fixed(20.0));

        // Tick 5: deficit of 40, but only 20 in storage. Partial brownout.
        let events = module.tick(5);
        let satisfaction = module.satisfaction(net).unwrap();
        // Supplied = 60 + 20 = 80, demand = 100. Satisfaction = 80/100.
        let expected = Fixed64::from_num(80) / Fixed64::from_num(100);
        assert_eq!(satisfaction, expected);
        assert_eq!(module.storage.get(&nodes[2]).unwrap().charge, fixed(0.0));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PowerEvent::PowerGridBrownout { .. }));
    }

    // -----------------------------------------------------------------------
    // Test 23: Network creation assigns unique IDs
    // -----------------------------------------------------------------------
    #[test]
    fn network_ids_are_unique() {
        let mut module = PowerModule::new();
        let a = module.create_network();
        let b = module.create_network();
        let c = module.create_network();

        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    // -----------------------------------------------------------------------
    // Test 24: Remove network
    // -----------------------------------------------------------------------
    #[test]
    fn remove_network_works() {
        let mut module = PowerModule::new();
        let net = module.create_network();

        assert!(module.network(net).is_some());
        module.remove_network(net);
        assert!(module.network(net).is_none());
        assert!(module.satisfaction(net).is_none());
    }

    // -----------------------------------------------------------------------
    // Test 25: No duplicate nodes in network
    // -----------------------------------------------------------------------
    #[test]
    fn no_duplicate_nodes_in_network() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let node = make_node_id();

        module.add_producer(net, node, PowerProducer { capacity: fixed(100.0) });
        // Adding same node again should not duplicate.
        module.add_producer(net, node, PowerProducer { capacity: fixed(200.0) });

        let network = module.network(net).unwrap();
        assert_eq!(network.producers.len(), 1);

        // But the spec should be updated.
        assert_eq!(module.producers.get(&node).unwrap().capacity, fixed(200.0));
    }

    // -----------------------------------------------------------------------
    // Test 26: Zero production with demand — full brownout
    // -----------------------------------------------------------------------
    #[test]
    fn zero_production_full_brownout() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let node = make_node_id();

        module.add_consumer(net, node, PowerConsumer { demand: fixed(100.0) });

        let events = module.tick(1);

        assert_eq!(module.satisfaction(net).unwrap(), fixed(0.0));
        assert_eq!(events.len(), 1);
        match &events[0] {
            PowerEvent::PowerGridBrownout { deficit, .. } => {
                assert_eq!(*deficit, fixed(100.0));
            }
            _ => panic!("expected PowerGridBrownout"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 27: Serialization round-trip (PowerModule is Serialize + Deserialize)
    // -----------------------------------------------------------------------
    #[test]
    fn power_module_is_serializable() {
        // Verify that PowerModule derives Serialize/Deserialize by constructing
        // and verifying the trait bound compiles. A real round-trip would require
        // a serializer, but the derive is the key check.
        fn assert_serde<T: serde::Serialize + serde::de::DeserializeOwned>() {}
        assert_serde::<PowerModule>();
        assert_serde::<PowerNetwork>();
        assert_serde::<PowerProducer>();
        assert_serde::<PowerConsumer>();
        assert_serde::<PowerStorage>();
        assert_serde::<PowerNetworkId>();
    }

    // -----------------------------------------------------------------------
    // Test 28: Storage fully covers deficit — no brownout
    // -----------------------------------------------------------------------
    #[test]
    fn storage_fully_covers_deficit_no_brownout() {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(3);

        // 0 production, 50 demand, storage has 500 with rate 100.
        module.add_consumer(net, nodes[0], PowerConsumer { demand: fixed(50.0) });
        module.add_storage(
            net,
            nodes[1],
            PowerStorage {
                capacity: fixed(1000.0),
                charge: fixed(500.0),
                charge_rate: fixed(100.0),
            },
        );

        let events = module.tick(1);

        // Storage covers all demand.
        assert_eq!(module.satisfaction(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty());

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.charge, fixed(450.0));
    }
}
