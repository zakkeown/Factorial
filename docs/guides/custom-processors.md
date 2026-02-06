# Custom Processors Guide

This guide explains the four processor types available in the Factorial engine,
how to configure them, and how to use modifiers and stacking rules to tune
building behavior.

## Processor Overview

Every node (building) in the production graph has an associated `Processor` that
defines what the building does each tick. The engine uses **enum dispatch** --
there are four fixed variants, not trait objects:

| Variant      | Purpose                                      | Example buildings             |
|-------------|----------------------------------------------|-------------------------------|
| `Source`     | Produces items from nothing                  | Mines, extractors, wells      |
| `Fixed`      | Consumes inputs, produces outputs over time  | Assemblers, smelters, plants  |
| `Property`   | Transforms a property on items passing through | Heaters, coolers, refiners   |
| `Demand`     | Consumes items at a steady rate              | Sinks, research labs          |

## ProcessorState Lifecycle

Every processor has an associated `ProcessorState` stored in SoA storage:

```
Idle --> Working { progress } --> Idle
  |                                |
  +--> Stalled { reason } <--------+
```

- **Idle**: No crafting in progress. The processor attempts to start a new cycle.
- **Working { progress }**: Crafting is in progress. `progress` increments each
  tick until it reaches the effective duration, then outputs are emitted and
  the state returns to Idle.
- **Stalled { reason }**: The processor cannot make progress. Reasons:
  - `MissingInputs` -- not enough items in the input inventory
  - `OutputFull` -- the output inventory has no space
  - `NoPower` -- power network not satisfied (future integration)
  - `Depleted` -- source has run out (finite depletion only)

A stalled processor automatically retries each tick. When the blocking condition
clears, it transitions back to Idle or directly to Working.

## Source Processor

Produces items from nothing. Models mines, extractors, and infinite wells.

### Fields

| Field         | Type        | Description                                          |
|--------------|-------------|------------------------------------------------------|
| `output_type` | `ItemTypeId` | Which item type to produce                          |
| `base_rate`   | `Fixed64`   | Items per tick before modifiers (fractional OK)     |
| `depletion`   | `Depletion` | How the source depletes over time                   |
| `accumulated` | `Fixed64`   | Internal fractional accumulator (start at 0)        |

### Depletion Models

- `Depletion::Infinite` -- never runs out
- `Depletion::Finite { remaining: Fixed64 }` -- produces until `remaining` hits 0, then stalls with `Depleted`
- `Depletion::Decaying { half_life: u64 }` -- production rate decays exponentially

### Example

```rust
use factorial_core::processor::*;
use factorial_core::fixed::Fixed64;
use factorial_core::id::ItemTypeId;

// Iron mine: 2 ore per tick, infinite supply.
let mine = Processor::Source(SourceProcessor {
    output_type: ItemTypeId(0),
    base_rate: Fixed64::from_num(2),
    depletion: Depletion::Infinite,
    accumulated: Fixed64::from_num(0),
});

// Oil well: 0.5 barrels per tick (produces 1 every 2 ticks), 1000 remaining.
let oil_well = Processor::Source(SourceProcessor {
    output_type: ItemTypeId(10),
    base_rate: Fixed64::from_num(0.5),
    depletion: Depletion::Finite {
        remaining: Fixed64::from_num(1000),
    },
    accumulated: Fixed64::from_num(0),
});
```

### Fractional Accumulation

When `base_rate` is less than 1 (e.g. 0.5), the source accumulates fractional
production each tick. When the accumulator reaches 1.0 or higher, whole items
are emitted and the fractional remainder carries forward. This enables precise
sub-1 production rates without rounding issues.

## Fixed Recipe Processor

Consumes a fixed set of inputs and produces a fixed set of outputs after a
fixed number of ticks. This is the workhorse processor for assemblers,
smelters, and chemical plants.

### Fields

| Field      | Type              | Description                              |
|-----------|-------------------|------------------------------------------|
| `inputs`   | `Vec<RecipeInput>` | Required input items and quantities     |
| `outputs`  | `Vec<RecipeOutput>` | Produced output items and quantities   |
| `duration` | `u32`             | Base ticks per crafting cycle           |

### Crafting Cycle

1. **Check outputs** -- ensure the output inventory has room for all outputs.
2. **Check inputs** -- ensure all required inputs are available (after applying
   efficiency modifiers).
