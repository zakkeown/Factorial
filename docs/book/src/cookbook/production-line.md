# Build a Multi-Step Production Line

**Goal:** Chain three buildings (mine, smelter, assembler) into a multi-step production line with a speed modifier on the final stage.
**Prerequisites:** [The Production Graph](../core-concepts/production-graph.md), [Processors](../core-concepts/processors.md), [Transport Strategies](../core-concepts/transport.md)
**Example:** `crates/factorial-core/examples/production_chain.rs`

## Steps

### 1. Create and connect a three-node chain

```rust
let p_mine = engine.graph.queue_add_node(BuildingTypeId(0));
let p_smelter = engine.graph.queue_add_node(BuildingTypeId(1));
let p_assembler = engine.graph.queue_add_node(BuildingTypeId(2));
let r = engine.graph.apply_mutations();
let mine = r.resolve_node(p_mine).unwrap();
let smelter = r.resolve_node(p_smelter).unwrap();
let assembler = r.resolve_node(p_assembler).unwrap();

let p_belt1 = engine.graph.queue_connect(mine, smelter);
let p_belt2 = engine.graph.queue_connect(smelter, assembler);
let r = engine.graph.apply_mutations();
let belt1 = r.resolve_edge(p_belt1).unwrap();
let belt2 = r.resolve_edge(p_belt2).unwrap();
```

Three [nodes](../introduction/glossary.md#node) are created and linked by two [edges](../introduction/glossary.md#edge) forming a linear [production graph](../introduction/glossary.md#production-graph): mine -> smelter -> assembler.

### 2. Configure each processor

```rust
// Mine: produces 3 iron ore per tick.
engine.set_processor(mine, Processor::Source(SourceProcessor {
    output_type: ItemTypeId(0),
    base_rate: Fixed64::from_num(3),
    depletion: Depletion::Infinite,
    accumulated: Fixed64::from_num(0),
    initial_properties: None,
}));

// Smelter: 1 iron ore -> 1 iron plate, 3 ticks.
engine.set_processor(smelter, Processor::Fixed(FixedRecipe {
    inputs: vec![RecipeInput { item_type: ItemTypeId(0), quantity: 1 }],
    outputs: vec![RecipeOutput { item_type: ItemTypeId(1), quantity: 1 }],
    duration: 3,
}));

// Assembler: 2 iron plates -> 1 iron gear, 5 ticks.
engine.set_processor(assembler, Processor::Fixed(FixedRecipe {
    inputs: vec![RecipeInput { item_type: ItemTypeId(1), quantity: 2 }],
    outputs: vec![RecipeOutput { item_type: ItemTypeId(2), quantity: 1 }],
    duration: 5,
}));
```

Each [processor](../introduction/glossary.md#processor) defines its own recipe. The intermediate product ([ItemTypeId](../introduction/glossary.md#item-type-id) 1, iron plate) flows from the smelter's output to the assembler's input.

### 3. Apply a speed modifier

```rust
engine.set_modifiers(
    assembler,
    vec![Modifier {
        id: ModifierId(0),
        kind: ModifierKind::Speed(Fixed64::from_num(1.5)),
        stacking: StackingRule::default(),
    }],
);
```

A 1.5x speed [modifier](../introduction/glossary.md#modifier) reduces the assembler's effective crafting duration. With a base of 5 ticks, the assembler now finishes each gear faster.

### 4. Set up inventories and transports

```rust
for node in [mine, smelter, assembler] {
    engine.set_input_inventory(node, Inventory::new(1, 1, 100));
    engine.set_output_inventory(node, Inventory::new(1, 1, 100));
}

let flow = |rate: f64| Transport::Flow(FlowTransport {
    rate: Fixed64::from_num(rate),
    buffer_capacity: Fixed64::from_num(100),
    latency: 0,
});
engine.set_transport(belt1, flow(5.0));
engine.set_transport(belt2, flow(5.0));
```

### 5. Run and observe

```rust
for tick in 0..30 {
    engine.step();
    if (tick + 1) % 5 == 0 {
        for snap in engine.snapshot_all_nodes() {
            // inspect snap.processor_state, snap.input_contents, snap.output_contents
        }
    }
}
```

## What's Happening

The engine evaluates the chain in topological order each [tick](../introduction/glossary.md#tick). The mine produces ore, the smelter waits until ore arrives in its input [inventory](../introduction/glossary.md#inventory), then begins a 3-tick crafting cycle to produce plates. Plates flow to the assembler, which requires 2 plates per gear. The speed modifier accelerates the assembler, so it does not become the bottleneck even though its base duration is longer. If any stage's output fills up, upstream buildings [stall](../introduction/glossary.md#stall) until space opens.

## Variations

- **Branching chains:** Use a [junction](../introduction/glossary.md#junction) (splitter) to split the smelter's output between two assemblers.
- **Productivity modifiers:** Replace `ModifierKind::Speed` with `ModifierKind::Productivity` to produce bonus outputs at the cost of slower cycles.
- **Longer chains:** Add more stages (e.g., gear -> engine -> car) by repeating the pattern of node, edge, processor, and inventory setup.
- **Different transports per stage:** Use `Flow` for high-throughput early stages and `Batch` for later stages. See [Choose the Right Transport Strategy](./transport-strategies.md).
