# Robustness Audit Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Bring factorial-core and factorial-ffi to 80%+ line coverage with mutation testing, then set up GitHub Actions CI to enforce it going forward.

**Architecture:** Measure-first approach. Install coverage tooling, capture baseline, systematically fill gaps module-by-module, run mutation testing, then automate via CI workflows.

**Tech Stack:** cargo-llvm-cov (coverage), cargo-mutants (mutation testing), cargo-nextest (test runner), GitHub Actions (CI)

---

### Task 1: Install Tooling & Capture Baseline

**Files:**
- Create: `docs/coverage/baseline.md`

**Step 1: Install cargo-llvm-cov, cargo-mutants, and cargo-nextest**

Run:
```bash
cargo install cargo-llvm-cov cargo-mutants cargo-nextest
```
Expected: All three install successfully.

**Step 2: Verify existing tests still pass**

Run: `cargo test --workspace`
Expected: 643 tests pass, 0 failures.

**Step 3: Generate baseline coverage report**

Run:
```bash
cargo llvm-cov --package factorial-core --package factorial-ffi --text 2>&1 | tee /tmp/coverage-baseline.txt
```
Expected: Coverage report showing per-file line coverage percentages.

**Step 4: Record baseline to docs**

Create `docs/coverage/baseline.md` with the coverage numbers from Step 3. Format as a markdown table:

```markdown
# Coverage Baseline

**Date:** 2026-02-06
**Tools:** cargo-llvm-cov (LLVM-based)

## Tier 1: factorial-core + factorial-ffi

| File | Lines | Covered | Coverage % |
|------|-------|---------|-----------|
| (fill from report) | | | |

**Aggregate Tier 1 Coverage:** X%
```

**Step 5: Commit**

```bash
git add docs/coverage/baseline.md
git commit -m "docs: add coverage baseline for Tier 1 crates"
```

---

### Task 2: Coverage Gap-Fill — query.rs (0 tests)

**Files:**
- Modify: `crates/factorial-core/src/query.rs`

query.rs currently has zero tests. It defines `NodeSnapshot` and `TransportSnapshot` which are pure data structs with no methods. The coverage gap here is that these structs are constructed in `engine.rs` via `snapshot_node()`, `snapshot_all_nodes()`, and `snapshot_transport()`. The tests for those already exist implicitly in serialize.rs test 16 (`serialize_inventory_contents_preserved`).

**Step 1: Add snapshot construction tests to engine.rs tests**

Add to the test module in `crates/factorial-core/src/engine.rs` (at the end of the existing `#[cfg(test)] mod tests` block). These tests exercise the snapshot_node / snapshot_transport / snapshot_all_nodes methods:

```rust
#[test]
fn snapshot_node_returns_correct_data() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let iron = test_utils::iron();
    let node = test_utils::add_node(
        &mut engine,
        test_utils::make_source(iron, 2.0),
        100,
        100,
    );

    // Step to produce items
    engine.step();

    let snap = engine.snapshot_node(node).expect("node should exist");
    assert_eq!(snap.id, node);
    assert_eq!(snap.processor_state, ProcessorState::Working { progress: 0 });
    assert!(!snap.output_contents.is_empty(), "source should have produced items");
}

#[test]
fn snapshot_node_nonexistent_returns_none() {
    let engine = Engine::new(SimulationStrategy::Tick);
    let fake_id = {
        // Create a throwaway slotmap just to get a valid-shaped but wrong NodeId
        let mut sm: slotmap::SlotMap<NodeId, ()> = slotmap::SlotMap::with_key();
        let id = sm.insert(());
        sm.remove(id);
        id
    };
    assert!(engine.snapshot_node(fake_id).is_none());
}

#[test]
fn snapshot_all_nodes_empty_engine() {
    let engine = Engine::new(SimulationStrategy::Tick);
    let snaps = engine.snapshot_all_nodes();
    assert!(snaps.is_empty());
}

#[test]
fn snapshot_all_nodes_returns_all() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let iron = test_utils::iron();
    test_utils::add_node(&mut engine, test_utils::make_source(iron, 1.0), 50, 50);
    test_utils::add_node(&mut engine, test_utils::make_source(iron, 1.0), 50, 50);
    test_utils::add_node(&mut engine, test_utils::make_source(iron, 1.0), 50, 50);

    let snaps = engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 3);
}

#[test]
fn snapshot_transport_returns_correct_data() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let iron = test_utils::iron();
    let src = test_utils::add_node(&mut engine, test_utils::make_source(iron, 5.0), 100, 100);
    let sink = test_utils::add_node(
        &mut engine,
        test_utils::make_recipe(vec![(iron, 1)], vec![(test_utils::gear(), 1)], 10),
        100,
        100,
    );
    let edge = test_utils::connect(&mut engine, src, sink, test_utils::make_flow_transport(3.0));

    engine.step();

    let snap = engine.snapshot_transport(edge).expect("edge should exist");
    assert_eq!(snap.id, edge);
    assert_eq!(snap.from, src);
    assert_eq!(snap.to, sink);
}

#[test]
fn snapshot_transport_nonexistent_returns_none() {
    let engine = Engine::new(SimulationStrategy::Tick);
    let fake_id = {
        let mut sm: slotmap::SlotMap<EdgeId, ()> = slotmap::SlotMap::with_key();
        let id = sm.insert(());
        sm.remove(id);
        id
    };
    assert!(engine.snapshot_transport(fake_id).is_none());
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test --package factorial-core -- snapshot_node snapshot_all_nodes snapshot_transport`
Expected: All 6 new tests pass.

