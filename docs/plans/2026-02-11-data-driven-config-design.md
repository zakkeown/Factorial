# Data-Driven Configuration Design

**Date:** 2026-02-11
**Status:** Draft
**Scope:** New `factorial-data` crate — load game content from data files (RON/TOML/JSON) instead of Rust code

---

## Problem

Game developers using Factorial must define all content (items, recipes, buildings, module config) in Rust code with raw numeric IDs like `ItemTypeId(0)`. Every content change requires recompilation. This slows prototyping and makes it harder to iterate on game balance.

## Goals

- Define game content in human-readable data files
- Support RON, TOML, and JSON — detect format from file extension
- Reference items, recipes, and buildings by string name instead of numeric ID
- Resolve all cross-references at load time with clear error messages
- Cover core registry (items, recipes, buildings) and all framework modules (power, fluid, tech tree, logic)
- Zero impact on developers who prefer pure-Rust registry building

## Non-Goals

- Runtime hot-reloading of data files (restart required)
- Modding sandboxing or untrusted input validation beyond parse/resolve errors
- Replacing the Rust API — data files are a convenience layer on top

---

## Crate Structure

New crate: `factorial-data`. Depends on core and all framework modules.

```
factorial-data
  ├── factorial-core
  ├── factorial-power
  ├── factorial-fluid
  ├── factorial-tech-tree
  ├── factorial-spatial
  └── factorial-logic
```

External dependencies: `ron`, `toml`, `serde_json` (serde already in workspace).

### File Layout

```
crates/factorial-data/
  Cargo.toml
  src/
    lib.rs            — public API (load_game_data, GameData, DataLoadError)
    schema.rs         — serde data file structs (ItemData, RecipeData, etc.)
    loader.rs         — resolution pipeline, format detection, deserialization
    module_config.rs  — PowerConfig, FluidConfig, etc. with apply() methods
  test_data/
    minimal_ron/      — items.ron, recipes.ron, buildings.ron
    minimal_json/     — same content in JSON
    full_game/        — all files including module configs
    errors/           — intentionally broken files for error path tests
```

---

## Game Data Directory

A game developer creates a directory with these files:

```
game_data/
  items.ron          (or .json or .toml)
  recipes.ron
  buildings.ron
  power.ron          (optional)
  fluids.ron         (optional)
  tech_tree.ron      (optional)
  logic.ron          (optional)
```

**Load order** (enforced by the loader):
1. Items — no dependencies
2. Recipes — references items by name
3. Buildings — references recipes by name, includes footprints
4. Power, fluids, tech tree, logic — reference buildings/items/recipes by name

Steps 1-3 are required. Step 4 modules are opt-in: if the file is absent, that module is not configured.

---

## Core Data File Schemas

### items.*

```ron
[
  Item(name: "iron_ore"),
  Item(name: "iron_plate"),
  Item(name: "copper_ore"),
  Item(
    name: "water",
    properties: [
      Property(name: "temperature", type: "fixed32", default: 20.0),
    ],
  ),
]
```

Serde struct:

```rust
#[derive(Debug, Deserialize)]
pub struct ItemData {
    pub name: String,
    #[serde(default)]
    pub properties: Vec<PropertyData>,
}

#[derive(Debug, Deserialize)]
pub struct PropertyData {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: PropertyType,
    pub default: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Fixed64,
    Fixed32,
    U32,
    U8,
}
```

### recipes.*

```ron
[
  Recipe(
    name: "smelt_iron",
    inputs: [("iron_ore", 1)],
    outputs: [("iron_plate", 1)],
    duration: 60,
  ),
  Recipe(
    name: "smelt_copper",
    inputs: [("copper_ore", 2)],
    outputs: [("copper_plate", 1)],
    duration: 90,
  ),
]
```

Serde struct:

```rust
#[derive(Debug, Deserialize)]
pub struct RecipeData {
    pub name: String,
    pub inputs: Vec<(String, u32)>,
    pub outputs: Vec<(String, u32)>,
    pub duration: u64,
}
```

### buildings.*

```ron
[
  Building(
    name: "iron_mine",
    processor: Source(item: "iron_ore", rate: 2.0),
    footprint: (width: 2, height: 2),
    inventories: (input_capacity: 100, output_capacity: 100),
  ),
  Building(
    name: "smelter",
    processor: Recipe(recipe: "smelt_iron"),
    footprint: (width: 3, height: 3),
    inventories: (input_capacity: 200, output_capacity: 200),
  ),
  Building(
    name: "storage_chest",
    processor: Demand(items: ["iron_plate", "copper_plate"]),
    footprint: (width: 1, height: 1),
    inventories: (input_capacity: 500, output_capacity: 500),
  ),
]
```

