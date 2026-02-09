//! Bridge between `LogicModule` and the factorial-core `Module` trait.
//!
//! [`LogicModuleBridge`] wraps a [`LogicModule`] and implements
//! [`factorial_core::module::Module`], so that logic networks are
//! automatically ticked in the engine's phase-4 component pass.

use factorial_core::module::{Module, ModuleContext, ModuleError};

use crate::LogicModule;

/// A [`Module`] adapter that owns a [`LogicModule`] and ticks it
/// during the engine's component phase.
///
/// Register with `engine.register_module(Box::new(LogicModuleBridge::new()))`.
/// Then retrieve via `engine.find_module_mut::<LogicModuleBridge>()` to
/// configure wire networks, combinators, and circuit controls.
#[derive(Debug)]
pub struct LogicModuleBridge {
    logic: LogicModule,
    /// Events from the most recent tick, available until the next tick.
    last_events: Vec<crate::LogicEvent>,
}

impl Default for LogicModuleBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicModuleBridge {
    /// Create a new bridge with an empty [`LogicModule`].
    pub fn new() -> Self {
        Self {
            logic: LogicModule::new(),
            last_events: Vec::new(),
        }
    }

    /// Access the inner [`LogicModule`] for queries.
    pub fn logic(&self) -> &LogicModule {
        &self.logic
    }

    /// Access the inner [`LogicModule`] for configuration.
    pub fn logic_mut(&mut self) -> &mut LogicModule {
        &mut self.logic
    }

    /// Events emitted during the most recent tick.
    pub fn last_events(&self) -> &[crate::LogicEvent] {
        &self.last_events
    }
}

impl Module for LogicModuleBridge {
    fn name(&self) -> &str {
        "logic"
    }

    fn on_tick(&mut self, ctx: &mut ModuleContext<'_>) {
        self.last_events = self.logic.tick(ctx.inputs, ctx.outputs, ctx.tick);
    }

    fn serialize_state(&self) -> Vec<u8> {
        bitcode::serialize(&self.logic).unwrap_or_default()
    }

    fn load_state(&mut self, data: &[u8]) -> Result<(), ModuleError> {
        self.logic = bitcode::deserialize(data)
            .map_err(|e| ModuleError::DeserializeFailed(e.to_string()))?;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combinator::SignalSelector;
    use crate::condition::{ComparisonOp, Condition};
    use crate::{SignalSet, WireColor};
    use factorial_core::engine::Engine;
    use factorial_core::fixed::Fixed64;
    use factorial_core::id::{ItemTypeId, NodeId};
    use factorial_core::sim::SimulationStrategy;

    fn fixed(v: f64) -> Fixed64 {
        Fixed64::from_num(v)
    }

    fn make_node_ids(count: usize) -> Vec<NodeId> {
        let mut sm = slotmap::SlotMap::<NodeId, ()>::with_key();
        (0..count).map(|_| sm.insert(())).collect()
    }

    #[test]
    fn bridge_name_is_logic() {
        let bridge = LogicModuleBridge::new();
        assert_eq!(bridge.name(), "logic");
    }

    #[test]
    fn bridge_ticks_logic_module_via_engine() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        engine.register_module(Box::new(LogicModuleBridge::new()));

        let nodes = make_node_ids(2);
        let iron = ItemTypeId(0);

        {
            let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();

            let net = bridge.logic_mut().create_network(WireColor::Red);
            bridge.logic_mut().add_to_network(net, nodes[0]);
            bridge.logic_mut().add_to_network(net, nodes[1]);

            let mut signals = SignalSet::new();
            signals.insert(iron, fixed(100.0));
            bridge.logic_mut().set_constant(nodes[0], signals, true);

            bridge.logic_mut().set_circuit_control(
                nodes[1],
                Condition {
                    left: SignalSelector::Signal(iron),
                    op: ComparisonOp::Gt,
                    right: SignalSelector::Constant(fixed(50.0)),
                },
                WireColor::Red,
            );
        }

        // Step the engine — logic module auto-ticks in phase 4.
        engine.step();

        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        // Circuit control should be active.
        assert_eq!(bridge.logic().is_active(nodes[1]), Some(true));
        // Should have an activation event.
        assert!(!bridge.last_events().is_empty());
    }

    #[test]
    fn bridge_serialize_load_round_trip() {
        let mut bridge = LogicModuleBridge::new();
        let nodes = make_node_ids(1);
        let iron = ItemTypeId(0);

        let net = bridge.logic_mut().create_network(WireColor::Red);
        bridge.logic_mut().add_to_network(net, nodes[0]);

        let mut signals = SignalSet::new();
        signals.insert(iron, fixed(42.0));
        bridge.logic_mut().set_constant(nodes[0], signals, true);

        // Serialize.
        let data = bridge.serialize_state();
        assert!(!data.is_empty());

        // Load into a new bridge.
        let mut restored = LogicModuleBridge::new();
        restored.load_state(&data).unwrap();

        assert_eq!(restored.logic().networks.len(), 1);
        assert_eq!(restored.logic().constants.len(), 1);
    }

    #[test]
    fn bridge_load_state_bad_data() {
        let mut bridge = LogicModuleBridge::new();
        let result = bridge.load_state(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn bridge_registered_in_engine_and_findable() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        engine.register_module(Box::new(LogicModuleBridge::new()));

        // find_module should locate it.
        let found = engine.find_module::<LogicModuleBridge>();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "logic");

        // find_module_mut should also work.
        let found_mut = engine.find_module_mut::<LogicModuleBridge>();
        assert!(found_mut.is_some());
    }

    #[test]
    fn bridge_auto_ticked_by_engine_step() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Register bridge and configure it before stepping.
        engine.register_module(Box::new(LogicModuleBridge::new()));

        {
            let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();
            let nodes = make_node_ids(1);
            let iron = ItemTypeId(0);

            let net = bridge.logic_mut().create_network(WireColor::Red);
            bridge.logic_mut().add_to_network(net, nodes[0]);

            let mut signals = SignalSet::new();
            signals.insert(iron, fixed(99.0));
            bridge.logic_mut().set_constant(nodes[0], signals, true);
        }

        // Step the engine — logic module should tick.
        engine.step();

        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        // The network should have merged signals after the tick.
        let net_id = *bridge.logic().networks.keys().next().unwrap();
        let merged = bridge.logic().network_signals(net_id).unwrap();
        assert_eq!(merged.get(&ItemTypeId(0)), Some(&fixed(99.0)));
    }

    #[test]
    fn bridge_default_impl() {
        let bridge = LogicModuleBridge::default();
        assert_eq!(bridge.logic().networks.len(), 0);
    }
}
