# Factorial Enhancements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement 9 enhancement items across 5 work groups, adding ~42 new tests and improving determinism, test coverage, serialization, and FFI completeness.

**Architecture:** Group A (infrastructure) must land first. Groups B, C, D, E are independent of each other and can run in parallel after A completes.

**Tech Stack:** Rust, bitcode, slotmap, criterion, cbindgen

---

## Task 1: Verify Clippy Warnings (Group A1)

**Files:**
- None modified (verification only)

**Step 1: Run clippy across full workspace**

Run: `cargo clippy --workspace --all-targets 2>&1`
Expected: 0 warnings. If any appear, fix them before proceeding.

**Step 2: Commit (only if fixes were needed)**

```bash
git add -A
git commit -m "fix: resolve clippy warnings across workspace"
```

---

## Task 2: Extract Shared Test Helpers (Group A2)

**Files:**
- Create: `crates/factorial-core/src/test_utils.rs`
- Modify: `crates/factorial-core/src/lib.rs`
- Modify: `crates/factorial-core/Cargo.toml`
- Modify: `crates/factorial-core/tests/integration.rs`
- Modify: `crates/factorial-core/benches/sim_bench.rs`

**Step 1: Add `test-utils` feature to Cargo.toml**

In `crates/factorial-core/Cargo.toml`, add:

```toml
[features]
test-utils = []
```

And update `[dev-dependencies]` to enable the feature for tests:

```toml
[dev-dependencies]
criterion = "0.5"
factorial-core = { path = ".", features = ["test-utils"] }
```

Wait — a crate can't depend on itself in dev-dependencies. Instead, the `#[cfg(test)]` gate handles unit tests automatically. For integration tests and benchmarks (which are external to the crate), we need the feature. Integration tests get it from:

```toml
[dev-dependencies]
criterion = "0.5"
```

And we gate the module so it's available in test builds automatically:

```toml
[features]
test-utils = []
```

Integration tests and benchmarks compile with `--test` or `--bench` flags which set `cfg(test)` for the crate's own code. Actually, that's only true for `#[cfg(test)]` in unit test modules — integration tests and benchmarks link to the crate as an external dependency and do NOT see `cfg(test)` items.

So the correct approach: use the feature flag and enable it in dev-dependencies of downstream crates. But since integration tests and benchmarks live in the same crate, they need the feature enabled. The idiomatic approach:

```toml
[features]
test-utils = []

[dev-dependencies]
criterion = "0.5"
```

Then in `crates/factorial-core/tests/integration.rs`, add at the top:
```rust
// The test-utils feature is automatically available in integration tests
// because Cargo enables all features for dev builds when running tests.
```

Actually the simplest correct approach: just use `#[cfg(any(test, feature = "test-utils"))]` on the module. The `cfg(test)` is set for the crate being tested (including when integration tests run), so the module IS available to integration tests. For benchmarks (which compile the crate without `cfg(test)`), they need the feature flag explicitly.

Let me verify: when you run `cargo test`, the library crate IS compiled with `cfg(test)` set. Integration tests in `tests/` link against this test-compiled library. So `cfg(test)` modules ARE available to integration tests. But when `cargo bench` compiles the library, `cfg(test)` is NOT set. So benchmarks need `feature = "test-utils"`.

**Correct approach:**

In `Cargo.toml`:
```toml
[features]
test-utils = []
```

The module gate: `#[cfg(any(test, feature = "test-utils"))]`

For benchmarks, they need the feature. Add to the bench's compilation:

In `Cargo.toml`, benches implicitly use dev-dependencies. We need to ensure the feature is enabled for bench builds. The cleanest way:

```toml
[features]
test-utils = []

# Enable test-utils when running benches
[dev-dependencies]
criterion = "0.5"

# Cargo doesn't support per-bench features directly, so we use a different approach:
# Make the bench file use cfg_attr to conditionally include the module
```

Actually the simplest: just add to `Cargo.toml`:

```toml
[features]
default = []
test-utils = []
```

And in `benches/sim_bench.rs`, change imports but also add a build-time check. The real simplest approach given Rust's current limitations:

Use `crates/factorial-core/Cargo.toml`:
```toml
[features]
test-utils = []

[dev-dependencies]
criterion = "0.5"
# Enable test-utils for dev builds (tests + benches)
factorial-core = { path = ".", features = ["test-utils"] }
```

Wait, self-dependency IS supported in dev-dependencies for enabling features. Let me use that.

Actually, in modern Cargo you can do:
```toml
[dev-dependencies]
factorial-core = { path = ".", features = ["test-utils"] }
```

This is specifically for enabling features of the current crate during dev builds. This works.

**Step 1: Update `crates/factorial-core/Cargo.toml`**

Add features section and self dev-dependency:

```toml
[features]
test-utils = []

[dev-dependencies]
criterion = "0.5"
factorial-core = { path = ".", features = ["test-utils"] }
```

**Step 2: Create `crates/factorial-core/src/test_utils.rs`**

```rust
//! Shared test helpers for factorial-core tests and benchmarks.
//!
//! Gated behind `#[cfg(any(test, feature = "test-utils"))]` so it's available
//! in unit tests, integration tests, and benchmarks (via the `test-utils` feature).

use crate::engine::Engine;
use crate::fixed::Fixed64;
use crate::id::*;
use crate::item::Inventory;
use crate::processor::*;
use crate::transport::*;

// ---------------------------------------------------------------------------
// Fixed-point helper
// ---------------------------------------------------------------------------

pub fn fixed(v: f64) -> Fixed64 {
    Fixed64::from_num(v)
}

// ---------------------------------------------------------------------------
// Item type constructors
// ---------------------------------------------------------------------------

pub fn iron() -> ItemTypeId {
    ItemTypeId(0)
}

pub fn copper() -> ItemTypeId {
    ItemTypeId(1)
}

pub fn gear() -> ItemTypeId {
    ItemTypeId(2)
}

pub fn water() -> ItemTypeId {
    ItemTypeId(3)
}

pub fn oxygen() -> ItemTypeId {
    ItemTypeId(4)
}

pub fn hydrogen() -> ItemTypeId {
    ItemTypeId(5)
}

// ---------------------------------------------------------------------------
// Building type constructor
// ---------------------------------------------------------------------------

pub fn building() -> BuildingTypeId {
    BuildingTypeId(0)
}

// ---------------------------------------------------------------------------
// Inventory helper
// ---------------------------------------------------------------------------

pub fn simple_inventory(capacity: u32) -> Inventory {
    Inventory::new(1, 1, capacity)
}

// ---------------------------------------------------------------------------
// Processor constructors
// ---------------------------------------------------------------------------

pub fn make_source(item: ItemTypeId, rate: f64) -> Processor {
    Processor::Source(SourceProcessor {
        output_type: item,
        base_rate: fixed(rate),
        depletion: Depletion::Infinite,
        accumulated: fixed(0.0),
    })
}