3. **Consume inputs** -- remove items from the input inventory.
4. **Work** -- increment progress each tick.
5. **Produce** -- when `progress >= effective_duration`, emit outputs (with
   productivity bonus) and return to Idle.

### Example

```rust
use factorial_core::processor::*;
use factorial_core::id::ItemTypeId;

// Assembler: 2 iron plates + 1 copper wire -> 1 circuit board, 8 ticks.
let assembler = Processor::Fixed(FixedRecipe {
    inputs: vec![
        RecipeInput { item_type: ItemTypeId(1), quantity: 2 },  // iron plate
        RecipeInput { item_type: ItemTypeId(3), quantity: 1 },  // copper wire
    ],
    outputs: vec![
        RecipeOutput { item_type: ItemTypeId(4), quantity: 1 }, // circuit board
    ],
    duration: 8,
});

// Smelter: 1 iron ore -> 1 iron plate, 3 ticks.
let smelter = Processor::Fixed(FixedRecipe {
    inputs: vec![
        RecipeInput { item_type: ItemTypeId(0), quantity: 1 },
    ],
    outputs: vec![
        RecipeOutput { item_type: ItemTypeId(1), quantity: 1 },
    ],
    duration: 3,
});
```

## Property Processor

Transforms a property on items passing through. One item is processed per tick.
The actual property value mutation is handled at the engine level; the processor
signals which items to consume and produce.

### Fields

| Field         | Type               | Description                            |
|--------------|---------------------|----------------------------------------|
| `input_type`  | `ItemTypeId`        | Items consumed                         |
| `output_type` | `ItemTypeId`        | Items produced (can be same type)      |
| `transform`   | `PropertyTransform` | What property change to apply          |

### PropertyTransform Variants

- `Set(PropertyId, Fixed64)` -- set a property to an absolute value
- `Add(PropertyId, Fixed64)` -- add a delta to a property
- `Multiply(PropertyId, Fixed64)` -- multiply a property by a factor

### Example

```rust
use factorial_core::processor::*;
use factorial_core::fixed::Fixed64;
use factorial_core::id::{ItemTypeId, PropertyId};

// Heater: heats iron ore (type 0) into hot iron ore (type 5),
// setting temperature property to 1500.
let heater = Processor::Property(PropertyProcessor {
    input_type: ItemTypeId(0),
    output_type: ItemTypeId(5),
    transform: PropertyTransform::Set(PropertyId(0), Fixed64::from_num(1500)),
});
```

## Demand Processor

Consumes items at a steady rate, like Source in reverse. Models sinks,
consumers, and research labs.

### Fields

| Field         | Type        | Description                                      |
|--------------|-------------|--------------------------------------------------|
| `input_type`  | `ItemTypeId` | Which item type to consume                      |
| `base_rate`   | `Fixed64`   | Items consumed per tick before modifiers         |
| `accumulated` | `Fixed64`   | Internal fractional accumulator (start at 0)     |

### Example

```rust
use factorial_core::processor::*;
use factorial_core::fixed::Fixed64;
use factorial_core::id::ItemTypeId;

// Research lab: consumes 1 science pack per tick.
let lab = Processor::Demand(DemandProcessor {
    input_type: ItemTypeId(20),
    base_rate: Fixed64::from_num(1),
    accumulated: Fixed64::from_num(0),
});

// Sink: slowly consumes excess iron at 0.25 per tick.
let sink = Processor::Demand(DemandProcessor {
    input_type: ItemTypeId(0),
    base_rate: Fixed64::from_num(0.25),
    accumulated: Fixed64::from_num(0),
});
```

## Modifiers

Modifiers adjust processor behavior without changing the recipe. They are stored
per-node and applied during each tick. Modifiers are sorted by `ModifierId` for
deterministic canonical ordering before being folded.

### ModifierKind

| Kind            | Effect                                           | Value meaning              |
|----------------|--------------------------------------------------|----------------------------|
| `Speed(v)`      | Multiplies effective speed (reduces duration)    | 2.0 = twice as fast       |
| `Productivity(v)` | Bonus output multiplier                       | 0.1 = +10% extra output   |
| `Efficiency(v)` | Reduces input consumption                        | 0.8 = uses 80% inputs     |

### How Modifiers Are Applied

- **Speed**: `effective_duration = ceil(base_duration / speed_multiplier)`, minimum 1 tick.
- **Productivity**: `output_quantity = floor(base_quantity * productivity_multiplier)`, minimum 1.
- **Efficiency**: `input_quantity = ceil(base_quantity * efficiency_multiplier)`, minimum 1.

