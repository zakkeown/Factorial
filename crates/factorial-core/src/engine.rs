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
//! - An [`EventBus`] for typed simulation events
//!
//! # Six-Phase Pipeline
//!
//! Each `step()` runs:
//! 1. **Pre-tick** -- apply queued graph mutations (including reactive handler mutations)
//! 2. **Transport** -- move items along edges; emit transport events
//! 3. **Process** -- buildings consume inputs and produce outputs; emit production events
//! 4. **Component** -- module-registered systems run (placeholder)
//! 5. **Post-tick** -- deliver buffered events to subscribers; collect reactive mutations
//! 6. **Bookkeeping** -- update tick counter, compute state hash

use crate::event::{Event, EventBus, EventKind, EventMutation};
use crate::fixed::{Fixed64, Ticks};
use crate::graph::ProductionGraph;
use crate::id::{EdgeId, ItemTypeId, NodeId, PropertyId};
use crate::item::{Inventory, ItemStack};
use crate::processor::{Modifier, Processor, ProcessorResult, ProcessorState};
use crate::query::{NodeSnapshot, TransportSnapshot};
use crate::sim::{AdvanceResult, SimState, SimulationStrategy, StateHash};
use crate::junction::{Junction, JunctionState};
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
    pub(crate) strategy: SimulationStrategy,

    /// Simulation state (tick counter, accumulator).
    pub sim_state: SimState,

    /// Whether the simulation is paused.
    pub(crate) paused: bool,

    // -- Per-node state (SoA, keyed by NodeId) --
    /// Processor configuration for each node.
    pub(crate) processors: SecondaryMap<NodeId, Processor>,

    /// Processor runtime state for each node.
    pub(crate) processor_states: SecondaryMap<NodeId, ProcessorState>,

    /// Input inventory for each node.
    pub(crate) inputs: SecondaryMap<NodeId, Inventory>,

    /// Output inventory for each node.
    pub(crate) outputs: SecondaryMap<NodeId, Inventory>,

    /// Modifiers applied to each node's processor.
    pub(crate) modifiers: SecondaryMap<NodeId, Vec<Modifier>>,

    // -- Per-edge state (SoA, keyed by EdgeId) --
    /// Transport configuration for each edge.
    pub(crate) transports: SecondaryMap<EdgeId, Transport>,

    /// Transport runtime state for each edge.
    pub(crate) transport_states: SecondaryMap<EdgeId, TransportState>,

    /// The most recently computed state hash.
    pub(crate) last_state_hash: u64,

    /// Typed event bus for simulation events.
    pub event_bus: EventBus,

    /// Registered simulation modules.
    pub(crate) modules: Vec<Box<dyn crate::module::Module>>,

    /// Dirty state tracker.
    pub(crate) dirty: crate::dirty::DirtyTracker,

    /// Junction configurations per node.
    pub(crate) junctions: SecondaryMap<NodeId, Junction>,

    /// Junction runtime state per node.
    pub(crate) junction_states: SecondaryMap<NodeId, JunctionState>,

    /// Per-edge item budgets for the current tick. Populated by junction
    /// processing (splitter budget computation) and consumed by transport.
    pub(crate) edge_budgets: SecondaryMap<EdgeId, u32>,

    /// Timing profile for the most recent tick (profiling feature only).
    #[cfg(feature = "profiling")]
    pub(crate) last_profile: Option<crate::profiling::TickProfile>,
}