**Step 3: Commit**

```bash
git add crates/factorial-core/src/engine.rs
git commit -m "test: add snapshot query tests for NodeSnapshot and TransportSnapshot"
```

---

### Task 3: Coverage Gap-Fill — Deserialization Error Paths

**Files:**
- Modify: `crates/factorial-core/src/serialize.rs`

The serialize.rs module already tests `InvalidMagic` and `FutureVersion` via header validation. But `TooShort` (empty/tiny data), `MissingPartition`, and `PartitionDecode` are untested.

**Step 1: Add deserialization error path tests**

Add to the existing `#[cfg(test)] mod tests` block in `crates/factorial-core/src/serialize.rs`:

```rust
#[test]
fn deserialize_empty_data_returns_decode_error() {
    let result = Engine::deserialize(&[]);
    assert!(result.is_err());
    assert!(matches!(result, Err(DeserializeError::Decode(_))));
}

#[test]
fn deserialize_truncated_data_returns_decode_error() {
    let engine = make_test_engine();
    let mut data = engine.serialize().unwrap();
    // Truncate to just a few bytes
    data.truncate(4);
    let result = Engine::deserialize(&data);
    assert!(result.is_err());
    assert!(matches!(result, Err(DeserializeError::Decode(_))));
}

#[test]
fn deserialize_corrupted_data_returns_error() {
    let engine = make_test_engine();
    let mut data = engine.serialize().unwrap();
    // Corrupt the middle of the data
    let mid = data.len() / 2;
    for i in mid..mid + 10 {
        if i < data.len() {
            data[i] = 0xFF;
        }
    }
    let result = Engine::deserialize(&data);
    // Should be some kind of error (Decode or InvalidMagic depending on what we corrupted)
    assert!(result.is_err());
}

#[test]
fn partitioned_deserialize_empty_data_returns_decode_error() {
    let result = Engine::deserialize_partitioned(&[]);
    assert!(result.is_err());
    assert!(matches!(result, Err(DeserializeError::Decode(_))));
}

#[test]
fn partitioned_deserialize_corrupted_partition_returns_error() {
    let engine = make_test_engine();
    let data = engine.serialize_partitioned().unwrap();
    // Decode, corrupt a partition, re-encode
    let mut snap: PartitionedSnapshot = bitcode::deserialize(&data).unwrap();
    snap.partitions[2] = vec![0xFF; 5]; // Corrupt inventory partition
    let corrupted = bitcode::serialize(&snap).unwrap();
    let result = Engine::deserialize_partitioned(&corrupted);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(DeserializeError::PartitionDecode { index: 2, .. })
    ));
}

#[test]
fn partitioned_future_version_returns_error() {
    let engine = make_test_engine();
    let data = engine.serialize_partitioned().unwrap();
    let mut snap: PartitionedSnapshot = bitcode::deserialize(&data).unwrap();
    snap.header.version = FORMAT_VERSION + 10;
    let modified = bitcode::serialize(&snap).unwrap();
    let result = Engine::deserialize_partitioned(&modified);
    assert!(matches!(result, Err(DeserializeError::FutureVersion(_))));
}

#[test]
fn partitioned_old_version_returns_unsupported() {
    let engine = make_test_engine();
    let data = engine.serialize_partitioned().unwrap();
    let mut snap: PartitionedSnapshot = bitcode::deserialize(&data).unwrap();
    snap.header.version = 0;
    let modified = bitcode::serialize(&snap).unwrap();
    let result = Engine::deserialize_partitioned(&modified);
    assert!(matches!(
        result,
        Err(DeserializeError::UnsupportedVersion(0))
    ));
}

#[test]
fn partitioned_bad_magic_returns_invalid_magic() {
    let engine = make_test_engine();
    let data = engine.serialize_partitioned().unwrap();
    let mut snap: PartitionedSnapshot = bitcode::deserialize(&data).unwrap();
    snap.header.magic = 0xDEADBEEF;
    let modified = bitcode::serialize(&snap).unwrap();
    let result = Engine::deserialize_partitioned(&modified);
    assert!(matches!(
        result,
        Err(DeserializeError::InvalidMagic(0xDEADBEEF))
    ));
}

#[test]
fn read_snapshot_header_from_valid_data() {
    let engine = make_test_engine();
    let data = engine.serialize().unwrap();
    let header = read_snapshot_header(&data).unwrap();
    assert_eq!(header.magic, SNAPSHOT_MAGIC);
    assert_eq!(header.version, FORMAT_VERSION);
    assert_eq!(header.tick, engine.sim_state.tick);
}

#[test]
fn read_snapshot_header_from_garbage_returns_decode_error() {
    let result = read_snapshot_header(&[0u8; 5]);
    assert!(matches!(result, Err(DeserializeError::Decode(_))));
}

#[test]
fn detect_format_single_byte_returns_unknown() {
    assert_eq!(Engine::detect_snapshot_format(&[0x42]), SnapshotFormat::Unknown);
}
```

**Step 2: Run tests**

Run: `cargo test --package factorial-core -- serialize`
Expected: All serialize tests pass (existing + new).

**Step 3: Commit**

```bash
git add crates/factorial-core/src/serialize.rs
git commit -m "test: add deserialization error path tests for all error variants"
```

---