pub fn make_recipe(
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

// ---------------------------------------------------------------------------
// Transport constructors
// ---------------------------------------------------------------------------

pub fn make_flow_transport(rate: f64) -> Transport {
    Transport::Flow(FlowTransport {
        rate: fixed(rate),
        buffer_capacity: fixed(1000.0),
        latency: 0,
    })
}

pub fn make_item_transport(slot_count: u32) -> Transport {
    Transport::Item(ItemTransport {
        speed: fixed(1.0),
        slot_count,
        lanes: 1,
    })
}

pub fn make_batch_transport(batch_size: u32, cycle_time: u32) -> Transport {
    Transport::Batch(BatchTransport {
        batch_size,
        cycle_time,
    })
}

pub fn make_vehicle_transport(capacity: u32, travel_time: u32) -> Transport {
    Transport::Vehicle(VehicleTransport {
        capacity,
        travel_time,
    })
}

// ---------------------------------------------------------------------------
// Engine helpers
// ---------------------------------------------------------------------------

/// Add a node to the engine with the given processor and inventories.
/// Returns the assigned NodeId.
pub fn add_node(
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

/// Connect two nodes and set transport. Returns the EdgeId.
pub fn connect(engine: &mut Engine, from: NodeId, to: NodeId, transport: Transport) -> EdgeId {
    let pending = engine.graph.queue_connect(from, to);
    let result = engine.graph.apply_mutations();
    let edge = result.resolve_edge(pending).unwrap();
    engine.set_transport(edge, transport);
    edge
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

/// Get the total quantity of a specific item in a node's output inventory.
pub fn output_quantity(engine: &Engine, node: NodeId, item: ItemTypeId) -> u32 {
    engine
        .get_output_inventory(node)
        .map(|inv| {
            inv.output_slots
                .iter()
                .map(|s| s.quantity(item))
                .sum::<u32>()
        })
        .unwrap_or(0)
}

/// Get the total quantity of a specific item in a node's input inventory.
pub fn input_quantity(engine: &Engine, node: NodeId, item: ItemTypeId) -> u32 {
    engine
        .get_input_inventory(node)
        .map(|inv| {
            inv.input_slots
                .iter()
                .map(|s| s.quantity(item))
                .sum::<u32>()
        })
        .unwrap_or(0)
}

/// Total items across all types in a node's input inventory.
pub fn input_total(engine: &Engine, node: NodeId) -> u32 {
    engine
        .get_input_inventory(node)
        .map(|inv| inv.input_slots.iter().map(|s| s.total()).sum::<u32>())
        .unwrap_or(0)
}

/// Total items across all types in a node's output inventory.
pub fn output_total(engine: &Engine, node: NodeId) -> u32 {
    engine
        .get_output_inventory(node)
        .map(|inv| inv.output_slots.iter().map(|s| s.total()).sum::<u32>())
        .unwrap_or(0)
}
```

**Step 3: Add module to `crates/factorial-core/src/lib.rs`**

Add at the end of `lib.rs`:

```rust
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
```

**Step 4: Update `crates/factorial-core/tests/integration.rs`**

Replace the entire Helpers section (lines 15-159) with:

```rust
use factorial_core::test_utils::*;
```

Keep the extra helpers `oxygen()` and `hydrogen()` only if they're used in tests that aren't in `test_utils`. Looking at the file: `oxygen()` and `hydrogen()` are used in `multi_output_recipe()` test. They're already in `test_utils` now, so remove the local definitions entirely.

**Step 5: Update `crates/factorial-core/benches/sim_bench.rs`**

Replace the Helpers section (lines 17-121) with:

```rust
use factorial_core::test_utils::*;
```

The bench-only helpers `make_item_transport` and `make_batch_transport` are now in `test_utils` with slightly different signatures:
- Old `make_batch_transport()` → New `make_batch_transport(10, 5)` (explicit args)
- Old `make_item_transport(slot_count)` → Same signature, no change needed

Update `build_medium_factory` to call `make_batch_transport(10, 5)` instead of `make_batch_transport()`.

**Step 6: Run all tests and benchmarks to verify refactor**

Run: `cargo test --workspace 2>&1`
Expected: All 264 tests pass.

Run: `cargo bench --no-run 2>&1`
Expected: Compiles successfully.

**Step 7: Run clippy with test-utils feature**

Run: `cargo clippy --workspace --all-targets --all-features 2>&1`
Expected: 0 warnings.

**Step 8: Commit**

```bash
git add crates/factorial-core/src/test_utils.rs crates/factorial-core/src/lib.rs crates/factorial-core/Cargo.toml crates/factorial-core/tests/integration.rs crates/factorial-core/benches/sim_bench.rs
git commit -m "refactor: extract shared test helpers to test_utils module

Deduplicate test helpers from integration.rs and sim_bench.rs into
a shared test_utils module behind the test-utils feature flag."
```

---

## Task 3: Delta Simulation Strategy Tests (Group B1)

**Depends on:** Task 2 (test_utils)

**Files:**
- Modify: `crates/factorial-core/src/engine.rs` (add tests to `#[cfg(test)] mod tests`)

**Step 1: Write Delta tests**

Add these tests to the existing `#[cfg(test)] mod tests` in `engine.rs`. The tests use `test_utils` helpers. Note: `SimState.accumulator` is `Ticks` which is `u64`, and `advance(dt)` takes `u64`. Delta mode uses integer ticks, not fractional — so "0.5 twice" from the design needs to be adapted. Looking at the code:

```rust
SimulationStrategy::Delta { fixed_timestep } => {
    self.sim_state.accumulator += dt;
    let step_size = fixed_timestep.max(1);
    while self.sim_state.accumulator >= step_size {
        self.sim_state.accumulator -= step_size;
        self.step_internal(&mut result);
    }
}
```

`Ticks` is `u64`, so `dt` is integer. With `fixed_timestep = 2`:
- `advance(1)` → accumulator=1, no step
- `advance(1)` → accumulator=2, one step runs, accumulator=0
- `advance(5)` → accumulator=5, two steps run, accumulator=1
- `advance(0)` → accumulator=0 (unchanged), no step

Write these tests in the test module of `engine.rs`:

```rust
#[test]
fn delta_sub_step_no_op() {
    // With fixed_timestep=2, advancing by 1 should not run any steps.
    use crate::test_utils::*;
    let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
    let _source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);

    let result = engine.advance(1);
    assert_eq!(result.steps_run, 0);
    assert_eq!(engine.sim_state.tick, 0);
    assert_eq!(engine.sim_state.accumulator, 1);
}

#[test]
fn delta_accumulates_then_steps() {
    // With fixed_timestep=2, two advance(1) calls should run exactly one step.
    use crate::test_utils::*;
    let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
    let source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);

    engine.advance(1); // accumulator=1, no step
    let result = engine.advance(1); // accumulator=2, one step, accumulator=0
    assert_eq!(result.steps_run, 1);
    assert_eq!(engine.sim_state.tick, 1);
    assert_eq!(engine.sim_state.accumulator, 0);

    // Source should have produced items in that one step.
    assert!(output_total(&engine, source) > 0);
}

#[test]
fn delta_multi_step_catchup() {
    // With fixed_timestep=2, advance(7) should run 3 steps with 1 remaining.
    use crate::test_utils::*;
    let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
    let _source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);

    let result = engine.advance(7);
    assert_eq!(result.steps_run, 3); // 7 / 2 = 3 full steps
    assert_eq!(engine.sim_state.tick, 3);
    assert_eq!(engine.sim_state.accumulator, 1); // 7 - 3*2 = 1
}

#[test]
fn delta_zero_dt_no_change() {
    use crate::test_utils::*;
    let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
    let _source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);

    let hash_before = engine.state_hash();
    let result = engine.advance(0);
    assert_eq!(result.steps_run, 0);
    assert_eq!(engine.sim_state.tick, 0);
    assert_eq!(engine.state_hash(), hash_before);
}

#[test]
fn delta_determinism() {
    // Two engines with identical setup and advance() calls produce identical hashes.
    use crate::test_utils::*;
    fn run_delta() -> Vec<u64> {
        let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 2 });
        let source = add_node(&mut engine, make_source(iron(), 3.0), 100, 100);
        let assembler = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 5),
            100, 100,
        );
        connect(&mut engine, source, assembler, make_flow_transport(10.0));

        let mut hashes = Vec::new();
        for dt in [1, 3, 2, 5, 1, 4, 7] {
            engine.advance(dt);
            hashes.push(engine.state_hash());
        }
        hashes
    }

    assert_eq!(run_delta(), run_delta());
}
```

**Step 2: Run the new tests**

Run: `cargo test -p factorial-core delta_ 2>&1`
Expected: All 5 new tests pass.

**Step 3: Commit**

```bash
git add crates/factorial-core/src/engine.rs
git commit -m "test: add Delta simulation strategy tests

Tests cover sub-step no-op, accumulation, multi-step catchup,
zero-dt no-change, and determinism across identical advance sequences."
```

---

## Task 4: Adversarial Graph Mutation Tests (Group B2)

**Depends on:** Task 2 (test_utils)

**Files:**
- Modify: `crates/factorial-core/src/graph.rs` (add tests to existing test module)

**Step 1: Write adversarial graph tests**

Add to the existing `#[cfg(test)] mod tests` in `graph.rs`:

```rust
// -----------------------------------------------------------------------
// Test 11: Self-loop detected as cycle
// -----------------------------------------------------------------------
#[test]
fn self_loop_detected_as_cycle() {
    let (mut graph, nodes) = make_graph_with_nodes(1);
    let a = nodes[0];

    graph.queue_connect(a, a);
    graph.apply_mutations();

    let result = graph.topological_order();
    assert!(result.is_err());
    assert!(matches!(result, Err(GraphError::CycleDetected)));
}

// -----------------------------------------------------------------------
// Test 12: Duplicate edges between same nodes
// -----------------------------------------------------------------------
#[test]
fn duplicate_edges_allowed() {
    let (mut graph, nodes) = make_graph_with_nodes(2);
    let [a, b] = [nodes[0], nodes[1]];

    let pe1 = graph.queue_connect(a, b);
    let pe2 = graph.queue_connect(a, b);
    let result = graph.apply_mutations();

    let e1 = result.resolve_edge(pe1).unwrap();
    let e2 = result.resolve_edge(pe2).unwrap();

    // Both edges should exist and be distinct.
    assert_ne!(e1, e2);
    assert_eq!(graph.edge_count(), 2);

    // A should have 2 outputs, B should have 2 inputs.
    assert_eq!(graph.get_outputs(a).len(), 2);
    assert_eq!(graph.get_inputs(b).len(), 2);

    // Topo order should still work (no cycle).
    let order = graph.topological_order().unwrap();
    assert_eq!(order.len(), 2);
    assert_eq!(order[0], a);
    assert_eq!(order[1], b);
}

// -----------------------------------------------------------------------
// Test 13: Remove non-existent node is a no-op (no panic)
// -----------------------------------------------------------------------
#[test]
fn remove_nonexistent_node_no_panic() {
    let (mut graph, nodes) = make_graph_with_nodes(1);
    let real_node = nodes[0];

    // Remove the real node first.
    graph.queue_remove_node(real_node);
    graph.apply_mutations();
    assert_eq!(graph.node_count(), 0);

    // Remove it again — should be a no-op, not panic.
    graph.queue_remove_node(real_node);
    graph.apply_mutations();
    assert_eq!(graph.node_count(), 0);
}

// -----------------------------------------------------------------------
// Test 14: Disconnect non-existent edge is a no-op (no panic)
// -----------------------------------------------------------------------
#[test]
fn disconnect_nonexistent_edge_no_panic() {
    let (mut graph, nodes) = make_graph_with_nodes(2);
    let [a, b] = [nodes[0], nodes[1]];

    let pe = graph.queue_connect(a, b);
    let result = graph.apply_mutations();
    let edge = result.resolve_edge(pe).unwrap();

    // Disconnect it once.
    graph.queue_disconnect(edge);
    graph.apply_mutations();
    assert_eq!(graph.edge_count(), 0);

    // Disconnect it again — should be a no-op, not panic.
    graph.queue_disconnect(edge);
    graph.apply_mutations();
    assert_eq!(graph.edge_count(), 0);
}

// -----------------------------------------------------------------------
// Test 15: Remove node with both inbound and outbound edges
// -----------------------------------------------------------------------
#[test]
fn remove_node_with_inbound_and_outbound_edges() {
    let (mut graph, nodes) = make_graph_with_nodes(3);
    let [a, b, c] = [nodes[0], nodes[1], nodes[2]];

    // A->B, B->C
    graph.queue_connect(a, b);
    graph.queue_connect(b, c);
    graph.apply_mutations();

    assert_eq!(graph.edge_count(), 2);

    // Remove B (has 1 inbound from A, 1 outbound to C).
    graph.queue_remove_node(b);
    graph.apply_mutations();

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 0);

    // A and C should have no adjacency left.
    assert_eq!(graph.get_outputs(a).len(), 0);
    assert_eq!(graph.get_inputs(c).len(), 0);

    // Topo order should work with remaining disconnected nodes.
    let order = graph.topological_order().unwrap();
    assert_eq!(order.len(), 2);
}

// -----------------------------------------------------------------------
// Test 16: Queued mutations don't affect topo order until applied
// -----------------------------------------------------------------------
#[test]
fn queued_mutations_dont_affect_topo_until_applied() {
    let (mut graph, nodes) = make_graph_with_nodes(2);
    let [a, b] = [nodes[0], nodes[1]];

    // Establish A->B and compute topo order.
    graph.queue_connect(a, b);
    graph.apply_mutations();
    let order = graph.topological_order().unwrap();
    assert_eq!(order, &[a, b]);

    // Queue a new node and edge but DON'T apply.
    let _pending_c = graph.queue_add_node(BuildingTypeId(0));
    assert!(graph.has_pending_mutations());

    // Topo order should still be [A, B] (mutations not yet applied).
    // Note: the dirty flag was cleared by the previous topological_order call,
    // and queuing doesn't set dirty (only apply_mutations does).
    let order = graph.topological_order().unwrap();
    assert_eq!(order, &[a, b]);
    assert_eq!(graph.node_count(), 2);
}
```

**Step 2: Run the new tests**

Run: `cargo test -p factorial-core graph::tests 2>&1`
Expected: All 16 graph tests pass (10 existing + 6 new).

**Step 3: Commit**

```bash
git add crates/factorial-core/src/graph.rs
git commit -m "test: add adversarial graph mutation tests

Tests cover self-loops, duplicate edges, removing non-existent
nodes/edges, removing nodes with both inbound and outbound edges,
and verifying queued mutations don't affect topo order until applied."
```

---

## Task 5: Vehicle/Batch Transport Integration Tests (Group B3)

**Depends on:** Task 2 (test_utils)

**Files:**
- Modify: `crates/factorial-core/tests/integration.rs`

**Step 1: Write integration tests**

Add these tests after the existing tests in `integration.rs`:

```rust
// ===========================================================================
// Test 7: Batch transport chain
// ===========================================================================
//
// Source --BatchTransport--> Consumer
// Verify items arrive in discrete chunks matching batch_size.

#[test]
fn batch_transport_chain() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Source: produces 5 iron per tick.
    let source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);

    // Consumer: recipe that consumes iron (large duration = sink).
    let consumer = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 999)], vec![(gear(), 1)], 9999),
        100,
        100,
    );

    // Batch transport: 10 items per batch, 5 tick cycle.
    connect(
        &mut engine,
        source,
        consumer,
        make_batch_transport(10, 5),
    );

    // Run enough ticks for at least one batch delivery.
    // Source produces 5/tick, batch needs 10 items and 5 tick cycle.
    // After ~5 ticks source has produced 25 items, first batch of 10 delivered.
    for _ in 0..20 {
        engine.step();
    }

    // Items should have arrived at the consumer.
    let consumer_input = input_quantity(&engine, consumer, iron());
    assert!(
        consumer_input > 0,
        "consumer should have received iron via batch transport, got {consumer_input}"
    );
}

// ===========================================================================
// Test 8: Vehicle transport round-trip
// ===========================================================================
//
// Source --VehicleTransport--> Consumer
// Verify vehicle delivers items with travel time delay.

#[test]
fn vehicle_transport_round_trip() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Source: produces 10 iron per tick.
    let source = add_node(&mut engine, make_source(iron(), 10.0), 100, 100);

    // Consumer: recipe that consumes iron (large duration = sink).
    let consumer = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 999)], vec![(gear(), 1)], 9999),
        200,
        100,
    );

    // Vehicle: capacity 20, travel time 3 ticks (6 tick round trip).
    connect(
        &mut engine,
        source,
        consumer,
        make_vehicle_transport(20, 3),
    );

    // Run enough ticks for the vehicle to complete at least one round trip.
    // Travel time = 3, so first delivery at tick ~3, return at tick ~6.
    for _ in 0..20 {
        engine.step();
    }

    // Items should have arrived at the consumer.
    let consumer_input = input_quantity(&engine, consumer, iron());
    assert!(
        consumer_input > 0,
        "consumer should have received iron via vehicle transport, got {consumer_input}"
    );

    // Vehicle should not deliver more than capacity per trip.
    // After 20 ticks with capacity 20 and 6-tick round trips, max ~3 trips = ~60 items.
    assert!(
        consumer_input <= 80,
        "vehicle shouldn't exceed capacity * trips; got {consumer_input}"
    );
}

// ===========================================================================
// Test 9: Mixed transport factory
// ===========================================================================
//
// Source1 --FlowTransport--> Assembler
// Source2 --ItemTransport--> Assembler
// Assembler --BatchTransport--> Buffer
// Buffer --VehicleTransport--> Sink
// All four transport types in one factory.

#[test]
fn mixed_transport_factory() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Two sources.
    let iron_source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);
    let copper_source = add_node(&mut engine, make_source(copper(), 5.0), 100, 100);

    // Assembler: 1 iron + 1 copper -> 1 gear, 3 ticks.
    let assembler = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 1), (copper(), 1)], vec![(gear(), 1)], 3),
        100,
        100,
    );

    // Buffer (intermediate, acts as pass-through sink for now).
    let buffer = add_node(
        &mut engine,
        make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 9999),
        100,
        100,
    );

    // Sink.
    let sink = add_node(
        &mut engine,
        make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 9999),
        100,
        100,
    );

    // Connect with all four transport types.
    connect(&mut engine, iron_source, assembler, make_flow_transport(10.0));
    connect(
        &mut engine,
        copper_source,
        assembler,
        make_item_transport(5),
    );
    connect(
        &mut engine,
        assembler,
        buffer,
        make_batch_transport(5, 3),
    );
    connect(
        &mut engine,
        buffer,
        sink,
        make_vehicle_transport(10, 2),
    );

    // Run 50 ticks to let the full pipeline warm up.
    for _ in 0..50 {
        engine.step();
    }

    // Verify items have flowed through the system.
    // The assembler should have consumed iron and copper.
    let assembler_gears = output_quantity(&engine, assembler, gear());
    let total_items = input_total(&engine, assembler)
        + output_total(&engine, assembler)
        + input_total(&engine, buffer)
        + output_total(&engine, buffer)
        + input_total(&engine, sink);

    assert!(
        total_items > 0,
        "items should flow through mixed transport factory; total items in system: {total_items}"
    );
}
```

**Step 2: Run the integration tests**

Run: `cargo test -p factorial-core --test integration 2>&1`
Expected: All 9 integration tests pass (6 existing + 3 new).

**Step 3: Commit**

```bash
git add crates/factorial-core/tests/integration.rs
git commit -m "test: add Vehicle/Batch transport integration tests

Tests cover batch transport chains, vehicle round-trips with capacity
limits, and a mixed transport factory using all four transport types."
```

---

## Task 6: BTreeMap in Power Module (Group C)

**Independent of Tasks 3-5. Can run in parallel.**

**Files:**
- Modify: `crates/factorial-power/src/lib.rs`

**Step 1: Replace HashMap with BTreeMap**

In `crates/factorial-power/src/lib.rs`:

1. Change `use std::collections::HashMap;` to `use std::collections::BTreeMap;`

2. Add `Ord, PartialOrd` to `PowerNetworkId` derives:
   Change: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]`
   To: `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]`

3. Replace all `HashMap` with `BTreeMap` in the following locations:
   - Line 175: `pub networks: HashMap<PowerNetworkId, PowerNetwork>` → `BTreeMap`
   - Line 177: `pub producers: HashMap<NodeId, PowerProducer>` → `BTreeMap`
   - Line 179: `pub consumers: HashMap<NodeId, PowerConsumer>` → `BTreeMap`
   - Line 181: `pub storage: HashMap<NodeId, PowerStorage>` → `BTreeMap`
   - Line 198 (in `new()`): `HashMap::new()` → `BTreeMap::new()` (4 occurrences)

**Step 2: Verify NodeId implements Ord**

NodeId is a slotmap key. Check: `slotmap::Key` types implement `Ord` via `KeyData`. Run `cargo check -p factorial-power` to confirm compilation.

Run: `cargo check -p factorial-power 2>&1`
Expected: Compiles. If NodeId doesn't implement Ord, we'll need a newtype wrapper.

**Step 3: Run existing power module tests**

Run: `cargo test -p factorial-power 2>&1`
Expected: All 28 tests pass.

**Step 4: Add deterministic distribution test**

Add to the test module in `crates/factorial-power/src/lib.rs`:

```rust
// -----------------------------------------------------------------------
// Test 29: Deterministic power distribution across runs
// -----------------------------------------------------------------------
#[test]
fn deterministic_power_distribution() {
    // Run the same underpowered scenario twice with multiple consumers
    // and verify identical satisfaction ratios. With HashMap this could
    // produce different iteration orders; with BTreeMap it's deterministic.
    fn run() -> (Fixed64, Vec<Fixed64>) {
        let mut module = PowerModule::new();
        let net = module.create_network();
        let nodes = make_node_ids(5);

        // 50W production, 4 consumers at 25W each = 100W demand.
        module.add_producer(net, nodes[0], PowerProducer { capacity: fixed(50.0) });
        module.add_consumer(net, nodes[1], PowerConsumer { demand: fixed(25.0) });
        module.add_consumer(net, nodes[2], PowerConsumer { demand: fixed(25.0) });
        module.add_consumer(net, nodes[3], PowerConsumer { demand: fixed(25.0) });
        module.add_consumer(net, nodes[4], PowerConsumer { demand: fixed(25.0) });

        module.tick(1);

        let satisfaction = module.satisfaction(net).unwrap();

        // Also capture storage state for any storage nodes to ensure full
        // determinism. In this case there's no storage, so satisfaction is
        // the key value.
        (satisfaction, vec![satisfaction])
    }

    let (sat1, vals1) = run();
    let (sat2, vals2) = run();

    assert_eq!(sat1, sat2, "satisfaction should be deterministic");
    assert_eq!(vals1, vals2, "all values should be deterministic");
    assert_eq!(sat1, fixed(0.5), "50W / 100W = 0.5");
}
```

**Step 5: Run all power tests**

Run: `cargo test -p factorial-power 2>&1`
Expected: All 29 tests pass.

**Step 6: Commit**

```bash
git add crates/factorial-power/src/lib.rs
git commit -m "fix: replace HashMap with BTreeMap in power module for determinism

Ensures power distribution iteration order is deterministic across runs,
matching the engine's core determinism guarantees."
```

---

## Task 7: Serialization Version Field (Group D)

**Independent of Tasks 3-6. Can run in parallel.**

**Files:**
- Modify: `crates/factorial-core/src/serialize.rs`

**Note:** Looking at the existing code, the serialization system ALREADY has a version field! The `SnapshotHeader` struct has `magic: u32` and `version: u32`, with `FORMAT_VERSION = 1` and `SNAPSHOT_MAGIC = 0xFAC7_0001`. The `validate()` method checks both magic and version, returning `DeserializeError::VersionMismatch` for wrong versions.

This means **Item 3 from the design is already implemented.** The existing `DeserializeError` has:
- `InvalidMagic(u32)` — for wrong magic number
- `VersionMismatch(u32)` — for wrong version

The only enhancement from the design not yet present is distinguishing "future version" from "past version" for migration purposes. Currently both return `VersionMismatch`.

**Step 1: Add FutureVersion error variant**

In `crates/factorial-core/src/serialize.rs`, split the version check:

Change the `DeserializeError` enum to add:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DeserializeError {
    #[error("data too short for snapshot header")]
    TooShort,
    #[error("invalid magic number: expected 0x{:08X}, got 0x{:08X}", SNAPSHOT_MAGIC, .0)]
    InvalidMagic(u32),
    #[error("unsupported format version: expected {}, got {}", FORMAT_VERSION, .0)]
    UnsupportedVersion(u32),
    #[error("snapshot from future version {0} (this build supports up to {FORMAT_VERSION})")]
    FutureVersion(u32),
    #[error("bitcode decoding failed: {0}")]
    Decode(String),
}
```

Update `SnapshotHeader::validate()`:

```rust
pub fn validate(&self) -> Result<(), DeserializeError> {
    if self.magic != SNAPSHOT_MAGIC {
        return Err(DeserializeError::InvalidMagic(self.magic));
    }
    if self.version > FORMAT_VERSION {
        return Err(DeserializeError::FutureVersion(self.version));
    }
    if self.version < FORMAT_VERSION {
        return Err(DeserializeError::UnsupportedVersion(self.version));
    }
    Ok(())
}
```

**Step 2: Update existing test that references `VersionMismatch`**

In Test 12 (`serialize_header_validation`), update the bad_version assertion:

```rust
let bad_version = SnapshotHeader {
    magic: SNAPSHOT_MAGIC,
    version: 999,
    tick: 0,
};
assert!(matches!(
    bad_version.validate(),
    Err(DeserializeError::FutureVersion(999))
));
```

**Step 3: Add tests for version distinction**

```rust
// -----------------------------------------------------------------------
// Test 21: Future version produces FutureVersion error
// -----------------------------------------------------------------------
#[test]
fn serialize_future_version_error() {
    let header = SnapshotHeader {
        magic: SNAPSHOT_MAGIC,
        version: FORMAT_VERSION + 1,
        tick: 0,
    };
    assert!(matches!(
        header.validate(),
        Err(DeserializeError::FutureVersion(_))
    ));
}

// -----------------------------------------------------------------------
// Test 22: Past version produces UnsupportedVersion error
// -----------------------------------------------------------------------
#[test]
fn serialize_past_version_error() {
    // Only testable when FORMAT_VERSION > 1. For now (FORMAT_VERSION=1),
    // version 0 is the only "past" version.
    let header = SnapshotHeader {
        magic: SNAPSHOT_MAGIC,
        version: 0,
        tick: 0,
    };
    assert!(matches!(
        header.validate(),
        Err(DeserializeError::UnsupportedVersion(0))
    ));
}

// -----------------------------------------------------------------------
// Test 23: Current version validates successfully
// -----------------------------------------------------------------------
#[test]
fn serialize_current_version_validates() {
    let header = SnapshotHeader::new(42);
    assert!(header.validate().is_ok());
    assert_eq!(header.version, FORMAT_VERSION);
}
```

**Step 4: Run all serialize tests**

Run: `cargo test -p factorial-core serialize 2>&1`
Expected: All tests pass (20 existing, with Test 12 updated, plus 3 new = 23).

**Step 5: Commit**

```bash
git add crates/factorial-core/src/serialize.rs
git commit -m "feat: distinguish FutureVersion from UnsupportedVersion in serialization

Split VersionMismatch into FutureVersion (newer than supported) and
UnsupportedVersion (older, migration hook for later). Enables forward
compatibility detection."
```

---

## Task 8: FFI Poisoned Flag (Group E1)

**Independent of Tasks 3-7. Can run in parallel.**

**Files:**
- Modify: `crates/factorial-ffi/src/lib.rs`

**Step 1: Replace type alias with wrapper struct**

The current FFI uses `pub type FactorialEngine = Engine;` as the opaque handle. We need to wrap it in a struct to add the `poisoned` flag.

Replace:
```rust
pub type FactorialEngine = Engine;
```
With:
```rust
/// Opaque engine wrapper with panic-poisoning support.
/// Callers receive `*mut FactorialEngine` from `factorial_create`
/// and pass it to all subsequent calls.
#[repr(C)]
pub struct FactorialEngine {
    inner: Engine,
    poisoned: bool,
}
```

**Step 2: Add Poisoned error code**

Add to the `FactorialResult` enum:

```rust
/// The engine was poisoned by a previous panic and is in an inconsistent state.
Poisoned = 8,
```

**Step 3: Add helper macro for poisoned check**

Add a helper function:

```rust
/// Check if the engine is poisoned. Returns `Err(FactorialResult::Poisoned)` if so.
fn check_poison(engine: &FactorialEngine) -> Result<(), FactorialResult> {
    if engine.poisoned {
        Err(FactorialResult::Poisoned)
    } else {
        Ok(())
    }
}
```

**Step 4: Update all existing FFI functions**

Every function that takes `*mut FactorialEngine` or `*const FactorialEngine` needs to:
1. Dereference to `&FactorialEngine` or `&mut FactorialEngine`
2. Check `check_poison()` before operating
3. Access `engine.inner` instead of just `engine`
4. On panic catch, set `engine.poisoned = true`

This is a large but mechanical refactor. For example, `factorial_step` becomes:

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_step(engine: *mut FactorialEngine) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        EVENT_CACHE.with(|c| c.borrow_mut().clear());
        engine.inner.step();
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            // Mark engine as poisoned on panic.
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}
```

Apply the same pattern to ALL existing functions. Also update:
- `factorial_create` / `factorial_create_delta`: wrap Engine in `FactorialEngine { inner: engine, poisoned: false }`
- `factorial_destroy`: dereference `FactorialEngine` instead of `Engine`
- All query functions: use `engine.inner` for reads

**Step 5: Add new poison query/clear functions**

```rust
/// Check if the engine is in a poisoned state.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_is_poisoned(engine: *const FactorialEngine) -> bool {
    if engine.is_null() {
        return false;
    }
    let engine = unsafe { &*engine };
    engine.poisoned
}