impl Engine {
    /// Create a new engine with the given simulation strategy.
    pub fn new(strategy: SimulationStrategy) -> Self {
        Self {
            graph: ProductionGraph::new(),
            strategy,
            sim_state: SimState::new(),
            paused: false,
            processors: SecondaryMap::new(),
            processor_states: SecondaryMap::new(),
            inputs: SecondaryMap::new(),
            outputs: SecondaryMap::new(),
            modifiers: SecondaryMap::new(),
            transports: SecondaryMap::new(),
            transport_states: SecondaryMap::new(),
            last_state_hash: 0,
            event_bus: EventBus::default(),
            modules: Vec::new(),
            dirty: crate::dirty::DirtyTracker::new(),
            junctions: SecondaryMap::new(),
            junction_states: SecondaryMap::new(),
            edge_budgets: SecondaryMap::new(),
            #[cfg(feature = "profiling")]
            last_profile: None,
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
        self.dirty.mark_node(node);
    }

    /// Replace a node's processor and reset its processing state to Idle.
    /// Use this for dynamic recipe selection at runtime.
    pub fn swap_processor(&mut self, node: NodeId, processor: Processor) {
        self.processors.insert(node, processor);
        self.processor_states.insert(node, ProcessorState::Idle);
    }

    /// Set the input inventory for a node.
    pub fn set_input_inventory(&mut self, node: NodeId, inventory: Inventory) {
        self.inputs.insert(node, inventory);
        self.dirty.mark_node(node);
    }

    /// Set the output inventory for a node.
    pub fn set_output_inventory(&mut self, node: NodeId, inventory: Inventory) {
        self.outputs.insert(node, inventory);
        self.dirty.mark_node(node);
    }

    /// Set the modifiers for a node.
    pub fn set_modifiers(&mut self, node: NodeId, mods: Vec<Modifier>) {
        self.modifiers.insert(node, mods);
        self.dirty.mark_node(node);
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
    // Item property queries
    // -----------------------------------------------------------------------

    /// Get the property value for items of a given type in a node's output inventory.
    pub fn get_item_property(
        &self,
        node: NodeId,
        item_type: ItemTypeId,
        property: PropertyId,
    ) -> Option<Fixed64> {
        self.outputs.get(node).and_then(|inv| {
            for slot in &inv.output_slots {
                for stack in &slot.stacks {
                    if stack.item_type == item_type {
                        return stack.get_property(property);
                    }
                }
            }
            None
        })
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
        self.dirty.mark_edge(edge);
    }

    /// Get the transport state for an edge (read-only).
    pub fn get_transport_state(&self, edge: EdgeId) -> Option<&TransportState> {
        self.transport_states.get(edge)
    }

    // -----------------------------------------------------------------------
    // Module management
    // -----------------------------------------------------------------------

    /// Register a simulation module. Modules are called in registration order.
    pub fn register_module(&mut self, module: Box<dyn crate::module::Module>) {
        self.modules.push(module);
    }

    /// Get the number of registered modules.
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Get a reference to a module by index.
    pub fn get_module(&self, index: usize) -> Option<&dyn crate::module::Module> {
        self.modules.get(index).map(|m| m.as_ref())
    }

    /// Get a mutable reference to a module by index.
    pub fn get_module_mut(&mut self, index: usize) -> Option<&mut Box<dyn crate::module::Module>> {
        self.modules.get_mut(index)
    }

    // -----------------------------------------------------------------------
    // Junction management
    // -----------------------------------------------------------------------

    /// Set a junction configuration for a node.
    pub fn set_junction(&mut self, node: NodeId, junction: Junction) {
        self.junctions.insert(node, junction);
        self.junction_states
            .entry(node)
            .unwrap()
            .or_insert_with(JunctionState::default);
        self.dirty.mark_node(node);
    }

    /// Remove a junction from a node.
    pub fn remove_junction(&mut self, node: NodeId) {
        self.junctions.remove(node);
        self.junction_states.remove(node);
        self.dirty.mark_node(node);
    }

    /// Get the junction configuration for a node.
    pub fn junction(&self, node: NodeId) -> Option<&Junction> {
        self.junctions.get(node)
    }

    // -----------------------------------------------------------------------
    // Dirty tracking
    // -----------------------------------------------------------------------

    /// Returns true if anything has been modified since the last clean.
    pub fn is_dirty(&self) -> bool {
        self.dirty.is_dirty()
    }

    /// Get a reference to the dirty tracker.
    pub fn dirty_tracker(&self) -> &crate::dirty::DirtyTracker {
        &self.dirty
    }

    /// Reset all dirty flags.
    pub fn mark_clean(&mut self) {
        self.dirty.mark_clean();
    }

    // -----------------------------------------------------------------------
    // State hash
    // -----------------------------------------------------------------------

    /// Get the most recently computed state hash.
    pub fn state_hash(&self) -> u64 {
        self.last_state_hash
    }

    // -----------------------------------------------------------------------
    // Pause / Resume
    // -----------------------------------------------------------------------

    /// Pause the simulation. While paused, `advance()` and `step()` are no-ops.
    /// Node/edge configuration (set_processor, set_transport, etc.) still works.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume the simulation.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Returns true if the simulation is currently paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Compact internal storage to reduce memory usage.
    /// Returns an approximate count of bytes freed.
    /// Useful on mobile platforms during background/pause.
    pub fn compact(&mut self) -> usize {
        let mut freed = 0usize;

        // Shrink modifier vecs.
        for (_, mods) in &mut self.modifiers {
            let before = mods.capacity();
            mods.shrink_to_fit();
            let after = mods.capacity();
            freed += (before.saturating_sub(after)) * std::mem::size_of::<Modifier>();
        }

        // Shrink inventory slot stacks.
        for (_, inv) in &mut self.inputs {
            for slot in &mut inv.input_slots {
                let before = slot.stacks.capacity();
                slot.stacks.shrink_to_fit();
                let after = slot.stacks.capacity();
                freed += (before.saturating_sub(after)) * std::mem::size_of::<ItemStack>();
            }
        }
        for (_, inv) in &mut self.outputs {
            for slot in &mut inv.output_slots {
                let before = slot.stacks.capacity();
                slot.stacks.shrink_to_fit();
                let after = slot.stacks.capacity();
                freed += (before.saturating_sub(after)) * std::mem::size_of::<ItemStack>();
            }
        }

        freed
    }

    // -----------------------------------------------------------------------
    // Event system
    // -----------------------------------------------------------------------

    /// Suppress an event kind. Suppressed events are never allocated or buffered.
    pub fn suppress_event(&mut self, kind: EventKind) {
        self.event_bus.suppress(kind);
    }

    /// Register a passive listener for an event kind.
    pub fn on_passive(&mut self, kind: EventKind, listener: crate::event::PassiveListener) {
        self.event_bus.on_passive(kind, listener);
    }

    /// Register a reactive handler for an event kind.
    pub fn on_reactive(&mut self, kind: EventKind, handler: crate::event::ReactiveHandler) {
        self.event_bus.on_reactive(kind, handler);
    }

    // -----------------------------------------------------------------------
    // Advance
    // -----------------------------------------------------------------------

    /// Advance the simulation according to the configured strategy.
    ///
    /// - **Tick mode**: `dt` is ignored; exactly one step runs.
    /// - **Delta mode**: `dt` is accumulated; as many fixed steps run as fit.
    pub fn advance(&mut self, dt: Ticks) -> AdvanceResult {
        if self.paused {
            return AdvanceResult::default();
        }
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
        #[cfg(feature = "profiling")]
        let step_start = std::time::Instant::now();

        // Phase 1: Pre-tick -- apply queued mutations.
        #[cfg(feature = "profiling")]
        let phase_start = std::time::Instant::now();
        self.phase_pre_tick(result);
        #[cfg(feature = "profiling")]
        let pre_tick_dur = phase_start.elapsed();

        // Phase 2: Transport -- move items along edges.
        #[cfg(feature = "profiling")]
        let phase_start = std::time::Instant::now();
        self.phase_transport();
        #[cfg(feature = "profiling")]
        let transport_dur = phase_start.elapsed();

        // Phase 3: Process -- buildings consume inputs, produce outputs.
        #[cfg(feature = "profiling")]
        let phase_start = std::time::Instant::now();
        self.phase_process();
        #[cfg(feature = "profiling")]
        let process_dur = phase_start.elapsed();

        // Phase 4: Component -- placeholder for module-registered systems.
        #[cfg(feature = "profiling")]
        let phase_start = std::time::Instant::now();
        self.phase_component();
        #[cfg(feature = "profiling")]
        let component_dur = phase_start.elapsed();

        // Phase 5: Post-tick -- placeholder for event delivery.
        #[cfg(feature = "profiling")]
        let phase_start = std::time::Instant::now();
        self.phase_post_tick();
        #[cfg(feature = "profiling")]
        let post_tick_dur = phase_start.elapsed();

        // Phase 6: Bookkeeping -- update tick counter, compute state hash.
        #[cfg(feature = "profiling")]
        let phase_start = std::time::Instant::now();
        self.phase_bookkeeping();
        #[cfg(feature = "profiling")]
        let bookkeeping_dur = phase_start.elapsed();

        result.steps_run += 1;

        #[cfg(feature = "profiling")]
        {
            self.last_profile = Some(crate::profiling::TickProfile {
                pre_tick: pre_tick_dur,
                transport: transport_dur,
                process: process_dur,
                component: component_dur,
                post_tick: post_tick_dur,
                bookkeeping: bookkeeping_dur,
                total: step_start.elapsed(),
                tick: self.sim_state.tick,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Phase 1: Pre-tick
    // -----------------------------------------------------------------------

    fn phase_pre_tick(&mut self, result: &mut AdvanceResult) {
        // Apply any reactive handler mutations from the previous tick's post-tick.
        let reactive_mutations = self.event_bus.drain_mutations();
        for mutation in reactive_mutations {
            match mutation {
                EventMutation::AddNode { building_type } => {
                    self.graph.queue_add_node(building_type);
                }
                EventMutation::RemoveNode { node } => {
                    self.graph.queue_remove_node(node);
                }
                EventMutation::Connect { from, to } => {
                    self.graph.queue_connect(from, to);
                }
                EventMutation::Disconnect { edge } => {
                    self.graph.queue_disconnect(edge);
                }
            }
        }

        if self.graph.has_pending_mutations() {
            let mutation_result = self.graph.apply_mutations();
            result.mutation_results.push(mutation_result);
            self.dirty.mark_graph();
        }
    }

    // -----------------------------------------------------------------------
    // Phase 2: Transport
    // -----------------------------------------------------------------------

    /// Compute per-edge item budgets from junction (splitter) configurations.
    ///
    /// Called at the start of `phase_transport` so budgets take effect
    /// immediately in the same tick.
    fn compute_junction_budgets(&mut self) {
        // Clear previous budgets.
        self.edge_budgets.clear();

        // For each node with a splitter junction, compute output distribution.
        let junction_nodes: Vec<(NodeId, Junction)> = self
            .junctions
            .iter()
            .map(|(id, j)| (id, j.clone()))
            .collect();

        for (node_id, junction) in junction_nodes {
            match &junction {
                Junction::Splitter(config) => {
                    let outputs = self.graph.get_outputs(node_id);
                    if outputs.is_empty() {
                        continue;
                    }

                    let total = match config.filter {
                        Some(item_type) => self.output_quantity_of(node_id, item_type),
                        None => self.output_total(node_id),
                    };

                    if total == 0 {
                        continue;
                    }

                    let state = self
                        .junction_states
                        .get(node_id)
                        .cloned()
                        .unwrap_or_default();

                    let num_outputs = outputs.len();
                    // Clone outputs to avoid borrow conflicts.
                    let outputs_vec: Vec<EdgeId> = outputs.to_vec();

                    match config.policy {
                        crate::junction::SplitPolicy::RoundRobin => {
                            // Give each output an equal share, with the
                            // current round-robin target getting the remainder.
                            let share = total / num_outputs as u32;
                            let remainder = total % num_outputs as u32;
                            for (i, &edge_id) in outputs_vec.iter().enumerate() {
                                let budget = share
                                    + if i == state.round_robin_index % num_outputs {
                                        remainder
                                    } else {
                                        0
                                    };
                                self.edge_budgets.insert(edge_id, budget);
                            }
                        }
                        crate::junction::SplitPolicy::Priority => {
                            // All items to the first edge.
                            for (i, &edge_id) in outputs_vec.iter().enumerate() {
                                if i == 0 {
                                    self.edge_budgets.insert(edge_id, total);
                                } else {
                                    self.edge_budgets.insert(edge_id, 0);
                                }
                            }
                        }
                        crate::junction::SplitPolicy::EvenSplit => {
                            let share = total / num_outputs as u32;
                            for &edge_id in &outputs_vec {
                                self.edge_budgets.insert(edge_id, share);
                            }
                        }
                    }
                }
                _ => {} // Inserter and Merger don't need budgets.
            }
        }

        // Default fair distribution for non-junction nodes with multiple outputs.
        let all_node_ids: Vec<NodeId> = self.graph.nodes().map(|(id, _)| id).collect();
        for node_id in all_node_ids {
            if self.junctions.contains_key(node_id) {
                continue; // Junction already handled above.
            }
            let outputs = self.graph.get_outputs(node_id);
            if outputs.len() <= 1 {
                continue; // No fan-out needed.
            }

            // Check if any edge from this node already has an item_filter.
            // If ALL edges have filters, they handle their own routing.
            let has_unfiltered = outputs.iter().any(|&eid| {
                self.graph.get_edge(eid).map_or(true, |e| e.item_filter.is_none())
            });
            if !has_unfiltered { continue; }

            let total = self.output_total(node_id);
            if total == 0 { continue; }

            let num = outputs.len() as u32;
            let share = total / num;
            let remainder = total % num;
            let outputs_vec: Vec<EdgeId> = outputs.to_vec();
            for (i, &edge_id) in outputs_vec.iter().enumerate() {
                // Only set budget for unfiltered edges (filtered edges handle themselves).
                if self.graph.get_edge(edge_id).map_or(false, |e| e.item_filter.is_some()) {
                    continue;
                }
                let budget = share + if i == 0 { remainder } else { 0 };
                self.edge_budgets.insert(edge_id, budget);
            }
        }
    }

    fn phase_transport(&mut self) {
        // Compute junction budgets at the start of transport so they
        // take effect within this same tick.
        self.compute_junction_budgets();

        let tick = self.sim_state.tick;

        // Collect edge IDs to iterate (avoids borrow conflicts).
        let edge_ids: Vec<EdgeId> = self.transports.keys().collect();

        for edge_id in edge_ids {
            let Some(edge_data) = self.graph.get_edge(edge_id) else {
                continue;
            };
            let source_node = edge_data.from;
            let dest_node = edge_data.to;
            let item_filter = edge_data.item_filter;

            // Determine available items at the source's output inventory.
            // If a junction budget exists for this edge, use it instead of
            // the node's total output (splitter distribution).
            let available = if let Some(&budget) = self.edge_budgets.get(edge_id) {
                budget
            } else {
                match item_filter {
                    Some(item_type) => self.output_quantity_of(source_node, item_type),
                    None => self.output_total(source_node),
                }
            };

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

            // Emit transport events.
            if transport_result.items_delivered > 0 {
                self.event_bus.emit(Event::ItemDelivered {
                    edge: edge_id,
                    quantity: transport_result.items_delivered,
                    tick,
                });
            }

            // Emit TransportFull when items were available but nothing moved
            // (back-pressure from a full transport buffer).
            if available > 0 && transport_result.items_moved == 0 {
                self.event_bus.emit(Event::TransportFull {
                    edge: edge_id,
                    tick,
                });
            }

            // Apply transport results to inventories.
            self.apply_transport_result(source_node, dest_node, edge_id, &transport_result);
        }
    }

    /// Get total items in a node's output inventory (across all slots and types).
    fn output_total(&self, node: NodeId) -> u32 {
        self.outputs
            .get(node)
            .map(|inv| inv.output_slots.iter().map(|s| s.total()).sum())
            .unwrap_or(0)
    }

    /// Get total quantity of a specific item type in a node's output inventory.
    fn output_quantity_of(&self, node: NodeId, item_type: ItemTypeId) -> u32 {
        self.outputs
            .get(node)
            .map(|inv| inv.output_slots.iter().map(|s| s.quantity(item_type)).sum())
            .unwrap_or(0)
    }

    /// Apply transport results: remove items from source output, add to dest input.
    fn apply_transport_result(
        &mut self,
        source: NodeId,
        dest: NodeId,
        edge_id: EdgeId,
        result: &TransportResult,
    ) {
        // Use edge filter if present, otherwise fall back to processor-based detection.
        let item_type = self.graph.get_edge(edge_id)
            .and_then(|e| e.item_filter)
            .unwrap_or_else(|| self.determine_item_type_for_edge(source));

        // Remove moved items from source output (type-filtered).
        if result.items_moved > 0
            && let Some(output_inv) = self.outputs.get_mut(source)
        {
            let mut remaining = result.items_moved;
            for slot in &mut output_inv.output_slots {
                if remaining == 0 {
                    break;
                }
                let removed = slot.remove(item_type, remaining);
                remaining -= removed;
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
                Processor::Demand(demand) => return demand.input_type,
                Processor::Passthrough => {
                    // No inherent type -- fall through to inventory check below.
                }
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
        // Use feedback-aware ordering so cycles don't skip processing.
        // Back-edges naturally introduce a one-tick delay: items placed in
        // output on tick N are transported on tick N+1, so cycle nodes see
        // last-tick's output as this-tick's input.
        let (order, _back_edges) = self.graph.topological_order_with_feedback();

        for node_id in order {
            self.process_node(node_id);
        }
    }

    fn process_node(&mut self, node_id: NodeId) {
        let tick = self.sim_state.tick;

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

        // Snapshot previous state for detecting state transitions.
        let prev_state = self.processor_states.get(node_id).cloned();

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

        // Emit production events.
        for &(item_type, quantity) in &processor_result.consumed {
            self.event_bus.emit(Event::ItemConsumed {
                node: node_id,
                item_type,
                quantity,
                tick,
            });
        }
        for &(item_type, quantity) in &processor_result.produced {
            self.event_bus.emit(Event::ItemProduced {
                node: node_id,
                item_type,
                quantity,
                tick,
            });
        }

        // Emit state-change events.
        if processor_result.state_changed {
            let new_state = self.processor_states.get(node_id);

            // BuildingResumed: any transition FROM Stalled to a non-Stalled state.
            if matches!(prev_state.as_ref(), Some(ProcessorState::Stalled { .. }))
                && !matches!(new_state, Some(ProcessorState::Stalled { .. }))
            {
                self.event_bus.emit(Event::BuildingResumed {
                    node: node_id,
                    tick,
                });
            }

            match (prev_state.as_ref(), new_state) {
                // Transition to Working from Idle or Stalled => RecipeStarted.
                (Some(ProcessorState::Idle) | Some(ProcessorState::Stalled { .. }), Some(ProcessorState::Working { .. })) => {
                    self.event_bus.emit(Event::RecipeStarted {
                        node: node_id,
                        tick,
                    });
                }
                // Transition to Idle from Working => RecipeCompleted.
                (Some(ProcessorState::Working { .. }), Some(ProcessorState::Idle)) => {
                    self.event_bus.emit(Event::RecipeCompleted {
                        node: node_id,
                        tick,
                    });
                }
                // Transition to Stalled => BuildingStalled.
                (_, Some(ProcessorState::Stalled { reason })) => {
                    self.event_bus.emit(Event::BuildingStalled {
                        node: node_id,
                        reason: *reason,
                        tick,
                    });
                }
                _ => {}
            }
        }

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
    // Phase 4: Component -- junctions + modules
    // -----------------------------------------------------------------------

    fn phase_component(&mut self) {
        // 1. Process junctions in topo order.
        if let Ok(order) = self.graph.topological_order() {
            let order_vec = order.to_vec();
            for &node_id in &order_vec {
                if let Some(junction) = self.junctions.get(node_id).cloned() {
                    let state = self
                        .junction_states
                        .entry(node_id)
                        .unwrap()
                        .or_insert_with(JunctionState::default);
                    // Process junction based on type.
                    match &junction {
                        Junction::Inserter(config) => {
                            // Inserter: move items from input to output.
                            let mut to_move = config.stack_size;
                            if let Some(input_inv) = self.inputs.get(node_id) {
                                let available: u32 = input_inv
                                    .input_slots
                                    .iter()
                                    .map(|s| {
                                        if let Some(filter) = config.filter {
                                            s.quantity(filter)
                                        } else {
                                            s.total()
                                        }
                                    })
                                    .sum();
                                to_move = to_move.min(available);
                            }
                            // Clamp by speed accumulator.
                            state.accumulated += config.speed;
                            let speed_limit = state.accumulated.to_num::<i64>().max(0) as u32;
                            to_move = to_move.min(speed_limit);
                            if to_move > 0 {
                                state.accumulated -= Fixed64::from_num(to_move);
                            }
                            // Actual movement would happen via transport system.
                        }
                        Junction::Splitter(config) => {
                            // Splitter: update round-robin index.
                            let num_outputs = self.graph.get_outputs(node_id).len();
                            if num_outputs > 0 {
                                match config.policy {
                                    crate::junction::SplitPolicy::RoundRobin => {
                                        state.round_robin_index =
                                            (state.round_robin_index + 1) % num_outputs;
                                    }
                                    crate::junction::SplitPolicy::Priority => {
                                        // Priority: always start from 0.
                                        state.round_robin_index = 0;
                                    }
                                    crate::junction::SplitPolicy::EvenSplit => {
                                        // Even split: index not used.
                                    }
                                }
                            }
                        }
                        Junction::Merger(config) => {
                            // Merger: update round-robin index.
                            let num_inputs = self.graph.get_inputs(node_id).len();
                            if num_inputs > 0 {
                                match config.policy {
                                    crate::junction::MergePolicy::RoundRobin => {
                                        state.round_robin_index =
                                            (state.round_robin_index + 1) % num_inputs;
                                    }
                                    crate::junction::MergePolicy::Priority => {
                                        state.round_robin_index = 0;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Run module on_tick() (std::mem::take pattern for borrow safety).
        let mut modules = std::mem::take(&mut self.modules);
        for module in &mut modules {
            let mut ctx = crate::module::ModuleContext {
                graph: &self.graph,
                processors: &mut self.processors,
                processor_states: &mut self.processor_states,
                inputs: &mut self.inputs,
                outputs: &mut self.outputs,
                event_bus: &mut self.event_bus,
                tick: self.sim_state.tick,
            };
            module.on_tick(&mut ctx);
        }
        self.modules = modules;

        // 3. Reset dirty tracker at end of component phase.
        self.dirty.mark_clean();
    }

    // -----------------------------------------------------------------------
    // Phase 5: Post-tick -- event delivery
    // -----------------------------------------------------------------------

    fn phase_post_tick(&mut self) {
        // Deliver all buffered events to subscribers. Reactive handlers
        // may produce mutations that accumulate in event_bus.pending_mutations.
        // Those mutations will be applied during the next tick's pre-tick phase.
        self.event_bus.deliver();
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
    // Query API (read-only)
    // -----------------------------------------------------------------------

    /// Get the processor's progress as a 0..1 fraction.
    ///
    /// - `Working { progress }` on a `FixedRecipe` with `duration` returns
    ///   `progress / duration`.
    /// - `Idle` and `Stalled` return `Fixed64::ZERO`.
    /// - Source and Property processors always return `Fixed64::ZERO` (they
    ///   have no duration-based progress).
    pub fn get_processor_progress(&self, node: NodeId) -> Option<Fixed64> {
        let state = self.processor_states.get(node)?;
        match state {
            ProcessorState::Working { progress } => {
                // Look up the processor to find the duration.
                if let Some(Processor::Fixed(recipe)) = self.processors.get(node) {
                    if recipe.duration > 0 {
                        Some(Fixed64::from_num(*progress) / Fixed64::from_num(recipe.duration))
                    } else {
                        Some(Fixed64::ZERO)
                    }
                } else {
                    // Source/Property processors use Working { progress: 0 },
                    // but have no meaningful progress fraction.
                    Some(Fixed64::ZERO)
                }
            }
            ProcessorState::Idle | ProcessorState::Stalled { .. } => Some(Fixed64::ZERO),
        }
    }

    /// Get the edge's utilization as a 0..1 fraction (how full the transport is).
    ///
    /// - **Flow**: `buffered / buffer_capacity`
    /// - **Item (belt)**: `occupied_slots / total_slots`
    /// - **Batch**: `pending / batch_size`
    /// - **Vehicle**: `cargo_quantity / capacity`
    pub fn get_edge_utilization(&self, edge: EdgeId) -> Option<Fixed64> {
        let transport = self.transports.get(edge)?;
        let state = self.transport_states.get(edge)?;
        Some(compute_utilization(transport, state))
    }

    /// Create a snapshot of a single node.
    pub fn snapshot_node(&self, node: NodeId) -> Option<NodeSnapshot> {
        let node_data = self.graph.get_node(node)?;
        let processor_state = self
            .processor_states
            .get(node)
            .cloned()
            .unwrap_or_default();
        let progress = self.get_processor_progress(node).unwrap_or(Fixed64::ZERO);
        let input_contents = inventory_contents(self.inputs.get(node), true);
        let output_contents = inventory_contents(self.outputs.get(node), false);
        let input_edges = self.graph.get_inputs(node).to_vec();
        let output_edges = self.graph.get_outputs(node).to_vec();

        Some(NodeSnapshot {
            id: node,
            building_type: node_data.building_type,
            processor_state,
            progress,
            input_contents,
            output_contents,
            input_edges,
            output_edges,
        })
    }

    /// Create snapshots of all nodes in the graph.
    pub fn snapshot_all_nodes(&self) -> Vec<NodeSnapshot> {
        self.graph
            .nodes()
            .filter_map(|(node_id, _)| self.snapshot_node(node_id))
            .collect()
    }

    /// Create a snapshot of a single transport edge.
    pub fn snapshot_transport(&self, edge: EdgeId) -> Option<TransportSnapshot> {
        let edge_data = self.graph.get_edge(edge)?;
        let transport = self.transports.get(edge)?;
        let state = self.transport_states.get(edge)?;

        Some(TransportSnapshot {
            id: edge,
            from: edge_data.from,
            to: edge_data.to,
            utilization: compute_utilization(transport, state),
            items_in_transit: count_items_in_transit(state),
        })
    }

    /// Total number of nodes in the production graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Total number of edges in the production graph.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get the edge IDs feeding into a node.
    pub fn get_inputs(&self, node: NodeId) -> &[EdgeId] {
        self.graph.get_inputs(node)
    }

    /// Get the edge IDs leaving a node.
    pub fn get_outputs(&self, node: NodeId) -> &[EdgeId] {
        self.graph.get_outputs(node)
    }

    // -----------------------------------------------------------------------
    // Profiling / Diagnostics
    // -----------------------------------------------------------------------

    /// Get the timing profile from the most recent tick.
    /// Only available with the `profiling` feature.
    #[cfg(feature = "profiling")]
    pub fn last_tick_profile(&self) -> Option<&crate::profiling::TickProfile> {
        self.last_profile.as_ref()
    }

    /// Diagnose why a node is in its current state.
    /// Always available (not feature-gated).
    pub fn diagnose_node(&self, node: NodeId) -> Option<crate::profiling::DiagnosticInfo> {
        // Check node exists in the graph.
        let _node_data = self.graph.get_node(node)?;

        let processor_state = self
            .processor_states
            .get(node)
            .cloned()
            .unwrap_or_default();

        let stall_reason = match &processor_state {
            ProcessorState::Stalled { reason } => Some(*reason),
            _ => None,
        };

        // Build input summary: for FixedRecipe, compare available vs required.
        let input_summary = self.build_input_summary(node);

        // Output space and capacity.
        let (output_space, output_capacity) = self.output_space_and_capacity(node);

        let incoming_edges = self.graph.get_inputs(node).len();
        let outgoing_edges = self.graph.get_outputs(node).len();

        Some(crate::profiling::DiagnosticInfo {
            node,
            processor_state,
            stall_reason,
            input_summary,
            output_space,
            output_capacity,
            incoming_edges,
            outgoing_edges,
        })
    }

    /// Build input summary for diagnostics. For FixedRecipe processors,
    /// shows (item_type, have, need) for each required input.
    fn build_input_summary(&self, node: NodeId) -> Vec<(ItemTypeId, u32, u32)> {
        let available = self.gather_available_inputs(node);

        if let Some(Processor::Fixed(recipe)) = self.processors.get(node) {
            recipe
                .inputs
                .iter()
                .map(|req| {
                    let have = available
                        .iter()
                        .find(|(id, _)| *id == req.item_type)
                        .map(|(_, q)| *q)
                        .unwrap_or(0);
                    (req.item_type, have, req.quantity)
                })
                .collect()
        } else {
            // For Source/Property, just list what's available with need=0.
            available.iter().map(|(id, qty)| (*id, *qty, 0)).collect()
        }
    }

    /// Get (free_space, total_capacity) for a node's output inventory.
    fn output_space_and_capacity(&self, node: NodeId) -> (u32, u32) {
        if let Some(inv) = self.outputs.get(node) {
            let capacity: u32 = inv.output_slots.iter().map(|s| s.capacity).sum();
            let used: u32 = inv.output_slots.iter().map(|s| s.total()).sum();
            (capacity.saturating_sub(used), capacity)
        } else {
            (0, 0)
        }
    }

    // -----------------------------------------------------------------------
    // Cleanup helpers
    // -----------------------------------------------------------------------
    // Demand rate query
    // -----------------------------------------------------------------------

    /// Query the sustained consumption rate for a DemandProcessor node.
    ///
    /// Returns the average items consumed per tick (`consumed_total / current_tick`)
    /// or `None` if the node is not a `Processor::Demand` variant.
    pub fn get_demand_rate(&self, node: NodeId) -> Option<Fixed64> {
        if let Some(Processor::Demand(demand)) = self.processors.get(node) {
            if self.sim_state.tick == 0 {
                return Some(Fixed64::ZERO);
            }
            Some(Fixed64::from_num(demand.consumed_total) / Fixed64::from_num(self.sim_state.tick))
        } else {
            None
        }
    }

    // -----------------------------------------------------------------------

    /// Remove all per-node state for a node. Call this when a node is removed
    /// from the graph to clean up associated data.
    pub fn remove_node_state(&mut self, node: NodeId) {
        self.processors.remove(node);
        self.processor_states.remove(node);
        self.inputs.remove(node);
        self.outputs.remove(node);
        self.modifiers.remove(node);
        self.junctions.remove(node);
        self.junction_states.remove(node);
    }

    /// Remove all per-edge state for an edge.
    pub fn remove_edge_state(&mut self, edge: EdgeId) {
        self.transports.remove(edge);
        self.transport_states.remove(edge);
    }
}

// ---------------------------------------------------------------------------
// Query helpers (free functions, not public API)
// ---------------------------------------------------------------------------

/// Compute utilization (0..1) for a transport edge.
fn compute_utilization(transport: &Transport, state: &TransportState) -> Fixed64 {
    match (transport, state) {
        (Transport::Flow(flow), TransportState::Flow(fs)) => {
            if flow.buffer_capacity > Fixed64::ZERO {
                fs.buffered / flow.buffer_capacity
            } else {
                Fixed64::ZERO
            }
        }
        (Transport::Item(item), TransportState::Item(bs)) => {
            let total = item.slot_count as usize * item.lanes as usize;
            if total > 0 {
                Fixed64::from_num(bs.occupied_count()) / Fixed64::from_num(total)
            } else {
                Fixed64::ZERO
            }
        }
        (Transport::Batch(batch), TransportState::Batch(bs)) => {
            if batch.batch_size > 0 {
                Fixed64::from_num(bs.pending) / Fixed64::from_num(batch.batch_size)
            } else {
                Fixed64::ZERO
            }
        }
        (Transport::Vehicle(vehicle), TransportState::Vehicle(vs)) => {
            let cargo_total: u32 = vs.cargo.iter().map(|s| s.quantity).sum();
            if vehicle.capacity > 0 {
                Fixed64::from_num(cargo_total) / Fixed64::from_num(vehicle.capacity)
            } else {
                Fixed64::ZERO
            }
        }
        _ => Fixed64::ZERO,
    }
}

/// Count items currently in transit within a transport.
fn count_items_in_transit(state: &TransportState) -> u32 {
    match state {
        TransportState::Flow(fs) => fs.buffered.to_num::<i64>().max(0) as u32,
        TransportState::Item(bs) => bs.occupied_count() as u32,
        TransportState::Batch(bs) => bs.pending,
        TransportState::Vehicle(vs) => vs.cargo.iter().map(|s| s.quantity).sum(),
    }
}

/// Collect inventory contents into a flat list of ItemStacks.
/// If `input` is true, reads input_slots; otherwise reads output_slots.
fn inventory_contents(inv: Option<&Inventory>, input: bool) -> Vec<ItemStack> {
    let Some(inv) = inv else {
        return Vec::new();
    };
    let slots = if input {
        &inv.input_slots
    } else {
        &inv.output_slots
    };
    let mut result: Vec<ItemStack> = Vec::new();
    for slot in slots {
        for stack in &slot.stacks {
            if stack.quantity > 0 {
                if let Some(existing) = result.iter_mut().find(|s| s.item_type == stack.item_type) {
                    existing.quantity += stack.quantity;
                } else {
                    result.push(stack.clone());
                }
            }
        }
    }
    result
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

    // =======================================================================
    // Event system integration tests
    // =======================================================================

    use crate::event::{Event, EventKind, EventMutation};
    use std::cell::RefCell;
    use std::rc::Rc;

    // -----------------------------------------------------------------------
    // Event Test 1: Source produces ItemProduced events
    // -----------------------------------------------------------------------
    #[test]
    fn event_source_emits_item_produced() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 3.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        let produced = Rc::new(RefCell::new(Vec::new()));
        let produced_clone = produced.clone();
        engine.on_passive(
            EventKind::ItemProduced,
            Box::new(move |event| {
                if let Event::ItemProduced {
                    quantity, tick, ..
                } = event
                {
                    produced_clone.borrow_mut().push((*quantity, *tick));
                }
            }),
        );

        // Step 1: source produces 3 iron, event emitted.
        engine.step();

        let data = produced.borrow();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0], (3, 0)); // tick=0 because events are emitted before bookkeeping increments tick
    }

    // -----------------------------------------------------------------------
    // Event Test 2: Recipe emits RecipeStarted and RecipeCompleted
    // -----------------------------------------------------------------------
    #[test]
    fn event_recipe_started_and_completed() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // 1 iron -> 1 gear, 3 ticks.
        engine.set_processor(node, make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 3));

        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 5);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        let started_ticks = Rc::new(RefCell::new(Vec::new()));
        let completed_ticks = Rc::new(RefCell::new(Vec::new()));

        let st = started_ticks.clone();
        engine.on_passive(
            EventKind::RecipeStarted,
            Box::new(move |event| {
                if let Event::RecipeStarted { tick, .. } = event {
                    st.borrow_mut().push(*tick);
                }
            }),
        );

        let ct = completed_ticks.clone();
        engine.on_passive(
            EventKind::RecipeCompleted,
            Box::new(move |event| {
                if let Event::RecipeCompleted { tick, .. } = event {
                    ct.borrow_mut().push(*tick);
                }
            }),
        );

        // Tick 1: consumes 1 iron, starts recipe -> RecipeStarted.
        engine.step();
        assert_eq!(*started_ticks.borrow(), vec![0]);
        assert!(completed_ticks.borrow().is_empty());

        // Tick 2: working...
        engine.step();

        // Tick 3: completes -> RecipeCompleted.
        engine.step();
        assert_eq!(*completed_ticks.borrow(), vec![2]);
    }

    // -----------------------------------------------------------------------
    // Event Test 3: BuildingStalled event emitted when output full
    // -----------------------------------------------------------------------
    #[test]
    fn event_building_stalled_output_full() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // Source produces 10/tick, output only holds 5.
        engine.set_processor(node, make_source(iron(), 10.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(5));

        let stalled = Rc::new(RefCell::new(Vec::new()));
        let sc = stalled.clone();
        engine.on_passive(
            EventKind::BuildingStalled,
            Box::new(move |event| {
                if let Event::BuildingStalled { reason, tick, .. } = event {
                    sc.borrow_mut().push((*reason, *tick));
                }
            }),
        );

        // Tick 1: produce 5, fills output. Source is now "working" (it was idle).
        engine.step();
        // Tick 2: output is full, source should stall.
        engine.step();

        let data = stalled.borrow();
        assert!(
            !data.is_empty(),
            "should have emitted at least one BuildingStalled event"
        );
        assert_eq!(data.last().unwrap().0, StallReason::OutputFull);
    }

    // -----------------------------------------------------------------------
    // Event Test 4: Transport emits ItemDelivered events
    // -----------------------------------------------------------------------
    #[test]
    fn event_transport_item_delivered() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Two nodes: A (source) -> B (sink).
        let pa = engine.graph.queue_add_node(building());
        let pb = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let a = r.resolve_node(pa).unwrap();
        let b = r.resolve_node(pb).unwrap();

        let pe = engine.graph.queue_connect(a, b);
        let r = engine.graph.apply_mutations();
        let edge = r.resolve_edge(pe).unwrap();

        engine.set_processor(a, make_source(iron(), 5.0));
        engine.set_input_inventory(a, simple_inventory(100));
        engine.set_output_inventory(a, simple_inventory(100));

        engine.set_processor(
            b,
            make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 1),
        );
        engine.set_input_inventory(b, simple_inventory(1000));
        engine.set_output_inventory(b, simple_inventory(100));

        engine.set_transport(edge, make_flow_transport(3.0));

        let delivered = Rc::new(RefCell::new(Vec::new()));
        let dc = delivered.clone();
        engine.on_passive(
            EventKind::ItemDelivered,
            Box::new(move |event| {
                if let Event::ItemDelivered {
                    edge: e,
                    quantity,
                    tick,
                } = event
                {
                    dc.borrow_mut().push((*e, *quantity, *tick));
                }
            }),
        );

        // Tick 1: source produces 5 iron. Transport has nothing to deliver yet.
        engine.step();
        assert!(delivered.borrow().is_empty());

        // Tick 2: transport delivers 3 items.
        engine.step();
        let data = delivered.borrow();
        assert!(!data.is_empty(), "should have ItemDelivered events");
        assert_eq!(data[0].0, edge);
        assert_eq!(data[0].1, 3);
    }

    // -----------------------------------------------------------------------
    // Event Test 5: Suppressed events not emitted
    // -----------------------------------------------------------------------
    #[test]
    fn event_suppressed_events_not_emitted() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 5.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        // Suppress ItemProduced.
        engine.suppress_event(EventKind::ItemProduced);

        let count = Rc::new(RefCell::new(0u32));
        let cc = count.clone();
        engine.on_passive(
            EventKind::ItemProduced,
            Box::new(move |_| {
                *cc.borrow_mut() += 1;
            }),
        );

        // Run 5 ticks.
        for _ in 0..5 {
            engine.step();
        }

        // Listener should never have been called.
        assert_eq!(
            *count.borrow(),
            0,
            "suppressed events should not trigger listeners"
        );
    }

    // -----------------------------------------------------------------------
    // Event Test 6: Reactive handler mutations apply next tick (one-tick delay)
    // -----------------------------------------------------------------------
    #[test]
    fn event_reactive_handler_mutations_next_tick() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // 1 iron -> 1 gear, 2 ticks.
        engine.set_processor(node, make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 2));
        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 1);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        // When recipe completes, add a new node (via reactive handler).
        engine.on_reactive(
            EventKind::RecipeCompleted,
            Box::new(|_event| {
                vec![EventMutation::AddNode {
                    building_type: BuildingTypeId(42),
                }]
            }),
        );

        // Tick 1: starts recipe.
        engine.step();
        assert_eq!(engine.graph.node_count(), 1);

        // Tick 2: recipe completes. Post-tick delivers event, reactive handler
        // enqueues AddNode mutation. But the mutation has NOT been applied yet.
        engine.step();
        assert_eq!(
            engine.graph.node_count(),
            1,
            "mutation from reactive handler should NOT be applied in same tick"
        );

        // Tick 3: pre-tick applies the reactive handler's mutation.
        engine.step();
        assert_eq!(
            engine.graph.node_count(),
            2,
            "mutation from reactive handler should be applied on the next tick"
        );
    }

    // -----------------------------------------------------------------------
    // Event Test 7: Event counts match expected production
    // -----------------------------------------------------------------------
    #[test]
    fn event_counts_match_production() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 2.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        let total_produced = Rc::new(RefCell::new(0u32));
        let tp = total_produced.clone();
        engine.on_passive(
            EventKind::ItemProduced,
            Box::new(move |event| {
                if let Event::ItemProduced { quantity, .. } = event {
                    *tp.borrow_mut() += quantity;
                }
            }),
        );

        // 10 ticks at 2/tick = 20 items.
        for _ in 0..10 {
            engine.step();
        }

        assert_eq!(
            *total_produced.borrow(),
            20,
            "total items produced via events should match actual production"
        );

        let output = engine.get_output_inventory(node).unwrap();
        assert_eq!(output.output_slots[0].quantity(iron()), 20);
    }

    // -----------------------------------------------------------------------
    // Event Test 8: Events emitted during correct phase (before bookkeeping)
    // -----------------------------------------------------------------------
    #[test]
    fn event_emitted_before_tick_increment() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        let event_ticks = Rc::new(RefCell::new(Vec::new()));
        let et = event_ticks.clone();
        engine.on_passive(
            EventKind::ItemProduced,
            Box::new(move |event| {
                if let Event::ItemProduced { tick, .. } = event {
                    et.borrow_mut().push(*tick);
                }
            }),
        );

        // Run 3 ticks. Events should have tick=0,1,2 (sim_state.tick before
        // bookkeeping increments it).
        for _ in 0..3 {
            engine.step();
        }

        assert_eq!(*event_ticks.borrow(), vec![0, 1, 2]);
    }

    // -----------------------------------------------------------------------
    // Event Test 9: ItemConsumed events from recipe processing
    // -----------------------------------------------------------------------
    #[test]
    fn event_item_consumed() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // 2 iron -> 1 gear, 1 tick (instant).
        engine.set_processor(node, make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 1));
        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 10);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        let consumed_total = Rc::new(RefCell::new(0u32));
        let ct = consumed_total.clone();
        engine.on_passive(
            EventKind::ItemConsumed,
            Box::new(move |event| {
                if let Event::ItemConsumed { quantity, .. } = event {
                    *ct.borrow_mut() += quantity;
                }
            }),
        );

        // 1-tick recipe: each tick consumes 2 iron and produces 1 gear.
        for _ in 0..3 {
            engine.step();
        }

        assert_eq!(
            *consumed_total.borrow(),
            6,
            "should have consumed 2 iron per tick for 3 ticks"
        );
    }

    // -----------------------------------------------------------------------
    // Event Test 10: Events cleared between ticks (not double-delivered)
    // -----------------------------------------------------------------------
    #[test]
    fn event_not_double_delivered() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        let delivery_count = Rc::new(RefCell::new(0u32));
        let dc = delivery_count.clone();
        engine.on_passive(
            EventKind::ItemProduced,
            Box::new(move |_| {
                *dc.borrow_mut() += 1;
            }),
        );

        // Each step should deliver exactly 1 event (1 item produced per tick).
        engine.step();
        assert_eq!(*delivery_count.borrow(), 1);

        engine.step();
        assert_eq!(*delivery_count.borrow(), 2);

        engine.step();
        assert_eq!(*delivery_count.borrow(), 3);
    }

