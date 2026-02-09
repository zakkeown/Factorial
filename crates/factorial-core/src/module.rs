//! Module system for extending the simulation engine with custom behaviors.
//!
//! Modules hook into the engine's tick pipeline via the [`Module`] trait,
//! receiving a [`ModuleContext`] that provides mutable access to the engine's
//! SoA storage. Each module can read the production graph, modify inventories
//! and processor states, emit events, and optionally serialize/deserialize
//! custom state for save/load support.

use crate::event::EventBus;
use crate::graph::ProductionGraph;
use crate::id::NodeId;
use crate::item::Inventory;
use crate::processor::{Processor, ProcessorState};
use slotmap::SecondaryMap;

// ---------------------------------------------------------------------------
// Module trait
// ---------------------------------------------------------------------------

/// A simulation module that hooks into the engine's tick pipeline.
///
/// Modules are called once per tick with a [`ModuleContext`] providing mutable
/// access to engine state. The default implementations of `on_tick`,
/// `serialize_state`, and `load_state` are no-ops, so modules only need to
/// override the methods they care about.
pub trait Module: std::fmt::Debug {
    /// The human-readable name of this module, used for lookup and debugging.
    fn name(&self) -> &str;

    /// Called once per simulation tick. Override to implement custom behavior.
    fn on_tick(&mut self, ctx: &mut ModuleContext<'_>) {
        let _ = ctx;
    }

    /// Serialize this module's internal state for save games.
    /// Returns an empty vec by default (stateless module).
    fn serialize_state(&self) -> Vec<u8> {
        Vec::new()
    }

    /// Load previously serialized state. Returns `Ok(())` by default (no-op).
    fn load_state(&mut self, _data: &[u8]) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Downcast to `&dyn Any` for type-safe access to concrete module types.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Downcast to `&mut dyn Any` for type-safe mutable access to concrete module types.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

// ---------------------------------------------------------------------------
// ModuleContext
// ---------------------------------------------------------------------------

/// Mutable context passed to modules during `on_tick`.
///
/// Provides access to the production graph, per-node SoA storage, the event
/// bus, and the current tick number.
pub struct ModuleContext<'a> {
    /// The production graph (read-only reference).
    pub graph: &'a ProductionGraph,
    /// Per-node processor configurations.
    pub processors: &'a mut SecondaryMap<NodeId, Processor>,
    /// Per-node processor runtime states.
    pub processor_states: &'a mut SecondaryMap<NodeId, ProcessorState>,
    /// Per-node input inventories.
    pub inputs: &'a mut SecondaryMap<NodeId, Inventory>,
    /// Per-node output inventories.
    pub outputs: &'a mut SecondaryMap<NodeId, Inventory>,
    /// The event bus for emitting or reading events.
    pub event_bus: &'a mut EventBus,
    /// The current simulation tick.
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// ModuleError
// ---------------------------------------------------------------------------

/// Errors that can occur during module operations.
#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    /// Failed to deserialize module state from saved data.
    #[error("deserialize failed: {0}")]
    DeserializeFailed(String),
    /// A module with the given name was not found.
    #[error("module not found: {0}")]
    NotFound(String),
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use crate::id::ItemTypeId;
    use crate::sim::SimulationStrategy;
    use crate::test_utils;

    // -----------------------------------------------------------------------
    // Test module: CounterModule -- increments a counter on each tick
    // -----------------------------------------------------------------------

    #[derive(Debug)]
    struct CounterModule {
        count: u32,
    }

    impl CounterModule {
        fn new() -> Self {
            Self { count: 0 }
        }
    }

    impl Module for CounterModule {
        fn name(&self) -> &str {
            "counter"
        }

