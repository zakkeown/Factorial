//! Fluid-to-item bridge for converting fluid consumption into inventory items.

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::{ItemTypeId, NodeId};
use serde::{Deserialize, Serialize};

use crate::FluidNetworkId;

/// Bridges a fluid network consumer to an engine node's input inventory.
///
/// Each tick, call `apply()` with the consumed fluid amount to convert
/// fluid into discrete inventory items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluidBridge {
    /// The fluid network this bridge reads from.
    pub network: FluidNetworkId,
    /// The engine node whose input inventory receives the items.
    pub node: NodeId,
    /// The item type to add when fluid is consumed.
    pub item_type: ItemTypeId,
    /// Fractional accumulator for sub-1 fluid amounts.
    #[serde(default)]
    pub accumulated: Fixed64,
}

impl FluidBridge {
    pub fn new(network: FluidNetworkId, node: NodeId, item_type: ItemTypeId) -> Self {
        Self {
            network,
            node,
            item_type,
            accumulated: Fixed64::ZERO,
        }
    }

    /// Convert consumed fluid into inventory items.
    ///
    /// `consumed` is the amount of fluid consumed this tick (from
    /// `FluidModule::get_consumed_this_tick`).
    pub fn apply(&mut self, engine: &mut Engine, consumed: Fixed64) {
        self.accumulated += consumed;
        let whole_items = self.accumulated.to_num::<i64>().max(0) as u32;
        if whole_items > 0 {
            self.accumulated -= Fixed64::from_num(whole_items);
            if let Some(inv) = engine.get_input_inventory_mut(self.node) {
                for slot in &mut inv.input_slots {
                    let overflow = slot.add(self.item_type, whole_items);
                    if overflow == 0 {
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use factorial_core::fixed::Fixed64;
    use factorial_core::id::ItemTypeId;
    use factorial_core::processor::Processor;
    use factorial_core::sim::SimulationStrategy;
    use factorial_core::test_utils;

    use crate::{FluidConsumer, FluidModule, FluidProducer};

    #[test]
    fn fluid_bridge_converts_flow_to_items() {
        let water = ItemTypeId(3);

        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Bridge node: a passthrough processor with inventory.
        let bridge_node = test_utils::add_node(&mut engine, Processor::Passthrough, 100, 100);

        let mut fluid = FluidModule::new();
        let net = fluid.create_network(water);

        // Producer feeds 10 water/tick into the network.
        let well_node = test_utils::add_node(&mut engine, Processor::Passthrough, 0, 0);
        fluid.add_producer(
            net,
            well_node,
            FluidProducer {
                rate: Fixed64::from_num(10),
            },
        );

        fluid.add_consumer(
            net,
            bridge_node,
            FluidConsumer {
                rate: Fixed64::from_num(10),
            },
        );

        // Bridge config.
        let mut bridge = FluidBridge::new(net, bridge_node, water);

        for tick in 0..10 {
            fluid.tick(tick);
            // Bridge reads fluid consumption and adds items to node's input inventory.
            let consumed = fluid.get_consumed_this_tick(net, bridge_node);
            bridge.apply(&mut engine, consumed);
        }

        let water_in_inventory = test_utils::input_quantity(&engine, bridge_node, water);
        assert!(
            water_in_inventory > 0,
            "Bridge should deposit fluid as items, got {water_in_inventory}"
        );
        // Should have deposited ~100 items (10/tick * 10 ticks).
        assert_eq!(water_in_inventory, 100);
    }
}
