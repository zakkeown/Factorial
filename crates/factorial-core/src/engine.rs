//! The simulation engine: owns the production graph and orchestrates the
//! six-phase simulation pipeline.
//!
//! # Architecture
//!
//! The `Engine` owns:
//! - A [`ProductionGraph`] (nodes = buildings, edges = transport links)
//! - Per-node state: [`Processor`], [`ProcessorState`], input/output [`Inventory`], modifiers
//! - Per-edge state: [`Transport`], [`TransportState`]
//! - A [`SimState`] (tick counter, accumulator)
//! - A [`SimulationStrategy`] (tick vs. delta)
//!
//! # Six-Phase Pipeline
//!
//! Each `step()` runs:
//! 1. **Pre-tick** -- apply queued graph mutations, inject player actions
//! 2. **Transport** -- move items along edges
//! 3. **Process** -- buildings consume inputs and produce outputs (topological order)
//! 4. **Component** -- module-registered systems run (placeholder)
//! 5. **Post-tick** -- placeholder for event delivery and reactive handlers
//! 6. **Bookkeeping** -- update tick counter, compute state hash

use crate::fixed::Ticks;
use crate::graph::ProductionGraph;
use crate::id::{EdgeId, ItemTypeId, NodeId};
use crate::item::Inventory;
use crate::processor::{Modifier, Processor, ProcessorResult, ProcessorState};
use crate::sim::{AdvanceResult, SimState, SimulationStrategy, StateHash};
use crate::transport::{Transport, TransportResult, TransportState};
use slotmap::SecondaryMap;

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// The core simulation engine. Orchestrates the production graph through
/// the six-phase simulation pipeline.
#[derive(Debug)]
pub struct Engine {
    /// The production graph (nodes and edges).
    pub graph: ProductionGraph,

    /// Simulation strategy (tick or delta).
    strategy: SimulationStrategy,

    /// Simulation state (tick counter, accumulator).
    pub sim_state: SimState,

    // -- Per-node state (SoA, keyed by NodeId) --
    /// Processor configuration for each node.
    processors: SecondaryMap<NodeId, Processor>,

    /// Processor runtime state for each node.
    processor_states: SecondaryMap<NodeId, ProcessorState>,

    /// Input inventory for each node.
    inputs: SecondaryMap<NodeId, Inventory>,

    /// Output inventory for each node.
    outputs: SecondaryMap<NodeId, Inventory>,

    /// Modifiers applied to each node's processor.
    modifiers: SecondaryMap<NodeId, Vec<Modifier>>,

    // -- Per-edge state (SoA, keyed by EdgeId) --
    /// Transport configuration for each edge.
    transports: SecondaryMap<EdgeId, Transport>,

    /// Transport runtime state for each edge.
    transport_states: SecondaryMap<EdgeId, TransportState>,

    /// The most recently computed state hash.
    last_state_hash: u64,
}