        fn on_tick(&mut self, _ctx: &mut ModuleContext<'_>) {
            self.count += 1;
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    // -----------------------------------------------------------------------
    // Test module: GraphInspectorModule -- reads graph node count
    // -----------------------------------------------------------------------

    #[derive(Debug)]
    struct GraphInspectorModule {
        last_node_count: usize,
    }

    impl GraphInspectorModule {
        fn new() -> Self {
            Self { last_node_count: 0 }
        }
    }

    impl Module for GraphInspectorModule {
        fn name(&self) -> &str {
            "graph_inspector"
        }

        fn on_tick(&mut self, ctx: &mut ModuleContext<'_>) {
            self.last_node_count = ctx.graph.node_count();
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    // -----------------------------------------------------------------------
    // Test module: InventoryModifierModule -- adds items to input inventories
    // -----------------------------------------------------------------------

    #[derive(Debug)]
    struct InventoryModifierModule {
        item_type: ItemTypeId,
        quantity: u32,
    }

    impl InventoryModifierModule {
        fn new(item_type: ItemTypeId, quantity: u32) -> Self {
            Self {
                item_type,
                quantity,
            }
        }
    }

    impl Module for InventoryModifierModule {
        fn name(&self) -> &str {
            "inventory_modifier"
        }

        fn on_tick(&mut self, ctx: &mut ModuleContext<'_>) {
            for (_node_id, inventory) in ctx.inputs.iter_mut() {
                for slot in &mut inventory.input_slots {
                    let _ = slot.add(self.item_type, self.quantity);
                }
            }
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    // -----------------------------------------------------------------------
    // Test module: StatefulModule -- serializes/deserializes a u64 counter
    // -----------------------------------------------------------------------

    #[derive(Debug)]
    struct StatefulModule {
        value: u64,
    }

    impl StatefulModule {
        fn new(value: u64) -> Self {
            Self { value }
        }
    }

    impl Module for StatefulModule {
        fn name(&self) -> &str {
            "stateful"
        }

        fn on_tick(&mut self, _ctx: &mut ModuleContext<'_>) {
            self.value += 1;
        }

        fn serialize_state(&self) -> Vec<u8> {
            self.value.to_le_bytes().to_vec()
        }

        fn load_state(&mut self, data: &[u8]) -> Result<(), ModuleError> {
            if data.len() != 8 {
                return Err(ModuleError::DeserializeFailed(format!(
                    "expected 8 bytes, got {}",
                    data.len()
                )));
            }
            let bytes: [u8; 8] = data.try_into().unwrap();
            self.value = u64::from_le_bytes(bytes);
            Ok(())
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    // -----------------------------------------------------------------------
    // Helper: build a ModuleContext from an Engine
    // -----------------------------------------------------------------------

    fn make_context(engine: &mut Engine) -> ModuleContext<'_> {
        ModuleContext {
            graph: &engine.graph,
            processors: &mut engine.processors,
            processor_states: &mut engine.processor_states,
            inputs: &mut engine.inputs,
            outputs: &mut engine.outputs,
            event_bus: &mut engine.event_bus,
            tick: engine.sim_state.tick,
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: module_on_tick_called_each_step
    // -----------------------------------------------------------------------
    #[test]
    fn module_on_tick_called_each_step() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let mut module = CounterModule::new();

        assert_eq!(module.count, 0);

        for _ in 0..5 {
            let mut ctx = make_context(&mut engine);
            module.on_tick(&mut ctx);
        }

        assert_eq!(module.count, 5);
    }

    // -----------------------------------------------------------------------
    // Test 2: module_registration_order_preserved
    // -----------------------------------------------------------------------
    #[test]
    fn module_registration_order_preserved() {
        let modules: Vec<Box<dyn Module>> = vec![
            Box::new(CounterModule::new()),
            Box::new(GraphInspectorModule::new()),
            Box::new(StatefulModule::new(0)),
        ];

        let names: Vec<&str> = modules.iter().map(|m| m.name()).collect();
        assert_eq!(names, vec!["counter", "graph_inspector", "stateful"]);
    }

    // -----------------------------------------------------------------------
    // Test 3: module_can_read_graph
    // -----------------------------------------------------------------------
    #[test]
    fn module_can_read_graph() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Add 3 nodes to the graph.
        let _n1 = test_utils::add_node(
            &mut engine,
            test_utils::make_source(test_utils::iron(), 1.0),
            100,
            100,
        );
        let _n2 = test_utils::add_node(
            &mut engine,
            test_utils::make_source(test_utils::iron(), 1.0),
            100,
            100,
        );
        let _n3 = test_utils::add_node(
            &mut engine,
            test_utils::make_source(test_utils::iron(), 1.0),
            100,
            100,
        );

        let mut module = GraphInspectorModule::new();
        assert_eq!(module.last_node_count, 0);

        let mut ctx = make_context(&mut engine);
        module.on_tick(&mut ctx);

        assert_eq!(module.last_node_count, 3);
    }

    // -----------------------------------------------------------------------
    // Test 4: module_can_modify_inventories
    // -----------------------------------------------------------------------
    #[test]
    fn module_can_modify_inventories() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let iron = test_utils::iron();

        let node = test_utils::add_node(&mut engine, test_utils::make_source(iron, 1.0), 100, 100);

        // Verify input inventory starts empty.
        assert_eq!(test_utils::input_quantity(&engine, node, iron), 0);

        let mut module = InventoryModifierModule::new(iron, 10);
        let mut ctx = make_context(&mut engine);
        module.on_tick(&mut ctx);

        // Now the input inventory should have 10 iron.
        assert_eq!(test_utils::input_quantity(&engine, node, iron), 10);
    }

    // -----------------------------------------------------------------------
    // Test 5: module_serialize_state_default_empty
    // -----------------------------------------------------------------------
    #[test]
    fn module_serialize_state_default_empty() {
        let module = CounterModule::new();
        let data = module.serialize_state();
        assert!(data.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 6: module_load_state_default_no_op
    // -----------------------------------------------------------------------
    #[test]
    fn module_load_state_default_no_op() {
        let mut module = CounterModule::new();
        let result = module.load_state(&[1, 2, 3]);
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // Test 7: module_name_lookup
    // -----------------------------------------------------------------------
    #[test]
    fn module_name_lookup() {
        let modules: Vec<Box<dyn Module>> = vec![
            Box::new(CounterModule::new()),
            Box::new(GraphInspectorModule::new()),
            Box::new(StatefulModule::new(42)),
        ];

        let found = modules.iter().find(|m| m.name() == "stateful");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "stateful");

        let not_found = modules.iter().find(|m| m.name() == "nonexistent");
        assert!(not_found.is_none());
    }

    // -----------------------------------------------------------------------
    // Test 8: module_stateful_round_trip
    // -----------------------------------------------------------------------
    #[test]
    fn module_stateful_round_trip() {
        let mut module = StatefulModule::new(42);

        // Tick a few times to change state.
        let mut engine = Engine::new(SimulationStrategy::Tick);
        for _ in 0..10 {
            let mut ctx = make_context(&mut engine);
            module.on_tick(&mut ctx);
        }
        assert_eq!(module.value, 52); // 42 + 10

        // Serialize.
        let data = module.serialize_state();
        assert_eq!(data.len(), 8);

        // Create a new module and load the state.
        let mut restored = StatefulModule::new(0);
        restored.load_state(&data).unwrap();
        assert_eq!(restored.value, 52);

        // Verify bad data produces an error.
        let bad_result = restored.load_state(&[1, 2, 3]);
        assert!(bad_result.is_err());
        assert!(matches!(bad_result, Err(ModuleError::DeserializeFailed(_))));
    }

    // -----------------------------------------------------------------------
    // Error path tests
    // -----------------------------------------------------------------------

    #[test]
    fn module_error_display_messages() {
        let err = ModuleError::DeserializeFailed("bad data".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("deserialize failed"), "got: {msg}");
        assert!(msg.contains("bad data"), "got: {msg}");

        let err = ModuleError::NotFound("power".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("module not found"), "got: {msg}");
        assert!(msg.contains("power"), "got: {msg}");
    }

    #[test]
    fn module_context_has_correct_tick() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        engine.step();
        engine.step();
        engine.step();
        let ctx = make_context(&mut engine);
        assert_eq!(ctx.tick, 3);
    }
}