/// Clear the poisoned flag. Use with caution — the engine state may be
/// inconsistent after a panic.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_clear_poison(engine: *mut FactorialEngine) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    let engine = unsafe { &mut *engine };
    engine.poisoned = false;
    FactorialResult::Ok
}
```

**Step 6: Update existing tests**

All tests that access `engine.inner` directly (e.g., Test 6, 7, 8, 16, 19) need to be updated. Where tests did:
```rust
let engine = unsafe { &mut *engine_ptr };
engine.set_processor(node_id, make_source(iron(), 3.0));
```
Change to:
```rust
let engine = unsafe { &mut *engine_ptr };
engine.inner.set_processor(node_id, make_source(iron(), 3.0));
```

**Step 7: Add poison tests**

```rust
// -----------------------------------------------------------------------
// Test 21: Poisoned engine blocks subsequent calls
// -----------------------------------------------------------------------
#[test]
fn poisoned_engine_blocks_calls() {
    let engine = factorial_create();
    assert!(!engine.is_null());

    // Manually poison the engine for testing.
    let engine_ref = unsafe { &mut *engine };
    engine_ref.poisoned = true;

    assert!(unsafe { factorial_is_poisoned(engine) });

    // All operations should return Poisoned.
    let result = unsafe { factorial_step(engine) };
    assert_eq!(result, FactorialResult::Poisoned);

    let mut tick: u64 = 0;
    let result = unsafe { factorial_get_tick(engine, &mut tick) };
    assert_eq!(result, FactorialResult::Poisoned);

    unsafe { factorial_destroy(engine) };
}