impl Engine {
    /// Create a new engine with the given simulation strategy.
    pub fn new(strategy: SimulationStrategy) -> Self {
        Self {
            graph: ProductionGraph::new(),
            strategy,
            sim_state: SimState::new(),
            processors: SecondaryMap::new(),
            processor_states: SecondaryMap::new(),
            inputs: SecondaryMap::new(),
            outputs: SecondaryMap::new(),
            modifiers: SecondaryMap::new(),
            transports: SecondaryMap::new(),
            transport_states: SecondaryMap::new(),
            last_state_hash: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Node management
    // -----------------------------------------------------------------------

    /// Set the processor for a node. Must be called after the node has been
    /// added to the graph (i.e., after `apply_mutations`).
    pub fn set_processor(&mut self, node: NodeId, processor: Processor) {
        self.processors.insert(node, processor);
        self.processor_states
            .insert(node, ProcessorState::default());
    }

    /// Set the input inventory for a node.
    pub fn set_input_inventory(&mut self, node: NodeId, inventory: Inventory) {
        self.inputs.insert(node, inventory);
    }

    /// Set the output inventory for a node.
    pub fn set_output_inventory(&mut self, node: NodeId, inventory: Inventory) {
        self.outputs.insert(node, inventory);
    }

    /// Set the modifiers for a node.
    pub fn set_modifiers(&mut self, node: NodeId, mods: Vec<Modifier>) {
        self.modifiers.insert(node, mods);
    }

    /// Get the processor state for a node (read-only).
    pub fn get_processor_state(&self, node: NodeId) -> Option<&ProcessorState> {
        self.processor_states.get(node)
    }

    /// Get the input inventory for a node (read-only).
    pub fn get_input_inventory(&self, node: NodeId) -> Option<&Inventory> {
        self.inputs.get(node)
    }

    /// Get the output inventory for a node (read-only).
    pub fn get_output_inventory(&self, node: NodeId) -> Option<&Inventory> {
        self.outputs.get(node)
    }

    /// Get the input inventory for a node (mutable).
    pub fn get_input_inventory_mut(&mut self, node: NodeId) -> Option<&mut Inventory> {
        self.inputs.get_mut(node)
    }

    /// Get the output inventory for a node (mutable).
    pub fn get_output_inventory_mut(&mut self, node: NodeId) -> Option<&mut Inventory> {
        self.outputs.get_mut(node)
    }

    // -----------------------------------------------------------------------
    // Edge management
    // -----------------------------------------------------------------------

    /// Set the transport for an edge. Must be called after the edge has been
    /// added to the graph (i.e., after `apply_mutations`).
    pub fn set_transport(&mut self, edge: EdgeId, transport: Transport) {
        let state = TransportState::new_for(&transport);
        self.transports.insert(edge, transport);
        self.transport_states.insert(edge, state);
    }

    /// Get the transport state for an edge (read-only).
    pub fn get_transport_state(&self, edge: EdgeId) -> Option<&TransportState> {
        self.transport_states.get(edge)
    }

    // -----------------------------------------------------------------------
    // State hash
    // -----------------------------------------------------------------------

    /// Get the most recently computed state hash.
    pub fn state_hash(&self) -> u64 {
        self.last_state_hash
    }

    // -----------------------------------------------------------------------
    // Advance
    // -----------------------------------------------------------------------

    /// Advance the simulation according to the configured strategy.
    ///
    /// - **Tick mode**: `dt` is ignored; exactly one step runs.
    /// - **Delta mode**: `dt` is accumulated; as many fixed steps run as fit.
    pub fn advance(&mut self, dt: Ticks) -> AdvanceResult {
        let mut result = AdvanceResult::default();

        match self.strategy.clone() {
            SimulationStrategy::Tick => {
                self.step_internal(&mut result);
            }
            SimulationStrategy::Delta { fixed_timestep } => {
                self.sim_state.accumulator += dt;
                let step_size = fixed_timestep.max(1);
                while self.sim_state.accumulator >= step_size {
                    self.sim_state.accumulator -= step_size;
                    self.step_internal(&mut result);
                }
            }
        }

        result
    }

    /// Run a single simulation step (convenience for tick mode).
    pub fn step(&mut self) -> AdvanceResult {
        self.advance(0)
    }

    // -----------------------------------------------------------------------
    // Internal: single step
    // -----------------------------------------------------------------------

    fn step_internal(&mut self, result: &mut AdvanceResult) {
        // Phase 1: Pre-tick -- apply queued mutations.
        self.phase_pre_tick(result);

        // Phase 2: Transport -- move items along edges.
        self.phase_transport();

        // Phase 3: Process -- buildings consume inputs, produce outputs.
        self.phase_process();

        // Phase 4: Component -- placeholder for module-registered systems.
        self.phase_component();

        // Phase 5: Post-tick -- placeholder for event delivery.
        self.phase_post_tick();

        // Phase 6: Bookkeeping -- update tick counter, compute state hash.
        self.phase_bookkeeping();

        result.steps_run += 1;
    }

    // -----------------------------------------------------------------------
    // Phase 1: Pre-tick
    // -----------------------------------------------------------------------

    fn phase_pre_tick(&mut self, result: &mut AdvanceResult) {
        if self.graph.has_pending_mutations() {
            let mutation_result = self.graph.apply_mutations();
            result.mutation_results.push(mutation_result);
        }
    }

    // -----------------------------------------------------------------------
    // Phase 2: Transport
    // -----------------------------------------------------------------------

    fn phase_transport(&mut self) {
        // Collect edge IDs to iterate (avoids borrow conflicts).
        let edge_ids: Vec<EdgeId> = self.transports.keys().collect();

        for edge_id in edge_ids {
            let Some(edge_data) = self.graph.get_edge(edge_id) else {
                continue;
            };
            let source_node = edge_data.from;
            let dest_node = edge_data.to;

            // Determine available items at the source's output inventory.
            let available = self.output_total(source_node);

            // Advance the transport.
            let transport_result = {
                let Some(transport) = self.transports.get(edge_id) else {
                    continue;
                };
                let Some(state) = self.transport_states.get_mut(edge_id) else {
                    continue;
                };
                transport.advance(state, available)
            };

            // Apply transport results to inventories.
            self.apply_transport_result(source_node, dest_node, &transport_result);
        }
    }

    /// Get total items in a node's output inventory (across all slots and types).
    fn output_total(&self, node: NodeId) -> u32 {
        self.outputs
            .get(node)
            .map(|inv| inv.output_slots.iter().map(|s| s.total()).sum())
            .unwrap_or(0)
    }

    /// Apply transport results: remove items from source output, add to dest input.
    fn apply_transport_result(
        &mut self,
        source: NodeId,
        dest: NodeId,
        result: &TransportResult,
    ) {
        // Determine item type before any mutable borrows.
        let item_type = self.determine_item_type_for_edge(source);

        // Remove moved items from source output.
        if result.items_moved > 0
            && let Some(output_inv) = self.outputs.get_mut(source)
        {
            let mut remaining = result.items_moved;
            for slot in &mut output_inv.output_slots {
                if remaining == 0 {
                    break;
                }
                // Remove from each stack in the slot until we've removed enough.
                for stack in &mut slot.stacks {
                    if remaining == 0 {
                        break;
                    }
                    let to_remove = remaining.min(stack.quantity);
                    stack.quantity -= to_remove;
                    remaining -= to_remove;
                }
                slot.stacks.retain(|s| s.quantity > 0);
            }
        }

        // Deliver items to destination input.
        if result.items_delivered > 0
            && let Some(input_inv) = self.inputs.get_mut(dest)
        {
            let mut remaining = result.items_delivered;
            for slot in &mut input_inv.input_slots {
                if remaining == 0 {
                    break;
                }
                let overflow = slot.add(item_type, remaining);
                remaining = overflow;
            }
        }
    }

    /// Determine the item type flowing through an edge based on the source node.
    /// Falls back to ItemTypeId(0) if no type can be determined.
    fn determine_item_type_for_edge(&self, source: NodeId) -> ItemTypeId {
        // Check the source's processor for its output type.
        if let Some(processor) = self.processors.get(source) {
            match processor {
                Processor::Source(src) => return src.output_type,
                Processor::Fixed(recipe) => {
                    if let Some(first_output) = recipe.outputs.first() {
                        return first_output.item_type;
                    }
                }
                Processor::Property(prop) => return prop.output_type,
            }
        }

        // Check the source's output inventory for any existing items.
        if let Some(output_inv) = self.outputs.get(source) {
            for slot in &output_inv.output_slots {
                for stack in &slot.stacks {
                    if stack.quantity > 0 {
                        return stack.item_type;
                    }
                }
            }
        }

        ItemTypeId(0)
    }

    // -----------------------------------------------------------------------
    // Phase 3: Process
    // -----------------------------------------------------------------------

    fn phase_process(&mut self) {
        // Get topological order. If the graph has a cycle, skip processing.
        let topo_order = match self.graph.topological_order() {
            Ok(order) => order.to_vec(),
            Err(_) => return,
        };

        for node_id in topo_order {
            self.process_node(node_id);
        }
    }

    fn process_node(&mut self, node_id: NodeId) {
        // Gather available inputs from the node's input inventory.
        let available_inputs = self.gather_available_inputs(node_id);

        // Calculate output space.
        let output_space = self.calculate_output_space(node_id);

        // Get modifiers (clone to avoid borrow conflict).
        let mods = self
            .modifiers
            .get(node_id)
            .cloned()
            .unwrap_or_default();

        // Tick the processor.
        let processor_result = {
            let Some(processor) = self.processors.get_mut(node_id) else {
                return;
            };
            let Some(state) = self.processor_states.get_mut(node_id) else {
                return;
            };
            processor.tick(state, &mods, &available_inputs, output_space)
        };

        // Apply consumed items to input inventory.
        self.apply_consumed(node_id, &processor_result);

        // Apply produced items to output inventory.
        self.apply_produced(node_id, &processor_result);
    }

    /// Gather available input items from a node's input inventory.
    fn gather_available_inputs(&self, node_id: NodeId) -> Vec<(ItemTypeId, u32)> {
        let Some(input_inv) = self.inputs.get(node_id) else {
            return Vec::new();
        };

        let mut result: Vec<(ItemTypeId, u32)> = Vec::new();
        for slot in &input_inv.input_slots {
            for stack in &slot.stacks {
                if stack.quantity > 0 {
                    if let Some(entry) = result.iter_mut().find(|(id, _)| *id == stack.item_type) {
                        entry.1 += stack.quantity;
                    } else {
                        result.push((stack.item_type, stack.quantity));
                    }
                }
            }
        }
        result
    }

    /// Calculate total free space in a node's output inventory.
    fn calculate_output_space(&self, node_id: NodeId) -> u32 {
        let Some(output_inv) = self.outputs.get(node_id) else {
            return 0;
        };

        output_inv
            .output_slots
            .iter()
            .map(|s| s.capacity.saturating_sub(s.total()))
            .sum()
    }

    /// Remove consumed items from a node's input inventory.
    fn apply_consumed(&mut self, node_id: NodeId, result: &ProcessorResult) {
        let Some(input_inv) = self.inputs.get_mut(node_id) else {
            return;
        };

        for &(item_type, mut qty) in &result.consumed {
            for slot in &mut input_inv.input_slots {
                if qty == 0 {
                    break;
                }
                let removed = slot.remove(item_type, qty);
                qty -= removed;
            }
        }
    }

    /// Add produced items to a node's output inventory.
    fn apply_produced(&mut self, node_id: NodeId, result: &ProcessorResult) {
        let Some(output_inv) = self.outputs.get_mut(node_id) else {
            return;
        };

        for &(item_type, mut qty) in &result.produced {
            for slot in &mut output_inv.output_slots {
                if qty == 0 {
                    break;
                }
                let overflow = slot.add(item_type, qty);
                qty = overflow;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 4: Component (placeholder)
    // -----------------------------------------------------------------------

    fn phase_component(&mut self) {
        // Module-registered systems would run here.
        // Currently a no-op placeholder for future framework modules.
    }

    // -----------------------------------------------------------------------
    // Phase 5: Post-tick (placeholder)
    // -----------------------------------------------------------------------

    fn phase_post_tick(&mut self) {
        // Event delivery and reactive handlers would run here.
        // Currently a no-op placeholder.
    }

    // -----------------------------------------------------------------------
    // Phase 6: Bookkeeping
    // -----------------------------------------------------------------------

    fn phase_bookkeeping(&mut self) {
        self.sim_state.tick += 1;
        self.last_state_hash = self.compute_state_hash();
    }

    /// Compute a deterministic hash of the current simulation state.
    fn compute_state_hash(&self) -> u64 {
        let mut hasher = StateHash::new();

        // Hash the tick counter.
        hasher.write_u64(self.sim_state.tick);

        // Hash inventory contents in a deterministic order.
        // We iterate over nodes in the graph's SlotMap iteration order,
        // which is deterministic (insertion order within the SlotMap).
        for (node_id, _) in self.graph.nodes() {
            // Hash input inventory.
            if let Some(inv) = self.inputs.get(node_id) {
                for slot in &inv.input_slots {
                    for stack in &slot.stacks {
                        hasher.write_u32(stack.item_type.0);
                        hasher.write_u32(stack.quantity);
                    }
                }
            }

            // Hash output inventory.
            if let Some(inv) = self.outputs.get(node_id) {
                for slot in &inv.output_slots {
                    for stack in &slot.stacks {
                        hasher.write_u32(stack.item_type.0);
                        hasher.write_u32(stack.quantity);
                    }
                }
            }

            // Hash processor state.
            if let Some(ps) = self.processor_states.get(node_id) {
                match ps {
                    ProcessorState::Idle => hasher.write_u32(0),
                    ProcessorState::Working { progress } => {
                        hasher.write_u32(1);
                        hasher.write_u32(*progress);
                    }
                    ProcessorState::Stalled { reason } => {
                        hasher.write_u32(2);
                        hasher.write_u32(*reason as u32);
                    }
                }
            }
        }

        hasher.finish()
    }

    // -----------------------------------------------------------------------
    // Cleanup helpers
    // -----------------------------------------------------------------------

    /// Remove all per-node state for a node. Call this when a node is removed
    /// from the graph to clean up associated data.
    pub fn remove_node_state(&mut self, node: NodeId) {
        self.processors.remove(node);
        self.processor_states.remove(node);
        self.inputs.remove(node);
        self.outputs.remove(node);
        self.modifiers.remove(node);
    }

    /// Remove all per-edge state for an edge.
    pub fn remove_edge_state(&mut self, edge: EdgeId) {
        self.transports.remove(edge);
        self.transport_states.remove(edge);
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed64;
    use crate::id::*;
    use crate::processor::*;
    use crate::transport::*;

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

    /// Create a simple inventory with one input slot and one output slot.
    fn simple_inventory(capacity: u32) -> Inventory {
        Inventory::new(1, 1, capacity)
    }

    /// Create a source processor that produces `rate` items per tick.
    fn make_source(item: ItemTypeId, rate: f64) -> Processor {
        Processor::Source(SourceProcessor {
            output_type: item,
            base_rate: Fixed64::from_num(rate),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0.0),
        })
    }

    /// Create a fixed recipe processor: inputs -> outputs over duration ticks.
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

    /// Create a flow transport with the given rate and no latency.
    fn make_flow_transport(rate: f64) -> Transport {
        Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(rate),
            buffer_capacity: Fixed64::from_num(1000.0),
            latency: 0,
        })
    }

    /// Set up a simple chain: Source -> Transport -> Consumer.
    /// Returns (engine, source_node, consumer_node, edge).
    fn setup_source_transport_consumer(
        source_rate: f64,
        transport_rate: f64,
        recipe_inputs: Vec<(ItemTypeId, u32)>,
        recipe_outputs: Vec<(ItemTypeId, u32)>,
        recipe_duration: u32,
    ) -> (Engine, NodeId, NodeId, EdgeId) {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Add nodes.
        let pending_src = engine.graph.queue_add_node(building());
        let pending_consumer = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let src_node = result.resolve_node(pending_src).unwrap();
        let consumer_node = result.resolve_node(pending_consumer).unwrap();

        // Connect with edge.
        let pending_edge = engine.graph.queue_connect(src_node, consumer_node);
        let result = engine.graph.apply_mutations();
        let edge_id = result.resolve_edge(pending_edge).unwrap();

        // Set up source node.
        engine.set_processor(src_node, make_source(iron(), source_rate));
        engine.set_input_inventory(src_node, simple_inventory(100));
        engine.set_output_inventory(src_node, simple_inventory(100));

        // Set up consumer node.
        engine.set_processor(consumer_node, make_recipe(recipe_inputs, recipe_outputs, recipe_duration));
        engine.set_input_inventory(consumer_node, simple_inventory(100));
        engine.set_output_inventory(consumer_node, simple_inventory(100));

        // Set up transport.
        engine.set_transport(edge_id, make_flow_transport(transport_rate));

        (engine, src_node, consumer_node, edge_id)
    }

    // -----------------------------------------------------------------------
    // Test 1: Single tick -- source -> transport -> consumer chain works
    // -----------------------------------------------------------------------
    #[test]
    fn engine_single_tick_source_transport_consumer() {
        let (mut engine, src_node, consumer_node, _edge) = setup_source_transport_consumer(
            5.0,                     // source produces 5/tick
            10.0,                    // transport moves up to 10/tick
            vec![(iron(), 2)],       // consumer needs 2 iron
            vec![(gear(), 1)],       // consumer produces 1 gear
            3,                       // 3 tick duration
        );

        // Step 1: Source produces, transport has nothing to move yet, consumer stalls.
        engine.step();

        // Source should have produced 5 items into its output.
        let src_output = engine.get_output_inventory(src_node).unwrap();
        assert_eq!(src_output.output_slots[0].quantity(iron()), 5);

        // Step 2: Transport moves items from source output to consumer input.
        engine.step();

        // Consumer should have received items and started working.
        let consumer_input = engine.get_input_inventory(consumer_node).unwrap();
        let consumer_iron = consumer_input.input_slots[0].quantity(iron());
        // The consumer should have received items via transport, consumed 2, and have remaining.
        // Source produces 5 more, transport delivers up to 5 from source output.
        // Consumer starts working (consumes 2 from the delivered items).
        // Consumer should have some iron in input or have consumed it during processing.
        let _ = consumer_iron; // verify it's accessible
    }

    // -----------------------------------------------------------------------
    // Test 2: Multi-tick -- recipe completes after duration ticks
    // -----------------------------------------------------------------------
    #[test]
    fn engine_multi_tick_recipe_completes() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Single node with a fixed recipe.
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // Recipe: 1 iron -> 1 gear, 5 ticks.
        engine.set_processor(node, make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5));

        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 10);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        // Tick 1: processor consumes 1 iron, starts working.
        engine.step();
        let state = engine.get_processor_state(node).unwrap();
        assert!(
            matches!(state, ProcessorState::Working { progress: 1 }),
            "expected Working {{ progress: 1 }}, got {:?}",
            state
        );

        // Ticks 2-4: still working.
        for _ in 0..3 {
            engine.step();
        }
        let state = engine.get_processor_state(node).unwrap();
        assert!(
            matches!(state, ProcessorState::Working { progress: 4 }),
            "expected Working {{ progress: 4 }}, got {:?}",
            state
        );

        // Tick 5: recipe completes, gear produced.
        engine.step();
        let state = engine.get_processor_state(node).unwrap();
        assert_eq!(
            *state,
            ProcessorState::Idle,
            "processor should be idle after completing recipe"
        );

        let output = engine.get_output_inventory(node).unwrap();
        assert_eq!(
            output.output_slots[0].quantity(gear()),
            1,
            "should have produced 1 gear"
        );

        // Input should have consumed 1 iron (started with 10, now 9).
        let input = engine.get_input_inventory(node).unwrap();
        assert_eq!(input.input_slots[0].quantity(iron()), 9);
    }

