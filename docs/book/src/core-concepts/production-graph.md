# The Production Graph

The [production graph](../introduction/glossary.md#production-graph) is the central data
structure in Factorial. It is a **directed graph** where
[nodes](../introduction/glossary.md#node) represent buildings and
[edges](../introduction/glossary.md#edge) represent transport connections between them.
Every frame, the engine evaluates the graph in
[topological order](../introduction/glossary.md#tick) to move items through the factory.

```text
            Edge (belt)          Edge (belt)
  [Mine] ----------------> [Smelter] ----------------> [Assembler]
  Node 0                   Node 1                      Node 2
  Source                   Fixed recipe                Fixed recipe
  (iron ore)               ore -> plate                plate -> gear
```

## Graph structure

The graph is stored in the `ProductionGraph` struct. It uses **SlotMap** arenas for
both nodes and edges, giving O(1) insert, remove, and lookup. Each node carries a
`BuildingTypeId` that ties it back to the game's
[registry](../introduction/glossary.md#registry). Each edge stores the source node,
destination node, and an optional `ItemTypeId` filter.

Adjacency is tracked per-node via `get_inputs()` (incoming edges) and `get_outputs()`
(outgoing edges), so you can walk the graph in either direction without a full scan.

## Creating nodes

Graph mutations are **queued**, not applied immediately. This is critical for
[determinism](../introduction/glossary.md#tick) -- all mutations from a single tick are
applied atomically at the start of the next tick.

The three-step pattern:

1. **Queue** the operation -- returns a `PendingNodeId`.
2. **Apply** all queued mutations in batch.
3. **Resolve** the pending ID to a real `NodeId`.

```rust
// From crates/factorial-core/examples/minimal_factory.rs

// Queue node additions (applied in batch for determinism).
let pending_mine = engine.graph.queue_add_node(BuildingTypeId(0));
let pending_assembler = engine.graph.queue_add_node(BuildingTypeId(1));
let result = engine.graph.apply_mutations();

// Resolve pending operations to get actual NodeIds.
let mine = result
    .resolve_node(pending_mine)
    .expect("mine node created");
let assembler = result
    .resolve_node(pending_assembler)
    .expect("assembler node created");
```

After resolving a `NodeId`, configure the node's
[processor](../introduction/glossary.md#processor),
[inventories](../introduction/glossary.md#inventory), and
[modifiers](../introduction/glossary.md#modifier) using `Engine` setter methods:

```rust
engine.set_processor(mine, Processor::Source(/* ... */));
engine.set_input_inventory(mine, Inventory::new(1, 1, 100));
engine.set_output_inventory(mine, Inventory::new(1, 1, 100));
```

## Connecting nodes

Edges work the same way -- queue, apply, resolve:

```rust
// From crates/factorial-core/examples/minimal_factory.rs

let pending_belt = engine.graph.queue_connect(mine, assembler);
let result = engine.graph.apply_mutations();
let belt = result
    .resolve_edge(pending_belt)
    .expect("belt edge created");
```

You can also create filtered edges that only carry a specific item type:

```rust
let pending = engine.graph.queue_connect_filtered(
    source_node,
    dest_node,
    Some(ItemTypeId(0)), // only iron ore
);
```

After resolving the edge, assign a [transport strategy](../introduction/glossary.md#transport-strategy):

```rust
engine.set_transport(
    belt,
    Transport::Flow(FlowTransport {
        rate: Fixed64::from_num(5),
        buffer_capacity: Fixed64::from_num(100),
        latency: 0,
    }),
);
```

## Removing nodes and edges

Removal follows the same queue-then-apply pattern:

```rust
// Remove a single edge.
engine.graph.queue_disconnect(edge_id);
engine.graph.apply_mutations();

// Remove a node (also removes all connected edges).
engine.graph.queue_remove_node(node_id);
engine.graph.apply_mutations();
```

Removing a node automatically cleans up every edge that connects to it, so you
do not need to disconnect edges manually before removing a node.

## Junctions

A [junction](../introduction/glossary.md#junction) is a node that routes items without
transforming them -- splitters, mergers, and balancers. Assign a junction to a node
with `set_junction`:

```rust
engine.set_junction(node_id, Junction::Splitter {
    mode: SplitterMode::RoundRobin,
});
```

The junction's `Passthrough` processor moves items from the node's input inventory
to its output inventory unchanged. The junction configuration controls *which* output
edge receives each item.

## Topological ordering

Before each [tick](../introduction/glossary.md#tick), the engine computes a topological
order over all nodes using **Kahn's algorithm** (`O(V+E)`). This guarantees that a
node's upstream producers are always evaluated before the node itself, so inputs
are available when the processor runs.

The topological order is cached and recomputed lazily -- only when a structural
mutation (add/remove node or edge) dirties the cache. If the graph contains a cycle,
`topological_order()` returns `Err(GraphError::CycleDetected)`. For graphs with
intentional feedback loops, use `topological_order_with_feedback()`, which identifies
back-edges that carry a one-tick delay.

## Putting it together

A complete graph-building sequence from the `minimal_factory` example:

```rust
// From crates/factorial-core/examples/minimal_factory.rs

// Create the engine with tick-based simulation.
let mut engine = Engine::new(SimulationStrategy::Tick);

// Add nodes.
let pending_mine = engine.graph.queue_add_node(BuildingTypeId(0));
let pending_assembler = engine.graph.queue_add_node(BuildingTypeId(1));
let result = engine.graph.apply_mutations();
let mine = result.resolve_node(pending_mine).expect("mine node created");
let assembler = result.resolve_node(pending_assembler).expect("assembler node created");

// Connect them.
let pending_belt = engine.graph.queue_connect(mine, assembler);
let result = engine.graph.apply_mutations();
let belt = result.resolve_edge(pending_belt).expect("belt edge created");

// Configure processors, inventories, transport.
engine.set_processor(mine, Processor::Source(SourceProcessor {
    output_type: ItemTypeId(0),
    base_rate: Fixed64::from_num(2),
    depletion: Depletion::Infinite,
    accumulated: Fixed64::from_num(0),
    initial_properties: None,
}));

engine.set_processor(assembler, Processor::Fixed(FixedRecipe {
    inputs: vec![RecipeInput { item_type: ItemTypeId(0), quantity: 2 }],
    outputs: vec![RecipeOutput { item_type: ItemTypeId(1), quantity: 1 }],
    duration: 5,
}));

for node in [mine, assembler] {
    engine.set_input_inventory(node, Inventory::new(1, 1, 100));
    engine.set_output_inventory(node, Inventory::new(1, 1, 100));
}

engine.set_transport(belt, Transport::Flow(FlowTransport {
    rate: Fixed64::from_num(5),
    buffer_capacity: Fixed64::from_num(100),
    latency: 0,
}));

// Run the simulation.
for _ in 0..10 {
    engine.step();
}
```

## Key API summary

| Operation | Method | Returns |
|---|---|---|
| Add node | `graph.queue_add_node(building_type)` | `PendingNodeId` |
| Remove node | `graph.queue_remove_node(node_id)` | -- |
| Connect | `graph.queue_connect(from, to)` | `PendingEdgeId` |
| Connect (filtered) | `graph.queue_connect_filtered(from, to, filter)` | `PendingEdgeId` |
| Disconnect | `graph.queue_disconnect(edge_id)` | -- |
| Apply all queued | `graph.apply_mutations()` | `MutationResult` |
| Resolve node | `result.resolve_node(pending)` | `Option<NodeId>` |
| Resolve edge | `result.resolve_edge(pending)` | `Option<EdgeId>` |
| Set junction | `engine.set_junction(node, junction)` | -- |
| Node count | `graph.node_count()` | `usize` |
| Edge count | `graph.edge_count()` | `usize` |
| Topo order | `graph.topological_order()` | `Result<&[NodeId], GraphError>` |