// -----------------------------------------------------------------------
// Test 22: Clear poison allows operations to resume
// -----------------------------------------------------------------------
#[test]
fn clear_poison_allows_resume() {
    let engine = factorial_create();
    let engine_ref = unsafe { &mut *engine };
    engine_ref.poisoned = true;

    assert!(unsafe { factorial_is_poisoned(engine) });

    let result = unsafe { factorial_clear_poison(engine) };
    assert_eq!(result, FactorialResult::Ok);
    assert!(!unsafe { factorial_is_poisoned(engine) });

    // Operations should work again.
    let result = unsafe { factorial_step(engine) };
    assert_eq!(result, FactorialResult::Ok);

    unsafe { factorial_destroy(engine) };
}

// -----------------------------------------------------------------------
// Test 23: New engine is not poisoned
// -----------------------------------------------------------------------
#[test]
fn new_engine_not_poisoned() {
    let engine = factorial_create();
    assert!(!unsafe { factorial_is_poisoned(engine) });
    unsafe { factorial_destroy(engine) };
}
```

**Step 8: Run all FFI tests**

Run: `cargo test -p factorial-ffi 2>&1`
Expected: All tests pass (20 existing updated + 3 new = 23).

**Step 9: Commit**

```bash
git add crates/factorial-ffi/src/lib.rs
git commit -m "feat: add poisoned flag to FFI engine wrapper

