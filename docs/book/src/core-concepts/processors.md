# Processors

A [processor](../introduction/glossary.md#processor) is the logic attached to a
[node](../introduction/glossary.md#node) that defines what a building *does* each
[tick](../introduction/glossary.md#tick). Processors consume items from the node's input
[inventory](../introduction/glossary.md#inventory), transform them according to a rule,
and produce items into the output inventory.

Factorial uses **enum dispatch** (not trait objects) for processors. This means all five
processor types are variants of a single `Processor` enum, giving predictable branch
prediction, no vtable overhead, and sized inline storage.

## The five processor types

### Source

Generates items from nothing. Models mines, extractors, and wells.

| Field | Type | Description |
|---|---|---|
| `output_type` | `ItemTypeId` | The item type produced |
| `base_rate` | `Fixed64` | Items produced per tick before modifiers |
| `depletion` | `Depletion` | How the source depletes over time |
| `accumulated` | `Fixed64` | Fractional production accumulator |
| `initial_properties` | `Option<BTreeMap<PropertyId, Fixed64>>` | Properties stamped onto produced items |

The `Depletion` enum controls resource lifetime:

- **`Infinite`** -- never runs out.
- **`Finite { remaining }`** -- fixed amount; source [stalls](../introduction/glossary.md#stall) with `Depleted` when exhausted.
- **`Decaying { half_life }`** -- production rate decays exponentially with the given half-life in ticks.

Fractional rates work correctly: a `base_rate` of `0.5` produces one item every two ticks by accumulating the fractional remainder.

### Fixed

Consumes a fixed set of inputs and produces a fixed set of outputs after a fixed number
of ticks. Models assemblers, smelters, and chemical plants.

| Field | Type | Description |
|---|---|---|
| `inputs` | `Vec<RecipeInput>` | Required input items and quantities |
| `outputs` | `Vec<RecipeOutput>` | Produced output items and quantities |
| `duration` | `u32` | Base ticks per crafting cycle (before speed modifiers) |

A crafting cycle works as follows:

1. Check that all inputs are available and output has space.
2. Consume inputs (adjusted by the Efficiency modifier).
3. Transition to `Working { progress }` for `duration` ticks.
4. After `duration` ticks, emit outputs (boosted by the Productivity modifier).

### Property

Transforms a property on items passing through. Models heating, cooling, and refining.

| Field | Type | Description |
|---|---|---|
| `input_type` | `ItemTypeId` | Item type consumed |
| `output_type` | `ItemTypeId` | Item type produced (can differ from input) |
| `transform` | `PropertyTransform` | The transformation applied |

`PropertyTransform` supports three operations:

- **`Set(property_id, value)`** -- set a property to an absolute value.
- **`Add(property_id, delta)`** -- add a delta to a property.
- **`Multiply(property_id, factor)`** -- multiply a property by a factor.

Property processors operate at full throughput (limited only by input availability and output space) with no crafting delay.

### Demand

Consumes items at a steady rate. Models sinks, consumers, and research labs. Works
like a Source in reverse.

| Field | Type | Description |
|---|---|---|
| `input_type` | `ItemTypeId` | Primary item type consumed |
| `base_rate` | `Fixed64` | Items consumed per tick before modifiers |
| `accumulated` | `Fixed64` | Fractional consumption accumulator |
| `consumed_total` | `u64` | Lifetime count of whole items consumed |
| `accepted_types` | `Option<Vec<ItemTypeId>>` | Optional list of accepted types (multi-type mode) |

When `accepted_types` is `Some`, the processor consumes from any matching type in the
input inventory, in list order. When `None`, it falls back to `input_type` only.

### Passthrough

Passes all items from input to output unchanged. Used for
[junction](../introduction/glossary.md#junction) nodes (splitters, mergers, balancers).
Has no configuration fields.

```rust
engine.set_processor(splitter_node, Processor::Passthrough);
```

## Processor state

Every processor has a runtime `ProcessorState` that tracks what the processor is
currently doing:

- **`Idle`** -- not working, waiting to start a new cycle.
- **`Working { progress: u32 }`** -- actively processing, `progress` counts ticks since the cycle began.
- **`Stalled { reason: StallReason }`** -- cannot make progress.

### Stall reasons

| Reason | When it occurs |
|---|---|
| `MissingInputs` | Input inventory does not have enough items to start a recipe |
| `OutputFull` | Output inventory is at capacity; no room for products |
| `NoPower` | Power module reports insufficient supply |
| `Depleted` | Source processor's finite resource is exhausted |

A [stalled](../introduction/glossary.md#stall) processor automatically resumes once the
blocking condition clears (e.g., items arrive or output space opens up).

## Modifiers

[Modifiers](../introduction/glossary.md#modifier) adjust a processor's behavior. Each
modifier has a `ModifierKind` and a `StackingRule`.

### Modifier kinds

| Kind | Effect | Example |
|---|---|---|
| `Speed(factor)` | Multiplies effective speed (reduces crafting duration). `2.0` = twice as fast. | `ModifierKind::Speed(Fixed64::from_num(1.5))` |
| `Productivity(factor)` | Bonus output multiplier. `0.1` = +10% extra output. | `ModifierKind::Productivity(Fixed64::from_num(1.1))` |
| `Efficiency(factor)` | Reduces input consumption. `0.8` = uses 80% of base inputs. | `ModifierKind::Efficiency(Fixed64::from_num(0.8))` |

### Stacking rules

When multiple modifiers of the same kind are applied to a single processor, the
`StackingRule` determines how they combine:

| Rule | Behavior |
|---|---|
| `Multiplicative` (default) | Each modifier multiplies the previous result. Two 1.5x speed mods = 2.25x. |
| `Additive` | Deltas are summed, then applied. Two 1.5x speed mods = 1.0 + 0.5 + 0.5 = 2.0x. |
| `Diminishing` | Each successive modifier's effect is halved. A 2.0x mod becomes 1.5x. |
| `Capped` | Only the strongest modifier applies. A 1.5x and 2.0x = 2.0x. |

Modifiers are sorted by `ModifierId` before application (canonical order), ensuring
deterministic results regardless of insertion order.

## Example: production chain with modifiers

```rust
// From crates/factorial-core/examples/production_chain.rs

// Smelter: 1 iron ore -> 1 iron plate, 3 ticks.
engine.set_processor(
    smelter,
    Processor::Fixed(FixedRecipe {
        inputs: vec![RecipeInput {
            item_type: ItemTypeId(0), // iron ore
            quantity: 1,
        }],
        outputs: vec![RecipeOutput {
            item_type: ItemTypeId(1), // iron plate
            quantity: 1,
        }],
        duration: 3,
    }),
);

// Apply a 1.5x speed modifier to the assembler.
engine.set_modifiers(
    assembler,
    vec![Modifier {
        id: ModifierId(0),
        kind: ModifierKind::Speed(Fixed64::from_num(1.5)),
        stacking: StackingRule::default(),
    }],
);
```

With the 1.5x speed modifier applied to a recipe with `duration: 5`, the effective
duration becomes `ceil(5 / 1.5) = 4` ticks.

For a deep dive into writing your own processor behaviors and advanced modifier
configurations, see the [Custom Processors](../guides/custom-processors.md) guide.
