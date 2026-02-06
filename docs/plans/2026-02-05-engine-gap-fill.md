# Engine Gap Fill Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fill the 14 engine gaps identified by game-inspired integration tests (Factorio, Satisfactory, Shapez, ONI) while keeping flexibility as the core tenet — everything configurable by the game dev.

**Architecture:** Changes are scoped to 3 layers: (1) core engine data structures (`EdgeData`, `Processor`, `Inventory`), (2) engine tick phases (transport, process, component), and (3) satellite modules (power, fluid). Each gap is addressed with the minimum viable change, using existing patterns (enum dispatch, SoA `SecondaryMap`, `Module` trait). No new trait objects or dynamic dispatch added.

**Tech Stack:** Rust, `slotmap`, `Fixed64` fixed-point arithmetic, `bitcode`/`serde` for serialization, existing `factorial-core`/`factorial-power`/`factorial-fluid`/`factorial-stats` crates.

---

## Priority Order & Dependencies

The gaps are ordered by dependency chain — earlier tasks unlock later ones:

1. **Per-output-type edge routing** (unlocks: multi-output recipes, mixed-phase routing, smart splitters)
2. **Feedback loops** (unlocks: SPOM self-powering, any circular production)
3. **Passthrough processor** (unlocks: junction nodes, splitters, mergers)
4. **Junction runtime behavior** (unlocks: fair fan-out, balancers)
5. **Fair fan-out distribution** (uses junction system)
6. **Dynamic recipe selection** (unlocks: make-anything machines, recipe swapping)
7. **Item property tracking** (unlocks: temperature chains, property-based power)
8. **Power priority system** (extends PowerModule)
9. **Dynamic power production** (extends PowerModule + property tracking)
10. **Dual-role nodes** (extends processor system)
11. **DemandProcessor sustained rate** (extends stats integration)
12. **Multi-type DemandProcessor** (extends DemandProcessor)
13. **Fluid module consumer keying** (extends FluidModule)
14. **Fluid-to-item bridge** (extends FluidModule + inventory system)

---

## Task 1: Per-Output-Type Edge Routing

**Why:** Multi-output recipes (electrolyzer: water→O2+H2, oil refinery: crude→petroleum+gas, cutter: circle→left+right) send ALL outputs to a single inventory. Downstream edges grab items indiscriminately (first-edge-wins). Game devs need to route specific output types to specific edges.

**Files:**
- Modify: `crates/factorial-core/src/graph.rs` (EdgeData)
- Modify: `crates/factorial-core/src/engine.rs` (phase_transport, apply_transport_result, determine_item_type_for_edge)
- Modify: `crates/factorial-core/src/test_utils.rs` (new helper)
- Test: `crates/factorial-core/src/engine.rs` (new unit tests in `mod tests`)

### Step 1: Write the failing test

Add to `crates/factorial-core/src/engine.rs` test module:

```rust
#[test]
fn edge_filter_routes_specific_item_type() {
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
```

### Step 2: Run test to verify it fails

Run: `cargo test -p factorial-core edge_filter_routes_specific_item_type -- --nocapture 2>&1 | tail -20`
Expected: FAIL — `connect_filtered` does not exist.

### Step 3: Add `item_filter` to EdgeData

In `crates/factorial-core/src/graph.rs`, add `item_filter` field to `EdgeData`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeData {
    pub from: NodeId,
    pub to: NodeId,
    /// Optional item type filter. When set, only this item type flows on this edge.
    pub item_filter: Option<ItemTypeId>,
}
```

Update all places that construct `EdgeData` (the `apply_mutations` method inside `Mutation::Connect`) to initialize `item_filter: None`.

### Step 4: Add `queue_connect_filtered` to ProductionGraph

In `crates/factorial-core/src/graph.rs`, add a new mutation variant and queue method:

```rust
// In enum Mutation:
ConnectFiltered {
    from: NodeId,
    to: NodeId,
    pending_id: PendingEdgeId,
    item_filter: Option<ItemTypeId>,
},

// New method on ProductionGraph:
pub fn queue_connect_filtered(
    &mut self,
    from: NodeId,
    to: NodeId,
    item_filter: Option<ItemTypeId>,
) -> PendingEdgeId {
    let id = PendingEdgeId(self.next_pending_edge);
    self.next_pending_edge += 1;
    self.pending.push_back(Mutation::ConnectFiltered {
        from,
        to,
        pending_id: id,
        item_filter,
    });
    self.dirty = true;
    id
}
```

Handle `ConnectFiltered` in `apply_mutations()` just like `Connect` but set `edge_data.item_filter = item_filter`.

### Step 5: Add `connect_filtered` helper to test_utils

In `crates/factorial-core/src/test_utils.rs`:

```rust
/// Connect two nodes with an item type filter on the edge.
pub fn connect_filtered(
    engine: &mut Engine,
    from: NodeId,
    to: NodeId,
    transport: Transport,
    item_filter: Option<ItemTypeId>,
) -> EdgeId {
    let pending = engine.graph.queue_connect_filtered(from, to, item_filter);
    let result = engine.graph.apply_mutations();
    let edge = result.resolve_edge(pending).unwrap();
    engine.set_transport(edge, transport);
    edge
}
```

### Step 6: Update `phase_transport` to respect edge filter

In `crates/factorial-core/src/engine.rs`, modify `phase_transport` and `apply_transport_result`:

1. In `phase_transport`, after getting `edge_data`, read the filter:
```rust
let item_filter = edge_data.item_filter;
```

2. Change `output_total` call to be filter-aware — compute `available` as the quantity of the filtered item type only:
```rust
let available = match item_filter {
    Some(item_type) => self.output_quantity_of(source_node, item_type),
    None => self.output_total(source_node),
};
```

3. Add helper `output_quantity_of`:
```rust
fn output_quantity_of(&self, node: NodeId, item_type: ItemTypeId) -> u32 {
    self.outputs
        .get(node)
        .map(|inv| inv.output_slots.iter().map(|s| s.quantity(item_type)).sum())
        .unwrap_or(0)
}
```

4. In `apply_transport_result`, use the edge filter to determine item type:
```rust
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

    // Remove only the filtered item type from source output.
    if result.items_moved > 0 {
        if let Some(output_inv) = self.outputs.get_mut(source) {
            let mut remaining = result.items_moved;
            for slot in &mut output_inv.output_slots {
                if remaining == 0 { break; }
                let removed = slot.remove(item_type, remaining);
                remaining -= removed;
            }
        }
    }

    // Deliver to destination input.
    if result.items_delivered > 0 {
        if let Some(input_inv) = self.inputs.get_mut(dest) {
            let mut remaining = result.items_delivered;
            for slot in &mut input_inv.input_slots {
                if remaining == 0 { break; }
                let overflow = slot.add(item_type, remaining);
                remaining = overflow;
            }
        }
    }
}
```

Update the call site in `phase_transport` to pass `edge_id`.

### Step 7: Run test to verify it passes

Run: `cargo test -p factorial-core edge_filter_routes_specific_item_type -- --nocapture 2>&1 | tail -20`
Expected: PASS

### Step 8: Run full test suite

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: All existing tests still pass.

### Step 9: Commit

```bash
git add crates/factorial-core/src/graph.rs crates/factorial-core/src/engine.rs crates/factorial-core/src/test_utils.rs
git commit -m "feat: add per-output-type edge routing with item_filter on EdgeData