Catches panics across the FFI boundary and marks the engine as poisoned.
Subsequent calls return Poisoned error code. Includes factorial_is_poisoned
and factorial_clear_poison functions for host language recovery."
```

---

## Task 9: FFI Configuration Functions (Group E2)

**Depends on:** Task 8 (poisoned flag, since new functions check it)

**Files:**
- Modify: `crates/factorial-ffi/src/lib.rs`
- Modify: `crates/factorial-ffi/Cargo.toml` (may need `fixed` dependency)

**Step 1: Add `fixed` dependency to FFI crate**

In `crates/factorial-ffi/Cargo.toml`, add:

```toml
[dependencies]
factorial-core = { path = "../factorial-core" }
slotmap = { workspace = true }
fixed = { workspace = true }
```

**Step 2: Add FFI-safe recipe struct**

Add to the types section of `lib.rs`:

```rust
/// C-compatible item stack (item type + quantity).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiItemStack {
    pub item_type: u32,
    pub quantity: u32,
}

/// C-compatible recipe for FixedRecipe processor.
#[repr(C)]
#[derive(Debug)]
pub struct FfiRecipe {
    pub input_count: u32,
    pub inputs: *const FfiItemStack,
    pub output_count: u32,
    pub outputs: *const FfiItemStack,
    /// Duration in ticks.
    pub duration: u32,
}
```

**Step 3: Add processor setup functions**

```rust
/// Set a node's processor to Source (produces items at a fixed rate).
///
/// `rate` is the raw Fixed64 bits (i64 reinterpreted as the fixed-point value).
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_source(
    engine: *mut FactorialEngine,
    node_id: FfiNodeId,
    item_type: u32,
    rate: i64,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        engine.inner.set_processor(
            nid,
            Processor::Source(SourceProcessor {
                output_type: ItemTypeId(item_type),
                base_rate: Fixed64::from_bits(rate),
                depletion: Depletion::Infinite,
                accumulated: Fixed64::from_num(0),
            }),
        );
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set a node's processor to FixedRecipe.
///
/// # Safety
///
/// `engine` must be a valid engine pointer. `recipe` must point to a valid FfiRecipe.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_fixed_processor(
    engine: *mut FactorialEngine,
    node_id: FfiNodeId,
    recipe: *const FfiRecipe,
) -> FactorialResult {
    if engine.is_null() || recipe.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        let recipe = unsafe { &*recipe };

        let inputs: Vec<RecipeInput> = if recipe.input_count > 0 && !recipe.inputs.is_null() {
            let slice = unsafe {
                std::slice::from_raw_parts(recipe.inputs, recipe.input_count as usize)
            };
            slice
                .iter()
                .map(|s| RecipeInput {
                    item_type: ItemTypeId(s.item_type),
                    quantity: s.quantity,
                })
                .collect()
        } else {
            Vec::new()
        };

        let outputs: Vec<RecipeOutput> = if recipe.output_count > 0 && !recipe.outputs.is_null() {
            let slice = unsafe {
                std::slice::from_raw_parts(recipe.outputs, recipe.output_count as usize)
            };
            slice
                .iter()
                .map(|s| RecipeOutput {
                    item_type: ItemTypeId(s.item_type),
                    quantity: s.quantity,
                })
                .collect()
        } else {
            Vec::new()
        };

        engine.inner.set_processor(
            nid,
            Processor::Fixed(FixedRecipe {
                inputs,
                outputs,
                duration: recipe.duration,
            }),
        );
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}
```

**Step 4: Add transport setup functions**

```rust
/// Set an edge's transport to FlowTransport.
///
/// `rate` is raw Fixed64 bits.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_flow_transport(
    engine: *mut FactorialEngine,
    edge_id: FfiEdgeId,
    rate: i64,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let eid = ffi_to_edge_id(edge_id);
        engine.inner.set_transport(
            eid,
            Transport::Flow(FlowTransport {
                rate: Fixed64::from_bits(rate),
                buffer_capacity: Fixed64::from_num(1000),
                latency: 0,
            }),
        );
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set an edge's transport to ItemTransport (belt).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_item_transport(
    engine: *mut FactorialEngine,
    edge_id: FfiEdgeId,
    speed: i64,
    slot_count: u32,
    lanes: u8,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let eid = ffi_to_edge_id(edge_id);
        engine.inner.set_transport(
            eid,
            Transport::Item(ItemTransport {
                speed: Fixed64::from_bits(speed),
                slot_count,
                lanes,
            }),
        );
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set an edge's transport to BatchTransport.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_batch_transport(
    engine: *mut FactorialEngine,
    edge_id: FfiEdgeId,
    batch_size: u32,
    cycle_time: u32,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let eid = ffi_to_edge_id(edge_id);
        engine.inner.set_transport(
            eid,
            Transport::Batch(BatchTransport {
                batch_size,
                cycle_time,
            }),
        );
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set an edge's transport to VehicleTransport.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_vehicle_transport(
    engine: *mut FactorialEngine,
    edge_id: FfiEdgeId,
    capacity: u32,
    travel_time: u32,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let eid = ffi_to_edge_id(edge_id);
        engine.inner.set_transport(
            eid,
            Transport::Vehicle(VehicleTransport {
                capacity,
                travel_time,
            }),
        );
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}
```

**Step 5: Add inventory setup functions**

```rust
/// Set a node's input inventory capacity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_input_capacity(
    engine: *mut FactorialEngine,
    node_id: FfiNodeId,
    capacity: u32,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        engine.inner.set_input_inventory(nid, Inventory::new(1, 1, capacity));
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set a node's output inventory capacity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_output_capacity(
    engine: *mut FactorialEngine,
    node_id: FfiNodeId,
    capacity: u32,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        engine.inner.set_output_inventory(nid, Inventory::new(1, 1, capacity));
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}
```

**Step 6: Add tests for new FFI functions**

```rust
// -----------------------------------------------------------------------
// Test 24: Set source processor via FFI
// -----------------------------------------------------------------------
#[test]
fn set_source_via_ffi() {
    let engine_ptr = factorial_create();
    let mut pending: FfiPendingNodeId = 0;
    unsafe { factorial_add_node(engine_ptr, 0, &mut pending) };
    let mut mr = FfiMutationResult {
        added_nodes: ptr::null(),
        added_node_count: 0,
        added_edges: ptr::null(),
        added_edge_count: 0,
    };
    unsafe { factorial_apply_mutations(engine_ptr, &mut mr) };
    let pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 1) };
    let node_ffi = pairs[0].real_id;

    // Set source via FFI: iron at rate 3.0
    let rate_bits = Fixed64::from_num(3.0).to_bits();
    let result = unsafe { factorial_set_source(engine_ptr, node_ffi, 0, rate_bits) };
    assert_eq!(result, FactorialResult::Ok);

    // Set inventories via FFI.
    let result = unsafe { factorial_set_input_capacity(engine_ptr, node_ffi, 100) };
    assert_eq!(result, FactorialResult::Ok);
    let result = unsafe { factorial_set_output_capacity(engine_ptr, node_ffi, 100) };
    assert_eq!(result, FactorialResult::Ok);

    // Step and verify production.
    unsafe { factorial_step(engine_ptr) };
    let mut count: u32 = 0;
    unsafe { factorial_get_output_inventory_count(engine_ptr, node_ffi, &mut count) };
    assert_eq!(count, 3);

    unsafe { factorial_destroy(engine_ptr) };
}