### Task 4: Coverage Gap-Fill — Graph, Registry, Module Error Paths

**Files:**
- Modify: `crates/factorial-core/src/graph.rs`
- Modify: `crates/factorial-core/src/registry.rs`
- Modify: `crates/factorial-core/src/module.rs`

**Step 1: Add GraphError tests for NodeNotFound and EdgeNotFound**

These error variants exist but are never directly triggered in tests. However, looking at the code, `GraphError::NodeNotFound` and `EdgeNotFound` are defined but not returned by any graph method -- they exist for downstream consumer code. We should test their Display/Error impls and construction:

Add to `graph.rs` tests:

```rust
#[test]
fn graph_error_display_messages() {
    let (graph, nodes) = make_graph_with_nodes(1);
    let node = nodes[0];
    let err = GraphError::NodeNotFound(node);
    let msg = format!("{err}");
    assert!(msg.contains("node not found"), "got: {msg}");

    let mut sm: slotmap::SlotMap<EdgeId, ()> = slotmap::SlotMap::with_key();
    let edge = sm.insert(());
    let err = GraphError::EdgeNotFound(edge);
    let msg = format!("{err}");
    assert!(msg.contains("edge not found"), "got: {msg}");

    let err = GraphError::CycleDetected;
    let msg = format!("{err}");
    assert!(msg.contains("cycle"), "got: {msg}");
}

#[test]
fn empty_graph_topological_order() {
    let mut graph = ProductionGraph::new();
    let order = graph.topological_order().unwrap();
    assert!(order.is_empty());
}

#[test]
fn topological_order_with_feedback_acyclic() {
    let (mut graph, nodes) = make_graph_with_nodes(3);
    let [a, b, c] = [nodes[0], nodes[1], nodes[2]];
    graph.queue_connect(a, b);
    graph.queue_connect(b, c);
    graph.apply_mutations();

    let (order, back_edges) = graph.topological_order_with_feedback();
    assert_eq!(order.len(), 3);
    assert!(back_edges.is_empty(), "acyclic graph should have no back edges");
}

#[test]
fn topological_order_with_feedback_cyclic() {
    let (mut graph, nodes) = make_graph_with_nodes(3);
    let [a, b, c] = [nodes[0], nodes[1], nodes[2]];
    graph.queue_connect(a, b);
    graph.queue_connect(b, c);
    graph.queue_connect(c, a);
    graph.apply_mutations();

    let (order, back_edges) = graph.topological_order_with_feedback();
    assert_eq!(order.len(), 3, "all nodes should appear in order even with cycle");
    assert!(!back_edges.is_empty(), "cyclic graph should have back edges");
}

#[test]
fn connect_filtered_edge_preserves_filter() {
    let (mut graph, nodes) = make_graph_with_nodes(2);
    let [a, b] = [nodes[0], nodes[1]];
    let iron = ItemTypeId(0);
    let pe = graph.queue_connect_filtered(a, b, Some(iron));
    let result = graph.apply_mutations();
    let edge = result.resolve_edge(pe).unwrap();
    let edge_data = graph.get_edge(edge).unwrap();
    assert_eq!(edge_data.item_filter, Some(iron));
}

#[test]
fn connect_filtered_edge_none_filter() {
    let (mut graph, nodes) = make_graph_with_nodes(2);
    let [a, b] = [nodes[0], nodes[1]];
    let pe = graph.queue_connect_filtered(a, b, None);
    let result = graph.apply_mutations();
    let edge = result.resolve_edge(pe).unwrap();
    let edge_data = graph.get_edge(edge).unwrap();
    assert_eq!(edge_data.item_filter, None);
}
```

**Step 2: Add Registry error tests**

Add to `registry.rs` tests:

```rust
#[test]
fn invalid_item_ref_error_variant() {
    let mut b = RegistryBuilder::new();
    b.register_recipe(
        "bad_output",
        vec![],
        vec![RecipeEntry {
            item: ItemTypeId(999),
            quantity: 1,
        }],
        60,
    );
    let result = b.build();
    assert!(result.is_err());
    match result {
        Err(RegistryError::InvalidItemRef(id)) => {
            assert_eq!(id, ItemTypeId(999));
            let msg = format!("{}", RegistryError::InvalidItemRef(id));
            assert!(msg.contains("invalid item reference"), "got: {msg}");
        }
        other => panic!("expected InvalidItemRef, got: {other:?}"),
    }
}

#[test]
fn mutate_nonexistent_building_fails() {
    let mut builder = setup_builder();
    let result = builder.mutate_building("nonexistent", |_| {});
    assert!(result.is_err());
    match result {
        Err(RegistryError::NotFound(name)) => {
            assert_eq!(name, "nonexistent");
        }
        other => panic!("expected NotFound, got: {other:?}"),
    }
}

#[test]
fn mutate_building_succeeds() {
    let mut builder = setup_builder();
    builder
        .mutate_building("smelter", |b| {
            b.recipe = None;
        })
        .unwrap();
    let reg = builder.build().unwrap();
    let smelter_id = reg.building_id("smelter").unwrap();
    let smelter = reg.get_building(smelter_id).unwrap();
    assert!(smelter.recipe.is_none());
}

#[test]
fn registry_get_nonexistent_returns_none() {
    let builder = setup_builder();
    let reg = builder.build().unwrap();
    assert!(reg.get_item(ItemTypeId(999)).is_none());
    assert!(reg.get_recipe(RecipeId(999)).is_none());
    assert!(reg.get_building(BuildingTypeId(999)).is_none());
    assert!(reg.building_id("nonexistent").is_none());
    assert!(reg.recipe_id("nonexistent").is_none());
}

#[test]
fn registry_item_has_properties_nonexistent_returns_false() {
    let builder = setup_builder();
    let reg = builder.build().unwrap();
    assert!(!reg.item_has_properties(ItemTypeId(999)));
}

#[test]
fn empty_registry_builds_successfully() {
    let b = RegistryBuilder::new();
    let reg = b.build().unwrap();
    assert_eq!(reg.item_count(), 0);
    assert_eq!(reg.recipe_count(), 0);
    assert_eq!(reg.building_count(), 0);
}
```

