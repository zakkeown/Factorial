//! Fluid Networks Module for the Factorial engine.
//!
//! Models fluid production, consumption, storage, and pipe transport across
//! independent networks. Each tick the module balances supply and demand per
//! network, computes a pressure ratio (0..1 as [`Fixed64`]), and emits events
//! on state transitions (low pressure/restored) and storage boundaries
//! (full/empty).
//!
//! # Design
//!
//! - Buildings are assigned to fluid networks via [`NodeId`].
//! - Each network carries a single fluid type ([`ItemTypeId`]).
//! - Each network tracks its own producers, consumers, storage, and pipe nodes.
//! - Per-node fluid specs are stored in the module (not in the core ECS).
//! - Pressure ratio affects building performance (applied externally).
//! - Events fire only on *transitions*, not every tick.

pub mod bridge;
pub use bridge::FluidBridge;

use std::collections::BTreeMap;

use factorial_core::fixed::{Fixed64, Ticks};
use factorial_core::id::{ItemTypeId, NodeId};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Network identifier
// ---------------------------------------------------------------------------

/// Identifies a fluid network. Cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FluidNetworkId(pub u32);

// ---------------------------------------------------------------------------
// Per-node fluid specs
// ---------------------------------------------------------------------------

/// A pipe that carries fluid in a network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FluidPipe {
    /// Maximum flow rate through this pipe (Fixed64).
    pub capacity: Fixed64,
}

/// A node that produces fluid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FluidProducer {
    /// Fluid produced per tick (Fixed64).
    pub rate: Fixed64,
}

/// A node that consumes fluid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FluidConsumer {
    /// Fluid consumed per tick (Fixed64).
    pub rate: Fixed64,
}

/// A node that stores fluid (tank, reservoir).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FluidStorage {
    /// Maximum volume (Fixed64).
    pub capacity: Fixed64,
    /// Current volume (Fixed64). Clamped to [0, capacity].
    pub current: Fixed64,
    /// Maximum fill/drain rate per tick (Fixed64).
    pub fill_rate: Fixed64,
}

// ---------------------------------------------------------------------------
// Fluid network
// ---------------------------------------------------------------------------

/// A single fluid network containing producers, consumers, storage, and pipes.
///
/// The pressure ratio indicates how well demand is met:
/// - 1.0: all consumers fully supplied
/// - 0.0: no fluid available
/// - Between: partial supply (buildings operate at reduced efficiency)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluidNetwork {
    /// Network identifier.
    pub id: FluidNetworkId,
    /// What fluid type this network carries.
    pub fluid_type: ItemTypeId,
    /// Producer node IDs (contiguous for cache-friendly iteration).
    pub producers: Vec<NodeId>,
    /// Consumer node IDs (contiguous for cache-friendly iteration).
    pub consumers: Vec<NodeId>,
    /// Storage node IDs (contiguous for cache-friendly iteration).
    pub storage: Vec<NodeId>,
    /// Pipe node IDs (contiguous for cache-friendly iteration).
    pub pipes: Vec<NodeId>,
    /// Current pressure ratio: 0.0 to 1.0 (Fixed64).
    pub pressure: Fixed64,
    /// Whether this network was in low-pressure state last tick.
    /// Used to detect transitions for event emission.
    pub was_low_pressure: bool,
}