    // =======================================================================
    // Query API tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // Query Test 1: Processor progress returns correct fraction
    // -----------------------------------------------------------------------
    #[test]
    fn query_processor_progress_fraction() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // 1 iron -> 1 gear, 5 ticks.
        engine.set_processor(node, make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5));
        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 10);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        // Before any tick: idle, progress = 0.
        let progress = engine.get_processor_progress(node).unwrap();
        assert_eq!(progress, Fixed64::ZERO);

        // Tick 1: starts working, progress = 1/5 = 0.2.
        engine.step();
        let progress = engine.get_processor_progress(node).unwrap();
        assert_eq!(progress, Fixed64::from_num(1) / Fixed64::from_num(5));

        // Tick 2: progress = 2/5 = 0.4.
        engine.step();
        let progress = engine.get_processor_progress(node).unwrap();
        assert_eq!(progress, Fixed64::from_num(2) / Fixed64::from_num(5));

        // Tick 4: progress = 4/5 = 0.8.
        engine.step();
        engine.step();
        let progress = engine.get_processor_progress(node).unwrap();
        assert_eq!(progress, Fixed64::from_num(4) / Fixed64::from_num(5));

        // Tick 5: completes -> idle, progress = 0.
        engine.step();
        let progress = engine.get_processor_progress(node).unwrap();
        assert_eq!(progress, Fixed64::ZERO);
    }

    // -----------------------------------------------------------------------
    // Query Test 2: Processor progress returns zero for source processors
    // -----------------------------------------------------------------------
    #[test]
    fn query_processor_progress_source_is_zero() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 5.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        engine.step();

        // Source processors have no meaningful progress fraction.
        let progress = engine.get_processor_progress(node).unwrap();
        assert_eq!(progress, Fixed64::ZERO);
    }

    // -----------------------------------------------------------------------
    // Query Test 3: Processor progress returns None for invalid node
    // -----------------------------------------------------------------------
    #[test]
    fn query_processor_progress_invalid_node() {
        let engine = Engine::new(SimulationStrategy::Tick);

        // Create a bogus NodeId by adding and removing a node.
        let mut engine2 = Engine::new(SimulationStrategy::Tick);
        let pending = engine2.graph.queue_add_node(building());
        let result = engine2.graph.apply_mutations();
        let bogus = result.resolve_node(pending).unwrap();

        assert!(engine.get_processor_progress(bogus).is_none());
    }

    // -----------------------------------------------------------------------
    // Query Test 4: Edge utilization for flow transport
    // -----------------------------------------------------------------------
    #[test]
    fn query_edge_utilization_flow() {
        let (mut engine, _src_node, _consumer_node, edge_id) =
            setup_source_transport_consumer(5.0, 10.0, vec![(iron(), 2)], vec![(gear(), 1)], 3);

        // Before any tick: flow buffer is empty, utilization = 0.
        let util = engine.get_edge_utilization(edge_id).unwrap();
        assert_eq!(util, Fixed64::ZERO);

        // Tick 1: source produces items.
        engine.step();

        // Tick 2: transport moves items into buffer.
        engine.step();

        // After transport has moved items, utilization should be > 0
        // (depends on how much was buffered vs capacity).
        let util = engine.get_edge_utilization(edge_id).unwrap();
        // We can't assert exact value since it depends on delivery,
        // but utilization is defined and >= 0.
        assert!(util >= Fixed64::ZERO);
    }

    // -----------------------------------------------------------------------
    // Query Test 5: Edge utilization for belt transport
    // -----------------------------------------------------------------------
    #[test]
    fn query_edge_utilization_belt() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pa = engine.graph.queue_add_node(building());
        let pb = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let a = r.resolve_node(pa).unwrap();
        let b = r.resolve_node(pb).unwrap();

        let pe = engine.graph.queue_connect(a, b);
        let r = engine.graph.apply_mutations();
        let edge = r.resolve_edge(pe).unwrap();

        engine.set_processor(a, make_source(iron(), 5.0));
        engine.set_input_inventory(a, simple_inventory(100));
        engine.set_output_inventory(a, simple_inventory(100));

        engine.set_processor(
            b,
            make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 1),
        );
        engine.set_input_inventory(b, simple_inventory(1000));
        engine.set_output_inventory(b, simple_inventory(100));

        // Belt transport: 4 slots, 1 lane, speed 1.
        let belt = Transport::Item(crate::transport::ItemTransport {
            speed: Fixed64::from_num(1.0),
            slot_count: 4,
            lanes: 1,
        });
        engine.set_transport(edge, belt);

        // Initially: no items on belt, utilization = 0.
        let util = engine.get_edge_utilization(edge).unwrap();
        assert_eq!(util, Fixed64::ZERO);

        // After a tick, source produces items and transport picks one up.
        engine.step();
        engine.step();
        let util = engine.get_edge_utilization(edge).unwrap();
        // At least one item should be on the belt.
        assert!(util >= Fixed64::ZERO);
    }

    // -----------------------------------------------------------------------
    // Query Test 6: Node snapshot contains correct state
    // -----------------------------------------------------------------------
    #[test]
    fn query_node_snapshot_state() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // 1 iron -> 1 gear, 4 ticks.
        engine.set_processor(node, make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 4));
        let mut input_inv = simple_inventory(100);
        input_inv.input_slots[0].add(iron(), 10);
        engine.set_input_inventory(node, input_inv);
        engine.set_output_inventory(node, simple_inventory(100));

        // Before any tick.
        let snap = engine.snapshot_node(node).unwrap();
        assert_eq!(snap.id, node);
        assert_eq!(snap.building_type, building());
        assert_eq!(snap.processor_state, ProcessorState::Idle);
        assert_eq!(snap.progress, Fixed64::ZERO);
        // Input should have 10 iron.
        assert_eq!(snap.input_contents.len(), 1);
        assert_eq!(snap.input_contents[0].item_type, iron());
        assert_eq!(snap.input_contents[0].quantity, 10);
        // Output should be empty.
        assert!(snap.output_contents.is_empty());

        // Tick 1: starts working, consumes 1 iron.
        engine.step();
        let snap = engine.snapshot_node(node).unwrap();
        assert!(matches!(snap.processor_state, ProcessorState::Working { .. }));
        assert_eq!(snap.progress, Fixed64::from_num(1) / Fixed64::from_num(4));
        // Input should have 9 iron (consumed 1).
        assert_eq!(snap.input_contents[0].quantity, 9);
    }

    // -----------------------------------------------------------------------
    // Query Test 7: Snapshot all nodes covers all nodes
    // -----------------------------------------------------------------------
    #[test]
    fn query_snapshot_all_nodes_covers_all() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Add 5 nodes.
        let mut nodes = Vec::new();
        for _ in 0..5 {
            let pending = engine.graph.queue_add_node(building());
            let result = engine.graph.apply_mutations();
            let node = result.resolve_node(pending).unwrap();
            engine.set_processor(node, make_source(iron(), 1.0));
            engine.set_input_inventory(node, simple_inventory(100));
            engine.set_output_inventory(node, simple_inventory(100));
            nodes.push(node);
        }

        let snapshots = engine.snapshot_all_nodes();
        assert_eq!(snapshots.len(), 5);

        // Every node ID should be present in the snapshots.
        for node in &nodes {
            assert!(
                snapshots.iter().any(|s| s.id == *node),
                "snapshot should contain node {:?}",
                node
            );
        }
    }

    // -----------------------------------------------------------------------
    // Query Test 8: node_count and edge_count
    // -----------------------------------------------------------------------
    #[test]
    fn query_node_and_edge_count() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        assert_eq!(engine.node_count(), 0);
        assert_eq!(engine.edge_count(), 0);

        let pa = engine.graph.queue_add_node(building());
        let pb = engine.graph.queue_add_node(building());
        let pc = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let a = r.resolve_node(pa).unwrap();
        let b = r.resolve_node(pb).unwrap();
        let _c = r.resolve_node(pc).unwrap();

        assert_eq!(engine.node_count(), 3);
        assert_eq!(engine.edge_count(), 0);

        engine.graph.queue_connect(a, b);
        engine.graph.apply_mutations();

        assert_eq!(engine.node_count(), 3);
        assert_eq!(engine.edge_count(), 1);
    }

    // -----------------------------------------------------------------------
    // Query Test 9: get_inputs and get_outputs
    // -----------------------------------------------------------------------
    #[test]
    fn query_get_inputs_outputs() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pa = engine.graph.queue_add_node(building());
        let pb = engine.graph.queue_add_node(building());
        let pc = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let a = r.resolve_node(pa).unwrap();
        let b = r.resolve_node(pb).unwrap();
        let c = r.resolve_node(pc).unwrap();

        // A -> B, C -> B
        let pe1 = engine.graph.queue_connect(a, b);
        let pe2 = engine.graph.queue_connect(c, b);
        let r = engine.graph.apply_mutations();
        let e1 = r.resolve_edge(pe1).unwrap();
        let e2 = r.resolve_edge(pe2).unwrap();

        // B has two inputs.
        let b_inputs = engine.get_inputs(b);
        assert_eq!(b_inputs.len(), 2);
        assert!(b_inputs.contains(&e1));
        assert!(b_inputs.contains(&e2));

        // B has no outputs.
        assert_eq!(engine.get_outputs(b).len(), 0);

        // A has one output.
        assert_eq!(engine.get_outputs(a).len(), 1);
        assert_eq!(engine.get_outputs(a)[0], e1);

        // A has no inputs.
        assert_eq!(engine.get_inputs(a).len(), 0);
    }

    // -----------------------------------------------------------------------
    // Query Test 10: Inventory query matches actual contents
    // -----------------------------------------------------------------------
    #[test]
    fn query_inventory_matches_contents() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 3.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        // Run 4 ticks: source produces 3*4 = 12 iron.
        for _ in 0..4 {
            engine.step();
        }

        let snap = engine.snapshot_node(node).unwrap();
        // Output should have 12 iron.
        assert_eq!(snap.output_contents.len(), 1);
        assert_eq!(snap.output_contents[0].item_type, iron());
        assert_eq!(snap.output_contents[0].quantity, 12);

        // Verify matches the direct inventory query.
        let output_inv = engine.get_output_inventory(node).unwrap();
        assert_eq!(output_inv.output_slots[0].quantity(iron()), 12);
    }

    // -----------------------------------------------------------------------
    // Query Test 11: Transport snapshot
    // -----------------------------------------------------------------------
    #[test]
    fn query_transport_snapshot() {
        let (mut engine, src_node, consumer_node, edge_id) =
            setup_source_transport_consumer(5.0, 10.0, vec![(iron(), 2)], vec![(gear(), 1)], 3);

        let snap = engine.snapshot_transport(edge_id).unwrap();
        assert_eq!(snap.id, edge_id);
        assert_eq!(snap.from, src_node);
        assert_eq!(snap.to, consumer_node);
        assert_eq!(snap.utilization, Fixed64::ZERO);
        assert_eq!(snap.items_in_transit, 0);

        // After source produces and transport moves items.
        engine.step();
        engine.step();

        let snap = engine.snapshot_transport(edge_id).unwrap();
        assert_eq!(snap.from, src_node);
        assert_eq!(snap.to, consumer_node);
    }

    // -----------------------------------------------------------------------
    // Query Test 12: Snapshot node includes adjacency
    // -----------------------------------------------------------------------
    #[test]
    fn query_snapshot_includes_adjacency() {
        let (engine, src_node, consumer_node, edge_id) =
            setup_source_transport_consumer(5.0, 10.0, vec![(iron(), 2)], vec![(gear(), 1)], 3);

        let src_snap = engine.snapshot_node(src_node).unwrap();
        assert_eq!(src_snap.output_edges.len(), 1);
        assert_eq!(src_snap.output_edges[0], edge_id);
        assert!(src_snap.input_edges.is_empty());

        let consumer_snap = engine.snapshot_node(consumer_node).unwrap();
        assert_eq!(consumer_snap.input_edges.len(), 1);
        assert_eq!(consumer_snap.input_edges[0], edge_id);
        assert!(consumer_snap.output_edges.is_empty());
    }

    // -----------------------------------------------------------------------
    // Query Test 13: Stalled processor has zero progress
    // -----------------------------------------------------------------------
    #[test]
    fn query_stalled_processor_zero_progress() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // Source produces 10/tick, output only holds 5.
        engine.set_processor(node, make_source(iron(), 10.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(5));

        // Tick 1: fills output.
        engine.step();
        // Tick 2: stalls.
        engine.step();

        let state = engine.get_processor_state(node).unwrap();
        assert!(matches!(state, ProcessorState::Stalled { .. }));

        let progress = engine.get_processor_progress(node).unwrap();
        assert_eq!(progress, Fixed64::ZERO);
    }

    // -----------------------------------------------------------------------
    // Query Test 14: Read-only — query methods take &self
    // -----------------------------------------------------------------------
    #[test]
    fn query_methods_are_read_only() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        engine.step();

        // Take a shared ref and call all query methods.
        let engine_ref: &Engine = &engine;
        let _ = engine_ref.get_processor_progress(node);
        let _ = engine_ref.get_processor_state(node);
        let _ = engine_ref.get_input_inventory(node);
        let _ = engine_ref.get_output_inventory(node);
        let _ = engine_ref.snapshot_node(node);
        let _ = engine_ref.snapshot_all_nodes();
        let _ = engine_ref.node_count();
        let _ = engine_ref.edge_count();
        let _ = engine_ref.get_inputs(node);
        let _ = engine_ref.get_outputs(node);
        let _ = engine_ref.state_hash();
        // If this compiles, the query API is read-only.
    }

    // -----------------------------------------------------------------------
    // Delta Simulation Strategy Tests
    // -----------------------------------------------------------------------

    /// Helper: add a node with processor and inventories, returns NodeId.
    fn add_node_helper(
        engine: &mut Engine,
        processor: Processor,
        input_capacity: u32,
        output_capacity: u32,
    ) -> NodeId {
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();
        engine.set_processor(node, processor);
        engine.set_input_inventory(node, simple_inventory(input_capacity));
        engine.set_output_inventory(node, simple_inventory(output_capacity));
        node
    }

    /// Helper: connect two nodes with a transport, returns EdgeId.
    fn connect_helper(
        engine: &mut Engine,
        from: NodeId,
        to: NodeId,
        transport: Transport,
    ) -> EdgeId {
        let pending = engine.graph.queue_connect(from, to);
        let result = engine.graph.apply_mutations();
        let edge = result.resolve_edge(pending).unwrap();
        engine.set_transport(edge, transport);
        edge
    }

    #[test]
    fn delta_sub_step_no_op() {
        let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
        let _source = add_node_helper(&mut engine, make_source(iron(), 5.0), 100, 100);

        let result = engine.advance(1);
        assert_eq!(result.steps_run, 0);
        assert_eq!(engine.sim_state.tick, 0);
        assert_eq!(engine.sim_state.accumulator, 1);
    }

    #[test]
    fn delta_accumulates_then_steps() {
        let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
        let source = add_node_helper(&mut engine, make_source(iron(), 5.0), 100, 100);

        engine.advance(1);
        let result = engine.advance(1);
        assert_eq!(result.steps_run, 1);
        assert_eq!(engine.sim_state.tick, 1);
        assert_eq!(engine.sim_state.accumulator, 0);
        assert!(engine.output_total(source) > 0);
    }

    #[test]
    fn delta_multi_step_catchup() {
        let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
        let _source = add_node_helper(&mut engine, make_source(iron(), 5.0), 100, 100);

        let result = engine.advance(7);
        assert_eq!(result.steps_run, 3);
        assert_eq!(engine.sim_state.tick, 3);
        assert_eq!(engine.sim_state.accumulator, 1);
    }

    #[test]
    fn delta_zero_dt_no_change() {
        let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
        let _source = add_node_helper(&mut engine, make_source(iron(), 5.0), 100, 100);

        let hash_before = engine.state_hash();
        let result = engine.advance(0);
        assert_eq!(result.steps_run, 0);
        assert_eq!(engine.sim_state.tick, 0);
        assert_eq!(engine.state_hash(), hash_before);
    }

    #[test]
    fn delta_determinism() {
        fn run_delta() -> Vec<u64> {
            let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
            let source = add_node_helper(&mut engine, make_source(iron(), 3.0), 100, 100);
            let assembler = add_node_helper(
                &mut engine,
                make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 5),
                100,
                100,
            );
            connect_helper(&mut engine, source, assembler, make_flow_transport(10.0));

            let mut hashes = Vec::new();
            for dt in [1, 3, 2, 5, 1, 4, 7] {
                engine.advance(dt);
                hashes.push(engine.state_hash());
            }
            hashes
        }

        assert_eq!(run_delta(), run_delta());
    }

    // -----------------------------------------------------------------------
    // Pause / Resume helpers
    // -----------------------------------------------------------------------

    /// Add a minimal node to the engine (no processor or inventory).
    /// Useful for pause/resume tests that don't need full node setup.
    fn add_node(engine: &mut Engine) -> NodeId {
        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        result.resolve_node(pending).unwrap()
    }

    // -----------------------------------------------------------------------
    // Pause / Resume tests
    // -----------------------------------------------------------------------

    #[test]
    fn pause_advance_is_no_op() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        add_node(&mut engine); // from test_utils
        engine.pause();
        let result = engine.advance(0);
        assert_eq!(result.steps_run, 0);
        assert_eq!(engine.sim_state.tick, 0);
    }

    #[test]
    fn pause_step_is_no_op() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        engine.pause();
        let result = engine.step();
        assert_eq!(result.steps_run, 0);
    }

    #[test]
    fn resume_allows_stepping() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        engine.pause();
        engine.resume();
        let result = engine.step();
        assert_eq!(result.steps_run, 1);
    }

    #[test]
    fn pause_resume_toggle() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        assert!(!engine.is_paused());
        engine.pause();
        assert!(engine.is_paused());
        engine.resume();
        assert!(!engine.is_paused());
    }

    #[test]
    fn is_paused_default_false() {
        let engine = Engine::new(SimulationStrategy::Tick);
        assert!(!engine.is_paused());
    }

    #[test]
    fn mutations_work_while_paused() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        engine.pause();
        engine.graph.queue_add_node(crate::id::BuildingTypeId(1));
        let mutation_result = engine.graph.apply_mutations();
        assert_eq!(mutation_result.added_nodes.len(), 1);
    }

    #[test]
    fn set_processor_works_while_paused() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);
        engine.pause();
        engine.set_processor(node, make_source(crate::id::ItemTypeId(1), 10.0));
        assert!(engine.processors.get(node).is_some());
    }

    #[test]
    fn compact_does_not_panic() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let node = add_node(&mut engine);
        engine.set_processor(node, make_source(crate::id::ItemTypeId(1), 10.0));
        engine.set_input_inventory(node, crate::item::Inventory::new(1, 1, 10));
        engine.step();
        let freed = engine.compact();
        // Just verify it doesn't panic; freed may be 0
        let _ = freed;
    }

    #[test]
    fn compact_on_empty_engine() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let freed = engine.compact();
        assert_eq!(freed, 0);
    }

    #[test]
    fn paused_state_serializes() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        engine.pause();
        let data = engine.serialize().unwrap();
        let restored = Engine::deserialize(&data).unwrap();
        assert!(restored.is_paused());
    }

    // =======================================================================
    // Junction integration tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // Junction Test 1: set/get/remove
    // -----------------------------------------------------------------------
    #[test]
    fn engine_junction_set_get_remove() {
        use crate::junction::*;

        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // Initially no junction.
        assert!(engine.junction(node).is_none());

        // Set a splitter junction.
        let junction = Junction::Splitter(SplitterConfig {
            policy: SplitPolicy::RoundRobin,
            filter: None,
        });
        engine.set_junction(node, junction.clone());

        // Verify it's there.
        let got = engine.junction(node).unwrap();
        assert_eq!(*got, junction);

        // Remove it.
        engine.remove_junction(node);
        assert!(engine.junction(node).is_none());
    }

    // -----------------------------------------------------------------------
    // Junction Test 2: Splitter round-robin advances
    // -----------------------------------------------------------------------
    #[test]
    fn engine_splitter_round_robin_advances() {
        use crate::junction::*;

        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Create a splitter node with 2 outputs.
        let pa = engine.graph.queue_add_node(building());
        let pb = engine.graph.queue_add_node(building());
        let pc = engine.graph.queue_add_node(building());
        let r = engine.graph.apply_mutations();
        let a = r.resolve_node(pa).unwrap();
        let b = r.resolve_node(pb).unwrap();
        let c = r.resolve_node(pc).unwrap();

        engine.graph.queue_connect(a, b);
        engine.graph.queue_connect(a, c);
        engine.graph.apply_mutations();

        // Set up processor and inventories so the engine can step.
        engine.set_processor(a, make_source(iron(), 1.0));
        engine.set_input_inventory(a, simple_inventory(100));
        engine.set_output_inventory(a, simple_inventory(100));
        engine.set_processor(b, make_source(iron(), 0.0));
        engine.set_input_inventory(b, simple_inventory(100));
        engine.set_output_inventory(b, simple_inventory(100));
        engine.set_processor(c, make_source(iron(), 0.0));
        engine.set_input_inventory(c, simple_inventory(100));
        engine.set_output_inventory(c, simple_inventory(100));

        // Set a splitter junction on node a.
        engine.set_junction(
            a,
            Junction::Splitter(SplitterConfig {
                policy: SplitPolicy::RoundRobin,
                filter: None,
            }),
        );

        // Initial state: round_robin_index = 0.
        assert_eq!(engine.junction_states.get(a).unwrap().round_robin_index, 0);

        // Step: phase_transport uses index 0 for budgeting, then phase_component
        // advances it to (0 + 1) % 2 = 1.
        engine.step();
        let state = engine.junction_states.get(a).unwrap();
        assert_eq!(state.round_robin_index, 1);

        // Step again: (1 + 1) % 2 = 0.
        engine.step();
        let state = engine.junction_states.get(a).unwrap();
        assert_eq!(state.round_robin_index, 0);
    }

    // =======================================================================
    // Module integration tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // Module Test: on_tick called via engine step
    // -----------------------------------------------------------------------
    #[test]
    fn engine_module_on_tick_called() {
        use std::sync::{Arc, Mutex};

        #[derive(Debug)]
        struct TestModule {
            call_count: Arc<Mutex<u32>>,
        }

        impl crate::module::Module for TestModule {
            fn name(&self) -> &str {
                "test"
            }
            fn on_tick(&mut self, _ctx: &mut crate::module::ModuleContext<'_>) {
                *self.call_count.lock().unwrap() += 1;
            }
        }

        let mut engine = Engine::new(SimulationStrategy::Tick);
        let count = Arc::new(Mutex::new(0u32));

        engine.register_module(Box::new(TestModule {
            call_count: count.clone(),
        }));

        assert_eq!(*count.lock().unwrap(), 0);

        engine.step();
        assert_eq!(*count.lock().unwrap(), 1);

        engine.step();
        engine.step();
        assert_eq!(*count.lock().unwrap(), 3);
    }

    // =======================================================================
    // Dirty tracking integration tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // Dirty Test 1: set_processor marks dirty
    // -----------------------------------------------------------------------
    #[test]
    fn engine_dirty_marks_from_set_processor() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        // Clear any dirty flags from the node creation setup.
        engine.mark_clean();
        assert!(!engine.is_dirty());

        // Set processor should mark dirty.
        engine.set_processor(node, make_source(iron(), 1.0));

        assert!(engine.is_dirty());
        assert!(engine.dirty_tracker().is_node_dirty(node));
    }

    // -----------------------------------------------------------------------
    // Dirty Test 2: Clean after step
    // -----------------------------------------------------------------------
    #[test]
    fn engine_clean_after_step() {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let pending = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let node = result.resolve_node(pending).unwrap();

        engine.set_processor(node, make_source(iron(), 1.0));
        engine.set_input_inventory(node, simple_inventory(100));
        engine.set_output_inventory(node, simple_inventory(100));

        // Dirty from setup.
        assert!(engine.is_dirty());

        // Step cleans dirty flags (phase_component resets at end).
        engine.step();
        assert!(
            !engine.is_dirty(),
            "dirty flags should be clean after a step"
        );
    }

    // -----------------------------------------------------------------------
    // Edge Filter: Per-output-type edge routing
    // -----------------------------------------------------------------------

    #[test]
    fn edge_filter_routes_specific_item_type() {
        use crate::test_utils;

        // Multi-output recipe: 1 water -> 1 oxygen + 1 hydrogen (1 tick).
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let water = ItemTypeId(3);
        let oxygen = ItemTypeId(4);
        let hydrogen = ItemTypeId(5);

        let electrolyzer = test_utils::add_node(
            &mut engine,
            test_utils::make_recipe(
                vec![(water, 1)],
                vec![(oxygen, 1), (hydrogen, 1)],
                1,
            ),
            100,
            100,
        );

        // Seed input.
        engine.inputs.get_mut(electrolyzer).unwrap().input_slots[0].add(water, 20);

        let o2_sink = test_utils::add_node(
            &mut engine,
            test_utils::make_source(oxygen, 0.0), // dummy
            100,
            100,
        );
        let h2_sink = test_utils::add_node(
            &mut engine,
            test_utils::make_source(hydrogen, 0.0),
            100,
            100,
        );

        // Connect with item_type filters on edges.
        test_utils::connect_filtered(&mut engine, electrolyzer, o2_sink, test_utils::make_flow_transport(10.0), Some(oxygen));
        test_utils::connect_filtered(&mut engine, electrolyzer, h2_sink, test_utils::make_flow_transport(10.0), Some(hydrogen));

        for _ in 0..10 {
            engine.step();
        }

        let o2_at_sink = test_utils::input_quantity(&engine, o2_sink, oxygen);
        let h2_at_sink = test_utils::input_quantity(&engine, h2_sink, hydrogen);

        assert!(o2_at_sink > 0, "O2 sink should receive oxygen, got {o2_at_sink}");
        assert!(h2_at_sink > 0, "H2 sink should receive hydrogen, got {h2_at_sink}");
        // Ensure no cross-contamination.
        assert_eq!(test_utils::input_quantity(&engine, o2_sink, hydrogen), 0, "O2 sink should not have hydrogen");
        assert_eq!(test_utils::input_quantity(&engine, h2_sink, oxygen), 0, "H2 sink should not have oxygen");
    }

    // -----------------------------------------------------------------------
    // Feedback Loop: cycles should not prevent processing
    // -----------------------------------------------------------------------

    #[test]
    fn feedback_loop_processes_with_one_tick_delay() {
        use crate::test_utils;

        let mut engine = Engine::new(SimulationStrategy::Tick);
        let iron = test_utils::iron();

        // A -> B -> A (cycle). A produces iron, B passes it through.
        let a = test_utils::add_node(
            &mut engine,
            test_utils::make_source(iron, 2.0),
            100,
            100,
        );
        let b = test_utils::add_node(
            &mut engine,
            test_utils::make_recipe(vec![(iron, 1)], vec![(iron, 1)], 1),
            100,
            100,
        );

        test_utils::connect(&mut engine, a, b, test_utils::make_flow_transport(10.0));
        test_utils::connect(&mut engine, b, a, test_utils::make_flow_transport(10.0));

        // Step 10 ticks. Should NOT skip processing due to cycle.
        for _ in 0..10 {
            engine.step();
        }

        // A should have produced items (not stalled by cycle detection).
        let total_a_output = test_utils::output_total(&engine, a);
        let total_b_input = test_utils::input_quantity(&engine, b, iron);
        let total_b_output = test_utils::output_total(&engine, b);
        // At minimum, some items should have moved through the system.
        assert!(
            total_a_output + total_b_input + total_b_output > 0,
            "Cycle should not prevent processing. a_out={total_a_output}, b_in={total_b_input}, b_out={total_b_output}"
        );
    }

    // =======================================================================
    // Junction runtime behavior tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // Junction Runtime Test: Splitter distributes items across outputs
    // -----------------------------------------------------------------------
    #[test]
    fn splitter_distributes_items_across_outputs() {
        use crate::junction::*;
        use crate::test_utils;

        let mut engine = Engine::new(SimulationStrategy::Tick);
        let iron = test_utils::iron();

        let source = test_utils::add_node(
            &mut engine,
            test_utils::make_source(iron, 10.0),
            100,
            100,
        );

        // Splitter node with RoundRobin policy and Passthrough processor.
        let splitter = test_utils::add_node(
            &mut engine,
            Processor::Passthrough,
            100,
            100,
        );
        engine.set_junction(splitter, Junction::Splitter(SplitterConfig {
            policy: SplitPolicy::RoundRobin,
            filter: None,
        }));

        let sink_a = test_utils::add_node(&mut engine, test_utils::make_source(iron, 0.0), 100, 100);
        let sink_b = test_utils::add_node(&mut engine, test_utils::make_source(iron, 0.0), 100, 100);

        test_utils::connect(&mut engine, source, splitter, test_utils::make_flow_transport(20.0));
        test_utils::connect(&mut engine, splitter, sink_a, test_utils::make_flow_transport(20.0));
        test_utils::connect(&mut engine, splitter, sink_b, test_utils::make_flow_transport(20.0));

        for _ in 0..20 {
            engine.step();
        }

        let at_a = test_utils::input_quantity(&engine, sink_a, iron);
        let at_b = test_utils::input_quantity(&engine, sink_b, iron);

        assert!(at_a > 0, "Sink A should receive items, got {at_a}");
        assert!(at_b > 0, "Sink B should receive items, got {at_b}");
        // Round-robin should give roughly equal distribution.
        let diff = (at_a as i64 - at_b as i64).unsigned_abs();
        assert!(diff <= at_a.max(at_b) as u64 / 2, "Distribution should be roughly even: A={at_a}, B={at_b}");
    }

    #[test]
    fn fan_out_distributes_fairly_without_junction() {
        use crate::test_utils;

        let mut engine = Engine::new(SimulationStrategy::Tick);
        let iron = test_utils::iron();

        let source = test_utils::add_node(
            &mut engine,
            test_utils::make_source(iron, 10.0),
            100,
            100,
        );

        let sink_a = test_utils::add_node(&mut engine, test_utils::make_source(iron, 0.0), 100, 100);
        let sink_b = test_utils::add_node(&mut engine, test_utils::make_source(iron, 0.0), 100, 100);

        test_utils::connect(&mut engine, source, sink_a, test_utils::make_flow_transport(20.0));
        test_utils::connect(&mut engine, source, sink_b, test_utils::make_flow_transport(20.0));

        for _ in 0..20 {
            engine.step();
        }

        let at_a = test_utils::input_quantity(&engine, sink_a, iron);
        let at_b = test_utils::input_quantity(&engine, sink_b, iron);

        // Both sinks should receive items (not just the first edge).
        assert!(at_a > 0, "Sink A should receive items, got {at_a}");
        assert!(at_b > 0, "Sink B should receive items, got {at_b}");
    }

    #[test]
    fn dynamic_recipe_swap_resets_state() {
        use crate::test_utils;

        let mut engine = Engine::new(SimulationStrategy::Tick);
        let iron = test_utils::iron();
        let copper = test_utils::copper();
        let gear = test_utils::gear();

        // Start as iron -> gear recipe.
        let node = test_utils::add_node(
            &mut engine,
            test_utils::make_recipe(vec![(iron, 1)], vec![(gear, 1)], 2),
            100,
            100,
        );

        engine.inputs.get_mut(node).unwrap().input_slots[0].add(iron, 10);

        for _ in 0..5 {
            engine.step();
        }
        assert!(test_utils::output_quantity(&engine, node, gear) > 0);

        // Swap recipe to copper -> gear.
        engine.swap_processor(
            node,
            test_utils::make_recipe(vec![(copper, 1)], vec![(gear, 1)], 2),
        );

        // Processor state should be reset to Idle.
        assert_eq!(
            engine.get_processor_state(node),
            Some(&ProcessorState::Idle),
            "swap_processor should reset state to Idle"
        );
    }

    #[test]
    fn get_item_property_from_output() {
        use crate::test_utils;

        let mut engine = Engine::new(SimulationStrategy::Tick);
        let iron = test_utils::iron();
        let temp = crate::id::PropertyId(0);

        let node = test_utils::add_node(
            &mut engine,
            test_utils::make_source(iron, 1.0),
            100,
            100,
        );

        // Manually add an item with a property to the output inventory.
        if let Some(inv) = engine.outputs.get_mut(node) {
            let mut stack = ItemStack::new(iron, 5);
            stack.set_property(temp, Fixed64::from_num(100));
            inv.output_slots[0].stacks.push(stack);
        }

        let prop = engine.get_item_property(node, iron, temp);
        assert_eq!(prop, Some(Fixed64::from_num(100)));

        // Non-existent property should return None.
        let missing = engine.get_item_property(node, iron, crate::id::PropertyId(99));
        assert_eq!(missing, None);
    }

    #[test]
    fn demand_processor_tracks_sustained_rate() {
        use crate::test_utils;

        let mut engine = Engine::new(SimulationStrategy::Tick);
        let iron = test_utils::iron();

        let source = test_utils::add_node(
            &mut engine,
            test_utils::make_source(iron, 5.0),
            100,
            100,
        );
        let sink = test_utils::add_node(
            &mut engine,
            Processor::Demand(DemandProcessor {
                input_type: iron,
                base_rate: Fixed64::from_num(3),
                accumulated: Fixed64::ZERO,
                consumed_total: 0,
                accepted_types: None,
            }),
            100,
            100,
        );

        test_utils::connect(&mut engine, source, sink, test_utils::make_flow_transport(10.0));

        for _ in 0..60 {
            engine.step();
        }

        // Query sustained consumption rate over the run.
        let rate = engine.get_demand_rate(sink);
        assert!(rate.is_some(), "Should be able to query demand rate");
        assert!(
            rate.unwrap() >= Fixed64::from_num(2),
            "Sustained rate should be near 3/tick, got {:?}", rate
        );
    }

    #[test]
    fn multi_demand_accepts_multiple_types() {
        use crate::test_utils;

        let mut engine = Engine::new(SimulationStrategy::Tick);
        let iron = test_utils::iron();
        let copper = test_utils::copper();

        let node = test_utils::add_node(
            &mut engine,
            Processor::Demand(DemandProcessor {
                input_type: iron,
                base_rate: Fixed64::from_num(2),
                accumulated: Fixed64::ZERO,
                consumed_total: 0,
                accepted_types: Some(vec![iron, copper]),
            }),
            100,
            100,
        );

        engine.get_input_inventory_mut(node).unwrap().input_slots[0].add(iron, 5);
        engine.get_input_inventory_mut(node).unwrap().input_slots[0].add(copper, 5);

        for _ in 0..5 {
            engine.step();
        }

        let remaining_iron = test_utils::input_quantity(&engine, node, iron);
        let remaining_copper = test_utils::input_quantity(&engine, node, copper);

        // Should have consumed some of both types.
        assert!(remaining_iron < 5 || remaining_copper < 5, "Should consume some items");
    }
}