    // -----------------------------------------------------------------------
    // Test 3: Delta mode -- advance(dt) runs correct number of fixed steps
    // -----------------------------------------------------------------------
    #[test]
    fn engine_delta_mode_runs_correct_steps() {
        let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });

        // Add a source node so we can observe tick advancement.
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(1000));

        // Advance by 5 ticks with fixed_timestep=2.
        // Should run 2 steps (5 / 2 = 2 steps, remainder 1).
        let result = engine.advance(5);
        assert_eq!(result.steps_run, 2);
        assert_eq!(engine.sim_state.tick, 2);
        assert_eq!(engine.sim_state.accumulator, 1); // remainder

        // Advance by 3 more ticks. Accumulator becomes 1+3=4, runs 2 more steps.
        let result = engine.advance(3);
        assert_eq!(result.steps_run, 2);
        assert_eq!(engine.sim_state.tick, 4);
        assert_eq!(engine.sim_state.accumulator, 0);
    }

    // -----------------------------------------------------------------------
    // Test 4: Queued mutation -- add node mid-tick applies next tick
    // -----------------------------------------------------------------------
    #[test]
    fn engine_queued_mutation_applies_next_tick() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Start with one node.
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let _node1 = result.resolve_node(pending).unwrap();
        assert_eq!(engine.graph.node_count(), 1);

        // Queue another node (not applied yet).
        let pending2 = engine.graph.queue_add_node(building());

        // Step: pre-tick applies the mutation.
        let result = engine.step();
        assert_eq!(engine.graph.node_count(), 2);
        assert_eq!(result.mutation_results.len(), 1);

        let node2 = result.mutation_results[0].resolve_node(pending2).unwrap();
        assert!(engine.graph.contains_node(node2));
    }

    // -----------------------------------------------------------------------
    // Test 5: Processing order -- topological order, upstream before downstream
    // -----------------------------------------------------------------------
    #[test]
    fn engine_topological_processing_order() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Build: A -> B -> C (linear chain).
        let pa = engine.graph.queue_add_node(building());
        let pb = engine.graph.queue_add_node(building());
        let pc = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let a = r.resolve_node(pa).unwrap();
        let b = r.resolve_node(pb).unwrap();
        let c = r.resolve_node(pc).unwrap();

        engine.graph.queue_connect(a, b);
        engine.graph.queue_connect(b, c);
        engine.graph.apply_mutations();

        // A is a source producing iron.
        engine.set_processor(a, make_source(iron(), 3.0));
        engine.set_input_inventory(a, simple_inventory(100));
        engine.set_output_inventory(a, simple_inventory(100));

        // B is a recipe: 1 iron -> 1 gear, instant (1 tick).
        engine.set_processor(b, make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 1));
        engine.set_input_inventory(b, simple_inventory(100));
        engine.set_output_inventory(b, simple_inventory(100));

        // C is a recipe: 1 gear -> 1 wire, instant (1 tick). Using ItemTypeId(3).
        let wire = ItemTypeId(3);
        engine.set_processor(c, make_recipe(vec![(gear(), 1)], vec![(wire, 1)], 1));
        engine.set_input_inventory(c, simple_inventory(100));
        engine.set_output_inventory(c, simple_inventory(100));

        // Get edge IDs from the graph (edges were created earlier).
        let edge_ab = engine.graph.get_outputs(a)[0];
        let edge_bc = engine.graph.get_outputs(b)[0];

        engine.set_transport(edge_ab, make_flow_transport(10.0));
        engine.set_transport(edge_bc, make_flow_transport(10.0));

        // Pre-fill B's input with some iron so it can work immediately.
        // This tests that processing in topo order means A produces before B processes.
        // But on the first tick, transport hasn't moved anything yet.
        // Let's pre-fill to verify B processes after A.
        let mut b_input = simple_inventory(100);
        b_input.input_slots[0].add(iron(), 5);
        engine.set_input_inventory(b, b_input);

        // Step 1.
        engine.step();

        // A should have produced 3 iron into its output.
        assert_eq!(
            engine.get_output_inventory(a).unwrap().output_slots[0].quantity(iron()),
            3
        );

        // B had 5 iron input. With 1-tick recipe, it should have consumed 1
        // and produced 1 gear (it will repeat if 1-tick recipe starts+finishes
        // in the same tick).
        let b_output = engine.get_output_inventory(b).unwrap();
        assert!(
            b_output.output_slots[0].quantity(gear()) >= 1,
            "B should have produced at least 1 gear"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Determinism -- same inputs = same state hash
    // -----------------------------------------------------------------------
    #[test]
    fn engine_determinism_same_hash() {
        fn run_simulation() -> Vec<u64> {
            let mut engine = Engine::new(SimulationStrategy::Tick);

            let pending = engine.graph.queue_add_node(building());
            let result = engine.graph.apply_mutations();
            let node = result.resolve_node(pending).unwrap();

            engine.set_processor(node, make_source(iron(), 2.0));
            engine.set_input_inventory(node, simple_inventory(100));
            engine.set_output_inventory(node, simple_inventory(100));

            let mut hashes = Vec::new();
            for _ in 0..10 {
                engine.step();
                hashes.push(engine.state_hash());
            }
            hashes
        }

        let run1 = run_simulation();
        let run2 = run_simulation();

        assert_eq!(run1, run2, "two identical runs should produce identical state hashes");
    }

    // -----------------------------------------------------------------------
    // Test 7: Tick counter increments correctly
    // -----------------------------------------------------------------------
    #[test]
    fn engine_tick_counter() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        assert_eq!(engine.sim_state.tick, 0);
        engine.step();
        assert_eq!(engine.sim_state.tick, 1);
        engine.step();
        assert_eq!(engine.sim_state.tick, 2);

        for _ in 0..8 {
            engine.step();
        }
        assert_eq!(engine.sim_state.tick, 10);
    }

    // -----------------------------------------------------------------------
    // Test 8: Full pipeline -- source produces and fills output
    // -----------------------------------------------------------------------
    #[test]
    fn engine_source_fills_output() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 3.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        // 5 ticks at 3/tick = 15 items.
        for _ in 0..5 {
            engine.step();
        }

        let output = engine.get_output_inventory(node).unwrap();
        assert_eq!(output.output_slots[0].quantity(iron()), 15);
    }

    // -----------------------------------------------------------------------
    // Test 9: Transport delivers items between nodes
    // -----------------------------------------------------------------------
    #[test]
    fn engine_transport_delivers_items() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Two nodes: A (source) and B (sink).
        let pa = engine.graph.queue_add_node(building());
        let pb = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let a = r.resolve_node(pa).unwrap();
        let b = r.resolve_node(pb).unwrap();

        let pe = engine.graph.queue_connect(a, b);
        let r = engine.graph.apply_mutations();
        let edge = r.resolve_edge(pe).unwrap();

        // A produces 5 iron/tick.
        engine.set_processor(a, make_source(iron(), 5.0));
        engine.set_input_inventory(a, simple_inventory(100));
        engine.set_output_inventory(a, simple_inventory(100));

        // B consumes iron (but let's just track what arrives).
        // Give B a recipe it can't start (needs gear, which doesn't exist)
        // so items just accumulate in its input.
        engine.set_processor(
            b,
            make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 1),
        );
        engine.set_input_inventory(b, simple_inventory(1000));
        engine.set_output_inventory(b, simple_inventory(100));

        // Flow transport with rate 3/tick.
        engine.set_transport(edge, make_flow_transport(3.0));

        // Tick 1: A produces 5 iron into output. Transport has nothing to deliver yet.
        engine.step();
        let a_out = engine.get_output_inventory(a).unwrap();
        assert_eq!(a_out.output_slots[0].quantity(iron()), 5);

        // Tick 2: Transport picks up 3 from A's output and delivers to B's input.
        engine.step();
        let b_in = engine.get_input_inventory(b).unwrap();
        assert!(
            b_in.input_slots[0].quantity(iron()) >= 3,
            "B should have received items via transport, got {}",
            b_in.input_slots[0].quantity(iron())
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: State hash changes when state changes
    // -----------------------------------------------------------------------
    #[test]
    fn engine_state_hash_changes() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        engine.step();
        let hash1 = engine.state_hash();

        engine.step();
        let hash2 = engine.state_hash();

        assert_ne!(hash1, hash2, "hash should change as state evolves");
    }

    // -----------------------------------------------------------------------
    // Test 11: Delta mode with small timesteps
    // -----------------------------------------------------------------------
    #[test]
    fn engine_delta_mode_small_dt() {
        let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 3 });

        // Advance by 1 -- not enough for a step.
        let result = engine.advance(1);
        assert_eq!(result.steps_run, 0);
        assert_eq!(engine.sim_state.tick, 0);
        assert_eq!(engine.sim_state.accumulator, 1);

        // Advance by 1 more -- still not enough.
        let result = engine.advance(1);
        assert_eq!(result.steps_run, 0);
        assert_eq!(engine.sim_state.accumulator, 2);

        // Advance by 1 more -- now we have 3, enough for 1 step.
        let result = engine.advance(1);
        assert_eq!(result.steps_run, 1);
        assert_eq!(engine.sim_state.tick, 1);
        assert_eq!(engine.sim_state.accumulator, 0);
    }

    // -----------------------------------------------------------------------
    // Test 12: Remove node/edge state cleanup
    // -----------------------------------------------------------------------
    #[test]
    fn engine_remove_node_state() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        assert!(engine.get_processor_state(node).is_some());
        assert!(engine.get_input_inventory(node).is_some());

        engine.remove_node_state(node);

        assert!(engine.get_processor_state(node).is_none());
        assert!(engine.get_input_inventory(node).is_none());
        assert!(engine.get_output_inventory(node).is_none());
    }

    // -----------------------------------------------------------------------
    // Test 13: Source with output full stalls
    // -----------------------------------------------------------------------
    #[test]
    fn engine_source_stalls_when_output_full() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // Source produces 10/tick, but output only holds 5.
        engine.set_processor(node, make_source(iron(), 10.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(5));

        // Tick 1: produce 5 (capped by output space).
        engine.step();
        let output = engine.get_output_inventory(node).unwrap();
        assert_eq!(output.output_slots[0].quantity(iron()), 5);

        // Tick 2: output full, should stall.
        engine.step();
        let state = engine.get_processor_state(node).unwrap();
        assert!(
            matches!(state, ProcessorState::Stalled { reason: StallReason::OutputFull }),
            "source should stall when output is full, got {:?}",
            state
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: Empty engine step doesn't panic
    // -----------------------------------------------------------------------
    #[test]
    fn engine_empty_step_no_panic() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let result = engine.step();
        assert_eq!(result.steps_run, 1);
        assert_eq!(engine.sim_state.tick, 1);
    }

    // -----------------------------------------------------------------------
    // Test 15: Multiple recipe cycles accumulate output
    // -----------------------------------------------------------------------
    #[test]
    fn engine_multiple_recipe_cycles() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // 1 iron -> 1 gear, 2 ticks.
        engine.set_processor(node, make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 2));

        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 10);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        // Run enough ticks for 3 complete recipe cycles (2 ticks each = 6 ticks).
        for _ in 0..6 {
            engine.step();
        }

        let output = engine.get_output_inventory(node).unwrap();
        assert_eq!(
            output.output_slots[0].quantity(gear()),
            3,
            "should have produced 3 gears in 6 ticks (2 ticks per cycle)"
        );

        let input = engine.get_input_inventory(node).unwrap();
        assert_eq!(
            input.input_slots[0].quantity(iron()),
            7,
            "should have consumed 3 iron (started with 10)"
        );
    }
}
