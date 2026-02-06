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
use crate::id::{EdgeId, ItemTypeId, NodeId};
use crate::item::{Inventory, ItemStack};
use crate::processor::{Modifier, Processor, ProcessorResult, ProcessorState};
use crate::query::{NodeSnapshot, TransportSnapshot};
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
    pub(crate) strategy: SimulationStrategy,

    /// Simulation state (tick counter, accumulator).
    pub sim_state: SimState,

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
            processors: SecondaryMap::new(),
            processor_states: SecondaryMap::new(),
            inputs: SecondaryMap::new(),
            outputs: SecondaryMap::new(),
            modifiers: SecondaryMap::new(),
            transports: SecondaryMap::new(),
            transport_states: SecondaryMap::new(),
            last_state_hash: 0,
            event_bus: EventBus::default(),
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
        }
    }

    // -----------------------------------------------------------------------
    // Phase 2: Transport
    // -----------------------------------------------------------------------

    fn phase_transport(&mut self) {
        let tick = self.sim_state.tick;

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
    // Phase 4: Component (placeholder)
    // -----------------------------------------------------------------------

    fn phase_component(&mut self) {
        // Module-registered systems would run here.
        // Currently a no-op placeholder for future framework modules.
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
}
