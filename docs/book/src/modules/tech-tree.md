# Tech Trees

The `factorial-tech-tree` crate provides research systems with prerequisites,
multiple cost models, unlock tracking, and infinite (repeatable) research with
cost scaling. Technologies are registered at startup and researched at runtime
through a series of contribution calls that depend on the cost model.

## Key concepts

- A [technology](../introduction/glossary.md#technology) is an immutable
  definition registered once at startup.
- Each technology has **prerequisites** (other `TechId`s that must be completed
  first), a **cost model**, and a list of **unlocks** applied on completion.
- The module tracks runtime research state (not started / in progress /
  completed) and emits [`TechEvent`](../introduction/glossary.md#event)s on
  start and completion.
- All state is fully serializable for save/load.

## Creating the tech tree

```rust
use factorial_tech_tree::*;
use factorial_core::id::{BuildingTypeId, RecipeId};

let mut tree = TechTree::new();
```

## Registering technologies

```rust
tree.register(Technology {
    id: TechId(0),
    name: "Basic Smelting".to_string(),
    prerequisites: vec![],
    cost: ResearchCost::Points(100),
    unlocks: vec![
        Unlock::Building(BuildingTypeId(1)),
        Unlock::Recipe(RecipeId(0)),
    ],
    repeatable: false,
    cost_scaling: None,
})?;

tree.register(Technology {
    id: TechId(1),
    name: "Advanced Smelting".to_string(),
    prerequisites: vec![TechId(0)],       // requires Basic Smelting
    cost: ResearchCost::Points(200),
    unlocks: vec![
        Unlock::Building(BuildingTypeId(2)),
        Unlock::Recipe(RecipeId(1)),
    ],
    repeatable: false,
    cost_scaling: None,
})?;
```

`register()` validates that:

- The `TechId` is not already registered.
- All listed prerequisites reference previously registered technologies.

It returns `Err(TechTreeError)` on failure.

### Unlock variants

| Variant | Description |
|---------|------------|
| `Unlock::Building(BuildingTypeId)` | Makes a building type available for placement |
| `Unlock::Recipe(RecipeId)` | Makes a recipe available in processors |
| `Unlock::Custom(String)` | Opaque key interpreted by game code |

## ResearchCost variants

The module supports six cost models, each matching a real factory game's
research mechanic:

| Variant | Game analogy | How it works |
|---------|-------------|-------------|
| `Items(Vec<(ItemTypeId, u32)>)` | Factorio / DSP | Consume specific items at research buildings |
| `Points(u32)` | ONI | Accumulate science points |
| `Delivery(Vec<(ItemTypeId, u32)>)` | Satisfactory | One-time delivery of items (all-or-nothing) |
| `Rate { points_per_tick, total }` | Captain of Industry | Accumulate points at a fixed rate per tick |
| `ItemRate { item, rate, duration }` | Shapez | Deliver items at a target rate for a duration |
| `Custom(ResearchCostFnId)` | Any | Game-defined completion logic via callback ID |

## Starting research

```rust
tree.start_research(TechId(0), current_tick)?;
```

`start_research` validates prerequisites and current state, initializes
progress tracking appropriate to the cost model, and emits a
`TechEvent::ResearchStarted` event.

Errors are returned if:

- The technology does not exist (`TechNotFound`).
- A prerequisite is not yet completed (`PrerequisiteNotMet`).
- The technology is already in progress (`AlreadyInProgress`).
- The technology is completed and not repeatable (`AlreadyCompleted`).

## Contributing toward completion

### Points cost model

```rust
let consumed: u32 = tree.contribute_points(TechId(0), 60, current_tick)?;
// consumed <= 60; excess is not taken
```

Returns the number of points actually consumed. Completes research when the
target is met.

### Items / Delivery cost model

```rust
let consumed: Vec<(ItemTypeId, u32)> = tree.contribute_items(
    TechId(1),
    &[(red_science, 50), (green_science, 50)],
    current_tick,
)?;
```

Returns the amount of each item actually consumed (may be less than offered).
Completes research when all required items are met.

### Rate cost model

```rust
let completed: bool = tree.tick_rate(TechId(0), current_tick)?;
```

Call once per tick. The technology's `points_per_tick` is added automatically.
Returns `true` on the tick that completes research.

### ItemRate cost model

```rust
let completed: bool = tree.tick_item_rate(TechId(0), current_tick)?;
```

Call once per tick while the item rate requirement is being met externally.
Completes after `duration` ticks.

### Custom cost model

```rust
tree.complete_custom(TechId(0), current_tick)?;
```

Game code decides when the custom condition is met and calls this to finalize.

## Querying state

```rust
tree.is_completed(TechId(0));         // true if completed at least once
tree.is_in_progress(TechId(0));       // true if currently researching
tree.prerequisites_met(TechId(1))?;   // true if all prereqs are Completed
tree.completion_count(TechId(0));     // number of times completed (repeatable)
```

## Collecting unlocks

```rust
let all: Vec<Unlock> = tree.all_unlocks();
```

Returns every unlock from every completed technology. Game code typically calls
this after loading a save or after draining events to synchronize the unlock
state.

## Draining events

```rust
let events: Vec<TechEvent> = tree.drain_events();
```

`drain_events()` returns all pending events and clears the internal list.
Events are transient and are not serialized.

| Event | Payload |
|-------|---------|
| `ResearchStarted` | `tech_id`, `tick` |
| `ResearchCompleted` | `tech_id`, `unlocks`, `level` (1-indexed), `tick` |

## CostScaling for repeatable research

Repeatable (infinite) technologies accept an optional `CostScaling`:

### Linear

```rust
cost_scaling: Some(CostScaling::Linear {
    base: 500,
    increment: 250,
}),
```

Cost at level *n* = `base + increment * n`. Level 0 costs 500, level 1 costs
750, level 2 costs 1000, and so on.

### Exponential

```rust
cost_scaling: Some(CostScaling::Exponential {
    base: 100,
    multiplier: Fixed64::from_num(2),
}),
```

Cost at level *n* = `base * multiplier^n`. Level 0 costs 100, level 1 costs
200, level 2 costs 400, and so on.

The effective cost for the current level is available via:

```rust
let cost: ResearchCost = tree.effective_cost(TechId(2))?;
```

## Full example

The following excerpt from `crates/factorial-examples/examples/tech_tree.rs`
registers a linear tech chain plus a repeatable technology, researches them, and
lists all unlocks.

```rust
use factorial_core::id::{BuildingTypeId, RecipeId};
use factorial_tech_tree::*;

let mut tree = TechTree::new();

// Tier 1: no prerequisites
tree.register(Technology {
    id: TechId(0),
    name: "Basic Smelting".to_string(),
    prerequisites: vec![],
    cost: ResearchCost::Points(100),
    unlocks: vec![
        Unlock::Building(BuildingTypeId(1)),
        Unlock::Recipe(RecipeId(0)),
    ],
    repeatable: false,
    cost_scaling: None,
}).unwrap();

// Tier 2: requires Tier 1
tree.register(Technology {
    id: TechId(1),
    name: "Advanced Smelting".to_string(),
    prerequisites: vec![TechId(0)],
    cost: ResearchCost::Points(200),
    unlocks: vec![
        Unlock::Building(BuildingTypeId(2)),
        Unlock::Recipe(RecipeId(1)),
    ],
    repeatable: false,
    cost_scaling: None,
}).unwrap();

// Repeatable: Mining Productivity (linear scaling, base 500, +250 per level)
tree.register(Technology {
    id: TechId(2),
    name: "Mining Productivity".to_string(),
    prerequisites: vec![TechId(0)],
    cost: ResearchCost::Points(500),
    unlocks: vec![Unlock::Custom("mining_productivity_bonus".to_string())],
    repeatable: true,
    cost_scaling: Some(CostScaling::Linear { base: 500, increment: 250 }),
}).unwrap();

// Research Basic Smelting
tree.start_research(TechId(0), 0).unwrap();
tree.contribute_points(TechId(0), 100, 1).unwrap();
assert!(tree.is_completed(TechId(0)));

// Research Advanced Smelting
tree.start_research(TechId(1), 2).unwrap();
tree.contribute_points(TechId(1), 200, 3).unwrap();

// Research Mining Productivity twice
for _ in 0..2 {
    let cost = match tree.effective_cost(TechId(2)).unwrap() {
        ResearchCost::Points(p) => p,
        _ => unreachable!(),
    };
    tree.start_research(TechId(2), 5).unwrap();
    tree.contribute_points(TechId(2), cost, 6).unwrap();
}
assert_eq!(tree.completion_count(TechId(2)), 2);

// Drain all events
for event in tree.drain_events() {
    println!("{event:?}");
}

// List all unlocks
for unlock in tree.all_unlocks() {
    println!("{unlock:?}");
}
```
