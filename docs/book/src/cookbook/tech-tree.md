# Gate Buildings Behind Research

**Goal:** Define a tech tree with prerequisites and unlock rewards, research technologies by contributing points, and query which buildings and recipes are available.
**Prerequisites:** [Tech Trees](../modules/tech-tree.md)
**Example:** `crates/factorial-examples/examples/tech_tree.rs`

## Steps

### 1. Register technologies

```rust
let mut tree = TechTree::new();

let basic_smelting_id = TechId(0);
tree.register(Technology {
    id: basic_smelting_id,
    name: "Basic Smelting".to_string(),
    prerequisites: vec![],
    cost: ResearchCost::Points(100),
    unlocks: vec![
        Unlock::Building(BuildingTypeId(1)),
        Unlock::Recipe(RecipeId(0)),
    ],
    repeatable: false,
    cost_scaling: None,
}).expect("register Basic Smelting");
```

Each `Technology` declares its prerequisites (other `TechId` values that must be completed first), a cost model, and the unlocks it grants. Here, Basic Smelting has no prerequisites and unlocks a stone furnace [building](../introduction/glossary.md#building-type-id) and an iron plate recipe.

### 2. Register dependent technologies

```rust
let advanced_smelting_id = TechId(1);
tree.register(Technology {
    id: advanced_smelting_id,
    name: "Advanced Smelting".to_string(),
    prerequisites: vec![basic_smelting_id],
    cost: ResearchCost::Points(200),
    unlocks: vec![
        Unlock::Building(BuildingTypeId(2)),
        Unlock::Recipe(RecipeId(1)),
    ],
    repeatable: false,
    cost_scaling: None,
}).expect("register Advanced Smelting");
```

Advanced Smelting requires Basic Smelting to be completed first. The tech tree enforces this ordering when you call `start_research()`.

### 3. Research by contributing points

```rust
tree.start_research(basic_smelting_id, 0).expect("start research");

let consumed = tree.contribute_points(basic_smelting_id, 60, 1).expect("contribute");
// consumed = 60, progress is 60/100

let consumed = tree.contribute_points(basic_smelting_id, 60, 2).expect("contribute");
// consumed = 40 (capped at remaining cost), technology completes

assert!(tree.is_completed(basic_smelting_id));
```

`contribute_points()` returns how many points were actually consumed. If you offer more than the remaining cost, the excess is not consumed. The second argument is a tick number for event ordering.

### 4. Check prerequisites and continue

```rust
let can_start = tree.prerequisites_met(advanced_smelting_id).expect("check");
assert!(can_start); // Basic Smelting is done

tree.start_research(advanced_smelting_id, 3).expect("start");
tree.contribute_points(advanced_smelting_id, 200, 4).expect("complete");
```

### 5. Register a repeatable technology with scaling costs

```rust
let mining_prod_id = TechId(2);
tree.register(Technology {
    id: mining_prod_id,
    name: "Mining Productivity".to_string(),
    prerequisites: vec![basic_smelting_id],
    cost: ResearchCost::Points(500),
    unlocks: vec![Unlock::Custom("mining_productivity_bonus".to_string())],
    repeatable: true,
    cost_scaling: Some(CostScaling::Linear { base: 500, increment: 250 }),
}).expect("register");

// Level 0: costs 500 points
tree.start_research(mining_prod_id, 5).expect("start");
tree.contribute_points(mining_prod_id, 500, 6).expect("complete level 0");

// Level 1: costs 750 points (500 + 250)
tree.start_research(mining_prod_id, 7).expect("start");
tree.contribute_points(mining_prod_id, 750, 8).expect("complete level 1");

println!("Completion count: {}", tree.completion_count(mining_prod_id)); // 2
```

### 6. Query all unlocks

```rust
for unlock in tree.all_unlocks() {
    // Unlock::Building(BuildingTypeId(1)), Unlock::Recipe(RecipeId(0)), etc.
}
```

## What's Happening

The tech tree module maintains a directed acyclic graph of technologies. When research begins, the module verifies all prerequisites are met. Points are contributed over time (typically by a lab building running a research recipe in the [production graph](../introduction/glossary.md#production-graph)). When the cost threshold is reached, the technology completes and its unlocks become available. Events are emitted for research start, progress, and completion. Your game logic can use `all_unlocks()` to gate which [nodes](../introduction/glossary.md#node) and recipes the player can build.

## Variations

- **Resource-based costs:** Use `ResearchCost::Items` instead of `Points` to require specific items (science packs) for research.
- **Non-linear scaling:** Replace `CostScaling::Linear` with `CostScaling::Exponential` for infinite research that gets progressively more expensive.
- **Custom unlocks:** `Unlock::Custom(String)` lets you gate arbitrary game features (map reveals, character abilities, etc.) behind research.
- **Integration with production:** Have a lab [processor](../introduction/glossary.md#processor) consume science packs and call `contribute_points()` on recipe completion. See [React to Production Events](./events.md) for wiring events to game logic.
