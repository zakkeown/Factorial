# React to Production Events

**Goal:** Register passive event listeners to observe production activity and use the query API to inspect engine state.
**Prerequisites:** [Events](../core-concepts/events.md), [Queries](../core-concepts/queries.md), [Processors](../core-concepts/processors.md)
**Example:** `crates/factorial-core/examples/events_and_queries.rs`

## Steps

### 1. Build a small factory

```rust
let p_mine = engine.graph.queue_add_node(BuildingTypeId(0));
let p_assembler = engine.graph.queue_add_node(BuildingTypeId(1));
let r = engine.graph.apply_mutations();
let mine = r.resolve_node(p_mine).unwrap();
let assembler = r.resolve_node(p_assembler).unwrap();

let p_belt = engine.graph.queue_connect(mine, assembler);
let r = engine.graph.apply_mutations();
let belt = r.resolve_edge(p_belt).unwrap();
```

A mine and assembler connected by a belt -- the same pattern as [Model a Smelting Chain](./smelting-chain.md).

### 2. Register passive event listeners

```rust
let produced_count = Rc::new(RefCell::new(0u32));
let counter = produced_count.clone();
engine.on_passive(
    EventKind::ItemProduced,
    Box::new(move |event| {
        if let Event::ItemProduced { quantity, .. } = event {
            *counter.borrow_mut() += quantity;
        }
    }),
);

let recipe_completions = Rc::new(RefCell::new(0u32));
let completions = recipe_completions.clone();
engine.on_passive(
    EventKind::RecipeCompleted,
    Box::new(move |_event| {
        *completions.borrow_mut() += 1;
    }),
);
```

Passive listeners are called after each [tick](../introduction/glossary.md#tick) with any events that occurred. They do not affect simulation state -- they are observation-only hooks for UI updates, statistics, or logging.

### 3. Query state with the snapshot API

```rust
for tick in 0..10 {
    engine.step();

    let snapshots = engine.snapshot_all_nodes();
    let progress = engine
        .get_processor_progress(assembler)
        .unwrap_or(Fixed64::ZERO);

    for snap in &snapshots {
        // snap.processor_state, snap.progress, snap.input_contents, snap.output_contents
    }
}
```

[Snapshots](../introduction/glossary.md#snapshot) are read-only views of a [node](../introduction/glossary.md#node)'s current state. `get_processor_progress()` returns the raw crafting progress as a [fixed-point](../introduction/glossary.md#fixed-point) value.

### 4. Inspect transport state

```rust
if let Some(tsnap) = engine.snapshot_transport(belt) {
    println!("utilization={:.2}, items_in_transit={}", tsnap.utilization, tsnap.items_in_transit);
}
```

Transport snapshots show utilization (0.0 to 1.0) and the count of items currently in transit along the [edge](../introduction/glossary.md#edge).

## What's Happening

Events fire during the `engine.step()` call as the engine processes each node. When the mine's `Source` [processor](../introduction/glossary.md#processor) generates ore, an `ItemProduced` event fires. When the assembler finishes crafting a gear, a `RecipeCompleted` event fires. After the step completes, all accumulated events are dispatched to registered passive listeners. The snapshot and query APIs read from the engine's current state without copying the entire graph -- they are designed to be called every frame for rendering.

## Variations

- **Filtered listeners:** Register separate listeners for `EventKind::NodeStalled` to detect when buildings run out of inputs or their output is full.
- **Per-node queries:** Use `engine.snapshot_node(node_id)` instead of `snapshot_all_nodes()` when you only need one building's state.
- **Transport events:** Listen for `EventKind::TransportDelivered` to track when items arrive at their destination.
- **Statistics aggregation:** Combine event listeners with the [Statistics module](../modules/stats.md) for rolling averages and throughput tracking.