**Step 3: Add Module error path tests**

Add to `module.rs` tests:

```rust
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
```

**Step 4: Run all tests**

Run: `cargo test --package factorial-core`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add crates/factorial-core/src/graph.rs crates/factorial-core/src/registry.rs crates/factorial-core/src/module.rs
git commit -m "test: add error path tests for graph, registry, and module error variants"
```

---

### Task 5: Coverage Gap-Fill — sim.rs, component.rs, id.rs

**Files:**
- Modify: `crates/factorial-core/src/sim.rs`
- Modify: `crates/factorial-core/src/component.rs`
- Modify: `crates/factorial-core/src/id.rs`

**Step 1: Add sim.rs tests**

Add to `sim.rs` tests:

```rust
#[test]
fn sim_state_default_is_zero() {
    let state = SimState::default();
    assert_eq!(state.tick, 0);
    assert_eq!(state.accumulator, 0);
}

#[test]
fn simulation_strategy_tick_debug() {
    let strategy = SimulationStrategy::Tick;
    let debug = format!("{strategy:?}");
    assert!(debug.contains("Tick"));
}

#[test]
fn simulation_strategy_delta_stores_timestep() {
    let strategy = SimulationStrategy::Delta { fixed_timestep: 2 };
    match strategy {
        SimulationStrategy::Delta { fixed_timestep } => assert_eq!(fixed_timestep, 2),
        _ => panic!("expected Delta"),
    }
}

#[test]
fn state_hash_empty_is_fnv_offset() {
    let h = StateHash::new();
    assert_eq!(h.0, 0xcbf29ce484222325);
}

#[test]
fn state_hash_default_equals_new() {
    assert_eq!(StateHash::default().0, StateHash::new().0);
}

#[test]
fn state_hash_write_fixed64() {
    use crate::fixed::Fixed64;
    let mut h1 = StateHash::new();
    h1.write_fixed64(Fixed64::from_num(42));

    let mut h2 = StateHash::new();
    h2.write_fixed64(Fixed64::from_num(42));

    assert_eq!(h1.finish(), h2.finish());

    let mut h3 = StateHash::new();
    h3.write_fixed64(Fixed64::from_num(43));
    assert_ne!(h1.finish(), h3.finish());
}

#[test]
fn advance_result_default() {
    let result = AdvanceResult::default();
    assert_eq!(result.steps_run, 0);
    assert!(result.mutation_results.is_empty());
}
```

**Step 2: Add component.rs tests**

Add to `component.rs` tests:

```rust
#[test]
fn component_storage_default() {
    let storage = ComponentStorage::default();
    assert!(storage.inventories.is_empty());
    assert!(storage.power_consumers.is_empty());
    assert!(storage.power_producers.is_empty());
}

#[test]
fn remove_node_nonexistent_is_noop() {
    let mut storage = ComponentStorage::new();
    let mut nodes: SlotMap<NodeId, ()> = SlotMap::with_key();
    let node = nodes.insert(());
    nodes.remove(node); // Remove from slotmap but keep the key
    // Should not panic
    storage.remove_node(node);
}

#[test]
fn power_producer_stored_and_retrieved() {
    let mut storage = ComponentStorage::new();
    let mut nodes: SlotMap<NodeId, ()> = SlotMap::with_key();
    let node = nodes.insert(());
    storage.power_producers.insert(
        node,
        PowerProducer {
            output: Fixed64::from_num(500),
        },
    );
    assert!(storage.power_producers.contains_key(node));
    assert_eq!(storage.power_producers[node].output, Fixed64::from_num(500));
}

#[test]
fn remove_node_only_affects_target() {
    let mut storage = ComponentStorage::new();
    let mut nodes: SlotMap<NodeId, ()> = SlotMap::with_key();
    let node_a = nodes.insert(());
    let node_b = nodes.insert(());
    storage.inventories.insert(node_a, Inventory::new(1, 1, 50));
    storage.inventories.insert(node_b, Inventory::new(1, 1, 50));
    storage.remove_node(node_a);
    assert!(!storage.inventories.contains_key(node_a));
    assert!(storage.inventories.contains_key(node_b));
}
```

**Step 3: Add id.rs tests**

Add to `id.rs` tests:

```rust
#[test]
fn recipe_id_equality_and_copy() {
    let a = RecipeId(0);
    let b = RecipeId(0);
    let c = RecipeId(1);
    assert_eq!(a, b);
    assert_ne!(a, c);
    let d = a; // Copy
    assert_eq!(a, d);
}

#[test]
fn property_id_ordering() {
    let a = PropertyId(1);
    let b = PropertyId(2);
    assert!(a < b);
    assert!(b > a);
}