Adds optional item_filter field to EdgeData so edges can be restricted
to carry only a specific ItemTypeId. Transport phase respects the filter
when calculating available items and removing from source output.

Unlocks multi-output recipe routing (electrolyzer O2/H2, oil refinery,
Shapez cutter left/right).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Feedback Loop Support

**Why:** The topological sort returns `Err(CycleDetected)` on cycles, causing `phase_process` to skip ALL nodes. Self-powering systems (SPOM: electrolyzer→H2 generator→power→electrolyzer) require cycles. Solution: detect back-edges, break them, process acyclic portion normally, then process back-edge targets with a one-tick delay.

**Files:**
- Modify: `crates/factorial-core/src/graph.rs` (add back-edge detection, modified topo sort)
- Modify: `crates/factorial-core/src/engine.rs` (phase_process handles cycles)
- Test: `crates/factorial-core/src/engine.rs` (new tests)

### Step 1: Write the failing test

```rust
#[test]
fn feedback_loop_processes_with_one_tick_delay() {
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
```

### Step 2: Run test to verify it fails

Run: `cargo test -p factorial-core feedback_loop_processes_with_one_tick_delay -- --nocapture 2>&1 | tail -20`
Expected: FAIL — cycle causes `phase_process` to skip everything.

### Step 3: Add `topological_order_with_feedback` to ProductionGraph

In `crates/factorial-core/src/graph.rs`, add a method that returns the processing order plus a set of back-edges:

```rust
/// Returns a processing order even when cycles exist.
/// Back-edges (edges that would create a cycle) are returned separately.
/// Nodes involved in cycles are appended at the end of the order.
pub fn topological_order_with_feedback(&self) -> (Vec<NodeId>, Vec<EdgeId>) {
    // Standard Kahn's algorithm, but instead of returning Err on cycle,
    // collect remaining nodes (those in cycles) and append them.
    // Back-edges are the edges between cycle nodes that point "backwards".
    // ... (implementation uses the existing topo sort infrastructure)
}
```

The implementation:
1. Run Kahn's algorithm as normal.
2. If `order.len() < node_count`, the remaining nodes are in cycles.
3. For cycle nodes, pick an arbitrary ordering (e.g., by NodeId) and append them.
4. Identify back-edges: any edge where `to` appears before `from` in the final order.
5. Return `(order, back_edges)`.

### Step 4: Modify `phase_process` to use feedback-aware ordering

In `crates/factorial-core/src/engine.rs`, change `phase_process`:

```rust
fn phase_process(&mut self) {
    let (order, _back_edges) = self.graph.topological_order_with_feedback();
    for node_id in order {
        self.process_node(node_id);
    }
}
```

Back-edges naturally introduce a one-tick delay: items placed in output on tick N are transported on tick N+1, so cycle nodes see last-tick's output as this-tick's input.

### Step 5: Run test to verify it passes

Run: `cargo test -p factorial-core feedback_loop_processes_with_one_tick_delay -- --nocapture 2>&1 | tail -20`
Expected: PASS