// -----------------------------------------------------------------------
// Test 25: Set fixed processor via FFI recipe struct
// -----------------------------------------------------------------------
#[test]
fn set_fixed_processor_via_ffi() {
    let engine_ptr = factorial_create();
    let mut pending: FfiPendingNodeId = 0;
    unsafe { factorial_add_node(engine_ptr, 0, &mut pending) };
    let mut mr = FfiMutationResult {
        added_nodes: ptr::null(),
        added_node_count: 0,
        added_edges: ptr::null(),
        added_edge_count: 0,
    };
    unsafe { factorial_apply_mutations(engine_ptr, &mut mr) };
    let pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 1) };
    let node_ffi = pairs[0].real_id;

    // Recipe: 2 iron -> 1 gear, 5 ticks.
    let inputs = [FfiItemStack { item_type: 0, quantity: 2 }];
    let outputs = [FfiItemStack { item_type: 2, quantity: 1 }];
    let recipe = FfiRecipe {
        input_count: 1,
        inputs: inputs.as_ptr(),
        output_count: 1,
        outputs: outputs.as_ptr(),
        duration: 5,
    };

    let result = unsafe { factorial_set_fixed_processor(engine_ptr, node_ffi, &recipe) };
    assert_eq!(result, FactorialResult::Ok);

    // Verify the processor is set by checking state.
    let mut info = FfiProcessorInfo {
        state: FfiProcessorState::Working,
        progress: 99,
    };
    unsafe { factorial_get_processor_state(engine_ptr, node_ffi, &mut info) };
    // Should be Idle (no inputs yet).
    assert_eq!(info.state, FfiProcessorState::Idle);

    unsafe { factorial_destroy(engine_ptr) };
}