impl FluidNetwork {
    /// Create a new empty fluid network.
    pub fn new(id: FluidNetworkId, fluid_type: ItemTypeId) -> Self {
        Self {
            id,
            fluid_type,
            producers: Vec::new(),
            consumers: Vec::new(),
            storage: Vec::new(),
            pipes: Vec::new(),
            pressure: Fixed64::from_num(1),
            was_low_pressure: false,
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

    /// Add a pipe node to this network.
    pub fn add_pipe(&mut self, node: NodeId) {
        if !self.pipes.contains(&node) {
            self.pipes.push(node);
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

    /// Remove a pipe node from this network.
    pub fn remove_pipe(&mut self, node: NodeId) {
        self.pipes.retain(|n| *n != node);
    }

    /// Remove a node from any role in this network.
    pub fn remove_node(&mut self, node: NodeId) {
        self.remove_producer(node);
        self.remove_consumer(node);
        self.remove_storage(node);
        self.remove_pipe(node);
    }
}

// ---------------------------------------------------------------------------
// Fluid events
// ---------------------------------------------------------------------------

/// Events emitted by the fluid module on state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FluidEvent {
    /// Emitted when a network transitions from adequate to low pressure.
    PressureLow {
        network_id: FluidNetworkId,
        pressure: Fixed64,
        tick: Ticks,
    },
    /// Emitted when a network transitions from low pressure to fully satisfied.
    PressureRestored {
        network_id: FluidNetworkId,
        tick: Ticks,
    },
    /// Emitted when a storage node reaches capacity.
    StorageFull {
        network_id: FluidNetworkId,
        node: NodeId,
        tick: Ticks,
    },
    /// Emitted when a storage node is completely drained.
    StorageEmpty {
        network_id: FluidNetworkId,
        node: NodeId,
        tick: Ticks,
    },
}

// ---------------------------------------------------------------------------
// Fluid module
// ---------------------------------------------------------------------------

/// Manages all fluid networks and per-node fluid specifications.
///
/// The module is the top-level API for the fluid system. It owns both the
/// network topology and the per-node specs (producers, consumers, storage, pipes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluidModule {
    /// All fluid networks, keyed by network ID.
    pub networks: BTreeMap<FluidNetworkId, FluidNetwork>,
    /// Per-node producer specs.
    pub producers: BTreeMap<NodeId, FluidProducer>,
    /// Per-node consumer specs.
    pub consumers: BTreeMap<NodeId, FluidConsumer>,
    /// Per-node storage specs (mutable volume state).
    pub storage: BTreeMap<NodeId, FluidStorage>,
    /// Per-node pipe specs.
    pub pipes: BTreeMap<NodeId, FluidPipe>,
    /// Next network ID to assign.
    next_network_id: u32,
    /// Per-consumer fluid consumption this tick, keyed by (network, node).
    /// Updated each tick during fluid distribution.
    #[serde(default)]
    pub consumer_consumption: BTreeMap<(FluidNetworkId, NodeId), Fixed64>,
}

impl Default for FluidModule {
    fn default() -> Self {
        Self::new()
    }
}

impl FluidModule {
    /// Create a new empty fluid module.
    pub fn new() -> Self {
        Self {
            networks: BTreeMap::new(),
            producers: BTreeMap::new(),
            consumers: BTreeMap::new(),
            storage: BTreeMap::new(),
            pipes: BTreeMap::new(),
            next_network_id: 0,
            consumer_consumption: BTreeMap::new(),
        }
    }

    /// Create a new fluid network for a given fluid type and return its ID.
    pub fn create_network(&mut self, fluid_type: ItemTypeId) -> FluidNetworkId {
        let id = FluidNetworkId(self.next_network_id);
        self.next_network_id += 1;
        self.networks
            .insert(id, FluidNetwork::new(id, fluid_type));
        id
    }

    /// Get a reference to a network by ID.
    pub fn network(&self, id: FluidNetworkId) -> Option<&FluidNetwork> {
        self.networks.get(&id)
    }

    /// Get a mutable reference to a network by ID.
    pub fn network_mut(&mut self, id: FluidNetworkId) -> Option<&mut FluidNetwork> {
        self.networks.get_mut(&id)
    }

    /// Remove a fluid network entirely.
    pub fn remove_network(&mut self, id: FluidNetworkId) {
        self.networks.remove(&id);
    }

    /// Register a producer node and add it to a network.
    pub fn add_producer(
        &mut self,
        network_id: FluidNetworkId,
        node: NodeId,
        producer: FluidProducer,
    ) {
        self.producers.insert(node, producer);
        if let Some(network) = self.networks.get_mut(&network_id) {
            network.add_producer(node);
        }
    }

    /// Register a consumer node and add it to a network.
    pub fn add_consumer(
        &mut self,
        network_id: FluidNetworkId,
        node: NodeId,
        consumer: FluidConsumer,
    ) {
        self.consumers.insert(node, consumer);
        if let Some(network) = self.networks.get_mut(&network_id) {
            network.add_consumer(node);
        }
    }

    /// Register a storage node and add it to a network.
    pub fn add_storage(
        &mut self,
        network_id: FluidNetworkId,
        node: NodeId,
        storage: FluidStorage,
    ) {
        self.storage.insert(node, storage);
        if let Some(network) = self.networks.get_mut(&network_id) {
            network.add_storage(node);
        }
    }

    /// Register a pipe node and add it to a network.
    pub fn add_pipe(
        &mut self,
        network_id: FluidNetworkId,
        node: NodeId,
        pipe: FluidPipe,
    ) {
        self.pipes.insert(node, pipe);
        if let Some(network) = self.networks.get_mut(&network_id) {
            network.add_pipe(node);
        }
    }

    /// Remove a node from the fluid system entirely (all networks and specs).
    pub fn remove_node(&mut self, node: NodeId) {
        self.producers.remove(&node);
        self.consumers.remove(&node);
        self.storage.remove(&node);
        self.pipes.remove(&node);
        for network in self.networks.values_mut() {
            network.remove_node(node);
        }
    }

    /// Get the pressure ratio for a network.
    pub fn pressure(&self, network_id: FluidNetworkId) -> Option<Fixed64> {
        self.networks.get(&network_id).map(|n| n.pressure)
    }

    /// Get how much fluid a consumer received this tick.
    pub fn get_consumed_this_tick(&self, network: FluidNetworkId, node: NodeId) -> Fixed64 {
        self.consumer_consumption
            .get(&(network, node))
            .copied()
            .unwrap_or(Fixed64::ZERO)
    }

    /// Advance all fluid networks by one tick.
    ///
    /// For each network:
    /// 1. Sum total production from all producer nodes.
    /// 2. Sum total demand from all consumer nodes.
    /// 3. If production >= demand: pressure = 1.0, fill storage with excess
    ///    (respecting fill_rate and capacity).
    /// 4. If production < demand: drain storage to cover deficit
    ///    (respecting fill_rate and current level).
    ///    - If storage covers it: pressure = 1.0.
    ///    - Otherwise: pressure = (production + drained) / demand, clamped [0, 1].
    /// 5. Emit PressureLow/PressureRestored events on state transitions only.
    /// 6. Emit StorageFull when storage reaches capacity, StorageEmpty when
    ///    storage reaches 0.
    ///
    /// Returns a list of events emitted this tick.
    pub fn tick(&mut self, current_tick: Ticks) -> Vec<FluidEvent> {
        let mut events = Vec::new();
        let zero = Fixed64::from_num(0);
        let one = Fixed64::from_num(1);

        // Clear per-consumer consumption tracking from last tick.
        self.consumer_consumption.clear();

        // Collect network IDs to iterate, then process each.
        let network_ids: Vec<FluidNetworkId> = self.networks.keys().copied().collect();

        for net_id in network_ids {
            let network = self.networks.get(&net_id).unwrap();

            // Step 1: Sum total production.
            let total_production: Fixed64 = network
                .producers
                .iter()
                .filter_map(|node_id| self.producers.get(node_id))
                .map(|p| p.rate)
                .fold(zero, |acc, val| acc + val);

            // Step 2: Sum total demand.
            let total_demand: Fixed64 = network
                .consumers
                .iter()
                .filter_map(|node_id| self.consumers.get(node_id))
                .map(|c| c.rate)
                .fold(zero, |acc, val| acc + val);

            // Collect storage node IDs for this network so we can mutate storage.
            let storage_nodes: Vec<NodeId> = network.storage.clone();
            let was_low_pressure = network.was_low_pressure;

            // Step 3 & 4: Balance production vs demand with storage.
            let pressure;

            if total_demand == zero {
                // No demand: fully satisfied. Fill storage with all production.
                pressure = one;
                let mut excess = total_production;
                for node_id in &storage_nodes {
                    if excess <= zero {
                        break;
                    }
                    if let Some(s) = self.storage.get_mut(node_id) {
                        let headroom = s.capacity - s.current;
                        let can_fill = excess.min(s.fill_rate).min(headroom);
                        if can_fill > zero {
                            s.current += can_fill;
                            excess -= can_fill;
                        }
                    }
                }
            } else if total_production >= total_demand {
                // Surplus: fully satisfied, fill storage with excess.
                pressure = one;
                let mut excess = total_production - total_demand;
                for node_id in &storage_nodes {
                    if excess <= zero {
                        break;
                    }
                    if let Some(s) = self.storage.get_mut(node_id) {
                        let headroom = s.capacity - s.current;
                        let can_fill = excess.min(s.fill_rate).min(headroom);
                        if can_fill > zero {
                            s.current += can_fill;
                            excess -= can_fill;
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
                        let can_drain = remaining_deficit.min(s.fill_rate).min(s.current);
                        if can_drain > zero {
                            s.current -= can_drain;
                            remaining_deficit -= can_drain;
                        }
                    }
                }

                if remaining_deficit <= zero {
                    // Storage covered the deficit.
                    pressure = one;
                } else {
                    // Partial pressure.
                    let supplied = total_demand - remaining_deficit;
                    pressure = if total_demand > zero {
                        let ratio = supplied / total_demand;
                        if ratio > one {
                            one
                        } else if ratio < zero {
                            zero
                        } else {
                            ratio
                        }
                    } else {
                        one
                    };
                }
            }

            // Record per-consumer consumption for this tick.
            let consumer_nodes: Vec<NodeId> = self
                .networks
                .get(&net_id)
                .unwrap()
                .consumers
                .clone();
            for &node_id in &consumer_nodes {
                if let Some(consumer) = self.consumers.get(&node_id) {
                    let consumed = if pressure >= one {
                        consumer.rate
                    } else {
                        consumer.rate * pressure
                    };
                    self.consumer_consumption.insert((net_id, node_id), consumed);
                }
            }

            // Step 6: Emit StorageFull/StorageEmpty events.
            for node_id in &storage_nodes {
                if let Some(s) = self.storage.get(node_id) {
                    if s.current >= s.capacity && s.capacity > zero {
                        events.push(FluidEvent::StorageFull {
                            network_id: net_id,
                            node: *node_id,
                            tick: current_tick,
                        });
                    }
                    if s.current <= zero {
                        events.push(FluidEvent::StorageEmpty {
                            network_id: net_id,
                            node: *node_id,
                            tick: current_tick,
                        });
                    }
                }
            }

            // Update network state.
            let network = self.networks.get_mut(&net_id).unwrap();
            network.pressure = pressure;

            let is_low_pressure = pressure < one;

            // Step 5: Emit events on state transitions only.
            if is_low_pressure && !was_low_pressure {
                network.was_low_pressure = true;
                events.push(FluidEvent::PressureLow {
                    network_id: net_id,
                    pressure,
                    tick: current_tick,
                });
            } else if !is_low_pressure && was_low_pressure {
                network.was_low_pressure = false;
                events.push(FluidEvent::PressureRestored {
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

    fn water() -> ItemTypeId {
        ItemTypeId(0)
    }

    fn oil() -> ItemTypeId {
        ItemTypeId(1)
    }

    // -----------------------------------------------------------------------
    // Test 1: Empty module tick - no networks
    // -----------------------------------------------------------------------
    #[test]
    fn empty_module_tick_no_events() {
        let mut module = FluidModule::new();
        let events = module.tick(1);
        assert!(events.is_empty(), "empty module should emit no events");
    }

    // -----------------------------------------------------------------------
    // Test 2: Create network - unique IDs
    // -----------------------------------------------------------------------
    #[test]
    fn network_ids_are_unique() {
        let mut module = FluidModule::new();
        let a = module.create_network(water());
        let b = module.create_network(water());
        let c = module.create_network(oil());

        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    // -----------------------------------------------------------------------
    // Test 3: Balanced network - pressure = 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn balanced_network_pressure_is_one() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(100.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(100.0) });

        let events = module.tick(1);

        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        // No transition events on a balanced network.
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }))
            .collect();
        assert!(pressure_events.is_empty(), "no pressure events on balanced network");
    }

    // -----------------------------------------------------------------------
    // Test 4: Overpowered network - pressure = 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn overpowered_network_pressure_is_one() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(200.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(50.0) });

        let events = module.tick(1);

        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }))
            .collect();
        assert!(pressure_events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 5: Underpowered network - pressure < 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn underpowered_network_pressure_below_one() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(50.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(100.0) });

        let events = module.tick(1);

        let pressure = module.pressure(net).unwrap();
        // 50 / 100 = 0.5
        assert_eq!(pressure, fixed(0.5));
        assert!(pressure < Fixed64::from_num(1));

        // Should emit PressureLow event.
        let low_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. }))
            .collect();
        assert_eq!(low_events.len(), 1);
        match &low_events[0] {
            FluidEvent::PressureLow { network_id, pressure, tick } => {
                assert_eq!(*network_id, net);
                assert_eq!(*pressure, fixed(0.5));
                assert_eq!(*tick, 1);
            }
            _ => panic!("expected PressureLow"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 6: Zero demand - pressure = 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn zero_demand_pressure_is_one() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let node = make_node_id();

        module.add_producer(net, node, FluidProducer { rate: fixed(100.0) });
        // No consumers.

        let events = module.tick(1);

        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }))
            .collect();
        assert!(pressure_events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 7: Empty network - pressure = 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn empty_network_pressure_is_one() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());

        let events = module.tick(1);

        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 8: Storage fills with excess
    // -----------------------------------------------------------------------
    #[test]
    fn storage_fills_with_excess() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(150.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(100.0) });
        module.add_storage(
            net,
            nodes[2],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(0.0),
                fill_rate: fixed(100.0),
            },
        );

        module.tick(1);

        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));

        // Storage should have filled by 50 (excess = 150 - 100 = 50, rate allows up to 100).
        let storage = module.storage.get(&nodes[2]).unwrap();
        assert_eq!(storage.current, fixed(50.0));
    }

    // -----------------------------------------------------------------------
    // Test 9: Storage drains during deficit
    // -----------------------------------------------------------------------
    #[test]
    fn storage_drains_during_deficit() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(50.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(100.0) });
        module.add_storage(
            net,
            nodes[2],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(500.0),
                fill_rate: fixed(100.0),
            },
        );

        module.tick(1);

        // Storage should cover the 50-unit deficit.
        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));

        // Storage should have drained by 50.
        let storage = module.storage.get(&nodes[2]).unwrap();
        assert_eq!(storage.current, fixed(450.0));
    }

    // -----------------------------------------------------------------------
    // Test 10: Storage partially covers deficit
    // -----------------------------------------------------------------------
    #[test]
    fn storage_partially_covers_deficit() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(30.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(100.0) });
        module.add_storage(
            net,
            nodes[2],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(20.0),
                fill_rate: fixed(100.0),
            },
        );

        let events = module.tick(1);

        // Production=30, storage drains 20, total supplied=50, demand=100.
        // Pressure = 50/100 = 0.5.
        let pressure = module.pressure(net).unwrap();
        assert_eq!(pressure, fixed(0.5));

        // Storage should be fully drained.
        let storage = module.storage.get(&nodes[2]).unwrap();
        assert_eq!(storage.current, fixed(0.0));

        // PressureLow event emitted.
        let low_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. }))
            .collect();
        assert_eq!(low_events.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Test 11: Storage fully covers deficit - no low pressure
    // -----------------------------------------------------------------------
    #[test]
    fn storage_fully_covers_deficit_no_low_pressure() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        // 0 production, 50 demand, storage has 500 with rate 100.
        module.add_consumer(net, nodes[0], FluidConsumer { rate: fixed(50.0) });
        module.add_storage(
            net,
            nodes[1],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(500.0),
                fill_rate: fixed(100.0),
            },
        );

        let events = module.tick(1);

        // Storage covers all demand.
        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }))
            .collect();
        assert!(pressure_events.is_empty());

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.current, fixed(450.0));
    }

    // -----------------------------------------------------------------------
    // Test 12: Storage fill rate respected
    // -----------------------------------------------------------------------
    #[test]
    fn storage_fill_rate_limits_filling() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        // 200 excess production, but storage can only fill at 30/tick.
        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(200.0) });
        module.add_storage(
            net,
            nodes[1],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(0.0),
                fill_rate: fixed(30.0),
            },
        );

        module.tick(1);

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.current, fixed(30.0));
    }

    // -----------------------------------------------------------------------
    // Test 13: Storage drain rate respected
    // -----------------------------------------------------------------------
    #[test]
    fn storage_drain_rate_limits_draining() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        // 100 demand, 0 production, storage has plenty but rate-limited to 40/tick.
        module.add_consumer(net, nodes[0], FluidConsumer { rate: fixed(100.0) });
        module.add_storage(
            net,
            nodes[1],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(500.0),
                fill_rate: fixed(40.0),
            },
        );

        let events = module.tick(1);

        // Can only drain 40, so pressure = 40/100 = 0.4.
        let pressure = module.pressure(net).unwrap();
        assert_eq!(pressure, fixed(0.4));

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.current, fixed(460.0));

        // PressureLow event.
        let low_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. }))
            .collect();
        assert_eq!(low_events.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Test 14: Storage does not overfill
    // -----------------------------------------------------------------------
    #[test]
    fn storage_does_not_overfill() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        // 100 excess, storage almost full.
        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(100.0) });
        module.add_storage(
            net,
            nodes[1],
            FluidStorage {
                capacity: fixed(50.0),
                current: fixed(45.0),
                fill_rate: fixed(100.0),
            },
        );

        module.tick(1);

        // Should fill at most 5 (headroom = 50 - 45).
        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.current, fixed(50.0));
    }

    // -----------------------------------------------------------------------
    // Test 15: Storage does not drain below zero
    // -----------------------------------------------------------------------
    #[test]
    fn storage_does_not_drain_below_zero() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        module.add_consumer(net, nodes[0], FluidConsumer { rate: fixed(100.0) });
        module.add_storage(
            net,
            nodes[1],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(10.0),
                fill_rate: fixed(200.0),
            },
        );

        module.tick(1);

        // Only 10 units available, demand 100. Supplied = 10, pressure = 10/100.
        let pressure = module.pressure(net).unwrap();
        let expected = Fixed64::from_num(10) / Fixed64::from_num(100);
        assert_eq!(pressure, expected);

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.current, fixed(0.0));
    }

    // -----------------------------------------------------------------------
    // Test 16: PressureLow event on transition only (not every tick)
    // -----------------------------------------------------------------------
    #[test]
    fn pressure_low_event_fires_only_on_transition() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(50.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(100.0) });

        // Tick 1: transition to low pressure -> event.
        let events = module.tick(1);
        let low_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. }))
            .collect();
        assert_eq!(low_events.len(), 1);

        // Tick 2: still low pressure -> NO event.
        let events = module.tick(2);
        let low_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. }))
            .collect();
        assert!(low_events.is_empty(), "no event when already in low pressure");

        // Tick 3: still low pressure -> NO event.
        let events = module.tick(3);
        let low_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. }))
            .collect();
        assert!(low_events.is_empty(), "no event when still in low pressure");
    }

    // -----------------------------------------------------------------------
    // Test 17: PressureRestored event on recovery
    // -----------------------------------------------------------------------
    #[test]
    fn pressure_restored_event_fires_on_recovery() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(3);

        // Start underpowered.
        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(50.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(100.0) });

        // Tick 1: low pressure.
        let events = module.tick(1);
        assert!(events.iter().any(|e| matches!(e, FluidEvent::PressureLow { .. })));

        // Add another producer to meet demand.
        module.add_producer(net, nodes[2], FluidProducer { rate: fixed(50.0) });

        // Tick 2: restored.
        let events = module.tick(2);
        let restored_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureRestored { .. }))
            .collect();
        assert_eq!(restored_events.len(), 1);
        match restored_events[0] {
            FluidEvent::PressureRestored { network_id, tick } => {
                assert_eq!(*network_id, net);
                assert_eq!(*tick, 2);
            }
            _ => panic!("expected PressureRestored"),
        }

        // Tick 3: still satisfied -> no event.
        let events = module.tick(3);
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }))
            .collect();
        assert!(pressure_events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 18: StorageFull event when storage reaches capacity
    // -----------------------------------------------------------------------
    #[test]
    fn storage_full_event_when_capacity_reached() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        // Excess production fills storage to capacity.
        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(100.0) });
        module.add_storage(
            net,
            nodes[1],
            FluidStorage {
                capacity: fixed(50.0),
                current: fixed(40.0),
                fill_rate: fixed(100.0),
            },
        );

        let events = module.tick(1);

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.current, fixed(50.0));

        let full_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::StorageFull { .. }))
            .collect();
        assert_eq!(full_events.len(), 1);
        match full_events[0] {
            FluidEvent::StorageFull { network_id, node, tick } => {
                assert_eq!(*network_id, net);
                assert_eq!(*node, nodes[1]);
                assert_eq!(*tick, 1);
            }
            _ => panic!("expected StorageFull"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 19: StorageEmpty event when storage empties
    // -----------------------------------------------------------------------
    #[test]
    fn storage_empty_event_when_drained() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(2);

        // Only storage, no production, some demand drains it.
        module.add_consumer(net, nodes[0], FluidConsumer { rate: fixed(10.0) });
        module.add_storage(
            net,
            nodes[1],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(10.0),
                fill_rate: fixed(100.0),
            },
        );

        let events = module.tick(1);

        let storage = module.storage.get(&nodes[1]).unwrap();
        assert_eq!(storage.current, fixed(0.0));

        let empty_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::StorageEmpty { .. }))
            .collect();
        assert_eq!(empty_events.len(), 1);
        match empty_events[0] {
            FluidEvent::StorageEmpty { network_id, node, tick } => {
                assert_eq!(*network_id, net);
                assert_eq!(*node, nodes[1]);
                assert_eq!(*tick, 1);
            }
            _ => panic!("expected StorageEmpty"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 20: Multiple networks are independent
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_networks_independent() {
        let mut module = FluidModule::new();
        let net_a = module.create_network(water());
        let net_b = module.create_network(oil());
        let nodes = make_node_ids(4);

        // Network A: balanced.
        module.add_producer(net_a, nodes[0], FluidProducer { rate: fixed(100.0) });
        module.add_consumer(net_a, nodes[1], FluidConsumer { rate: fixed(100.0) });

        // Network B: underpowered.
        module.add_producer(net_b, nodes[2], FluidProducer { rate: fixed(25.0) });
        module.add_consumer(net_b, nodes[3], FluidConsumer { rate: fixed(100.0) });

        let events = module.tick(1);

        // A is satisfied.
        assert_eq!(module.pressure(net_a), Some(Fixed64::from_num(1)));

        // B is not.
        let pres_b = module.pressure(net_b).unwrap();
        assert_eq!(pres_b, fixed(0.25));

        // Only one PressureLow event (for network B).
        let low_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. }))
            .collect();
        assert_eq!(low_events.len(), 1);
        match low_events[0] {
            FluidEvent::PressureLow { network_id, .. } => {
                assert_eq!(*network_id, net_b);
            }
            _ => panic!("expected PressureLow for network B"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 21: Remove node clears from all maps
    // -----------------------------------------------------------------------
    #[test]
    fn remove_node_clears_everything() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(4);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(100.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(50.0) });
        module.add_storage(
            net,
            nodes[2],
            FluidStorage {
                capacity: fixed(100.0),
                current: fixed(50.0),
                fill_rate: fixed(10.0),
            },
        );
        module.add_pipe(net, nodes[3], FluidPipe { capacity: fixed(200.0) });

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

        // Remove pipe.
        module.remove_node(nodes[3]);
        assert!(!module.pipes.contains_key(&nodes[3]));
        let network = module.network(net).unwrap();
        assert!(!network.pipes.contains(&nodes[3]));
    }

    // -----------------------------------------------------------------------
    // Test 22: Serialization trait bounds compile (assert_serde pattern)
    // -----------------------------------------------------------------------
    #[test]
    fn fluid_module_is_serializable() {
        fn assert_serde<T: serde::Serialize + serde::de::DeserializeOwned>() {}
        assert_serde::<FluidModule>();
        assert_serde::<FluidNetwork>();
        assert_serde::<FluidProducer>();
        assert_serde::<FluidConsumer>();
        assert_serde::<FluidStorage>();
        assert_serde::<FluidPipe>();
        assert_serde::<FluidNetworkId>();
    }

    // -----------------------------------------------------------------------
    // Test 23: Deterministic distribution (BTreeMap)
    // -----------------------------------------------------------------------
    #[test]
    fn deterministic_fluid_distribution() {
        fn run() -> Fixed64 {
            let mut module = FluidModule::new();
            let net = module.create_network(ItemTypeId(0));
            let nodes = make_node_ids(5);
            module.add_producer(net, nodes[0], FluidProducer { rate: Fixed64::from_num(50) });
            module.add_consumer(net, nodes[1], FluidConsumer { rate: Fixed64::from_num(25) });
            module.add_consumer(net, nodes[2], FluidConsumer { rate: Fixed64::from_num(25) });
            module.add_consumer(net, nodes[3], FluidConsumer { rate: Fixed64::from_num(25) });
            module.add_consumer(net, nodes[4], FluidConsumer { rate: Fixed64::from_num(25) });
            module.tick(1);
            module.pressure(net).unwrap()
        }

        let p1 = run();
        let p2 = run();
        assert_eq!(p1, p2, "pressure should be deterministic");
        assert_eq!(p1, Fixed64::from_num(50) / Fixed64::from_num(100), "50/100 = 0.5");
    }

    // -----------------------------------------------------------------------
    // Test 24: Remove network works
    // -----------------------------------------------------------------------
    #[test]
    fn remove_network_works() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());

        assert!(module.network(net).is_some());
        module.remove_network(net);
        assert!(module.network(net).is_none());
        assert!(module.pressure(net).is_none());
    }

    // -----------------------------------------------------------------------
    // Test 25: No duplicate nodes in network
    // -----------------------------------------------------------------------
    #[test]
    fn no_duplicate_nodes_in_network() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let node = make_node_id();

        module.add_producer(net, node, FluidProducer { rate: fixed(100.0) });
        // Adding same node again should not duplicate.
        module.add_producer(net, node, FluidProducer { rate: fixed(200.0) });

        let network = module.network(net).unwrap();
        assert_eq!(network.producers.len(), 1);

        // But the spec should be updated.
        assert_eq!(module.producers.get(&node).unwrap().rate, fixed(200.0));
    }

    // -----------------------------------------------------------------------
    // Test 26: Zero production full low pressure
    // -----------------------------------------------------------------------
    #[test]
    fn zero_production_full_low_pressure() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let node = make_node_id();

        module.add_consumer(net, node, FluidConsumer { rate: fixed(100.0) });

        let events = module.tick(1);

        assert_eq!(module.pressure(net).unwrap(), fixed(0.0));
        let low_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. }))
            .collect();
        assert_eq!(low_events.len(), 1);
        match low_events[0] {
            FluidEvent::PressureLow { pressure, .. } => {
                assert_eq!(*pressure, fixed(0.0));
            }
            _ => panic!("expected PressureLow"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 27: Full cycle — low pressure then restored
    // -----------------------------------------------------------------------
    #[test]
    fn full_cycle_low_pressure_then_restored() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(50.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(100.0) });

        // Tick 1: low pressure.
        let events = module.tick(1);
        assert!(events.iter().any(|e| matches!(e, FluidEvent::PressureLow { .. })));
        assert_eq!(module.pressure(net).unwrap(), fixed(0.5));

        // Tick 2: still low pressure, no event.
        let events = module.tick(2);
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }))
            .collect();
        assert!(pressure_events.is_empty());

        // Add producer to meet demand.
        module.add_producer(net, nodes[2], FluidProducer { rate: fixed(50.0) });

        // Tick 3: restored.
        let events = module.tick(3);
        assert!(events.iter().any(|e| matches!(e, FluidEvent::PressureRestored { .. })));
        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));

        // Tick 4: still satisfied, no event.
        let events = module.tick(4);
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }))
            .collect();
        assert!(pressure_events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 28: Storage absorbs excess then releases during deficit
    // -----------------------------------------------------------------------
    #[test]
    fn storage_absorbs_then_releases() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(3);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(150.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(100.0) });
        module.add_storage(
            net,
            nodes[2],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(0.0),
                fill_rate: fixed(200.0),
            },
        );

        // Tick 1: excess 50 fills storage.
        module.tick(1);
        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        assert_eq!(module.storage.get(&nodes[2]).unwrap().current, fixed(50.0));

        // Tick 2: excess 50 more.
        module.tick(2);
        assert_eq!(module.storage.get(&nodes[2]).unwrap().current, fixed(100.0));

        // Now reduce production to create deficit.
        module.producers.get_mut(&nodes[0]).unwrap().rate = fixed(60.0);

        // Tick 3: deficit of 40, storage covers it.
        module.tick(3);
        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        assert_eq!(module.storage.get(&nodes[2]).unwrap().current, fixed(60.0));

        // Tick 4: deficit of 40 again.
        module.tick(4);
        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        assert_eq!(module.storage.get(&nodes[2]).unwrap().current, fixed(20.0));

        // Tick 5: deficit of 40, but only 20 in storage. Partial low pressure.
        let events = module.tick(5);
        let pressure = module.pressure(net).unwrap();
        // Supplied = 60 + 20 = 80, demand = 100. Pressure = 80/100.
        let expected = Fixed64::from_num(80) / Fixed64::from_num(100);
        assert_eq!(pressure, expected);
        assert_eq!(module.storage.get(&nodes[2]).unwrap().current, fixed(0.0));
        assert!(events.iter().any(|e| matches!(e, FluidEvent::PressureLow { .. })));
    }

    // -----------------------------------------------------------------------
    // Test 29: Multiple storage nodes used in order
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_storage_nodes_fill() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(3);

        // 100 excess, two storage nodes each with rate 60.
        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(100.0) });
        module.add_storage(
            net,
            nodes[1],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(0.0),
                fill_rate: fixed(60.0),
            },
        );
        module.add_storage(
            net,
            nodes[2],
            FluidStorage {
                capacity: fixed(1000.0),
                current: fixed(0.0),
                fill_rate: fixed(60.0),
            },
        );

        module.tick(1);

        // First storage gets min(100, 60) = 60. Remaining excess = 40.
        // Second storage gets min(40, 60) = 40.
        let s1 = module.storage.get(&nodes[1]).unwrap();
        let s2 = module.storage.get(&nodes[2]).unwrap();
        assert_eq!(s1.current, fixed(60.0));
        assert_eq!(s2.current, fixed(40.0));
    }

    // -----------------------------------------------------------------------
    // Test 30: Pipe node registration and removal
    // -----------------------------------------------------------------------
    #[test]
    fn pipe_node_registration_and_removal() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let node = make_node_id();

        module.add_pipe(net, node, FluidPipe { capacity: fixed(500.0) });

        assert!(module.pipes.contains_key(&node));
        let network = module.network(net).unwrap();
        assert!(network.pipes.contains(&node));

        module.remove_node(node);
        assert!(!module.pipes.contains_key(&node));
        let network = module.network(net).unwrap();
        assert!(!network.pipes.contains(&node));
    }

    // -----------------------------------------------------------------------
    // Test 31: Fluid type stored on network
    // -----------------------------------------------------------------------
    #[test]
    fn fluid_type_stored_on_network() {
        let mut module = FluidModule::new();
        let net_water = module.create_network(water());
        let net_oil = module.create_network(oil());

        assert_eq!(module.network(net_water).unwrap().fluid_type, water());
        assert_eq!(module.network(net_oil).unwrap().fluid_type, oil());
    }

    // -----------------------------------------------------------------------
    // Test 32: Multiple producers sum correctly
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_producers_sum() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(4);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(30.0) });
        module.add_producer(net, nodes[1], FluidProducer { rate: fixed(40.0) });
        module.add_producer(net, nodes[2], FluidProducer { rate: fixed(30.0) });
        module.add_consumer(net, nodes[3], FluidConsumer { rate: fixed(100.0) });

        let events = module.tick(1);

        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }))
            .collect();
        assert!(pressure_events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 33: Multiple consumers sum correctly
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_consumers_sum() {
        let mut module = FluidModule::new();
        let net = module.create_network(water());
        let nodes = make_node_ids(4);

        module.add_producer(net, nodes[0], FluidProducer { rate: fixed(100.0) });
        module.add_consumer(net, nodes[1], FluidConsumer { rate: fixed(40.0) });
        module.add_consumer(net, nodes[2], FluidConsumer { rate: fixed(30.0) });
        module.add_consumer(net, nodes[3], FluidConsumer { rate: fixed(30.0) });

        let events = module.tick(1);

        assert_eq!(module.pressure(net), Some(Fixed64::from_num(1)));
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }))
            .collect();
        assert!(pressure_events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 34: Fluid consumer keyed by network (and thus by fluid type)
    // -----------------------------------------------------------------------
    #[test]
    fn fluid_consumer_keyed_by_network() {
        let mut module = FluidModule::new();
        let water_net = module.create_network(water());
        let steam_net = module.create_network(ItemTypeId(100)); // steam

        let consumer = make_node_id();

        // Consumer wants water from the water network.
        module.add_consumer(
            water_net,
            consumer,
            FluidConsumer {
                rate: fixed(10.0),
            },
        );

        // Verify consumer is on the water network, not the steam network.
        assert!(module
            .network(water_net)
            .unwrap()
            .consumers
            .contains(&consumer));
        assert!(!module
            .network(steam_net)
            .unwrap()
            .consumers
            .contains(&consumer));

        // Verify the water network carries water type.
        assert_eq!(module.network(water_net).unwrap().fluid_type, water());
    }

    // -----------------------------------------------------------------------
    // Test 35: A single node on multiple fluid networks with different roles
    // -----------------------------------------------------------------------
    #[test]
    fn node_on_multiple_fluid_networks() {
        let mut module = FluidModule::new();
        let water_net = module.create_network(water());
        let steam_net = module.create_network(ItemTypeId(100)); // steam

        let node = make_node_id();

        // Node produces steam and consumes water.
        module.add_producer(
            steam_net,
            node,
            FluidProducer {
                rate: fixed(5.0),
            },
        );
        module.add_consumer(
            water_net,
            node,
            FluidConsumer {
                rate: fixed(10.0),
            },
        );

        // Node should be on both networks with correct roles.
        assert!(module
            .network(steam_net)
            .unwrap()
            .producers
            .contains(&node));
        assert!(!module
            .network(steam_net)
            .unwrap()
            .consumers
            .contains(&node));
        assert!(module
            .network(water_net)
            .unwrap()
            .consumers
            .contains(&node));
        assert!(!module
            .network(water_net)
            .unwrap()
            .producers
            .contains(&node));

        // Tick to verify both networks work independently with the shared node.
        // Add a consumer for steam and a producer for water so networks are balanced.
        let other_nodes = make_node_ids(2);
        module.add_consumer(
            steam_net,
            other_nodes[0],
            FluidConsumer {
                rate: fixed(5.0),
            },
        );
        module.add_producer(
            water_net,
            other_nodes[1],
            FluidProducer {
                rate: fixed(10.0),
            },
        );

        let events = module.tick(1);

        // Both networks should be balanced.
        assert_eq!(module.pressure(steam_net), Some(Fixed64::from_num(1)));
        assert_eq!(module.pressure(water_net), Some(Fixed64::from_num(1)));

        // No pressure events.
        let pressure_events: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    FluidEvent::PressureLow { .. } | FluidEvent::PressureRestored { .. }
                )
            })
            .collect();
        assert!(pressure_events.is_empty());
    }
}