### Step 6: Run full test suite

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: All tests pass (existing tests that don't have cycles still use the same order).

### Step 7: Commit

```bash
git add crates/factorial-core/src/graph.rs crates/factorial-core/src/engine.rs
git commit -m "feat: support feedback loops with one-tick delay

topological_order_with_feedback() runs Kahn's algorithm and appends
cycle nodes at the end rather than returning an error. phase_process
now uses this method, so cyclic graphs (SPOM, feedback loops) are
processed with natural one-tick delay on back-edges.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Passthrough Processor

**Why:** Junction nodes (splitters, mergers, balancers) need a processor that simply passes items from input to output without transformation. Currently there is no such processor variant.

**Files:**
- Modify: `crates/factorial-core/src/processor.rs` (add Passthrough variant)
- Modify: `crates/factorial-core/src/engine.rs` (handle Passthrough in process_node, determine_item_type_for_edge)
- Modify: `crates/factorial-core/src/test_utils.rs` (add helper)
- Test: `crates/factorial-core/src/processor.rs` (new tests)

### Step 1: Write the failing test

In `crates/factorial-core/src/processor.rs` test module:

```rust
#[test]
fn passthrough_moves_items_from_input_to_output() {
    use crate::engine::Engine;
    use crate::sim::SimulationStrategy;
    use crate::test_utils;

    let mut engine = Engine::new(SimulationStrategy::Tick);
    let iron = test_utils::iron();

    let source = test_utils::add_node(
        &mut engine,
        test_utils::make_source(iron, 5.0),
        100,
        100,
    );
    let pass = test_utils::add_node(
        &mut engine,
        Processor::Passthrough,
        100,
        100,
    );
    let sink = test_utils::add_node(
        &mut engine,
        test_utils::make_source(iron, 0.0),
        100,
        100,
    );

    test_utils::connect(&mut engine, source, pass, test_utils::make_flow_transport(10.0));
    test_utils::connect(&mut engine, pass, sink, test_utils::make_flow_transport(10.0));

    for _ in 0..10 {
        engine.step();
    }

    let at_sink = test_utils::input_quantity(&engine, sink, iron);
    assert!(at_sink > 0, "Passthrough should forward items to sink, got {at_sink}");
}
```

### Step 2: Run test to verify it fails

Run: `cargo test -p factorial-core passthrough_moves_items -- --nocapture 2>&1 | tail -20`
Expected: FAIL — `Processor::Passthrough` variant does not exist.

### Step 3: Add Passthrough variant

In `crates/factorial-core/src/processor.rs`:

```rust
pub enum Processor {
    Source(SourceProcessor),
    Fixed(FixedRecipe),
    Property(PropertyProcessor),
    Demand(DemandProcessor),
    /// Passes all items from input to output unchanged.
    /// Used for junction nodes (splitters, mergers, balancers).
    Passthrough,
}
```

Add the tick handler:

```rust
Processor::Passthrough => tick_passthrough(state, available_inputs, output_space),
```

Implement `tick_passthrough`:

```rust
fn tick_passthrough(
    state: &mut ProcessorState,
    available_inputs: &[(ItemTypeId, u32)],
    output_space: u32,
) -> ProcessorResult {
    if available_inputs.is_empty() {
        *state = ProcessorState::Idle;
        return ProcessorResult::default();
    }

    let mut result = ProcessorResult::default();
    let mut space_remaining = output_space;

    for &(item_type, qty) in available_inputs {
        if space_remaining == 0 { break; }
        let to_move = qty.min(space_remaining);
        if to_move > 0 {
            result.consumed.push((item_type, to_move));
            result.produced.push((item_type, to_move));
            space_remaining -= to_move;
        }
    }

    if result.consumed.is_empty() {
        *state = ProcessorState::Stalled { reason: StallReason::OutputFull };
    } else {
        *state = ProcessorState::Working { progress: 0 };
    }

    result.state_changed = true;
    result
}
```

### Step 4: Update `determine_item_type_for_edge` in engine.rs

Add a `Processor::Passthrough` arm that checks the output inventory for existing items (same fallback logic already there):

```rust
Processor::Passthrough => {
    // No inherent type — fall through to inventory check.
}
```

### Step 5: Run test to verify it passes

Run: `cargo test -p factorial-core passthrough_moves_items -- --nocapture 2>&1 | tail -20`
Expected: PASS

### Step 6: Commit

```bash
git add crates/factorial-core/src/processor.rs crates/factorial-core/src/engine.rs
git commit -m "feat: add Passthrough processor variant for junction nodes

Passthrough moves all items from input to output unchanged, respecting
output_space. Used as the processor for splitter/merger/balancer nodes
that need item routing without transformation.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 4: Junction Runtime Behavior

**Why:** Junction processing in `phase_component` only updates round-robin indices. Splitters don't actually route items to different output edges. Mergers don't pull from specific input edges. Game devs need junctions that actively move items.

**Files:**
- Modify: `crates/factorial-core/src/engine.rs` (rewrite junction processing in phase_component)
- Modify: `crates/factorial-core/src/junction.rs` (add FilterMap to SplitterConfig)
- Modify: `crates/factorial-core/src/test_utils.rs` (add junction helpers)
- Test: `crates/factorial-core/src/engine.rs` (new tests)

### Step 1: Write the failing test

```rust
#[test]
fn splitter_distributes_items_across_outputs() {
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
```

### Step 2: Run test to verify it fails

Run: `cargo test -p factorial-core splitter_distributes_items_across_outputs -- --nocapture 2>&1 | tail -20`
Expected: FAIL — splitter doesn't actually route items; one sink gets everything.

### Step 3: Implement junction item routing in phase_component

Rewrite the junction processing in `phase_component` to actively distribute items across output edges based on the junction policy. The key change: after the Passthrough processor moves items to the splitter's output inventory, the junction logic selectively removes items and places them into the output inventories that map to specific edges.

**Approach:** Junctions modify the splitter node's output inventory to be split across outbound edges. During transport phase, each edge pulls from the node's output inventory. The junction pre-distributes items into per-edge "virtual buckets" by manipulating the output inventory slots.

Alternative simpler approach: The junction modifies the `transports` available amount. When the transport phase queries `output_total(splitter_node)`, the junction intercepts and returns per-edge amounts.

**Simplest approach that works:** Add a `SecondaryMap<EdgeId, u32>` field `edge_budgets` to Engine. Junction processing fills this map. Transport phase reads from it:

```rust
// In Engine struct:
pub(crate) edge_budgets: SecondaryMap<EdgeId, u32>,
```

In junction processing for Splitter with RoundRobin:
1. Get total items in the splitter's output inventory.
2. Get the list of output edges from the graph.
3. Distribute the total across edges in round-robin fashion.
4. Write each edge's budget into `edge_budgets`.

In `phase_transport`, when computing `available`:
```rust
let available = match self.edge_budgets.get(edge_id) {
    Some(&budget) => budget,
    None => match item_filter { ... }, // existing logic
};
```

### Step 4: Implement the changes

Implement in `engine.rs`:
- Add `edge_budgets: SecondaryMap<EdgeId, u32>` to `Engine` struct.
- In `phase_component`, after junction policy processing, compute budgets:
  - RoundRobin: `total / num_outputs` per edge, with remainder to current index edge.
  - Priority: all to first edge with capacity, then overflow to next.
  - EvenSplit: `total / num_outputs` per edge (same as RoundRobin but without remainder bias).
- In `phase_transport`, check `edge_budgets` before computing `available`.
- Clear `edge_budgets` at start of `phase_component`.

### Step 5: Run test to verify it passes

Run: `cargo test -p factorial-core splitter_distributes_items_across_outputs -- --nocapture 2>&1 | tail -20`
Expected: PASS

### Step 6: Write merger test and implement

```rust
#[test]
fn merger_pulls_from_multiple_inputs() {
    // Two sources -> merger -> sink. Both sources should contribute.
    // ... (similar structure)
}
```

Merger processing: during junction phase, merge inputs from multiple incoming edges using the merger policy. This is naturally handled by the existing transport + inventory system — items from multiple sources accumulate in the merger's input inventory. The merger's Passthrough processor moves them to output. No special logic needed beyond what already exists.

### Step 7: Run full test suite

Run: `cargo test --workspace 2>&1 | tail -30`

### Step 8: Commit

```bash
git add crates/factorial-core/src/engine.rs crates/factorial-core/src/junction.rs crates/factorial-core/src/test_utils.rs
git commit -m "feat: implement junction runtime item routing via edge budgets

Splitter junctions now actively distribute items across output edges
using the configured policy (RoundRobin, Priority, EvenSplit). The
edge_budgets map controls how much each outbound edge can transport
per tick. Merger junctions work naturally via inventory accumulation.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Fair Fan-Out Distribution

**Why:** Without a junction, multiple outgoing edges from a single node compete for items. The first edge processed wins all the output (first-edge-wins). ONI gas pipes, Shapez balancers, and Satisfactory conveyor splits need fair distribution.

**Files:**
- Modify: `crates/factorial-core/src/engine.rs` (phase_transport fair distribution)
- Test: `crates/factorial-core/src/engine.rs` (new test)

### Step 1: Write the failing test

```rust
#[test]
fn fan_out_distributes_fairly_without_junction() {
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
```

### Step 2: Run test to verify it fails

Expected: FAIL — first edge monopolizes output.

### Step 3: Implement fair fan-out in phase_transport

Modify `phase_transport` to pre-compute fair shares for nodes with multiple outgoing edges:

1. Before iterating edges, group outgoing edges by source node.
2. For each source with >1 outgoing edge: divide `output_total` (or filtered amount) evenly across edges.
3. Write these shares into `edge_budgets` (same mechanism as junction budgets, but as a default for non-junction nodes).
4. Junction budgets override the default fair distribution.

```rust
fn compute_default_edge_budgets(&mut self) {
    // Only compute for nodes WITHOUT junctions (junctions handle their own budgets).
    for (node_id, _) in self.graph.nodes_iter() {
        if self.junctions.contains_key(node_id) {
            continue; // Junction handles this.
        }
        let outputs = self.graph.get_outputs(node_id);
        if outputs.len() <= 1 {
            continue; // No fan-out, no budget needed.
        }
        let total = self.output_total(node_id);
        let share = total / outputs.len() as u32;
        let remainder = total % outputs.len() as u32;
        for (i, &edge_id) in outputs.iter().enumerate() {
            let budget = share + if i == 0 { remainder } else { 0 };
            self.edge_budgets.insert(edge_id, budget);
        }
    }
}
```

Call `compute_default_edge_budgets()` at the start of `phase_transport`, after `edge_budgets` has been populated by junction processing in `phase_component` of the previous tick (or cleared if first tick).

### Step 4: Run test to verify it passes

Run: `cargo test -p factorial-core fan_out_distributes_fairly -- --nocapture 2>&1 | tail -20`
Expected: PASS

### Step 5: Run full test suite and commit

```bash
git add crates/factorial-core/src/engine.rs
git commit -m "feat: fair fan-out distribution for multi-output edges

Nodes with multiple outgoing edges now distribute items evenly across
all edges by default. Junction-configured nodes override this with
their own policy. Fixes first-edge-wins behavior for non-junction
fan-out (ONI gas pipes, Shapez balancers).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 6: Dynamic Recipe Selection

**Why:** Game devs need nodes that can change what they produce at runtime (Shapez make-anything machine, Satisfactory alternate recipe swap). Currently each node has a fixed `Processor` that never changes.

**Files:**
- Modify: `crates/factorial-core/src/engine.rs` (add `set_processor` public method — already exists; add `swap_processor` that resets state)
- Test: `crates/factorial-core/src/engine.rs` (new test)

### Step 1: Write the failing test

```rust
#[test]
fn dynamic_recipe_swap_resets_state() {
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
```

### Step 2: Run test to verify it fails

Expected: FAIL — `swap_processor` does not exist.

### Step 3: Implement `swap_processor`

In `crates/factorial-core/src/engine.rs`:

```rust
/// Replace a node's processor and reset its processing state to Idle.
/// Use this for dynamic recipe selection at runtime.
pub fn swap_processor(&mut self, node: NodeId, processor: Processor) {
    self.processors.insert(node, processor);
    self.processor_states.insert(node, ProcessorState::Idle);
}
```

### Step 4: Run test to verify it passes

Run: `cargo test -p factorial-core dynamic_recipe_swap -- --nocapture 2>&1 | tail -20`
Expected: PASS

### Step 5: Commit

```bash
git add crates/factorial-core/src/engine.rs
git commit -m "feat: add swap_processor for dynamic recipe selection

New public method swap_processor() replaces a node's processor and
resets its state to Idle. Enables runtime recipe swapping for
make-anything machines, alternate recipe selection, and similar
game mechanics.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 7: Item Property Tracking

**Why:** ONI needs temperature on water/steam items. PropertyProcessor declares transforms but items don't carry property values. Game devs need items to have per-instance properties (temperature, quality, etc.) that persist through inventories and transport.

**Files:**
- Modify: `crates/factorial-core/src/item.rs` (add property storage to ItemStack)
- Modify: `crates/factorial-core/src/processor.rs` (PropertyProcessor applies transform to tracked properties)
- Modify: `crates/factorial-core/src/engine.rs` (property propagation, query API)
- Test: new tests in `item.rs` and `engine.rs`

### Step 1: Write the failing test

In `crates/factorial-core/src/item.rs` tests:

```rust
#[test]
fn item_stack_with_properties() {
    use crate::id::PropertyId;
    use crate::fixed::Fixed64;

    let mut stack = ItemStack {
        item_type: ItemTypeId(0),
        quantity: 10,
        properties: Default::default(),
    };

    let temp = PropertyId(0);
    stack.set_property(temp, Fixed64::from_num(95));
    assert_eq!(stack.get_property(temp), Some(Fixed64::from_num(95)));
}
```

### Step 2: Run test to verify it fails

Expected: FAIL — `properties` field and methods don't exist.

### Step 3: Add property storage to ItemStack

In `crates/factorial-core/src/item.rs`:

```rust
use crate::id::PropertyId;
use crate::fixed::Fixed64;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_type: ItemTypeId,
    pub quantity: u32,
    /// Per-instance properties (e.g., temperature, quality).
    /// Empty by default. Game code sets properties via processors or modules.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<PropertyId, Fixed64>,
}

impl ItemStack {
    pub fn set_property(&mut self, id: PropertyId, value: Fixed64) {
        self.properties.insert(id, value);
    }

    pub fn get_property(&self, id: PropertyId) -> Option<Fixed64> {
        self.properties.get(&id).copied()
    }
}
```

Update all `ItemStack` construction sites to include `properties: BTreeMap::new()` (or use `Default` via a constructor).

Add a convenience constructor:

```rust
impl ItemStack {
    pub fn new(item_type: ItemTypeId, quantity: u32) -> Self {
        Self {
            item_type,
            quantity,
            properties: BTreeMap::new(),
        }
    }
}
```

### Step 4: Update PropertyProcessor to apply transforms

In `crates/factorial-core/src/processor.rs`, modify `tick_property` to propagate properties from input items to output items and apply the transform:

```rust
fn tick_property(
    prop: &PropertyProcessor,
    state: &mut ProcessorState,
    available_inputs: &[(ItemTypeId, u32)],
    output_space: u32,
    input_properties: &BTreeMap<PropertyId, Fixed64>, // NEW parameter
) -> (ProcessorResult, BTreeMap<PropertyId, Fixed64>) {
    // ... existing logic for consuming/producing ...

    // Apply property transform to output properties.
    let mut output_props = input_properties.clone();
    match &prop.transform {
        PropertyTransform::Set(id, val) => { output_props.insert(*id, *val); }
        PropertyTransform::Add(id, delta) => {
            let current = output_props.get(id).copied().unwrap_or(Fixed64::ZERO);
            output_props.insert(*id, current + *delta);
        }
        PropertyTransform::Multiply(id, factor) => {
            let current = output_props.get(id).copied().unwrap_or(Fixed64::from_num(1));
            output_props.insert(*id, current * *factor);
        }
    }

    (result, output_props)
}
```

### Step 5: Add query API to Engine

In `crates/factorial-core/src/engine.rs`:

```rust
/// Get the average property value for items of a given type in a node's output inventory.
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
```

### Step 6: Run tests, full suite, commit

```bash
git add crates/factorial-core/src/item.rs crates/factorial-core/src/processor.rs crates/factorial-core/src/engine.rs
git commit -m "feat: add item property tracking on ItemStack

ItemStack now carries a BTreeMap<PropertyId, Fixed64> for per-instance
properties (temperature, quality, etc). PropertyProcessor applies
transforms to tracked properties. Engine exposes get_item_property()
for querying property values on inventory items.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 8: Power Priority System

**Why:** ONI has power priorities — life support gets power before research. The current PowerModule computes a single satisfaction ratio for the entire network. Game devs need per-consumer priority so high-priority buildings get power first.

**Files:**
- Modify: `crates/factorial-power/src/lib.rs` (add priority field, priority-aware balancing)
- Test: `crates/factorial-power/src/lib.rs` (new test)

### Step 1: Write the failing test

```rust
#[test]
fn power_priority_high_gets_power_first() {
    let net_id = PowerNetworkId(0);

    let producer_node = NodeId::from(KeyData::from_ffi(1));
    let high_consumer = NodeId::from(KeyData::from_ffi(2));
    let low_consumer = NodeId::from(KeyData::from_ffi(3));

    let mut power = PowerModule::new();
    power.create_network(net_id);

    power.add_producer(net_id, producer_node, PowerProducer {
        capacity: Fixed64::from_num(100),
    });
    power.add_consumer_with_priority(net_id, high_consumer, PowerConsumer {
        demand: Fixed64::from_num(100),
    }, PowerPriority::High);
    power.add_consumer_with_priority(net_id, low_consumer, PowerConsumer {
        demand: Fixed64::from_num(100),
    }, PowerPriority::Low);

    power.tick(0);

    // Total demand: 200W, supply: 100W.
    // High priority gets 100%, low gets 0%.
    let high_sat = power.get_consumer_satisfaction(net_id, high_consumer);
    let low_sat = power.get_consumer_satisfaction(net_id, low_consumer);

    assert_eq!(high_sat, Fixed64::from_num(1), "High priority should be fully powered");
    assert_eq!(low_sat, Fixed64::from_num(0), "Low priority should get no power");
}
```

### Step 2: Run test to verify it fails

Expected: FAIL — `PowerPriority`, `add_consumer_with_priority`, `get_consumer_satisfaction` don't exist.

### Step 3: Implement priority system

Add to `crates/factorial-power/src/lib.rs`:

```rust
/// Power priority level for consumers. Higher priority consumers
/// receive power before lower priority ones during shortage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PowerPriority {
    /// Gets power last.
    Low = 0,
    /// Default priority.
    Medium = 1,
    /// Gets power first.
    High = 2,
}

impl Default for PowerPriority {
    fn default() -> Self {
        Self::Medium
    }
}
```

Add per-consumer priority storage in `PowerModule`:

```rust
/// Per-consumer priority. Defaults to Medium if not set.
consumer_priorities: BTreeMap<(PowerNetworkId, NodeId), PowerPriority>,

/// Per-consumer satisfaction ratio (computed during tick).
consumer_satisfaction: BTreeMap<(PowerNetworkId, NodeId), Fixed64>,
```

In the `tick()` method, change the balancing algorithm:
1. Sort consumers by priority (High first, Low last).
2. Allocate power to High consumers first until supply exhausted.
3. Then Medium, then Low.
4. Store per-consumer satisfaction.

Add `add_consumer_with_priority()` and `get_consumer_satisfaction()` methods.

### Step 4: Run test to verify it passes

Run: `cargo test -p factorial-power power_priority_high_gets_power_first -- --nocapture 2>&1 | tail -20`
Expected: PASS

### Step 5: Commit

```bash
git add crates/factorial-power/src/lib.rs
git commit -m "feat: add power priority system to PowerModule

Consumers can be assigned PowerPriority (High, Medium, Low). During
shortage, high-priority consumers get power first. Per-consumer
satisfaction is tracked and queryable via get_consumer_satisfaction().

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 9: Dynamic Power Production

**Why:** ONI's steam turbine produces power proportional to input steam temperature. The current PowerProducer has a fixed `capacity`. Game devs need to update power output each tick based on game state.

**Files:**
- Modify: `crates/factorial-power/src/lib.rs` (add mutable producer capacity update)
- Test: `crates/factorial-power/src/lib.rs` (new test)

### Step 1: Write the failing test

```rust
#[test]
fn dynamic_power_production_updates_each_tick() {
    let net_id = PowerNetworkId(0);
    let producer = NodeId::from(KeyData::from_ffi(1));
    let consumer = NodeId::from(KeyData::from_ffi(2));

    let mut power = PowerModule::new();
    power.create_network(net_id);
    power.add_producer(net_id, producer, PowerProducer {
        capacity: Fixed64::from_num(100),
    });
    power.add_consumer(net_id, consumer, PowerConsumer {
        demand: Fixed64::from_num(200),
    });

    power.tick(0);
    let sat_before = power.get_satisfaction(net_id);
    assert_eq!(sat_before, Fixed64::from_num(0).saturating_add(Fixed64::from_num(1) / Fixed64::from_num(2)));

    // Double the producer's output dynamically.
    power.set_producer_capacity(net_id, producer, Fixed64::from_num(200));
    power.tick(1);
    let sat_after = power.get_satisfaction(net_id);
    assert_eq!(sat_after, Fixed64::from_num(1), "Should be fully powered after capacity increase");
}
```

### Step 2: Implement `set_producer_capacity`

```rust
/// Update a producer's power output capacity at runtime.
/// Use this for dynamic power generation (steam turbines, solar panels, etc.).
pub fn set_producer_capacity(&mut self, network: PowerNetworkId, node: NodeId, capacity: Fixed64) {
    if let Some(spec) = self.producer_specs.get_mut(&(network, node)) {
        spec.capacity = capacity;
    }
}
```

### Step 3: Run test, full suite, commit

```bash
git add crates/factorial-power/src/lib.rs
git commit -m "feat: add dynamic power production via set_producer_capacity

PowerModule now supports updating producer capacity at runtime,
enabling steam turbines, solar panels, and other variable-output
power sources.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 10: Dual-Role Nodes (Recipe + Power)

**Why:** ONI's steam turbine both converts steam→water (recipe) AND produces power. A single node currently can only be one processor type. Solution: allow a node to be registered as both a processor AND a power producer, with the module handling the power side.

**Files:**
- No core engine changes needed. This is already possible via the Module system.
- Document the pattern: a node has a FixedRecipe processor (steam→water) AND is registered as a PowerProducer in the PowerModule. Game code uses a Module's `on_tick` to call `power.set_producer_capacity()` based on recipe throughput.
- Test: `crates/factorial-integration-tests/tests/` (integration test)

### Step 1: Write the test demonstrating the pattern

```rust
#[test]
fn dual_role_node_recipe_plus_power() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let steam = ItemTypeId(100);
    let water = ItemTypeId(3);

    // Steam turbine: 1 steam -> 1 water, 1 tick.
    let turbine = test_utils::add_node(
        &mut engine,
        test_utils::make_recipe(vec![(steam, 1)], vec![(water, 1)], 1),
        100,
        100,
    );

    // Seed steam.
    engine.inputs.get_mut(turbine).unwrap().input_slots[0].add(steam, 50);

    // Power module registers this node as a producer.
    let mut power = PowerModule::new();
    let net = PowerNetworkId(0);
    power.create_network(net);
    power.add_producer(net, turbine, PowerProducer {
        capacity: Fixed64::from_num(0), // starts at 0, updated dynamically
    });

    for tick in 0..10 {
        engine.step();
        // After each step, check how much water was produced this tick.
        let water_out = test_utils::output_quantity(&engine, turbine, water);
        // Set power proportional to output (e.g., 100W per water).
        power.set_producer_capacity(net, turbine, Fixed64::from_num(water_out * 100));
        power.tick(tick);
    }

    // Turbine should have produced both water and power.
    assert!(test_utils::output_quantity(&engine, turbine, water) > 0);
    assert!(power.get_satisfaction(net) > Fixed64::from_num(0));
}
```

### Step 2: Verify it compiles and passes with Tasks 1-9 implemented

### Step 3: Commit

```bash
git add crates/factorial-integration-tests/tests/
git commit -m "test: demonstrate dual-role node pattern (recipe + power)

Shows how a steam turbine node uses a FixedRecipe processor for the
steam->water conversion and simultaneously acts as a PowerProducer
with dynamic capacity updated each tick based on recipe throughput.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 11: DemandProcessor Sustained Rate Measurement

**Why:** Shapez hubs require items delivered at a sustained rate. The DemandProcessor consumes items but has no stats integration for rate tracking. Game devs need to query whether a DemandProcessor has been meeting its target rate over a time window.

**Files:**
- Modify: `crates/factorial-core/src/processor.rs` (add rate tracking to DemandProcessor)
- Modify: `crates/factorial-core/src/engine.rs` (query API)
- Test: new tests

### Step 1: Write the failing test

```rust
#[test]
fn demand_processor_tracks_sustained_rate() {
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
        }),
        100,
        100,
    );

    test_utils::connect(&mut engine, source, sink, test_utils::make_flow_transport(10.0));

    for _ in 0..60 {
        engine.step();
    }

    // Query sustained consumption rate over last 60 ticks.
    let rate = engine.get_demand_rate(sink);
    assert!(rate.is_some(), "Should be able to query demand rate");
    assert!(
        rate.unwrap() >= Fixed64::from_num(2),
        "Sustained rate should be near 3/tick, got {:?}", rate
    );
}
```

### Step 2: Implement

Add a `consumed_this_tick: u32` field to `DemandProcessor` (or to `ProcessorState` via a new variant). Track consumption in `tick_demand`. Expose `get_demand_rate` on Engine that reads from the stats module or from the processor state.

Simpler approach: Use the existing `ProcessorResult.consumed` which is already emitted as `Event::ItemConsumed`. The stats module (`factorial-stats`) already tracks consumption rates. The gap is that there's no convenience method on Engine to query it.

Add to Engine:
```rust
pub fn get_demand_rate(&self, node: NodeId) -> Option<Fixed64> {
    // Read from the stats module if registered, or from processor state.
    // This is a convenience wrapper.
}
```

### Step 3: Run test, commit

```bash
git add crates/factorial-core/src/engine.rs crates/factorial-core/src/processor.rs
git commit -m "feat: add demand rate tracking and query API