Serde structs:

```rust
#[derive(Debug, Deserialize)]
pub struct BuildingData {
    pub name: String,
    pub processor: ProcessorData,
    #[serde(default = "default_footprint")]
    pub footprint: FootprintData,
    #[serde(default)]
    pub inventories: InventoryData,
}

#[derive(Debug, Deserialize)]
pub struct FootprintData {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct InventoryData {
    #[serde(default = "default_capacity")]
    pub input_capacity: u32,
    #[serde(default = "default_capacity")]
    pub output_capacity: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ProcessorData {
    Source { item: String, rate: f64 },
    Recipe { recipe: String },
    Demand { items: Vec<String> },
    Passthrough,
}
```

---

## Module Data File Schemas

### power.*

```ron
Power(
  generators: [
    (building: "coal_plant", output: 50.0, priority: "high"),
    (building: "solar_panel", output: 10.0, priority: "medium"),
  ],
  consumers: [
    (building: "smelter", draw: 30.0, priority: "medium"),
    (building: "assembler", draw: 20.0, priority: "low"),
  ],
  storage: [
    (building: "battery", capacity: 500.0, charge_rate: 10.0, discharge_rate: 15.0),
  ],
)
```

```rust
#[derive(Debug, Deserialize)]
pub struct PowerData {
    #[serde(default)]
    pub generators: Vec<PowerGeneratorData>,
    #[serde(default)]
    pub consumers: Vec<PowerConsumerData>,
    #[serde(default)]
    pub storage: Vec<PowerStorageData>,
}

#[derive(Debug, Deserialize)]
pub struct PowerGeneratorData {
    pub building: String,
    pub output: f64,
    pub priority: PriorityData,
}

#[derive(Debug, Deserialize)]
pub struct PowerConsumerData {
    pub building: String,
    pub draw: f64,
    pub priority: PriorityData,
}

#[derive(Debug, Deserialize)]
pub struct PowerStorageData {
    pub building: String,
    pub capacity: f64,
    pub charge_rate: f64,
    pub discharge_rate: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorityData {
    High,
    Medium,
    Low,
}
```

### fluids.*

```ron
Fluids(
  types: ["water", "crude_oil", "steam"],
  producers: [
    (building: "water_pump", fluid: "water", rate: 5.0),
  ],
  consumers: [
    (building: "boiler", fluid: "water", rate: 3.0),
  ],
  storage: [
    (building: "tank", fluid: "water", capacity: 1000.0, fill_rate: 10.0, drain_rate: 10.0),
  ],
)
```

```rust
#[derive(Debug, Deserialize)]
pub struct FluidData {
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub producers: Vec<FluidProducerData>,
    #[serde(default)]
    pub consumers: Vec<FluidConsumerData>,
    #[serde(default)]
    pub storage: Vec<FluidStorageData>,
}

#[derive(Debug, Deserialize)]
pub struct FluidProducerData {
    pub building: String,
    pub fluid: String,
    pub rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct FluidConsumerData {
    pub building: String,
    pub fluid: String,
    pub rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct FluidStorageData {
    pub building: String,
    pub fluid: String,
    pub capacity: f64,
    pub fill_rate: f64,
    pub drain_rate: f64,
}
```

### tech_tree.*

```ron
[
  Research(
    name: "basic_smelting",
    cost: Points(amount: 100),
    unlocks: [Building("smelter"), Recipe("smelt_iron")],
  ),
  Research(
    name: "advanced_mining",
    cost: Items(items: [("iron_plate", 50)]),
    prerequisites: ["basic_smelting"],
    unlocks: [Building("advanced_drill")],
  ),
  Research(
    name: "automation",
    cost: Points(amount: 500),
    prerequisites: ["basic_smelting"],
    unlocks: [Building("assembler")],
    repeatable: Linear(base: 500, increment: 100),
  ),
]
```

```rust
#[derive(Debug, Deserialize)]
pub struct ResearchData {
    pub name: String,
    pub cost: ResearchCostData,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub unlocks: Vec<UnlockData>,
    pub repeatable: Option<RepeatableData>,
}

#[derive(Debug, Deserialize)]
pub enum ResearchCostData {
    Points { amount: u64 },
    Items { items: Vec<(String, u32)> },
}

#[derive(Debug, Deserialize)]
pub enum UnlockData {
    Building(String),
    Recipe(String),
    Custom(String),
}

#[derive(Debug, Deserialize)]
pub enum RepeatableData {
    Linear { base: u64, increment: u64 },
    Exponential { base: u64, multiplier: f64 },
}
```

### logic.*