// -----------------------------------------------------------------------
// Test 26: Set transport types via FFI
// -----------------------------------------------------------------------
#[test]
fn set_transport_types_via_ffi() {
    let engine_ptr = factorial_create();

    // Add two nodes and connect them.
    let mut pa: FfiPendingNodeId = 0;
    let mut pb: FfiPendingNodeId = 0;
    unsafe { factorial_add_node(engine_ptr, 0, &mut pa) };
    unsafe { factorial_add_node(engine_ptr, 0, &mut pb) };
    let mut mr = FfiMutationResult {
        added_nodes: ptr::null(),
        added_node_count: 0,
        added_edges: ptr::null(),
        added_edge_count: 0,
    };
    unsafe { factorial_apply_mutations(engine_ptr, &mut mr) };
    let pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 2) };
    let na = pairs[0].real_id;
    let nb = pairs[1].real_id;

    let mut pe: FfiPendingEdgeId = 0;
    unsafe { factorial_connect(engine_ptr, na, nb, &mut pe) };
    let mut mr2 = FfiMutationResult {
        added_nodes: ptr::null(),
        added_node_count: 0,
        added_edges: ptr::null(),
        added_edge_count: 0,
    };
    unsafe { factorial_apply_mutations(engine_ptr, &mut mr2) };
    let epairs = unsafe { std::slice::from_raw_parts(mr2.added_edges, 1) };
    let edge = epairs[0].real_id;

    // Set flow transport.
    let rate = Fixed64::from_num(5.0).to_bits();
    let result = unsafe { factorial_set_flow_transport(engine_ptr, edge, rate) };
    assert_eq!(result, FactorialResult::Ok);

    // Set item transport (overwrite).
    let speed = Fixed64::from_num(1.0).to_bits();
    let result = unsafe { factorial_set_item_transport(engine_ptr, edge, speed, 10, 1) };
    assert_eq!(result, FactorialResult::Ok);

    // Set batch transport (overwrite).
    let result = unsafe { factorial_set_batch_transport(engine_ptr, edge, 10, 5) };
    assert_eq!(result, FactorialResult::Ok);

    // Set vehicle transport (overwrite).
    let result = unsafe { factorial_set_vehicle_transport(engine_ptr, edge, 20, 3) };
    assert_eq!(result, FactorialResult::Ok);

    unsafe { factorial_destroy(engine_ptr) };
}