DemandProcessor consumption is tracked via existing event system.
Engine exposes get_demand_rate() for querying sustained consumption
rate, enabling Shapez-style hub rate goals.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 12: Multi-Type DemandProcessor

**Why:** A make-anything hub needs to consume multiple item types. Current DemandProcessor only has a single `input_type`. Game devs need a demand sink that accepts any of a set of item types.

**Files:**
- Modify: `crates/factorial-core/src/processor.rs` (add MultiDemand variant or extend DemandProcessor)
- Test: new test

### Step 1: Write the failing test

```rust
#[test]
fn multi_demand_accepts_multiple_types() {
    use crate::engine::Engine;
    use crate::sim::SimulationStrategy;
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
            accepted_types: Some(vec![iron, copper]), // NEW field
        }),
        100,
        100,
    );

    engine.inputs.get_mut(node).unwrap().input_slots[0].add(iron, 5);
    engine.inputs.get_mut(node).unwrap().input_slots[0].add(copper, 5);

    for _ in 0..5 {
        engine.step();
    }

    let remaining_iron = test_utils::input_quantity(&engine, node, iron);
    let remaining_copper = test_utils::input_quantity(&engine, node, copper);

    // Should have consumed from both types.
    assert!(remaining_iron < 5 || remaining_copper < 5, "Should consume some items");
}
```

