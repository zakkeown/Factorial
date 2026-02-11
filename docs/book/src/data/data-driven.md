# Data-Driven Configuration

The `factorial-data` crate lets you define your game's items, recipes,
buildings, and module configurations in external data files instead of
hard-coding them in Rust.

## Supported Formats

Data files can be written in any of these formats:

- **RON** (`.ron`) -- Rusty Object Notation. Recommended default.
- **JSON** (`.json`) -- Standard JSON.
- **TOML** (`.toml`) -- TOML (uses key-based list extraction for top-level arrays).

Format is detected automatically by file extension. Only one format per
logical file is allowed -- having both `items.ron` and `items.json` in the
same directory produces an error.

## Directory Layout

Place your data files in a single directory. The loader expects:

| File           | Required | Contents                              |
|----------------|----------|---------------------------------------|
| `items.*`      | Yes      | Item type definitions                 |
| `recipes.*`    | Yes      | Recipe definitions                    |
| `buildings.*`  | Yes      | Building type definitions             |
| `power.*`      | No       | Power generators, consumers, storage  |
| `fluids.*`     | No       | Fluid types, producers, consumers     |
| `tech_tree.*`  | No       | Research nodes with costs and unlocks |
| `logic.*`      | No       | Circuit controls and combinators      |

## Schema

### Items

Each item has a name and optional properties:

```ron
[
    (name: "Iron Ore"),
    (name: "Iron Plate", properties: {"temperature": 25.0}),
]
```

### Recipes

Recipes reference items by name. The loader resolves names to IDs:

```ron
[
    (
        name: "Smelt Iron",
        inputs: [("Iron Ore", 1)],
        outputs: [("Iron Plate", 1)],
        duration: 30,
    ),
]
```

### Buildings

Buildings reference a processor type and optionally a recipe by name:

```ron
[
    (
        name: "Iron Mine",
        processor: Source(item: "Iron Ore", rate: 2.0),
    ),
    (
        name: "Furnace",
        processor: Recipe(recipe: "Smelt Iron"),
        input_slots: 2,
        output_slots: 2,
    ),
]
```

## Resolution Pipeline

`load_game_data(dir)` processes files in dependency order:

1. **Items** -- registered first, producing a name-to-ID map.
2. **Recipes** -- item names in inputs/outputs are resolved against the item map.
3. **Buildings** -- recipe and item names are resolved; processors are constructed.
4. **Modules** (optional) -- power, fluid, tech-tree, and logic configs resolve
   names against the maps built above.

Any unresolved name produces a `DataLoadError::UnresolvedRef`.

## Usage

```rust,ignore
use factorial_data::load_game_data;
use factorial_core::engine::Engine;
use factorial_core::sim::SimulationStrategy;

let game_data = load_game_data("assets/data")?;
let mut engine = Engine::new_with_registry(
    SimulationStrategy::Tick,
    game_data.registry,
);

// Apply building processors, inventories, and module configs
// from game_data as needed.
```

## Error Handling

The loader returns `DataLoadError` variants covering:

- `MissingRequired` -- a required file was not found.
- `ConflictingFormats` -- multiple formats for the same logical file.
- `Parse` -- deserialization failure with file and detail.
- `UnresolvedRef` -- a name could not be resolved to a registered ID.
- `DuplicateName` -- two entries share the same name in one file.