```ron
Logic(
  circuit_controlled: [
    (building: "smelter", wire: "red", condition: Threshold(signal: "iron_ore", op: "gte", value: 10)),
  ],
  constant_combinators: [
    (building: "constant_1", signals: [("iron_ore", 42)]),
  ],
)
```

```rust
#[derive(Debug, Deserialize)]
pub struct LogicData {
    #[serde(default)]
    pub circuit_controlled: Vec<CircuitControlData>,
    #[serde(default)]
    pub constant_combinators: Vec<ConstantCombinatorData>,
}

#[derive(Debug, Deserialize)]
pub struct CircuitControlData {
    pub building: String,
    pub wire: WireColorData,
    pub condition: ConditionData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireColorData {
    Red,
    Green,
}

#[derive(Debug, Deserialize)]
pub enum ConditionData {
    Threshold { signal: String, op: String, value: i64 },
}

#[derive(Debug, Deserialize)]
pub struct ConstantCombinatorData {
    pub building: String,
    pub signals: Vec<(String, i32)>,
}
```

---

## Public API

### Primary Entry Point

```rust
/// Everything loaded and resolved from data files.
pub struct GameData {
    pub registry: Registry,
    pub building_footprints: BTreeMap<BuildingTypeId, BuildingFootprint>,
    pub building_processors: BTreeMap<BuildingTypeId, Processor>,
    pub building_inventories: BTreeMap<BuildingTypeId, InventoryData>,
    pub power_config: Option<PowerConfig>,
    pub fluid_config: Option<FluidConfig>,
    pub tech_tree_config: Option<TechTreeConfig>,
    pub logic_config: Option<LogicConfig>,
}

/// Load all game data from a directory.
///
/// Expects at minimum: items.*, recipes.*, buildings.* (any supported format).
/// Optional: power.*, fluids.*, tech_tree.*, logic.*
pub fn load_game_data(dir: &Path) -> Result<GameData, DataLoadError>;
```

### Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum DataLoadError {
    #[error("required file not found: {file}")]
    MissingRequired { file: &'static str },

    #[error("unsupported format for {file}")]
    UnsupportedFormat { file: PathBuf },

    #[error("conflicting formats: found both {a} and {b}")]
    ConflictingFormats { a: PathBuf, b: PathBuf },

    #[error("parse error in {file}: {source}")]
    Parse { file: PathBuf, source: String },

    #[error("unresolved reference in {file}: \"{name}\" (expected {expected_kind})")]
    UnresolvedRef {
        file: PathBuf,
        name: String,
        expected_kind: &'static str,
    },

    #[error("duplicate name in {file}: \"{name}\"")]
    DuplicateName { file: PathBuf, name: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Usage Pattern

```rust
use factorial_data::load_game_data;
use factorial_core::engine::Engine;
use factorial_core::sim::SimulationStrategy;

fn main() {
    let data = load_game_data(Path::new("game_data/"))
        .expect("failed to load game data");

    let mut engine = Engine::new_with_registry(
        SimulationStrategy::Tick,
        data.registry,
    );

    if let Some(power) = data.power_config {
        power.apply(&mut engine);
    }
    if let Some(fluid) = data.fluid_config {
        fluid.apply(&mut engine);
    }
    // ...
}
```

### Engine Constructor

New method on `Engine` (in `factorial-core/src/engine.rs`):

```rust
impl Engine {
    /// Create an engine with a pre-built registry.
    pub fn new_with_registry(strategy: SimulationStrategy, registry: Registry) -> Self;
}
```

The existing `Engine::new()` remains unchanged.

---

## Resolution Pipeline

Inside `load_game_data`:

```
1. Scan directory for items.* / recipes.* / buildings.*
   - Exactly one format per base name (error on items.ron + items.json)
   - items/recipes/buildings are required

2. Deserialize items → Vec<ItemData>
   - Check for duplicate names
   - Register each into RegistryBuilder
   - Build name→ItemTypeId lookup table

3. Deserialize recipes → Vec<RecipeData>
   - Check for duplicate names
   - Resolve item names to ItemTypeIds (error if unresolved)
   - Register into RegistryBuilder
   - Build name→RecipeId lookup table

4. Deserialize buildings → Vec<BuildingData>
   - Check for duplicate names
   - Resolve recipe names to RecipeIds
   - Resolve item names in processor definitions
   - Build Processor from ProcessorData
   - Register into RegistryBuilder
   - Build name→BuildingTypeId lookup table
   - Extract footprints into BTreeMap

5. RegistryBuilder::build() — validates all internal references

6. For each optional module file found:
   - Deserialize module data
   - Resolve all name references using lookup tables
   - Build module config struct

7. Return GameData
```

### Format Detection

```rust
pub enum Format { Ron, Toml, Json }

fn detect_format(path: &Path) -> Result<Format, DataLoadError> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ron") => Ok(Format::Ron),
        Some("toml") => Ok(Format::Toml),
        Some("json") => Ok(Format::Json),
        _ => Err(DataLoadError::UnsupportedFormat { file: path.into() }),
    }
}

fn deserialize_file<T: DeserializeOwned>(path: &Path) -> Result<T, DataLoadError> {
    let text = std::fs::read_to_string(path)?;
    let format = detect_format(path)?;
    match format {
        Format::Ron => ron::from_str(&text),
        Format::Toml => toml::from_str(&text),
        Format::Json => serde_json::from_str(&text),
    }
    .map_err(|e| DataLoadError::Parse {
        file: path.into(),
        source: e.to_string(),
    })
}
```

### File Discovery

```rust
fn find_data_file(dir: &Path, base_name: &str) -> Result<Option<PathBuf>, DataLoadError> {
    let candidates: Vec<PathBuf> = ["ron", "toml", "json"]
        .iter()
        .map(|ext| dir.join(format!("{base_name}.{ext}")))
        .filter(|p| p.exists())
        .collect();

    match candidates.len() {
        0 => Ok(None),
        1 => Ok(Some(candidates.into_iter().next().unwrap())),
        _ => Err(DataLoadError::ConflictingFormats {
            a: candidates[0].clone(),
            b: candidates[1].clone(),
        }),
    }
}
```

---

## Tests

1. **`roundtrip_ron`** — Load from RON, verify registry contents match expected items/recipes/buildings.
2. **`roundtrip_json`** — Same content in JSON, verify identical GameData.
3. **`roundtrip_toml`** — Same content in TOML, verify identical GameData.
4. **`format_equivalence`** — Load RON and JSON variants, assert registry counts and name lookups are equal.
5. **`unresolved_item_in_recipe`** — Recipe references nonexistent item. Assert `UnresolvedRef` with correct file and name.
6. **`unresolved_recipe_in_building`** — Building references nonexistent recipe. Assert `UnresolvedRef`.
7. **`unresolved_building_in_power`** — Power config references nonexistent building. Assert `UnresolvedRef`.
8. **`duplicate_item_name`** — Two items named "iron_ore". Assert `DuplicateName`.
9. **`missing_required_file`** — No items file. Assert `MissingRequired`.
10. **`conflicting_formats`** — Both items.ron and items.json exist. Assert `ConflictingFormats`.
11. **`optional_modules_absent`** — Only core files present. Module configs are `None`.
12. **`full_integration`** — Load complete game definition with all modules, build engine, run 10 ticks, verify production.
13. **`empty_files`** — Empty item/recipe/building lists. Builds valid empty registry.
14. **`processor_types`** — Verify Source, Recipe, Demand, Passthrough processors all parse and resolve correctly.
15. **`tech_tree_prerequisites`** — Research with prerequisite chain resolves correctly.
16. **`properties_on_items`** — Items with property definitions produce correct PropertyDef entries.

Test fixtures live in `crates/factorial-data/test_data/` with subdirectories per scenario.

---

## Files Modified

| File | Changes |
|------|---------|
| `Cargo.toml` (workspace) | Add `factorial-data` to members, add `ron` and `toml` to workspace dependencies |
| `crates/factorial-data/Cargo.toml` | New crate with dependencies on core, all modules, ron, toml, serde_json |
| `crates/factorial-data/src/lib.rs` | Public API: `load_game_data`, `GameData`, `DataLoadError` |
| `crates/factorial-data/src/schema.rs` | All serde data file structs |
| `crates/factorial-data/src/loader.rs` | Resolution pipeline, format detection, file discovery |
| `crates/factorial-data/src/module_config.rs` | `PowerConfig`, `FluidConfig`, `TechTreeConfig`, `LogicConfig` with `apply()` methods |
| `crates/factorial-core/src/engine.rs` | Add `Engine::new_with_registry()` constructor |
| `crates/factorial-data/test_data/` | Fixture files in RON, JSON, TOML |

## Implementation Order

```
1. Scaffold crate: Cargo.toml, lib.rs, workspace integration
2. Schema structs (schema.rs) — all serde data types
3. Format detection and deserialization helpers (loader.rs)
4. Core loading pipeline: items → recipes → buildings (loader.rs)
5. Engine::new_with_registry (engine.rs)
6. Module config structs and apply() methods (module_config.rs)
7. Module loading: power, fluids, tech tree, logic (loader.rs)
8. Test fixtures in all three formats
9. Tests: core round-trips, error paths, format equivalence
10. Tests: module loading, full integration
11. CI: cargo test, clippy, fmt
```

Estimated: ~800 lines of source, ~400 lines of tests, ~200 lines of test fixtures.