### Step 2: Implement

Add `accepted_types: Option<Vec<ItemTypeId>>` to `DemandProcessor`. When `Some`, the processor consumes from any matching type in the input inventory (round-robin or proportional). When `None`, falls back to `input_type` only (backwards compatible).

### Step 3: Run test, commit

```bash
git add crates/factorial-core/src/processor.rs crates/factorial-core/src/engine.rs
git commit -m "feat: add multi-type acceptance to DemandProcessor

DemandProcessor now has an optional accepted_types field. When set,
the processor consumes from any matching item type in the input
inventory, enabling make-anything hubs and multi-resource sinks.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 13: Fluid Module Consumer Keying

**Why:** Factorio test identified that FluidModule consumers are added by `NodeId` but there's no way to specify WHICH fluid type a consumer node wants from a multi-fluid system. Game devs need consumer-to-fluid-type association.

**Files:**
- Modify: `crates/factorial-fluid/src/lib.rs` (consumers keyed by fluid type)
- Test: new test

### Step 1: Write the failing test

```rust
#[test]
fn fluid_consumer_keyed_by_type() {
    let water = ItemTypeId(3);
    let steam = ItemTypeId(100);

    let mut fluid = FluidModule::new();

    let water_net = FluidNetworkId(0);
    let steam_net = FluidNetworkId(1);
    fluid.create_network(water_net, water);
    fluid.create_network(steam_net, steam);

    let consumer = NodeId::from(KeyData::from_ffi(1));

    // Consumer wants water from the water network.
    fluid.add_consumer(water_net, consumer, FluidConsumer {
        rate: Fixed64::from_num(10),
    });

    // Verify consumer is on the water network, not the steam network.
    assert!(fluid.get_network(water_net).unwrap().consumers.contains(&consumer));
    assert!(!fluid.get_network(steam_net).unwrap().consumers.contains(&consumer));
}
```

This should already work since networks are typed. The real gap is when a single node consumes from MULTIPLE fluid networks. Add a test for that:

```rust
#[test]
fn node_on_multiple_fluid_networks() {
    let water = ItemTypeId(3);
    let steam = ItemTypeId(100);

    let mut fluid = FluidModule::new();
    let water_net = FluidNetworkId(0);
    let steam_net = FluidNetworkId(1);
    fluid.create_network(water_net, water);
    fluid.create_network(steam_net, steam);

    let node = NodeId::from(KeyData::from_ffi(1));

    // Node produces steam and consumes water.
    fluid.add_producer(steam_net, node, FluidProducer {
        rate: Fixed64::from_num(5),
    });
    fluid.add_consumer(water_net, node, FluidConsumer {
        rate: Fixed64::from_num(10),
    });

    // Node should be on both networks.
    assert!(fluid.get_network(steam_net).unwrap().producers.contains(&node));
    assert!(fluid.get_network(water_net).unwrap().consumers.contains(&node));
}
```

### Step 2: Verify and fix any issues

The current FluidModule may already support this. If so, this task is just adding tests to document the capability. If not, fix the implementation.

### Step 3: Commit

```bash
git add crates/factorial-fluid/src/lib.rs
git commit -m "test: verify fluid consumer keying and multi-network nodes