#[test]
fn modifier_id_equality() {
    let a = ModifierId(10);
    let b = ModifierId(10);
    let c = ModifierId(20);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn pending_node_id_equality() {
    let a = PendingNodeId(0);
    let b = PendingNodeId(0);
    let c = PendingNodeId(1);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn pending_edge_id_equality() {
    let a = PendingEdgeId(0);
    let b = PendingEdgeId(0);
    let c = PendingEdgeId(1);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn ids_debug_format() {
    let item = ItemTypeId(42);
    let debug = format!("{item:?}");
    assert!(debug.contains("42"), "got: {debug}");

    let building = BuildingTypeId(7);
    let debug = format!("{building:?}");
    assert!(debug.contains("7"), "got: {debug}");
}

#[test]
fn item_type_id_ordering() {
    let a = ItemTypeId(1);
    let b = ItemTypeId(2);
    assert!(a < b);
}

#[test]
fn building_type_id_hashable() {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert(BuildingTypeId(0), "furnace");
    map.insert(BuildingTypeId(1), "assembler");
    assert_eq!(map[&BuildingTypeId(0)], "furnace");
    assert_eq!(map.len(), 2);
}
```

**Step 4: Run all tests**

Run: `cargo test --package factorial-core`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add crates/factorial-core/src/sim.rs crates/factorial-core/src/component.rs crates/factorial-core/src/id.rs
git commit -m "test: add coverage tests for sim, component, and id modules"
```

---

### Task 6: Coverage Gap-Fill — Engine Edge Cases

**Files:**
- Modify: `crates/factorial-core/src/engine.rs`

**Step 1: Add engine edge case tests**

Add to the existing test module in `engine.rs`. These target untested paths in the engine: advance with delta mode, pause/resume, empty graph stepping.

```rust
#[test]
fn delta_mode_advance_runs_correct_steps() {
    let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 3 });
    let iron = test_utils::iron();
    let _node = test_utils::add_node(&mut engine, test_utils::make_source(iron, 1.0), 100, 100);

    // Advance by 10 time units with timestep of 3 = 3 full steps (9 consumed, 1 leftover)
    let result = engine.advance(10);
    assert_eq!(result.steps_run, 3);
    assert_eq!(engine.sim_state.tick, 3);
    assert_eq!(engine.sim_state.accumulator, 1); // remainder
}

#[test]
fn delta_mode_advance_zero_dt() {
    let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 5 });
    let result = engine.advance(0);
    assert_eq!(result.steps_run, 0);
    assert_eq!(engine.sim_state.tick, 0);
}

#[test]
fn delta_mode_accumulates_across_calls() {
    let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep: 3 });
    // Advance by 2 -- not enough for a step
    let result = engine.advance(2);
    assert_eq!(result.steps_run, 0);
    assert_eq!(engine.sim_state.accumulator, 2);

    // Advance by 2 more -- now we have 4, enough for one step of 3
    let result = engine.advance(2);
    assert_eq!(result.steps_run, 1);
    assert_eq!(engine.sim_state.tick, 1);
    assert_eq!(engine.sim_state.accumulator, 1);
}

#[test]
fn tick_mode_advance_runs_one_step() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    // In tick mode, advance(dt) should run exactly dt steps
    let result = engine.advance(5);
    assert_eq!(result.steps_run, 5);
    assert_eq!(engine.sim_state.tick, 5);
}

#[test]
fn step_empty_engine() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    // Should not panic
    let result = engine.step();
    assert_eq!(result.steps_run, 1);
    assert_eq!(engine.sim_state.tick, 1);
}

#[test]
fn pause_prevents_stepping() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    engine.set_paused(true);
    assert!(engine.is_paused());

    let result = engine.step();
    assert_eq!(result.steps_run, 0);
    assert_eq!(engine.sim_state.tick, 0);
}

#[test]
fn unpause_resumes_stepping() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    engine.set_paused(true);
    engine.step();
    assert_eq!(engine.sim_state.tick, 0);

    engine.set_paused(false);
    engine.step();
    assert_eq!(engine.sim_state.tick, 1);
}

#[test]
fn node_and_edge_count_accessors() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    assert_eq!(engine.node_count(), 0);
    assert_eq!(engine.edge_count(), 0);

    let iron = test_utils::iron();
    let n1 = test_utils::add_node(&mut engine, test_utils::make_source(iron, 1.0), 50, 50);
    let n2 = test_utils::add_node(&mut engine, test_utils::make_source(iron, 1.0), 50, 50);
    assert_eq!(engine.node_count(), 2);

    test_utils::connect(&mut engine, n1, n2, test_utils::make_flow_transport(1.0));
    assert_eq!(engine.edge_count(), 1);
}

#[test]
fn get_inventories_for_nonexistent_node() {
    let engine = Engine::new(SimulationStrategy::Tick);
    let fake_id = {
        let mut sm: slotmap::SlotMap<NodeId, ()> = slotmap::SlotMap::with_key();
        let id = sm.insert(());
        sm.remove(id);
        id
    };
    assert!(engine.get_input_inventory(fake_id).is_none());
    assert!(engine.get_output_inventory(fake_id).is_none());
    assert!(engine.get_processor_state(fake_id).is_none());
}
```

**Step 2: Run tests**

Run: `cargo test --package factorial-core -- engine`
Expected: All engine tests pass.

**Step 3: Commit**

```bash
git add crates/factorial-core/src/engine.rs
git commit -m "test: add engine edge case tests for delta mode, pause, and accessors"
```

---

### Task 7: Coverage Gap-Fill — factorial-ffi Adversarial Tests

**Files:**
- Modify: `crates/factorial-ffi/src/lib.rs`

**Step 1: Add FFI adversarial tests**

Add to the existing `#[cfg(test)] mod tests` block in `crates/factorial-ffi/src/lib.rs`:

```rust
// -----------------------------------------------------------------------
// Test 28: Deserialize with null out_engine pointer
// -----------------------------------------------------------------------
#[test]
fn deserialize_null_out_engine() {
    let data = [1u8; 10];
    let result = unsafe { factorial_deserialize(data.as_ptr(), data.len(), ptr::null_mut()) };
    assert_eq!(result, FactorialResult::NullPointer);
}

// -----------------------------------------------------------------------
// Test 29: Serialize null output pointer
// -----------------------------------------------------------------------
#[test]
fn serialize_null_output() {
    let engine = factorial_create();
    let result = unsafe { factorial_serialize(engine, ptr::null_mut()) };
    assert_eq!(result, FactorialResult::NullPointer);
    unsafe { factorial_destroy(engine) };
}

// -----------------------------------------------------------------------
// Test 30: Deserialize zero-length data
// -----------------------------------------------------------------------
#[test]
fn deserialize_zero_length_data() {
    let data = [0u8; 0];
    let mut engine_ptr: *mut FactorialEngine = ptr::null_mut();
    // data.as_ptr() on empty slice is technically valid, but len=0
    let result = unsafe { factorial_deserialize(data.as_ptr(), 0, &mut engine_ptr) };
    assert_eq!(result, FactorialResult::DeserializeError);
    assert!(engine_ptr.is_null());
}

// -----------------------------------------------------------------------
// Test 31: Operations on poisoned engine all return Poisoned
// -----------------------------------------------------------------------
#[test]
fn poisoned_engine_rejects_all_operations() {
    let engine = factorial_create();
    let engine_ref = unsafe { &mut *engine };
    engine_ref.poisoned = true;

    // Mutation operations
    let mut pending: FfiPendingNodeId = 0;
    assert_eq!(
        unsafe { factorial_add_node(engine, 0, &mut pending) },
        FactorialResult::Poisoned
    );
    assert_eq!(
        unsafe { factorial_remove_node(engine, 0) },
        FactorialResult::Poisoned
    );
    assert_eq!(
        unsafe { factorial_connect(engine, 0, 0, &mut 0u64) },
        FactorialResult::Poisoned
    );
    assert_eq!(
        unsafe { factorial_disconnect(engine, 0) },
        FactorialResult::Poisoned
    );

    let mut mr = FfiMutationResult {
        added_nodes: ptr::null(),
        added_node_count: 0,
        added_edges: ptr::null(),
        added_edge_count: 0,
    };
    assert_eq!(
        unsafe { factorial_apply_mutations(engine, &mut mr) },
        FactorialResult::Poisoned
    );

    // Advance
    assert_eq!(
        unsafe { factorial_advance(engine, 10) },
        FactorialResult::Poisoned
    );

    // Queries
    let mut count: u32 = 0;
    assert_eq!(
        unsafe { factorial_node_count(engine, &mut count) },
        FactorialResult::Poisoned
    );
    assert_eq!(
        unsafe { factorial_edge_count(engine, &mut count) },
        FactorialResult::Poisoned
    );
    let mut hash: u64 = 0;
    assert_eq!(
        unsafe { factorial_get_state_hash(engine, &mut hash) },
        FactorialResult::Poisoned
    );

    // Serialization
    let mut buf = FfiByteBuffer {
        data: ptr::null_mut(),
        len: 0,
    };
    assert_eq!(
        unsafe { factorial_serialize(engine, &mut buf) },
        FactorialResult::Poisoned
    );

    // Events
    let mut eb = FfiEventBuffer {
        events: ptr::null(),
        count: 0,
    };
    assert_eq!(
        unsafe { factorial_poll_events(engine, &mut eb) },
        FactorialResult::Poisoned
    );

    // Configuration
    let rate_bits = Fixed64::from_num(1).to_bits();
    assert_eq!(
        unsafe { factorial_set_source(engine, 0, 0, rate_bits) },
        FactorialResult::Poisoned
    );
    let inputs = [FfiItemStack { item_type: 0, quantity: 1 }];
    let outputs = [FfiItemStack { item_type: 1, quantity: 1 }];
    let recipe = FfiRecipe {
        input_count: 1,
        inputs: inputs.as_ptr(),
        output_count: 1,
        outputs: outputs.as_ptr(),
        duration: 5,
    };
    assert_eq!(
        unsafe { factorial_set_fixed_processor(engine, 0, &recipe) },
        FactorialResult::Poisoned
    );
    assert_eq!(
        unsafe { factorial_set_flow_transport(engine, 0, rate_bits) },
        FactorialResult::Poisoned
    );

    unsafe { factorial_destroy(engine) };
}

// -----------------------------------------------------------------------
// Test 32: Set transport null pointer checks
// -----------------------------------------------------------------------
#[test]
fn transport_null_pointer_checks() {
    let rate_bits = Fixed64::from_num(1).to_bits();
    assert_eq!(
        unsafe { factorial_set_flow_transport(ptr::null_mut(), 0, rate_bits) },
        FactorialResult::NullPointer
    );
    assert_eq!(
        unsafe { factorial_set_item_transport(ptr::null_mut(), 0, rate_bits, 10, 1) },
        FactorialResult::NullPointer
    );
    assert_eq!(
        unsafe { factorial_set_batch_transport(ptr::null_mut(), 0, 10, 5) },
        FactorialResult::NullPointer
    );
    assert_eq!(
        unsafe { factorial_set_vehicle_transport(ptr::null_mut(), 0, 50, 10) },
        FactorialResult::NullPointer
    );
}

// -----------------------------------------------------------------------
// Test 33: Set capacity null pointer checks
// -----------------------------------------------------------------------
#[test]
fn capacity_null_pointer_checks() {
    assert_eq!(
        unsafe { factorial_set_input_capacity(ptr::null_mut(), 0, 100) },
        FactorialResult::NullPointer
    );
    assert_eq!(
        unsafe { factorial_set_output_capacity(ptr::null_mut(), 0, 100) },
        FactorialResult::NullPointer
    );
}

// -----------------------------------------------------------------------
// Test 34: Source null pointer check
// -----------------------------------------------------------------------
#[test]
fn source_null_pointer_check() {
    let rate_bits = Fixed64::from_num(1).to_bits();
    assert_eq!(
        unsafe { factorial_set_source(ptr::null_mut(), 0, 0, rate_bits) },
        FactorialResult::NullPointer
    );
}

// -----------------------------------------------------------------------
// Test 35: Multiple engines interleaved
// -----------------------------------------------------------------------
#[test]
fn multiple_engines_interleaved() {
    let engine_a = factorial_create();
    let engine_b = factorial_create();

    // Step engine A 5 times
    for _ in 0..5 {
        unsafe { factorial_step(engine_a) };
    }

    // Step engine B 3 times
    for _ in 0..3 {
        unsafe { factorial_step(engine_b) };
    }

    let mut tick_a: u64 = 0;
    let mut tick_b: u64 = 0;
    unsafe { factorial_get_tick(engine_a, &mut tick_a) };
    unsafe { factorial_get_tick(engine_b, &mut tick_b) };
    assert_eq!(tick_a, 5);
    assert_eq!(tick_b, 3);

    unsafe { factorial_destroy(engine_a) };
    unsafe { factorial_destroy(engine_b) };
}

// -----------------------------------------------------------------------
// Test 36: Serialize then deserialize large engine
// -----------------------------------------------------------------------
#[test]
fn serialize_large_engine() {
    let engine_ptr = factorial_create();

    // Add many nodes
    for _ in 0..50 {
        ffi_add_node_and_apply(engine_ptr, 0);
    }

    let mut count: u32 = 0;
    unsafe { factorial_node_count(engine_ptr, &mut count) };
    assert_eq!(count, 50);

    // Serialize
    let mut buf = FfiByteBuffer {
        data: ptr::null_mut(),
        len: 0,
    };
    let result = unsafe { factorial_serialize(engine_ptr, &mut buf) };
    assert_eq!(result, FactorialResult::Ok);
    assert!(buf.len > 0);

    // Deserialize
    let mut restored: *mut FactorialEngine = ptr::null_mut();
    let result = unsafe { factorial_deserialize(buf.data, buf.len, &mut restored) };
    assert_eq!(result, FactorialResult::Ok);

    let mut restored_count: u32 = 0;
    unsafe { factorial_node_count(restored, &mut restored_count) };
    assert_eq!(restored_count, 50);

    unsafe { factorial_free_buffer(buf) };
    unsafe { factorial_destroy(restored) };
    unsafe { factorial_destroy(engine_ptr) };
}
```

**Step 2: Run tests**

Run: `cargo test --package factorial-ffi`
Expected: All FFI tests pass.

**Step 3: Commit**

```bash
git add crates/factorial-ffi/src/lib.rs
git commit -m "test: add adversarial FFI tests for poisoned state, null pointers, and edge cases"
```

---

### Task 8: Measure Coverage After Gap-Fill

**Files:**
- Modify: `docs/coverage/baseline.md`

**Step 1: Run coverage again**

Run:
```bash
cargo llvm-cov --package factorial-core --package factorial-ffi --text 2>&1 | tee /tmp/coverage-after.txt
```
Expected: Coverage percentages should be significantly higher than baseline.

**Step 2: Update baseline.md with before/after table**

Add an "After" section to `docs/coverage/baseline.md` with the new numbers. Calculate delta for each file.

**Step 3: Verify 80% target**

Check that aggregate Tier 1 coverage is at or above 80%. If not, identify remaining low-coverage files and note what additional tests would be needed.

**Step 4: Commit**

```bash
git add docs/coverage/baseline.md
git commit -m "docs: update coverage baseline with post-gap-fill numbers"
```

---

### Task 9: Mutation Testing

**Files:**
- Create: `docs/coverage/mutation-report.md`

**Step 1: Run mutation testing on factorial-core**

Run:
```bash
cargo mutants --package factorial-core --timeout 60 2>&1 | tee /tmp/mutants-core.txt
```
Expected: This will take a while. Record total mutants, killed, survived, timeout.

Note: If the run is too slow, restrict to specific files:
```bash
cargo mutants --package factorial-core --file src/engine.rs --file src/serialize.rs --file src/graph.rs --timeout 60
```

**Step 2: Run mutation testing on factorial-ffi**

Run:
```bash
cargo mutants --package factorial-ffi --timeout 60 2>&1 | tee /tmp/mutants-ffi.txt
```

**Step 3: Analyze surviving mutants**

Review the survivors. For each surviving mutant, categorize:
- **Worth killing:** Production logic (write a targeted test)
- **Not worth killing:** Display impls, debug formatting, logging (skip)

**Step 4: Write targeted tests for high-value survivors**

Add tests to the appropriate modules to kill the most impactful surviving mutants. Commit each batch.

**Step 5: Record results**

Create `docs/coverage/mutation-report.md`:

```markdown
# Mutation Testing Report

**Date:** 2026-02-06
**Tool:** cargo-mutants
**Scope:** factorial-core, factorial-ffi

## Summary

| Package | Total Mutants | Killed | Survived | Timeout | Kill Rate |
|---------|--------------|--------|----------|---------|-----------|
| factorial-core | X | X | X | X | X% |
| factorial-ffi | X | X | X | X | X% |

## Notable Survivors

(List any surviving mutants in production logic with notes on why they survived and whether they warrant additional tests)

## Accepted Survivors

(List survivors in Display impls, debug formatting, etc. that are not worth killing)
```

**Step 6: Commit**

```bash
git add docs/coverage/mutation-report.md
git commit -m "docs: add mutation testing report for Tier 1 crates"
```

---

### Task 10: CI Workflow — Tests & Lint

**Files:**
- Create: `.github/workflows/test.yml`

**Step 1: Create the workflow file**

```yaml
name: Tests & Lint

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -Dwarnings

jobs:
  test:
    name: Test Suite
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run tests
        run: cargo test --workspace

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - name: Run clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

  fmt:
    name: Formatting
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - name: Check formatting
        run: cargo fmt --all -- --check
```

**Step 2: Run clippy locally to verify clean**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: No warnings.

**Step 3: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "ci: add test, clippy, and formatting workflow"
```

---

### Task 11: CI Workflow — Coverage Gate

**Files:**
- Create: `.github/workflows/coverage.yml`

**Step 1: Create the workflow file**

```yaml
name: Coverage Gate

on:
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  coverage:
    name: Coverage Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - uses: Swatinem/rust-cache@v2
      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov
      - name: Generate coverage report
        run: |
          cargo llvm-cov --package factorial-core --package factorial-ffi \
            --fail-under-lines 80 \
            --text 2>&1 | tee coverage-report.txt
      - name: Upload coverage report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: coverage-report
          path: coverage-report.txt
      - name: Post coverage summary to PR
        if: github.event_name == 'pull_request' && always()
        uses: marocchino/sticky-pull-request-comment@v2
        with:
          header: coverage
          message: |
            ## Coverage Report
            ```
            $(cat coverage-report.txt | tail -20)
            ```
```

**Step 2: Commit**

```bash
git add .github/workflows/coverage.yml
git commit -m "ci: add coverage gate workflow (80% threshold for Tier 1)"
```

---

### Task 12: CI Workflow — Mutation Testing

**Files:**
- Create: `.github/workflows/mutation.yml`

**Step 1: Create the workflow file**

```yaml
name: Mutation Testing

on:
  schedule:
    - cron: '0 6 * * 1' # Weekly on Monday at 6 AM UTC
  workflow_dispatch: # Manual trigger

env:
  CARGO_TERM_COLOR: always

jobs:
  mutants:
    name: Mutation Testing
    runs-on: ubuntu-latest
    timeout-minutes: 120
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install cargo-mutants
        run: cargo install cargo-mutants
      - name: Run mutation testing (factorial-core)
        run: |
          cargo mutants --package factorial-core \
            --timeout 120 \
            --output mutants-core 2>&1 | tee mutants-core-summary.txt
      - name: Run mutation testing (factorial-ffi)
        run: |
          cargo mutants --package factorial-ffi \
            --timeout 120 \
            --output mutants-ffi 2>&1 | tee mutants-ffi-summary.txt
      - name: Upload results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: mutation-results
          path: |
            mutants-core/
            mutants-ffi/
            mutants-core-summary.txt
            mutants-ffi-summary.txt
```

**Step 2: Commit**

```bash
git add .github/workflows/mutation.yml
git commit -m "ci: add weekly mutation testing workflow for Tier 1"
```

---

### Task 13: Final Verification & Docs

**Files:**
- Modify: `docs/coverage/baseline.md`

**Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass (original 643 + new tests).

**Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: Zero warnings.

**Step 3: Run final coverage measurement**

Run: `cargo llvm-cov --package factorial-core --package factorial-ffi --text`
Expected: 80%+ line coverage on Tier 1.

**Step 4: Update docs with final numbers**

Add "Final" section to `docs/coverage/baseline.md` with:
- Total test count (before and after)
- Per-module coverage comparison
- Tier 2/3 roadmap section listing remaining gaps

**Step 5: Final commit**

```bash
git add -A
git commit -m "docs: finalize robustness audit with coverage and mutation results"
```

---

## Execution Notes

- Tasks 2-7 are the bulk of the work (writing tests). They can be executed by subagents in parallel where independent.
- Tasks 8-9 depend on Tasks 2-7 being complete (need the tests to exist for coverage/mutation measurements).
- Tasks 10-12 are independent CI workflows and can be done in parallel.
- Task 13 is the final verification pass.
- If any test reveals an actual bug, fix it in a separate commit before moving on.
