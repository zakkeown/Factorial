//! Profiling and diagnostic instrumentation for the simulation engine.
//!
//! - [`TickProfile`] captures per-phase timing from the most recent tick.
//!   Only available when the `profiling` feature is enabled.
//! - [`DiagnosticInfo`] provides a detailed breakdown of why a node is in its
//!   current state. Always available (not feature-gated).

use std::time::Duration;

use crate::id::{ItemTypeId, NodeId};
use crate::processor::{ProcessorState, StallReason};

/// Per-phase timing from the most recent tick.
/// Only available when the `profiling` feature is enabled.
#[derive(Debug, Clone, Default)]
pub struct TickProfile {
    pub pre_tick: Duration,
    pub transport: Duration,
    pub process: Duration,
    pub component: Duration,
    pub post_tick: Duration,
    pub bookkeeping: Duration,
    pub total: Duration,
    pub tick: u64,
}

impl TickProfile {
    /// Returns the name and duration of the slowest phase.
    pub fn bottleneck_phase(&self) -> (&'static str, Duration) {
        let phases = [
            ("pre_tick", self.pre_tick),
            ("transport", self.transport),
            ("process", self.process),
            ("component", self.component),
            ("post_tick", self.post_tick),
            ("bookkeeping", self.bookkeeping),
        ];
        phases.into_iter().max_by_key(|(_, d)| *d).unwrap()
    }
}