Adds tests confirming that fluid consumers are properly keyed by
network (and thus by fluid type), and that a single node can be a
producer on one fluid network while consuming from another.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 14: Fluid-to-Item Bridge

**Why:** Some games need fluid networks to feed into the item-based production graph (e.g., petroleum from fluid network becomes items in a polymer press's input). Game devs need a bridge node.

**Files:**
- Add: `crates/factorial-fluid/src/bridge.rs` (FluidBridge module that converts fluid network flow to inventory items)
- Modify: `crates/factorial-fluid/src/lib.rs` (re-export bridge)
- Test: new test

### Step 1: Write the failing test

```rust
#[test]
fn fluid_bridge_converts_flow_to_items() {
    let water = ItemTypeId(3);

    let mut engine = Engine::new(SimulationStrategy::Tick);
    let mut fluid = FluidModule::new();
    let net = FluidNetworkId(0);
    fluid.create_network(net, water);

    // Producer feeds 10 water/tick into the network.
    let well = NodeId::from(KeyData::from_ffi(1));
    fluid.add_producer(net, well, FluidProducer {
        rate: Fixed64::from_num(10),
    });

    // Bridge node: consumes from fluid network, places items in engine inventory.
    let bridge_node = test_utils::add_node(
        &mut engine,
        Processor::Passthrough,
        100,
        100,
    );
    fluid.add_consumer(net, bridge_node, FluidConsumer {
        rate: Fixed64::from_num(10),
    });

    // Bridge config.
    let bridge = FluidBridge::new(net, bridge_node, water);

    for tick in 0..10 {
        fluid.tick(tick);
        // Bridge reads fluid consumption and adds items to node's input inventory.
        let consumed = fluid.get_consumed_this_tick(net, bridge_node);
        bridge.apply(&mut engine, consumed);
    }

    let water_in_inventory = test_utils::input_quantity(&engine, bridge_node, water);
    assert!(water_in_inventory > 0, "Bridge should deposit fluid as items, got {water_in_inventory}");
}
```

### Step 2: Implement FluidBridge

A simple struct that tracks which fluid network and node to bridge:

```rust
pub struct FluidBridge {
    pub network: FluidNetworkId,
    pub node: NodeId,
    pub item_type: ItemTypeId,
}

impl FluidBridge {
    pub fn new(network: FluidNetworkId, node: NodeId, item_type: ItemTypeId) -> Self {
        Self { network, node, item_type }
    }

    /// Convert consumed fluid into inventory items.
    pub fn apply(&self, engine: &mut Engine, consumed: Fixed64) {
        let whole_items = consumed.to_num::<i64>().max(0) as u32;
        if whole_items > 0 {
            if let Some(inv) = engine.inputs.get_mut(self.node) {
                for slot in &mut inv.input_slots {
                    let overflow = slot.add(self.item_type, whole_items);
                    if overflow == 0 { break; }
                }
            }
        }
    }
}
```

Also add `get_consumed_this_tick` to FluidModule that returns how much fluid a consumer received on the current tick.

### Step 3: Run test, commit

```bash
git add crates/factorial-fluid/src/lib.rs crates/factorial-fluid/src/bridge.rs
git commit -m "feat: add FluidBridge for fluid-to-item conversion

FluidBridge converts fluid network consumption into inventory items,
enabling fluid outputs to feed into item-based production (e.g.,
petroleum from pipes into polymer press inputs).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Post-Implementation: Re-run Integration Tests

After all 14 tasks are complete, re-run the game-inspired integration tests:

```bash
cargo test -p factorial-integration-tests -- --nocapture 2>&1 | tail -50
```

The ONI tests that were previously failing should now pass (or be closer to passing). Update any ENGINE GAP comments that have been resolved.

### Final commit

```bash
git add -A
git commit -m "chore: update ENGINE GAP comments in integration tests

Mark resolved gaps and update test assertions to use new engine
capabilities (edge filters, feedback loops, junction routing, etc).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| # | Gap | Core Change | Lines (est.) |
|---|-----|-------------|-------------|
| 1 | Per-output-type edge routing | `EdgeData.item_filter`, transport filter | ~120 |
| 2 | Feedback loops | `topological_order_with_feedback()` | ~80 |
| 3 | Passthrough processor | `Processor::Passthrough` variant | ~50 |
| 4 | Junction runtime behavior | `edge_budgets` + junction routing | ~150 |
| 5 | Fair fan-out | Default even distribution for multi-output | ~40 |
| 6 | Dynamic recipe selection | `swap_processor()` | ~15 |
| 7 | Item property tracking | `ItemStack.properties` + propagation | ~100 |
| 8 | Power priority | `PowerPriority` + per-consumer satisfaction | ~100 |
| 9 | Dynamic power production | `set_producer_capacity()` | ~15 |
| 10 | Dual-role nodes | Documentation + integration test | ~40 |
| 11 | Demand rate tracking | `get_demand_rate()` query | ~30 |
| 12 | Multi-type DemandProcessor | `accepted_types` field | ~40 |
| 13 | Fluid consumer keying | Tests documenting existing capability | ~30 |
| 14 | Fluid-to-item bridge | `FluidBridge` struct | ~50 |

**Total: ~860 lines of production code + tests across 14 commits.**