// -----------------------------------------------------------------------
// Test 27: Full FFI lifecycle without direct Rust access
// -----------------------------------------------------------------------
#[test]
fn full_ffi_lifecycle_no_direct_access() {
    // This test proves a Godot/Unity client can set up a factory entirely via FFI.
    let engine_ptr = factorial_create();

    // Add source and consumer.
    let mut ps: FfiPendingNodeId = 0;
    let mut pc: FfiPendingNodeId = 0;
    unsafe { factorial_add_node(engine_ptr, 0, &mut ps) };
    unsafe { factorial_add_node(engine_ptr, 1, &mut pc) };
    let mut mr = FfiMutationResult {
        added_nodes: ptr::null(),
        added_node_count: 0,
        added_edges: ptr::null(),
        added_edge_count: 0,
    };
    unsafe { factorial_apply_mutations(engine_ptr, &mut mr) };
    let pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 2) };
    let src_ffi = pairs[0].real_id;
    let con_ffi = pairs[1].real_id;

    // Connect.
    let mut pe: FfiPendingEdgeId = 0;
    unsafe { factorial_connect(engine_ptr, src_ffi, con_ffi, &mut pe) };
    let mut mr2 = FfiMutationResult {
        added_nodes: ptr::null(),
        added_node_count: 0,
        added_edges: ptr::null(),
        added_edge_count: 0,
    };
    unsafe { factorial_apply_mutations(engine_ptr, &mut mr2) };
    let epairs = unsafe { std::slice::from_raw_parts(mr2.added_edges, 1) };
    let edge_ffi = epairs[0].real_id;

    // Configure source (entirely via FFI).
    let rate = Fixed64::from_num(5.0).to_bits();
    unsafe { factorial_set_source(engine_ptr, src_ffi, 0, rate) };
    unsafe { factorial_set_input_capacity(engine_ptr, src_ffi, 100) };
    unsafe { factorial_set_output_capacity(engine_ptr, src_ffi, 100) };

    // Configure consumer (recipe: 2 iron -> 1 gear, 3 ticks).
    let inputs = [FfiItemStack { item_type: 0, quantity: 2 }];
    let outputs = [FfiItemStack { item_type: 2, quantity: 1 }];
    let recipe = FfiRecipe {
        input_count: 1,
        inputs: inputs.as_ptr(),
        output_count: 1,
        outputs: outputs.as_ptr(),
        duration: 3,
    };
    unsafe { factorial_set_fixed_processor(engine_ptr, con_ffi, &recipe) };
    unsafe { factorial_set_input_capacity(engine_ptr, con_ffi, 100) };
    unsafe { factorial_set_output_capacity(engine_ptr, con_ffi, 100) };

    // Set flow transport.
    let transport_rate = Fixed64::from_num(10.0).to_bits();
    unsafe { factorial_set_flow_transport(engine_ptr, edge_ffi, transport_rate) };

    // Step 20 times.
    for _ in 0..20 {
        unsafe { factorial_step(engine_ptr) };
    }

    // Verify items flowed through.
    let mut src_out: u32 = 0;
    let mut con_in: u32 = 0;
    unsafe { factorial_get_output_inventory_count(engine_ptr, src_ffi, &mut src_out) };
    unsafe { factorial_get_input_inventory_count(engine_ptr, con_ffi, &mut con_in) };

    // Source should have produced items, consumer should have received some.
    let total = src_out + con_in;
    assert!(
        total > 0,
        "items should flow through FFI-configured factory; src_out={src_out}, con_in={con_in}"
    );

    unsafe { factorial_destroy(engine_ptr) };
}
```

**Step 7: Run all FFI tests**

Run: `cargo test -p factorial-ffi 2>&1`
Expected: All tests pass.

**Step 8: Run full workspace tests**

Run: `cargo test --workspace 2>&1`
Expected: All tests pass across all crates.

**Step 9: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features 2>&1`
Expected: 0 warnings.

**Step 10: Commit**

```bash
git add crates/factorial-ffi/src/lib.rs crates/factorial-ffi/Cargo.toml
git commit -m "feat: add FFI configuration functions for processor/transport/inventory

Enables Godot/Unity to set up factories entirely via C API without
direct Rust access. Includes set_source, set_fixed_processor,
set_flow_transport, set_item_transport, set_batch_transport,
set_vehicle_transport, set_input_capacity, set_output_capacity."
```

---

## Task 10: Final Verification

**Depends on:** All previous tasks

**Step 1: Run full test suite**

Run: `cargo test --workspace 2>&1`
Expected: ~306 tests, all passing.

**Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features 2>&1`
Expected: 0 warnings.

**Step 3: Run benchmarks (verify no regressions)**

Run: `cargo bench --bench sim_bench 2>&1`
Expected: Benchmarks compile and run. No significant regressions.

**Step 4: Commit any final fixes if needed**

---

## Execution Summary

| Task | Group | Files Changed | New Tests | Depends On |
|------|-------|---------------|-----------|------------|
| T1 | A1 | (verification) | 0 | — |
| T2 | A2 | 5 files | 0 (refactor) | T1 |
| T3 | B1 | engine.rs | 5 | T2 |
| T4 | B2 | graph.rs | 6 | T2 |
| T5 | B3 | integration.rs | 3 | T2 |
| T6 | C | power/lib.rs | 1 | T2 |
| T7 | D | serialize.rs | 3 | T2 |
| T8 | E1 | ffi/lib.rs | 3 | T2 |
| T9 | E2 | ffi/lib.rs, Cargo.toml | 4 | T8 |
| T10 | — | (verification) | 0 | all |

**Parallelism:** After T2 completes, T3-T8 can all run in parallel. T9 depends on T8. T10 is the final gate.

**Total new tests:** ~25 (some design estimates adjusted based on actual code review).
