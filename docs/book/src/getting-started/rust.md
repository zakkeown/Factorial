# Rust Quick Start

This guide walks through building a minimal factory simulation in Rust: an iron mine feeding an assembler via a transport belt. By the end you will have a running loop that produces iron gears from iron ore.

## 1. Add the dependency

```bash
cargo add factorial-core
```

Or add it directly to your `Cargo.toml`:

```toml
[dependencies]
factorial-core = "0.1"
```

## 2. Create an engine

The [Engine](../introduction/glossary.md#production-graph) is the top-level object that owns the [production graph](../introduction/glossary.md#production-graph), simulation state, and event system. Create one with a [tick](../introduction/glossary.md#tick)-based simulation strategy:

```rust
use factorial_core::engine::Engine;
use factorial_core::sim::SimulationStrategy;

let mut engine = Engine::new(SimulationStrategy::Tick);
```

`SimulationStrategy::Tick` advances exactly one discrete step per call to `engine.step()`.

## 3. Add nodes

[Nodes](../introduction/glossary.md#node) represent buildings in the production graph. Graph mutations are queued and applied in batch to preserve [determinism](../introduction/glossary.md#fixed-point):

```rust
use factorial_core::id::*;

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

[`BuildingTypeId`](../introduction/glossary.md#building-type-id) is a numeric identifier that maps to your game's [registry](../introduction/glossary.md#registry) of building definitions. The engine does not interpret it -- it is your label.

## 4. Connect them

[Edges](../introduction/glossary.md#edge) carry items between nodes. Like node additions, connections are queued and resolved after `apply_mutations`:

```rust
let pending_belt = engine.graph.queue_connect(mine, assembler);
let result = engine.graph.apply_mutations();
let belt = result
    .resolve_edge(pending_belt)
    .expect("belt edge created");
```

## 5. Configure processors

A [processor](../introduction/glossary.md#processor) defines what a node does each tick. The mine is a **Source** (generates items); the assembler is a **Fixed** recipe (transforms inputs into outputs):

```rust
use factorial_core::fixed::Fixed64;
use factorial_core::processor::*;

// Mine: produces 2 iron ore per tick, infinite supply.
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

// Assembler: 2 iron ore -> 1 iron gear, takes 5 ticks.
engine.set_processor(
    assembler,
    Processor::Fixed(FixedRecipe {
        inputs: vec![RecipeInput {
            item_type: ItemTypeId(0),
            quantity: 2,
        }],
        outputs: vec![RecipeOutput {
            item_type: ItemTypeId(1), // iron gear
            quantity: 1,
        }],
        duration: 5,
    }),
);
```

All numeric quantities use [`Fixed64`](../introduction/glossary.md#fixed-point) (Q32.32 fixed-point) to guarantee cross-platform determinism.

## 6. Set up inventories

Each node needs input and output [inventories](../introduction/glossary.md#inventory) to hold items between ticks:

```rust
use factorial_core::item::Inventory;

// Inventory::new(input_slot_count, output_slot_count, capacity_per_slot)
engine.set_input_inventory(mine, Inventory::new(1, 1, 100));
engine.set_output_inventory(mine, Inventory::new(1, 1, 100));
engine.set_input_inventory(assembler, Inventory::new(1, 1, 100));
engine.set_output_inventory(assembler, Inventory::new(1, 1, 100));
```

## 7. Configure transport

The edge needs a [transport strategy](../introduction/glossary.md#transport-strategy) to move items from the mine's output inventory to the assembler's input inventory. A **Flow** transport moves items at a continuous rate:

```rust
use factorial_core::transport::*;

engine.set_transport(
    belt,
    Transport::Flow(FlowTransport {
        rate: Fixed64::from_num(5), // 5 items/tick throughput
        buffer_capacity: Fixed64::from_num(100),
        latency: 0,
    }),
);
```

## 8. Run the simulation

Call `engine.step()` each tick. The engine evaluates all nodes in topological order, runs processors, moves items through transports, and emits events:

```rust
for tick in 0..10 {
    let result = engine.step();

    println!(
        "=== Tick {} (steps run: {}) ===",
        tick + 1,
        result.steps_run
    );
}
```

## 9. Query state

After each step, query the engine for rendering or debugging. [Snapshots](../introduction/glossary.md#snapshot) are cheap read-only views of node state:

```rust
let snapshots = engine.snapshot_all_nodes();

for snap in &snapshots {
    println!(
        "  Node {:?} (building {:?}): state={:?}, progress={:.2}",
        snap.id, snap.building_type, snap.processor_state, snap.progress
    );
    if !snap.input_contents.is_empty() {
        println!("    Input:  {:?}", snap.input_contents);
    }
    if !snap.output_contents.is_empty() {
        println!("    Output: {:?}", snap.output_contents);
    }
}
```

You can also query individual properties:

- `engine.get_processor_state(node_id)` -- returns the current `ProcessorState` (Idle, Working, or [Stalled](../introduction/glossary.md#stall)).
- `engine.state_hash()` -- returns a deterministic [state hash](../introduction/glossary.md#state-hash) for desync detection.

```rust
println!("Final state hash: {}", engine.state_hash());
```

## What to explore next

- **More processor types** -- Demand, Property, Passthrough. See [Processors](../core-concepts/processors.md).
- **Other transport strategies** -- Item, Batch, Vehicle. See [Transport Strategies](../core-concepts/transport.md).
- **Events** -- react to production changes in real time. See [Events](../core-concepts/events.md).
- **Serialization** -- save and load engine state. See [Serialization](../core-concepts/serialization.md).

*Full source: `crates/factorial-core/examples/minimal_factory.rs`*