For Source and Demand processors, Speed affects the effective rate:
`effective_rate = base_rate * speed_multiplier`.

### Example

```rust
use factorial_core::processor::*;
use factorial_core::fixed::Fixed64;
use factorial_core::id::ModifierId;

// Speed beacon: 50% speed bonus.
let speed_mod = Modifier {
    id: ModifierId(0),
    kind: ModifierKind::Speed(Fixed64::from_num(1.5)),
    stacking: StackingRule::default(), // Multiplicative
};

// Productivity module: +10% bonus output.
let prod_mod = Modifier {
    id: ModifierId(1),
    kind: ModifierKind::Productivity(Fixed64::from_num(1.1)),
    stacking: StackingRule::default(),
};

// Efficiency module: uses only 80% of inputs.
let eff_mod = Modifier {
    id: ModifierId(2),
    kind: ModifierKind::Efficiency(Fixed64::from_num(0.8)),
    stacking: StackingRule::default(),
};
```

Set modifiers on a node via the engine:

```rust
engine.set_modifiers(node_id, vec![speed_mod, prod_mod, eff_mod]);
```

## Stacking Rules

When multiple modifiers of the same kind are applied, the `StackingRule`
determines how they combine.

### StackingRule Variants

| Rule             | Behavior                                                    | Example (two 1.5x speed mods)      |
|-----------------|-------------------------------------------------------------|-------------------------------------|
| `Multiplicative` | Each modifier multiplies the previous result (default)     | 1.0 * 1.5 * 1.5 = 2.25            |
| `Additive`       | Deltas are summed, then applied as a single multiplier     | 1.0 + (0.5) + (0.5) = 2.0         |
| `Diminishing`    | Each successive modifier has halved effect                  | 1.0 * (1 + 0.5/2) = 1.25 each     |
| `Capped`         | Only the strongest modifier applies                         | max(1.5, 1.5) = 1.5               |

### Canonical Ordering

Modifiers are always sorted by `ModifierId` before folding. This guarantees
deterministic results regardless of insertion order. Two sets of identical
modifiers will always produce the same effective multipliers.

### Example: Mixing Stacking Rules

```rust
use factorial_core::processor::*;
use factorial_core::fixed::Fixed64;
use factorial_core::id::ModifierId;

let mods = vec![
    // First speed beacon: 1.5x, multiplicative.
    Modifier {
        id: ModifierId(0),
        kind: ModifierKind::Speed(Fixed64::from_num(1.5)),
        stacking: StackingRule::Multiplicative,
    },
    // Second speed beacon: 1.5x, additive stacking.
    Modifier {
        id: ModifierId(1),
        kind: ModifierKind::Speed(Fixed64::from_num(1.5)),
        stacking: StackingRule::Additive,
    },
];

// Result: ModifierId(0) is processed first (Multiplicative: 1.0 * 1.5 = 1.5),
// then ModifierId(1) (Additive: 1.5 + (1.5 - 1.0) = 2.0).
// Final speed multiplier: 2.0
```

## Putting It All Together

A complete setup for a node with a processor and modifiers:

```rust
use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::item::Inventory;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;

let mut engine = Engine::new(SimulationStrategy::Tick);

// Create a node.
let pending = engine.graph.queue_add_node(BuildingTypeId(0));
let result = engine.graph.apply_mutations();
let node = result.resolve_node(pending).unwrap();

// Configure as an assembler with a recipe.
engine.set_processor(node, Processor::Fixed(FixedRecipe {
    inputs: vec![RecipeInput { item_type: ItemTypeId(0), quantity: 2 }],
    outputs: vec![RecipeOutput { item_type: ItemTypeId(1), quantity: 1 }],
    duration: 10,
}));

// Give it inventories.
engine.set_input_inventory(node, Inventory::new(1, 1, 100));
engine.set_output_inventory(node, Inventory::new(1, 1, 100));

// Add a speed module.
engine.set_modifiers(node, vec![
    Modifier {
        id: ModifierId(0),
        kind: ModifierKind::Speed(Fixed64::from_num(2.0)),
        stacking: StackingRule::Multiplicative,
    },
]);

// Run the simulation.
for _ in 0..20 {
    engine.step();
}

// Query state.
if let Some(state) = engine.get_processor_state(node) {
    println!("Processor state: {:?}", state);
}
```
