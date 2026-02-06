# Model a Smelting Chain

**Goal:** Create a mine that produces ore and an assembler that smelts it into plates, connected by a transport belt.
**Prerequisites:** [The Production Graph](../core-concepts/production-graph.md), [Processors](../core-concepts/processors.md), [Transport Strategies](../core-concepts/transport.md)
**Example:** `crates/factorial-core/examples/minimal_factory.rs`

## Steps

### 1. Create the engine and add nodes

```rust
let mut engine = Engine::new(SimulationStrategy::Tick);

let pending_mine = engine.graph.queue_add_node(BuildingTypeId(0));
let pending_assembler = engine.graph.queue_add_node(BuildingTypeId(1));
let result = engine.graph.apply_mutations();

let mine = result.resolve_node(pending_mine).expect("mine node created");
let assembler = result.resolve_node(pending_assembler).expect("assembler node created");
```

Each [node](../introduction/glossary.md#node) is queued and then applied in batch so the [production graph](../introduction/glossary.md#production-graph) stays consistent. `apply_mutations()` returns a result you use to resolve pending operations into actual `NodeId` values.

### 2. Connect the nodes with an edge

```rust
let pending_belt = engine.graph.queue_connect(mine, assembler);
let result = engine.graph.apply_mutations();
let belt = result.resolve_edge(pending_belt).expect("belt edge created");
```

This creates an [edge](../introduction/glossary.md#edge) from the mine's output to the assembler's input.

### 3. Configure processors

```rust
engine.set_processor(
    mine,
    Processor::Source(SourceProcessor {
        output_type: ItemTypeId(0), // iron ore
        base_rate: Fixed64::from_num(2),
        depletion: Depletion::Infinite,
        accumulated: Fixed64::from_num(0),
        initial_properties: None,
    }),
);

engine.set_processor(
    assembler,
    Processor::Fixed(FixedRecipe {
        inputs: vec![RecipeInput { item_type: ItemTypeId(0), quantity: 2 }],
        outputs: vec![RecipeOutput { item_type: ItemTypeId(1), quantity: 1 }],
        duration: 5,
    }),
);
```

The mine uses a `Source` [processor](../introduction/glossary.md#processor) that generates 2 iron ore per [tick](../introduction/glossary.md#tick). The assembler uses a `Fixed` processor that consumes 2 ore and produces 1 iron gear over 5 ticks.

### 4. Assign inventories and transport

```rust
engine.set_input_inventory(mine, Inventory::new(1, 1, 100));
engine.set_output_inventory(mine, Inventory::new(1, 1, 100));
engine.set_input_inventory(assembler, Inventory::new(1, 1, 100));
engine.set_output_inventory(assembler, Inventory::new(1, 1, 100));

engine.set_transport(
    belt,
    Transport::Flow(FlowTransport {
        rate: Fixed64::from_num(5),
        buffer_capacity: Fixed64::from_num(100),
        latency: 0,
    }),
);
```

Each node needs an [inventory](../introduction/glossary.md#inventory) for input and output storage. The [transport strategy](../introduction/glossary.md#transport-strategy) on the edge determines throughput -- here a `Flow` transport moves up to 5 items per tick.

### 5. Run the simulation

```rust
for tick in 0..10 {
    let result = engine.step();
    let snapshots = engine.snapshot_all_nodes();
    // inspect snapshots for each node's state, progress, and inventory contents
}
```

## What's Happening

Each tick, the engine evaluates the production graph in topological order. The mine produces ore into its output inventory. The `Flow` transport moves items from the mine's output to the assembler's input at the configured rate. The assembler checks its input inventory for the required 2 ore, and when available, begins crafting. After 5 ticks of crafting, 1 iron gear appears in the assembler's output. If the assembler's input runs dry, it [stalls](../introduction/glossary.md#stall) with `MissingInputs` until more ore arrives.

## Variations

- **Finite ore deposits:** Change `Depletion::Infinite` to `Depletion::Finite { remaining: Fixed64::from_num(500) }` to model a mine that runs out.
- **Faster belts:** Increase `FlowTransport::rate` to raise throughput, or switch to a `Batch` transport for bulk delivery.
- **Multiple outputs:** Chain another processor after the assembler to continue the production line. See [Build a Multi-Step Production Line](./production-line.md).
- **Speed modifiers:** Attach a [modifier](../introduction/glossary.md#modifier) to the assembler to speed up crafting. See [Build a Multi-Step Production Line](./production-line.md).