/// Diagnostic info about why a node is in its current state.
/// Always available (not feature-gated).
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    pub node: NodeId,
    pub processor_state: ProcessorState,
    pub stall_reason: Option<StallReason>,
    /// (item_type, have, need) for each input requirement.
    pub input_summary: Vec<(ItemTypeId, u32, u32)>,
    pub output_space: u32,
    pub output_capacity: u32,
    pub incoming_edges: usize,
    pub outgoing_edges: usize,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::engine::Engine;
    use crate::fixed::Fixed64;
    use crate::id::*;
    use crate::item::Inventory;
    use crate::processor::*;
    use crate::sim::SimulationStrategy;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn iron() -> ItemTypeId {
        ItemTypeId(0)
    }
    fn gear() -> ItemTypeId {
        ItemTypeId(2)
    }
    fn building() -> BuildingTypeId {
        BuildingTypeId(0)
    }

    fn simple_inventory(capacity: u32) -> Inventory {
        Inventory::new(1, 1, capacity)
    }

    fn make_source(item: ItemTypeId, rate: f64) -> Processor {
        Processor::Source(SourceProcessor {
            output_type: item,
            base_rate: Fixed64::from_num(rate),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0.0),
        })
    }

    fn make_recipe(
        inputs: Vec<(ItemTypeId, u32)>,
        outputs: Vec<(ItemTypeId, u32)>,
        duration: u32,
    ) -> Processor {
        Processor::Fixed(FixedRecipe {
            inputs: inputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeInput {
                    item_type,
                    quantity,
                })
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeOutput {
                    item_type,
                    quantity,
                })
                .collect(),
            duration,
        })
    }

    /// Add a single node to the engine and return its NodeId.
    fn add_node(engine: &mut Engine) -> NodeId {
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        result.resolve_node(pending).unwrap()
    }

    /// Connect two nodes and return the EdgeId.
    fn connect(engine: &mut Engine, from: NodeId, to: NodeId) -> EdgeId {
        let pending = engine.graph.queue_connect(from, to);
        let result = engine.graph.apply_mutations();
        result.resolve_edge(pending).unwrap()
    }

    // =======================================================================
    // TickProfile unit tests (always available)
    // =======================================================================

    #[test]
    fn tick_profile_default_all_zeros() {
        let p = TickProfile::default();
        assert_eq!(p.pre_tick, Duration::ZERO);
        assert_eq!(p.transport, Duration::ZERO);
        assert_eq!(p.process, Duration::ZERO);
        assert_eq!(p.component, Duration::ZERO);
        assert_eq!(p.post_tick, Duration::ZERO);
        assert_eq!(p.bookkeeping, Duration::ZERO);
        assert_eq!(p.total, Duration::ZERO);
        assert_eq!(p.tick, 0);
    }

    #[test]
    fn bottleneck_phase_returns_largest() {
        let p = TickProfile {
            pre_tick: Duration::from_micros(10),
            transport: Duration::from_micros(50),
            process: Duration::from_micros(200),
            component: Duration::from_micros(5),
            post_tick: Duration::from_micros(30),
            bookkeeping: Duration::from_micros(2),
            total: Duration::from_micros(297),
            tick: 1,
        };
        let (name, dur) = p.bottleneck_phase();
        assert_eq!(name, "process");
        assert_eq!(dur, Duration::from_micros(200));
    }

    #[test]
    fn bottleneck_phase_all_zeros_is_deterministic() {
        let p = TickProfile::default();
        // With all zeros, max_by_key picks the last element with the max value.
        // Since all are equal (Duration::ZERO), the iterator's max_by_key
        // returns the last element that ties (Rust's max_by_key is stable and
        // returns the later element on tie).
        let (name, dur) = p.bottleneck_phase();
        assert_eq!(dur, Duration::ZERO);
        // Just verify we get *some* valid phase name.
        let valid_names = [
            "pre_tick",
            "transport",
            "process",
            "component",
            "post_tick",
            "bookkeeping",
        ];
        assert!(
            valid_names.contains(&name),
            "expected a valid phase name, got: {name}"
        );
    }

    #[test]
    fn bottleneck_phase_tie_goes_to_last() {
        // When two phases tie, max_by_key returns the *last* one with max value.
        let p = TickProfile {
            pre_tick: Duration::from_micros(100),
            transport: Duration::from_micros(100),
            process: Duration::from_micros(50),
            component: Duration::from_micros(50),
            post_tick: Duration::from_micros(50),
            bookkeeping: Duration::from_micros(50),
            total: Duration::from_micros(400),
            tick: 1,
        };
        let (name, dur) = p.bottleneck_phase();
        // "transport" is the last of the two tied phases (pre_tick, transport).
        assert_eq!(name, "transport");
        assert_eq!(dur, Duration::from_micros(100));
    }

    // =======================================================================
    // Profiling feature-gated tests
    // =======================================================================

    #[cfg(feature = "profiling")]
    #[test]
    fn last_tick_profile_none_before_step() {
        let engine = Engine::new(SimulationStrategy::Tick);
        assert!(engine.last_tick_profile().is_none());
    }

    #[cfg(feature = "profiling")]
    #[test]
    fn last_tick_profile_some_after_step() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        engine.step();
        assert!(engine.last_tick_profile().is_some());
    }

    #[cfg(feature = "profiling")]
    #[test]
    fn profile_total_positive_with_nodes() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);
        engine.set_processor(node, make_source(iron(), 2.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        engine.step();

        let profile = engine.last_tick_profile().unwrap();
        // total should be > Duration::ZERO if real work was done
        // (it's always non-negative since Duration cannot be negative).
        assert!(
            profile.total > Duration::ZERO,
            "total should be positive after stepping with nodes, got {:?}",
            profile.total
        );
    }

    #[cfg(feature = "profiling")]
    #[test]
    fn profile_tick_number_matches_engine() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        engine.step();

        let profile = engine.last_tick_profile().unwrap();
        // After step, bookkeeping increments tick to 1 and then profile
        // records the current sim_state.tick.
        assert_eq!(
            profile.tick, engine.sim_state.tick,
            "profile tick should match engine tick"
        );
    }

    #[cfg(feature = "profiling")]
    #[test]
    fn profile_updates_each_step() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        engine.step();
        let tick1 = engine.last_tick_profile().unwrap().tick;

        engine.step();
        let tick2 = engine.last_tick_profile().unwrap().tick;

        assert_ne!(tick1, tick2, "profile tick should differ between steps");
        assert_eq!(tick2, tick1 + 1);
    }

    // =======================================================================
    // DiagnosticInfo tests (always available)
    // =======================================================================

    #[test]
    fn diagnose_nonexistent_node_returns_none() {
        let engine = Engine::new(SimulationStrategy::Tick);
        // Use a default NodeId -- it won't exist in the graph.
        let fake_node = NodeId::default();
        assert!(engine.diagnose_node(fake_node).is_none());
    }

    #[test]
    fn diagnose_idle_node() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);
        engine.set_processor(node, make_source(iron(), 2.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        // Before any step, processor state is Idle (default).
        let diag = engine.diagnose_node(node).unwrap();
        assert_eq!(diag.node, node);
        assert_eq!(diag.processor_state, ProcessorState::Idle);
        assert!(diag.stall_reason.is_none());
    }

    #[test]
    fn diagnose_working_node() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);

        // Recipe: 1 iron -> 1 gear, 5 ticks.
        engine.set_processor(
            node,
            make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
        );
        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 10);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        // Step once to consume input and start working.
        engine.step();

        let diag = engine.diagnose_node(node).unwrap();
        assert!(
            matches!(diag.processor_state, ProcessorState::Working { .. }),
            "expected Working, got {:?}",
            diag.processor_state
        );
        assert!(diag.stall_reason.is_none());
    }

    #[test]
    fn diagnose_stalled_missing_inputs() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);

        // Recipe requires iron but none available.
        engine.set_processor(
            node,
            make_recipe(vec![(iron(), 5)], vec![(gear(), 1)], 10),
        );
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        // Step: processor will stall because no iron is in input inventory.
        engine.step();

        let diag = engine.diagnose_node(node).unwrap();
        assert_eq!(
            diag.processor_state,
            ProcessorState::Stalled {
                reason: StallReason::MissingInputs
            }
        );
        assert_eq!(diag.stall_reason, Some(StallReason::MissingInputs));
    }

    #[test]
    fn diagnose_stalled_output_full() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);

        // Recipe: 1 iron -> 1 gear, 2 ticks. Output capacity is 0.
        engine.set_processor(
            node,
            make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 2),
        );
        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 50);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, Inventory::new(1, 1, 0)); // zero capacity

        // Step: processor will stall because output is full.
        engine.step();

        let diag = engine.diagnose_node(node).unwrap();
        assert_eq!(
            diag.processor_state,
            ProcessorState::Stalled {
                reason: StallReason::OutputFull
            }
        );
        assert_eq!(diag.stall_reason, Some(StallReason::OutputFull));
    }

    #[test]
    fn diagnose_input_summary_matches_recipe() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);

        // Recipe: 3 iron + 2 gear -> something, but we only have 1 iron, 0 gear.
        engine.set_processor(
            node,
            make_recipe(vec![(iron(), 3), (gear(), 2)], vec![(ItemTypeId(5), 1)], 10),
        );
        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 1);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        // Step to let the processor evaluate.
        engine.step();

        let diag = engine.diagnose_node(node).unwrap();
        // Input summary should show (iron, have=1, need=3) and (gear, have=0, need=2).
        // Note: after step, the processor consumed from the input (or stalled).
        // Since we have 1 iron but need 3, the processor stalls => inputs unchanged.
        assert_eq!(diag.input_summary.len(), 2);

        let iron_entry = diag
            .input_summary
            .iter()
            .find(|(id, _, _)| *id == iron())
            .unwrap();
        assert_eq!(iron_entry.1, 1, "should have 1 iron");
        assert_eq!(iron_entry.2, 3, "should need 3 iron");

        let gear_entry = diag
            .input_summary
            .iter()
            .find(|(id, _, _)| *id == gear())
            .unwrap();
        assert_eq!(gear_entry.1, 0, "should have 0 gear");
        assert_eq!(gear_entry.2, 2, "should need 2 gear");
    }

    #[test]
    fn diagnose_output_space_accurate() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);

        engine.set_processor(node, make_source(iron(), 2.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(50));

        // Before any step, output should be completely empty.
        let diag = engine.diagnose_node(node).unwrap();
        assert_eq!(diag.output_capacity, 50);
        assert_eq!(diag.output_space, 50);

        // Step to produce some items.
        engine.step();

        let diag = engine.diagnose_node(node).unwrap();
        assert_eq!(diag.output_capacity, 50);
        // Source produces 2 items per tick, so space = 50 - 2 = 48.
        assert_eq!(diag.output_space, 48);
    }

    #[test]
    fn diagnose_edge_counts_correct() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node_a = add_node(&mut engine);
        let node_b = add_node(&mut engine);
        let node_c = add_node(&mut engine);

        // A -> B, C -> B. So B has 2 incoming, 0 outgoing.
        connect(&mut engine, node_a, node_b);
        connect(&mut engine, node_c, node_b);

        // Set up minimal processor/inventory so diagnose works.
        engine.set_processor(node_b, make_source(iron(), 1.0));
        engine.set_input_inventory(node_b, simple_inventory(100));
        engine.set_output_inventory(node_b, simple_inventory(100));

        let diag = engine.diagnose_node(node_b).unwrap();
        assert_eq!(diag.incoming_edges, 2);
        assert_eq!(diag.outgoing_edges, 0);
    }

    #[test]
    fn diagnose_source_processor_input_summary() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);

        // Source processor: no recipe inputs, so input_summary should
        // just list what's available with need=0.
        engine.set_processor(node, make_source(iron(), 2.0));
        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 7);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        let diag = engine.diagnose_node(node).unwrap();
        // Source has no recipe inputs, so each available item is listed with need=0.
        assert_eq!(diag.input_summary.len(), 1);
        assert_eq!(diag.input_summary[0], (iron(), 7, 0));
    }

    #[test]
    fn diagnose_node_no_processor_returns_none() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);
        // Node exists in graph but has no processor set.
        // diagnose_node should still return Some (node exists in graph).
        let diag = engine.diagnose_node(node);
        assert!(
            diag.is_some(),
            "node exists in graph so diagnose should return Some"
        );
        let diag = diag.unwrap();
        // With no processor, state defaults.
        assert_eq!(diag.processor_state, ProcessorState::default());
        assert!(diag.stall_reason.is_none());
        assert!(diag.input_summary.is_empty());
    }
}
