# Data-Driven Configuration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a `factorial-data` crate that loads game content from RON/TOML/JSON files into the engine's registry and module systems.

**Architecture:** New `factorial-data` crate with four source files (lib, schema, loader, module_config). The loader reads a directory of data files in dependency order (items -> recipes -> buildings -> modules), resolves string name references to typed IDs, and produces a `GameData` struct ready to feed into the engine. A new `Engine::new_with_registry()` constructor accepts pre-built registries.

**Tech Stack:** Rust 2024 edition, serde, ron, toml, serde_json, thiserror

---

## Task 1: Scaffold the Crate

**Files:**
- Create: `crates/factorial-data/Cargo.toml`
- Create: `crates/factorial-data/src/lib.rs`
- Create: `crates/factorial-data/src/schema.rs`
- Create: `crates/factorial-data/src/loader.rs`
- Create: `crates/factorial-data/src/module_config.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Add workspace dependencies and member**

In root `Cargo.toml`, add `factorial-data` to workspace members and add `ron`, `toml`, `serde_json` to workspace dependencies:

```toml
# Add to [workspace] members list:
"crates/factorial-data",

# Add to [workspace.dependencies]:
ron = "0.8"
toml = "0.8"
serde_json = "1"
```

**Step 2: Create `crates/factorial-data/Cargo.toml`**

```toml
[package]
name = "factorial-data"
version = "0.1.0"
edition = "2024"

[dependencies]
factorial-core = { path = "../factorial-core", features = ["test-utils"] }
factorial-power = { path = "../factorial-power" }
factorial-fluid = { path = "../factorial-fluid" }
factorial-tech-tree = { path = "../factorial-tech-tree" }
factorial-spatial = { path = "../factorial-spatial" }
factorial-logic = { path = "../factorial-logic" }
serde = { workspace = true }
serde_json = { workspace = true }
ron = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
factorial-core = { path = "../factorial-core", features = ["test-utils"] }
```

**Step 3: Create empty source files**

`crates/factorial-data/src/lib.rs`:
```rust
pub mod loader;
pub mod module_config;
pub mod schema;
```

`crates/factorial-data/src/schema.rs`:
```rust
//! Serde data file structs for game content definitions.
```

`crates/factorial-data/src/loader.rs`:
```rust
//! Resolution pipeline: reads data files, resolves cross-references, builds registry.
```

`crates/factorial-data/src/module_config.rs`:
```rust
//! Module configuration structs with apply() methods for wiring into the engine.
```

**Step 4: Verify it compiles**

Run: `cargo check --package factorial-data`
Expected: compiles with no errors

**Step 5: Commit**

```bash
git add crates/factorial-data/ Cargo.toml
git commit -m "feat(data): scaffold factorial-data crate"
```

---

## Task 2: Schema Structs — Core (Items, Recipes, Buildings)

**Files:**
- Modify: `crates/factorial-data/src/schema.rs`

**Step 1: Write the schema structs**

```rust
//! Serde data file structs for game content definitions.
//!
//! These types define the file format. They use string names for cross-references
//! instead of numeric IDs. The loader resolves names to IDs during loading.

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ItemData {
    pub name: String,
    #[serde(default)]
    pub properties: Vec<PropertyData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PropertyData {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: PropertyType,
    pub default: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Fixed64,
    Fixed32,
    U32,
    U8,
}

// ---------------------------------------------------------------------------
// Recipes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeData {
    pub name: String,
    pub inputs: Vec<(String, u32)>,
    pub outputs: Vec<(String, u32)>,
    pub duration: u64,
}

// ---------------------------------------------------------------------------
// Buildings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct BuildingData {
    pub name: String,
    pub processor: ProcessorData,
    #[serde(default = "default_footprint")]
    pub footprint: FootprintData,
    #[serde(default)]
    pub inventories: InventoryData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FootprintData {
    pub width: u32,
    pub height: u32,
}

fn default_footprint() -> FootprintData {
    FootprintData {
        width: 1,
        height: 1,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct InventoryData {
    #[serde(default = "default_capacity")]
    pub input_capacity: u32,
    #[serde(default = "default_capacity")]
    pub output_capacity: u32,
}

fn default_capacity() -> u32 {
    100
}

impl Default for InventoryData {
    fn default() -> Self {
        Self {
            input_capacity: default_capacity(),
            output_capacity: default_capacity(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub enum ProcessorData {
    Source {
        item: String,
        rate: f64,
    },
    Recipe {
        recipe: String,
    },
    Demand {
        items: Vec<String>,
    },
    Passthrough,
}
```

**Step 2: Verify it compiles**

Run: `cargo check --package factorial-data`
Expected: compiles with no errors

**Step 3: Write a deserialization test for each format**

Add to `crates/factorial-data/src/schema.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const ITEMS_RON: &str = r#"[
        (name: "iron_ore"),
        (name: "iron_plate"),
    ]"#;

    const ITEMS_JSON: &str = r#"[
        {"name": "iron_ore"},
        {"name": "iron_plate"}
    ]"#;

    const ITEMS_TOML: &str = r#"
[[items]]
name = "iron_ore"

[[items]]
name = "iron_plate"
"#;

    #[test]
    fn deserialize_items_ron() {
        let items: Vec<ItemData> = ron::from_str(ITEMS_RON).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "iron_ore");
        assert_eq!(items[1].name, "iron_plate");
    }

    #[test]
    fn deserialize_items_json() {
        let items: Vec<ItemData> = serde_json::from_str(ITEMS_JSON).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "iron_ore");
    }

    // TOML doesn't support top-level arrays, so we wrap in a struct
    #[derive(Deserialize)]
    struct TomlItems {
        items: Vec<ItemData>,
    }

    #[test]
    fn deserialize_items_toml() {
        let wrapper: TomlItems = toml::from_str(ITEMS_TOML).unwrap();
        assert_eq!(wrapper.items.len(), 2);
        assert_eq!(wrapper.items[0].name, "iron_ore");
    }

    const RECIPES_RON: &str = r#"[
        (name: "smelt_iron", inputs: [("iron_ore", 1)], outputs: [("iron_plate", 1)], duration: 60),
    ]"#;

    #[test]
    fn deserialize_recipes_ron() {
        let recipes: Vec<RecipeData> = ron::from_str(RECIPES_RON).unwrap();
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].inputs[0], ("iron_ore".to_string(), 1));
    }

    const BUILDINGS_RON: &str = r#"[
        (
            name: "iron_mine",
            processor: Source(item: "iron_ore", rate: 2.0),
        ),
        (
            name: "smelter",
            processor: Recipe(recipe: "smelt_iron"),
            footprint: (width: 3, height: 3),
            inventories: (input_capacity: 200, output_capacity: 200),
        ),
        (
            name: "chest",
            processor: Passthrough,
        ),
    ]"#;

    #[test]
    fn deserialize_buildings_ron() {
        let buildings: Vec<BuildingData> = ron::from_str(BUILDINGS_RON).unwrap();
        assert_eq!(buildings.len(), 3);
        assert!(matches!(buildings[0].processor, ProcessorData::Source { .. }));
        assert!(matches!(buildings[1].processor, ProcessorData::Recipe { .. }));
        assert!(matches!(buildings[2].processor, ProcessorData::Passthrough));
        // Defaults
        assert_eq!(buildings[0].footprint.width, 1);
        assert_eq!(buildings[0].inventories.input_capacity, 100);
        // Overrides
        assert_eq!(buildings[1].footprint.width, 3);
        assert_eq!(buildings[1].inventories.input_capacity, 200);
    }
}
```

**Step 4: Run tests**

Run: `cargo test --package factorial-data`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/factorial-data/src/schema.rs
git commit -m "feat(data): add core schema structs with serde tests"
```

---

## Task 3: Schema Structs — Module Data (Power, Fluid, Tech Tree, Logic)

**Files:**
- Modify: `crates/factorial-data/src/schema.rs`

**Step 1: Add module schema structs**

Append to `crates/factorial-data/src/schema.rs` (before the `#[cfg(test)]` block):

```rust
// ---------------------------------------------------------------------------
// Power
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct PowerData {
    #[serde(default)]
    pub generators: Vec<PowerGeneratorData>,
    #[serde(default)]
    pub consumers: Vec<PowerConsumerData>,
    #[serde(default)]
    pub storage: Vec<PowerStorageData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PowerGeneratorData {
    pub building: String,
    pub output: f64,
    #[serde(default = "default_priority")]
    pub priority: PriorityData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PowerConsumerData {
    pub building: String,
    pub draw: f64,
    #[serde(default = "default_priority")]
    pub priority: PriorityData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PowerStorageData {
    pub building: String,
    pub capacity: f64,
    pub charge_rate: f64,
    pub discharge_rate: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PriorityData {
    High,
    #[default]
    Medium,
    Low,
}

fn default_priority() -> PriorityData {
    PriorityData::Medium
}

// ---------------------------------------------------------------------------
// Fluids
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct FluidProducerData {
    pub building: String,
    pub fluid: String,
    pub rate: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FluidConsumerData {
    pub building: String,
    pub fluid: String,
    pub rate: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FluidStorageData {
    pub building: String,
    pub fluid: String,
    pub capacity: f64,
    pub fill_rate: f64,
    pub drain_rate: f64,
}

// ---------------------------------------------------------------------------
// Tech Tree
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ResearchData {
    pub name: String,
    pub cost: ResearchCostData,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub unlocks: Vec<UnlockData>,
    #[serde(default)]
    pub repeatable: bool,
    pub cost_scaling: Option<CostScalingData>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum ResearchCostData {
    Points { amount: u32 },
    Items { items: Vec<(String, u32)> },
    Delivery { items: Vec<(String, u32)> },
    Rate { points_per_tick: f64, total: u32 },
}

#[derive(Debug, Clone, Deserialize)]
pub enum UnlockData {
    Building(String),
    Recipe(String),
    Custom(String),
}

#[derive(Debug, Clone, Deserialize)]
pub enum CostScalingData {
    Linear { base: u32, increment: u32 },
    Exponential { base: u32, multiplier: f64 },
}

// ---------------------------------------------------------------------------
// Logic
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct LogicData {
    #[serde(default)]
    pub circuit_controlled: Vec<CircuitControlData>,
    #[serde(default)]
    pub constant_combinators: Vec<ConstantCombinatorData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CircuitControlData {
    pub building: String,
    pub wire: WireColorData,
    pub condition: ConditionData,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireColorData {
    Red,
    Green,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConditionData {
    pub signal: String,
    pub op: String,
    pub value: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConstantCombinatorData {
    pub building: String,
    pub signals: Vec<(String, i32)>,
}

// ---------------------------------------------------------------------------
// TOML wrapper types (TOML requires top-level tables, not arrays)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct TomlItems {
    pub items: Vec<ItemData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TomlRecipes {
    pub recipes: Vec<RecipeData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TomlBuildings {
    pub buildings: Vec<BuildingData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TomlResearch {
    pub research: Vec<ResearchData>,
}
```

**Step 2: Add module deserialization tests**

Append to the tests module:

```rust
    const POWER_RON: &str = r#"(
        generators: [(building: "coal_plant", output: 50.0, priority: "high")],
        consumers: [(building: "smelter", draw: 30.0)],
        storage: [(building: "battery", capacity: 500.0, charge_rate: 10.0, discharge_rate: 15.0)],
    )"#;

    #[test]
    fn deserialize_power_ron() {
        let power: PowerData = ron::from_str(POWER_RON).unwrap();
        assert_eq!(power.generators.len(), 1);
        assert_eq!(power.consumers.len(), 1);
        assert_eq!(power.storage.len(), 1);
        assert!(matches!(power.generators[0].priority, PriorityData::High));
        // Consumer has default priority
        assert!(matches!(power.consumers[0].priority, PriorityData::Medium));
    }

    const TECH_RON: &str = r#"[
        (
            name: "basic_smelting",
            cost: Points(amount: 100),
            unlocks: [Building("smelter"), Recipe("smelt_iron")],
        ),
        (
            name: "advanced_mining",
            cost: Items(items: [("iron_plate", 50)]),
            prerequisites: ["basic_smelting"],
            unlocks: [Building("advanced_drill")],
            repeatable: true,
            cost_scaling: Some(Linear(base: 100, increment: 50)),
        ),
    ]"#;

    #[test]
    fn deserialize_tech_tree_ron() {
        let techs: Vec<ResearchData> = ron::from_str(TECH_RON).unwrap();
        assert_eq!(techs.len(), 2);
        assert!(matches!(techs[0].cost, ResearchCostData::Points { amount: 100 }));
        assert_eq!(techs[1].prerequisites, vec!["basic_smelting"]);
        assert!(techs[1].repeatable);
    }

    const LOGIC_RON: &str = r#"(
        circuit_controlled: [
            (building: "smelter", wire: Red, condition: (signal: "iron_ore", op: "gte", value: 10)),
        ],
        constant_combinators: [
            (building: "constant_1", signals: [("iron_ore", 42)]),
        ],
    )"#;

    #[test]
    fn deserialize_logic_ron() {
        let logic: LogicData = ron::from_str(LOGIC_RON).unwrap();
        assert_eq!(logic.circuit_controlled.len(), 1);
        assert_eq!(logic.constant_combinators.len(), 1);
        assert_eq!(logic.constant_combinators[0].signals[0], ("iron_ore".to_string(), 42));
    }

    const FLUID_RON: &str = r#"(
        types: ["water", "steam"],
        producers: [(building: "pump", fluid: "water", rate: 5.0)],
        consumers: [(building: "boiler", fluid: "water", rate: 3.0)],
        storage: [(building: "tank", fluid: "water", capacity: 1000.0, fill_rate: 10.0, drain_rate: 10.0)],
    )"#;

    #[test]
    fn deserialize_fluid_ron() {
        let fluid: FluidData = ron::from_str(FLUID_RON).unwrap();
        assert_eq!(fluid.types.len(), 2);
        assert_eq!(fluid.producers.len(), 1);
        assert_eq!(fluid.storage[0].capacity, 1000.0);
    }
```

**Step 3: Run tests**

Run: `cargo test --package factorial-data`
Expected: all tests pass

**Step 4: Commit**

```bash
git add crates/factorial-data/src/schema.rs
git commit -m "feat(data): add module schema structs (power, fluid, tech-tree, logic)"
```

---

## Task 4: Loader — Format Detection, File Discovery, Error Types

**Files:**
- Modify: `crates/factorial-data/src/loader.rs`
- Modify: `crates/factorial-data/src/lib.rs`

**Step 1: Write the loader foundation**

`crates/factorial-data/src/loader.rs`:

```rust
//! Resolution pipeline: reads data files, resolves cross-references, builds registry.

use crate::schema::*;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum DataLoadError {
    #[error("required file not found: {file} (looked for .ron, .json, .toml in {dir})")]
    MissingRequired { file: &'static str, dir: PathBuf },

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

// ---------------------------------------------------------------------------
// Format detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Ron,
    Toml,
    Json,
}

fn detect_format(path: &Path) -> Result<Format, DataLoadError> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ron") => Ok(Format::Ron),
        Some("toml") => Ok(Format::Toml),
        Some("json") => Ok(Format::Json),
        _ => Err(DataLoadError::UnsupportedFormat {
            file: path.to_path_buf(),
        }),
    }
}

// ---------------------------------------------------------------------------
// File discovery
// ---------------------------------------------------------------------------

/// Find a data file by base name. Returns the path if exactly one format exists.
/// Returns None if no file found. Errors if multiple formats found.
pub(crate) fn find_data_file(
    dir: &Path,
    base_name: &str,
) -> Result<Option<PathBuf>, DataLoadError> {
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

/// Find a required data file. Errors if not found.
pub(crate) fn require_data_file(
    dir: &Path,
    base_name: &'static str,
) -> Result<PathBuf, DataLoadError> {
    find_data_file(dir, base_name)?.ok_or(DataLoadError::MissingRequired {
        file: base_name,
        dir: dir.to_path_buf(),
    })
}

// ---------------------------------------------------------------------------
// Deserialization
// ---------------------------------------------------------------------------

/// Deserialize a file, auto-detecting format from extension.
pub(crate) fn deserialize_file<T: DeserializeOwned>(path: &Path) -> Result<T, DataLoadError> {
    let text = std::fs::read_to_string(path)?;
    let format = detect_format(path)?;
    match format {
        Format::Ron => ron::from_str(&text),
        Format::Toml => toml::from_str(&text),
        Format::Json => serde_json::from_str(&text),
    }
    .map_err(|e| DataLoadError::Parse {
        file: path.to_path_buf(),
        source: e.to_string(),
    })
}

/// Deserialize a list from a file. For TOML, wraps in the appropriate wrapper struct
/// since TOML doesn't support top-level arrays.
pub(crate) fn deserialize_list<T: DeserializeOwned>(
    path: &Path,
    toml_key: &str,
) -> Result<Vec<T>, DataLoadError> {
    let text = std::fs::read_to_string(path)?;
    let format = detect_format(path)?;

    match format {
        Format::Ron | Format::Json => {
            let deser_fn = match format {
                Format::Ron => ron::from_str::<Vec<T>>,
                _ => serde_json::from_str::<Vec<T>>,
            };
            deser_fn(&text).map_err(|e| DataLoadError::Parse {
                file: path.to_path_buf(),
                source: e.to_string(),
            })
        }
        Format::Toml => {
            // TOML requires a table wrapper. Parse as generic toml::Value,
            // then extract the array under the given key.
            let table: toml::Value =
                toml::from_str(&text).map_err(|e| DataLoadError::Parse {
                    file: path.to_path_buf(),
                    source: e.to_string(),
                })?;
            let arr = table
                .get(toml_key)
                .ok_or_else(|| DataLoadError::Parse {
                    file: path.to_path_buf(),
                    source: format!("missing key \"{toml_key}\" in TOML file"),
                })?
                .clone();
            arr.try_into().map_err(|e: toml::de::Error| DataLoadError::Parse {
                file: path.to_path_buf(),
                source: e.to_string(),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Name lookup helper
// ---------------------------------------------------------------------------

pub(crate) fn resolve_name<'a, V: Copy>(
    map: &'a BTreeMap<String, V>,
    name: &str,
    file: &Path,
    expected_kind: &'static str,
) -> Result<V, DataLoadError> {
    map.get(name).copied().ok_or_else(|| DataLoadError::UnresolvedRef {
        file: file.to_path_buf(),
        name: name.to_string(),
        expected_kind,
    })
}

pub(crate) fn check_duplicate(
    map: &BTreeMap<String, impl Copy>,
    name: &str,
    file: &Path,
) -> Result<(), DataLoadError> {
    if map.contains_key(name) {
        Err(DataLoadError::DuplicateName {
            file: file.to_path_buf(),
            name: name.to_string(),
        })
    } else {
        Ok(())
    }
}
```

**Step 2: Update `lib.rs` to re-export the error type**

```rust
pub mod loader;
pub mod module_config;
pub mod schema;

pub use loader::DataLoadError;
```

**Step 3: Write tests for file discovery and format detection**

Append tests to `crates/factorial-data/src/loader.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    // We'll use simple file creation for tests since we don't want
    // to pull in tempfile as a dep. Use std::env::temp_dir instead.

    #[test]
    fn find_data_file_ron() {
        let dir = std::env::temp_dir().join("factorial_test_find_ron");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("items.ron"), "[]").unwrap();

        let result = find_data_file(&dir, "items").unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("items.ron"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn find_data_file_missing() {
        let dir = std::env::temp_dir().join("factorial_test_find_missing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let result = find_data_file(&dir, "items").unwrap();
        assert!(result.is_none());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn find_data_file_conflict() {
        let dir = std::env::temp_dir().join("factorial_test_find_conflict");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("items.ron"), "[]").unwrap();
        fs::write(dir.join("items.json"), "[]").unwrap();

        let result = find_data_file(&dir, "items");
        assert!(matches!(result, Err(DataLoadError::ConflictingFormats { .. })));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn require_data_file_missing() {
        let dir = std::env::temp_dir().join("factorial_test_require_missing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let result = require_data_file(&dir, "items");
        assert!(matches!(result, Err(DataLoadError::MissingRequired { .. })));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn detect_format_variants() {
        assert_eq!(detect_format(Path::new("foo.ron")).unwrap(), Format::Ron);
        assert_eq!(detect_format(Path::new("foo.json")).unwrap(), Format::Json);
        assert_eq!(detect_format(Path::new("foo.toml")).unwrap(), Format::Toml);
        assert!(detect_format(Path::new("foo.yaml")).is_err());
    }
}
```

**Step 4: Run tests**

Run: `cargo test --package factorial-data`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/factorial-data/src/loader.rs crates/factorial-data/src/lib.rs
git commit -m "feat(data): add loader foundation — format detection, file discovery, errors"
```

---

## Task 5: `Engine::new_with_registry` Constructor

**Files:**
- Modify: `crates/factorial-core/src/engine.rs`

**Step 1: Write a test for the new constructor**

Find the `#[cfg(test)] mod tests` block in `engine.rs`. Add:

```rust
    #[test]
    fn new_with_registry() {
        use crate::registry::*;
        let mut builder = RegistryBuilder::new();
        let iron = builder.register_item("iron", vec![]);
        builder.register_recipe(
            "smelt",
            vec![RecipeEntry { item: iron, quantity: 1 }],
            vec![],
            60,
        );
        builder.register_building("smelter", builder.recipe_id("smelt"));
        let registry = builder.build().unwrap();

        let engine = Engine::new_with_registry(SimulationStrategy::Tick, registry);
        assert_eq!(engine.node_count(), 0);
        assert!(engine.registry().is_some());
        assert_eq!(engine.registry().unwrap().item_count(), 1);
    }
```

**Step 2: Run test — verify it fails**

Run: `cargo test --package factorial-core -- new_with_registry`
Expected: FAIL — `new_with_registry` and `registry()` don't exist yet

**Step 3: Implement the constructor**

In `engine.rs`, add a `registry` field to the `Engine` struct (after the existing fields):

```rust
    /// Optional registry for data-driven configuration.
    registry: Option<crate::registry::Registry>,
```

Update `Engine::new()` to set `registry: None`.

Add the new constructor and accessor:

```rust
    /// Create an engine with a pre-built registry.
    pub fn new_with_registry(strategy: SimulationStrategy, registry: crate::registry::Registry) -> Self {
        let mut engine = Self::new(strategy);
        engine.registry = Some(registry);
        engine
    }

    /// Get a reference to the registry, if one was provided.
    pub fn registry(&self) -> Option<&crate::registry::Registry> {
        self.registry.as_ref()
    }
```

Make sure the `registry` field is handled in serialization/deserialization (skip it with `#[serde(skip)]` or equivalent if the snapshot doesn't include registry data — registry is rebuild-from-data-files, not stored in snapshots).

**Step 4: Run test — verify it passes**

Run: `cargo test --package factorial-core -- new_with_registry`
Expected: PASS

**Step 5: Run full test suite to check for regressions**

Run: `cargo test --package factorial-core`
Expected: all tests pass (serialization round-trips must still work with the new field)

**Step 6: Commit**

```bash
git add crates/factorial-core/src/engine.rs
git commit -m "feat(engine): add Engine::new_with_registry constructor"
```

---

## Task 6: Core Loading Pipeline (items -> recipes -> buildings)

**Files:**
- Modify: `crates/factorial-data/src/loader.rs`
- Modify: `crates/factorial-data/src/lib.rs`

**Step 1: Create test fixture files**

Create `crates/factorial-data/test_data/minimal_ron/items.ron`:
```ron
[
    (name: "iron_ore"),
    (name: "iron_plate"),
    (name: "copper_ore"),
]
```

Create `crates/factorial-data/test_data/minimal_ron/recipes.ron`:
```ron
[
    (name: "smelt_iron", inputs: [("iron_ore", 1)], outputs: [("iron_plate", 1)], duration: 60),
]
```

Create `crates/factorial-data/test_data/minimal_ron/buildings.ron`:
```ron
[
    (
        name: "iron_mine",
        processor: Source(item: "iron_ore", rate: 2.0),
        footprint: (width: 2, height: 2),
    ),
    (
        name: "smelter",
        processor: Recipe(recipe: "smelt_iron"),
        footprint: (width: 3, height: 3),
        inventories: (input_capacity: 200, output_capacity: 200),
    ),
    (
        name: "chest",
        processor: Demand(items: ["iron_plate"]),
    ),
]
```

**Step 2: Write the `GameData` struct and `load_game_data` function**

In `crates/factorial-data/src/lib.rs`:

```rust
pub mod loader;
pub mod module_config;
pub mod schema;

pub use loader::{load_game_data, DataLoadError, GameData};
```

In `crates/factorial-data/src/loader.rs`, add after the helpers:

```rust
use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::item::Inventory;
use factorial_core::processor::*;
use factorial_core::registry::*;
use factorial_spatial::BuildingFootprint;

use crate::module_config::*;

// ---------------------------------------------------------------------------
// GameData
// ---------------------------------------------------------------------------

/// Everything loaded and resolved from data files.
pub struct GameData {
    /// The built registry with all items, recipes, and buildings.
    pub registry: Registry,
    /// Per-building-type footprints for spatial placement.
    pub building_footprints: BTreeMap<BuildingTypeId, BuildingFootprint>,
    /// Per-building-type default processors.
    pub building_processors: BTreeMap<BuildingTypeId, Processor>,
    /// Per-building-type default inventory capacities.
    pub building_inventories: BTreeMap<BuildingTypeId, (u32, u32)>,
    /// Power module configuration (if power.* file exists).
    pub power_config: Option<PowerConfig>,
    /// Fluid module configuration (if fluids.* file exists).
    pub fluid_config: Option<FluidConfig>,
    /// Tech tree configuration (if tech_tree.* file exists).
    pub tech_tree_config: Option<TechTreeConfig>,
    /// Logic module configuration (if logic.* file exists).
    pub logic_config: Option<LogicConfig>,
}

// ---------------------------------------------------------------------------
// Core loading pipeline
// ---------------------------------------------------------------------------

/// Load all game data from a directory.
///
/// Expects at minimum: `items.*`, `recipes.*`, `buildings.*` (any supported format).
/// Optional: `power.*`, `fluids.*`, `tech_tree.*`, `logic.*`
pub fn load_game_data(dir: &Path) -> Result<GameData, DataLoadError> {
    let dir = dir.to_path_buf();

    // --- Phase 1: Find files ---
    let items_path = require_data_file(&dir, "items")?;
    let recipes_path = require_data_file(&dir, "recipes")?;
    let buildings_path = require_data_file(&dir, "buildings")?;

    let power_path = find_data_file(&dir, "power")?;
    let fluids_path = find_data_file(&dir, "fluids")?;
    let tech_tree_path = find_data_file(&dir, "tech_tree")?;
    let logic_path = find_data_file(&dir, "logic")?;

    // --- Phase 2: Load items ---
    let items: Vec<ItemData> = deserialize_list(&items_path, "items")?;
    let mut builder = RegistryBuilder::new();
    let mut item_names: BTreeMap<String, ItemTypeId> = BTreeMap::new();

    for item in &items {
        check_duplicate(&item_names, &item.name, &items_path)?;
        let props: Vec<PropertyDef> = item
            .properties
            .iter()
            .map(|p| PropertyDef {
                name: p.name.clone(),
                size: match p.prop_type {
                    PropertyType::Fixed64 => PropertySize::Fixed64,
                    PropertyType::Fixed32 => PropertySize::Fixed32,
                    PropertyType::U32 => PropertySize::U32,
                    PropertyType::U8 => PropertySize::U8,
                },
                default: match p.prop_type {
                    PropertyType::Fixed64 => {
                        PropertyDefault::Fixed64(Fixed64::from_num(p.default))
                    }
                    PropertyType::Fixed32 => {
                        PropertyDefault::Fixed32(factorial_core::fixed::Fixed32::from_num(
                            p.default as f32,
                        ))
                    }
                    PropertyType::U32 => PropertyDefault::U32(p.default as u32),
                    PropertyType::U8 => PropertyDefault::U8(p.default as u8),
                },
            })
            .collect();
        let id = builder.register_item(&item.name, props);
        item_names.insert(item.name.clone(), id);
    }

    // --- Phase 3: Load recipes ---
    let recipes: Vec<RecipeData> = deserialize_list(&recipes_path, "recipes")?;
    let mut recipe_names: BTreeMap<String, RecipeId> = BTreeMap::new();

    for recipe in &recipes {
        check_duplicate(&recipe_names, &recipe.name, &recipes_path)?;
        let inputs: Vec<RecipeEntry> = recipe
            .inputs
            .iter()
            .map(|(name, qty)| {
                let id = resolve_name(&item_names, name, &recipes_path, "item")?;
                Ok(RecipeEntry {
                    item: id,
                    quantity: *qty,
                })
            })
            .collect::<Result<_, DataLoadError>>()?;
        let outputs: Vec<RecipeEntry> = recipe
            .outputs
            .iter()
            .map(|(name, qty)| {
                let id = resolve_name(&item_names, name, &recipes_path, "item")?;
                Ok(RecipeEntry {
                    item: id,
                    quantity: *qty,
                })
            })
            .collect::<Result<_, DataLoadError>>()?;
        let id = builder.register_recipe(&recipe.name, inputs, outputs, recipe.duration);
        recipe_names.insert(recipe.name.clone(), id);
    }

    // --- Phase 4: Load buildings ---
    let buildings: Vec<BuildingData> = deserialize_list(&buildings_path, "buildings")?;
    let mut building_names: BTreeMap<String, BuildingTypeId> = BTreeMap::new();
    let mut building_footprints: BTreeMap<BuildingTypeId, BuildingFootprint> = BTreeMap::new();
    let mut building_processors: BTreeMap<BuildingTypeId, Processor> = BTreeMap::new();
    let mut building_inventories: BTreeMap<BuildingTypeId, (u32, u32)> = BTreeMap::new();

    for bld in &buildings {
        check_duplicate(&building_names, &bld.name, &buildings_path)?;

        // Resolve processor
        let processor = match &bld.processor {
            ProcessorData::Source { item, rate } => {
                let item_id = resolve_name(&item_names, item, &buildings_path, "item")?;
                Processor::Source(SourceProcessor {
                    output_type: item_id,
                    base_rate: Fixed64::from_num(*rate),
                    depletion: Depletion::Infinite,
                    accumulated: Fixed64::from_num(0),
                    initial_properties: None,
                })
            }
            ProcessorData::Recipe { recipe } => {
                let recipe_id =
                    resolve_name(&recipe_names, recipe, &buildings_path, "recipe")?;
                let recipe_def = builder
                    .get_recipe(recipe_id)
                    .expect("recipe was just registered");
                Processor::Fixed(FixedRecipe {
                    inputs: recipe_def
                        .inputs
                        .iter()
                        .map(|e| RecipeInput {
                            item_type: e.item,
                            quantity: e.quantity,
                        })
                        .collect(),
                    outputs: recipe_def
                        .outputs
                        .iter()
                        .map(|e| RecipeOutput {
                            item_type: e.item,
                            quantity: e.quantity,
                        })
                        .collect(),
                    duration: recipe_def.duration as u32,
                })
            }
            ProcessorData::Demand { items } => {
                let first_item =
                    resolve_name(&item_names, &items[0], &buildings_path, "item")?;
                let accepted: Vec<ItemTypeId> = items
                    .iter()
                    .map(|name| resolve_name(&item_names, name, &buildings_path, "item"))
                    .collect::<Result<_, _>>()?;
                Processor::Demand(DemandProcessor {
                    input_type: first_item,
                    base_rate: Fixed64::from_num(1),
                    accumulated: Fixed64::from_num(0),
                    consumed_total: 0,
                    accepted_types: if accepted.len() > 1 {
                        Some(accepted)
                    } else {
                        None
                    },
                })
            }
            ProcessorData::Passthrough => Processor::Passthrough,
        };

        // Resolve recipe reference for registry (optional)
        let recipe_ref = match &bld.processor {
            ProcessorData::Recipe { recipe } => recipe_names.get(recipe.as_str()).copied(),
            _ => None,
        };

        let id = builder.register_building(&bld.name, recipe_ref);
        building_names.insert(bld.name.clone(), id);
        building_footprints.insert(
            id,
            BuildingFootprint {
                width: bld.footprint.width,
                height: bld.footprint.height,
            },
        );
        building_processors.insert(id, processor);
        building_inventories.insert(
            id,
            (bld.inventories.input_capacity, bld.inventories.output_capacity),
        );
    }

    // --- Phase 5: Build registry ---
    let registry = builder.build().map_err(|e| DataLoadError::Parse {
        file: buildings_path.clone(),
        source: e.to_string(),
    })?;

    // --- Phase 6: Load optional modules ---
    let power_config = match power_path {
        Some(path) => Some(load_power_config(&path, &building_names)?),
        None => None,
    };
    let fluid_config = match fluids_path {
        Some(path) => Some(load_fluid_config(&path, &item_names, &building_names)?),
        None => None,
    };
    let tech_tree_config = match tech_tree_path {
        Some(path) => Some(load_tech_tree_config(
            &path,
            &item_names,
            &recipe_names,
            &building_names,
        )?),
        None => None,
    };
    let logic_config = match logic_path {
        Some(path) => Some(load_logic_config(&path, &item_names, &building_names)?),
        None => None,
    };

    Ok(GameData {
        registry,
        building_footprints,
        building_processors,
        building_inventories,
        power_config,
        fluid_config,
        tech_tree_config,
        logic_config,
    })
}
```

**Step 3: Add `get_recipe` to `RegistryBuilder`**

In `crates/factorial-core/src/registry.rs`, add to `impl RegistryBuilder`:

```rust
    /// Get a recipe definition by ID (for use during building).
    pub fn get_recipe(&self, id: RecipeId) -> Option<&RecipeDef> {
        self.recipes.get(id.0 as usize)
    }
```

**Step 4: Write the core loading test**

Add to `loader.rs` tests:

```rust
    #[test]
    fn load_minimal_ron() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/minimal_ron");
        let data = load_game_data(&dir).unwrap();

        assert_eq!(data.registry.item_count(), 3);
        assert_eq!(data.registry.recipe_count(), 1);
        assert_eq!(data.registry.building_count(), 3);

        // Check name lookups work
        assert!(data.registry.item_id("iron_ore").is_some());
        assert!(data.registry.item_id("iron_plate").is_some());
        assert!(data.registry.recipe_id("smelt_iron").is_some());
        assert!(data.registry.building_id("iron_mine").is_some());

        // Check footprints
        let mine_id = data.registry.building_id("iron_mine").unwrap();
        let fp = data.building_footprints.get(&mine_id).unwrap();
        assert_eq!(fp.width, 2);
        assert_eq!(fp.height, 2);

        // Check defaults
        let chest_id = data.registry.building_id("chest").unwrap();
        let fp = data.building_footprints.get(&chest_id).unwrap();
        assert_eq!(fp.width, 1);

        // Check processors resolved
        assert_eq!(data.building_processors.len(), 3);

        // No module configs
        assert!(data.power_config.is_none());
        assert!(data.fluid_config.is_none());
        assert!(data.tech_tree_config.is_none());
        assert!(data.logic_config.is_none());
    }
```

Note: This test will fail until module_config.rs stubs exist. We'll add those stubs in the next step.

**Step 5: Run tests**

Run: `cargo test --package factorial-data`
Expected: all tests pass

**Step 6: Commit**

```bash
git add crates/factorial-data/ crates/factorial-core/src/registry.rs
git commit -m "feat(data): core loading pipeline — items, recipes, buildings"
```

---

## Task 7: Module Config Structs and Loading

**Files:**
- Modify: `crates/factorial-data/src/module_config.rs`
- Modify: `crates/factorial-data/src/loader.rs`

**Step 1: Write module config types and loaders**

`crates/factorial-data/src/module_config.rs`:

```rust
//! Module configuration structs resolved from data files.
//!
//! Each config struct holds resolved IDs (not string names).
//! The `apply()` method wires the config into the engine's module system.

use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_power::{PowerConsumer, PowerModule, PowerNetworkId, PowerPriority, PowerProducer, PowerStorage};
use factorial_fluid::{FluidConsumer, FluidModule, FluidNetworkId, FluidProducer, FluidStorage};
use factorial_tech_tree::{CostScaling, ResearchCost, Technology, TechId, TechTree, Unlock};
use factorial_logic::{LogicModule, WireColor};
use factorial_logic::combinator::SignalSelector;
use factorial_logic::condition::{CircuitControl, ComparisonOp, Condition};
use std::collections::BTreeMap;

use crate::loader::DataLoadError;
use crate::schema::*;
use std::path::Path;

// ---------------------------------------------------------------------------
// Power
// ---------------------------------------------------------------------------

pub struct PowerConfig {
    pub generators: Vec<(BuildingTypeId, f64, PowerPriority)>,
    pub consumers: Vec<(BuildingTypeId, f64, PowerPriority)>,
    pub storage: Vec<(BuildingTypeId, f64, f64, f64)>,
}

impl PowerConfig {
    pub fn build_module(&self) -> PowerModule {
        let mut module = PowerModule::new();
        // All buildings go on a single default network for now.
        // Game code can create multiple networks as needed.
        let net = PowerNetworkId(0);
        module.networks.insert(net, factorial_power::PowerNetwork::new(net));
        module
    }
}

pub(crate) fn load_power_config(
    path: &Path,
    building_names: &BTreeMap<String, BuildingTypeId>,
) -> Result<PowerConfig, DataLoadError> {
    let data: PowerData = crate::loader::deserialize_file(path)?;

    let generators = data
        .generators
        .iter()
        .map(|g| {
            let id = crate::loader::resolve_name(building_names, &g.building, path, "building")?;
            let priority = match g.priority {
                PriorityData::High => PowerPriority::High,
                PriorityData::Medium => PowerPriority::Medium,
                PriorityData::Low => PowerPriority::Low,
            };
            Ok((id, g.output, priority))
        })
        .collect::<Result<_, DataLoadError>>()?;

    let consumers = data
        .consumers
        .iter()
        .map(|c| {
            let id = crate::loader::resolve_name(building_names, &c.building, path, "building")?;
            let priority = match c.priority {
                PriorityData::High => PowerPriority::High,
                PriorityData::Medium => PowerPriority::Medium,
                PriorityData::Low => PowerPriority::Low,
            };
            Ok((id, c.draw, priority))
        })
        .collect::<Result<_, DataLoadError>>()?;

    let storage = data
        .storage
        .iter()
        .map(|s| {
            let id = crate::loader::resolve_name(building_names, &s.building, path, "building")?;
            Ok((id, s.capacity, s.charge_rate, s.discharge_rate))
        })
        .collect::<Result<_, DataLoadError>>()?;

    Ok(PowerConfig {
        generators,
        consumers,
        storage,
    })
}

// ---------------------------------------------------------------------------
// Fluid
// ---------------------------------------------------------------------------

pub struct FluidConfig {
    pub fluid_types: Vec<(String, ItemTypeId)>,
    pub producers: Vec<(BuildingTypeId, ItemTypeId, f64)>,
    pub consumers: Vec<(BuildingTypeId, ItemTypeId, f64)>,
    pub storage: Vec<(BuildingTypeId, ItemTypeId, f64, f64, f64)>,
}

pub(crate) fn load_fluid_config(
    path: &Path,
    item_names: &BTreeMap<String, ItemTypeId>,
    building_names: &BTreeMap<String, BuildingTypeId>,
) -> Result<FluidConfig, DataLoadError> {
    let data: FluidData = crate::loader::deserialize_file(path)?;

    let fluid_types: Vec<(String, ItemTypeId)> = data
        .types
        .iter()
        .map(|name| {
            let id = crate::loader::resolve_name(item_names, name, path, "item (fluid type)")?;
            Ok((name.clone(), id))
        })
        .collect::<Result<_, DataLoadError>>()?;

    let producers = data
        .producers
        .iter()
        .map(|p| {
            let bid = crate::loader::resolve_name(building_names, &p.building, path, "building")?;
            let fid = crate::loader::resolve_name(item_names, &p.fluid, path, "item (fluid type)")?;
            Ok((bid, fid, p.rate))
        })
        .collect::<Result<_, DataLoadError>>()?;

    let consumers = data
        .consumers
        .iter()
        .map(|c| {
            let bid = crate::loader::resolve_name(building_names, &c.building, path, "building")?;
            let fid = crate::loader::resolve_name(item_names, &c.fluid, path, "item (fluid type)")?;
            Ok((bid, fid, c.rate))
        })
        .collect::<Result<_, DataLoadError>>()?;

    let storage = data
        .storage
        .iter()
        .map(|s| {
            let bid = crate::loader::resolve_name(building_names, &s.building, path, "building")?;
            let fid = crate::loader::resolve_name(item_names, &s.fluid, path, "item (fluid type)")?;
            Ok((bid, fid, s.capacity, s.fill_rate, s.drain_rate))
        })
        .collect::<Result<_, DataLoadError>>()?;

    Ok(FluidConfig {
        fluid_types,
        producers,
        consumers,
        storage,
    })
}

// ---------------------------------------------------------------------------
// Tech Tree
// ---------------------------------------------------------------------------

pub struct TechTreeConfig {
    pub technologies: Vec<ResolvedTech>,
}

pub struct ResolvedTech {
    pub name: String,
    pub cost: ResearchCost,
    pub prerequisites: Vec<String>,
    pub unlocks: Vec<Unlock>,
    pub repeatable: bool,
    pub cost_scaling: Option<CostScaling>,
}

pub(crate) fn load_tech_tree_config(
    path: &Path,
    item_names: &BTreeMap<String, ItemTypeId>,
    recipe_names: &BTreeMap<String, RecipeId>,
    building_names: &BTreeMap<String, BuildingTypeId>,
) -> Result<TechTreeConfig, DataLoadError> {
    let data: Vec<ResearchData> = crate::loader::deserialize_list(path, "research")?;

    let mut technologies = Vec::new();
    for tech in &data {
        let cost = match &tech.cost {
            ResearchCostData::Points { amount } => ResearchCost::Points(*amount),
            ResearchCostData::Items { items } => {
                let resolved: Vec<(ItemTypeId, u32)> = items
                    .iter()
                    .map(|(name, qty)| {
                        let id = crate::loader::resolve_name(item_names, name, path, "item")?;
                        Ok((id, *qty))
                    })
                    .collect::<Result<_, DataLoadError>>()?;
                ResearchCost::Items(resolved)
            }
            ResearchCostData::Delivery { items } => {
                let resolved: Vec<(ItemTypeId, u32)> = items
                    .iter()
                    .map(|(name, qty)| {
                        let id = crate::loader::resolve_name(item_names, name, path, "item")?;
                        Ok((id, *qty))
                    })
                    .collect::<Result<_, DataLoadError>>()?;
                ResearchCost::Delivery(resolved)
            }
            ResearchCostData::Rate {
                points_per_tick,
                total,
            } => ResearchCost::Rate {
                points_per_tick: Fixed64::from_num(*points_per_tick),
                total: *total,
            },
        };

        let unlocks: Vec<Unlock> = tech
            .unlocks
            .iter()
            .map(|u| match u {
                UnlockData::Building(name) => {
                    let id = crate::loader::resolve_name(building_names, name, path, "building")?;
                    Ok(Unlock::Building(id))
                }
                UnlockData::Recipe(name) => {
                    let id = crate::loader::resolve_name(recipe_names, name, path, "recipe")?;
                    Ok(Unlock::Recipe(id))
                }
                UnlockData::Custom(s) => Ok(Unlock::Custom(s.clone())),
            })
            .collect::<Result<_, DataLoadError>>()?;

        let cost_scaling = tech.cost_scaling.as_ref().map(|cs| match cs {
            CostScalingData::Linear { base, increment } => CostScaling::Linear {
                base: *base,
                increment: *increment,
            },
            CostScalingData::Exponential { base, multiplier } => CostScaling::Exponential {
                base: *base,
                multiplier: Fixed64::from_num(*multiplier),
            },
        });

        technologies.push(ResolvedTech {
            name: tech.name.clone(),
            cost,
            prerequisites: tech.prerequisites.clone(),
            unlocks,
            repeatable: tech.repeatable,
            cost_scaling,
        });
    }

    Ok(TechTreeConfig { technologies })
}

// ---------------------------------------------------------------------------
// Logic
// ---------------------------------------------------------------------------

pub struct LogicConfig {
    pub circuit_controlled: Vec<(BuildingTypeId, WireColor, ItemTypeId, ComparisonOp, i64)>,
    pub constant_combinators: Vec<(BuildingTypeId, Vec<(ItemTypeId, i32)>)>,
}

fn parse_comparison_op(op: &str, path: &Path) -> Result<ComparisonOp, DataLoadError> {
    match op {
        "gt" => Ok(ComparisonOp::Gt),
        "lt" => Ok(ComparisonOp::Lt),
        "eq" => Ok(ComparisonOp::Eq),
        "gte" => Ok(ComparisonOp::Gte),
        "lte" => Ok(ComparisonOp::Lte),
        "ne" => Ok(ComparisonOp::Ne),
        _ => Err(DataLoadError::Parse {
            file: path.to_path_buf(),
            source: format!("unknown comparison operator: \"{op}\" (expected gt, lt, eq, gte, lte, ne)"),
        }),
    }
}

pub(crate) fn load_logic_config(
    path: &Path,
    item_names: &BTreeMap<String, ItemTypeId>,
    building_names: &BTreeMap<String, BuildingTypeId>,
) -> Result<LogicConfig, DataLoadError> {
    let data: LogicData = crate::loader::deserialize_file(path)?;

    let circuit_controlled = data
        .circuit_controlled
        .iter()
        .map(|cc| {
            let bid = crate::loader::resolve_name(building_names, &cc.building, path, "building")?;
            let wire = match cc.wire {
                WireColorData::Red => WireColor::Red,
                WireColorData::Green => WireColor::Green,
            };
            let signal = crate::loader::resolve_name(item_names, &cc.condition.signal, path, "item (signal)")?;
            let op = parse_comparison_op(&cc.condition.op, path)?;
            Ok((bid, wire, signal, op, cc.condition.value))
        })
        .collect::<Result<_, DataLoadError>>()?;

    let constant_combinators = data
        .constant_combinators
        .iter()
        .map(|cc| {
            let bid = crate::loader::resolve_name(building_names, &cc.building, path, "building")?;
            let signals: Vec<(ItemTypeId, i32)> = cc
                .signals
                .iter()
                .map(|(name, val)| {
                    let id = crate::loader::resolve_name(item_names, name, path, "item (signal)")?;
                    Ok((id, *val))
                })
                .collect::<Result<_, DataLoadError>>()?;
            Ok((bid, signals))
        })
        .collect::<Result<_, DataLoadError>>()?;

    Ok(LogicConfig {
        circuit_controlled,
        constant_combinators,
    })
}
```

**Step 2: Write tests for module loading**

Create test fixture: `crates/factorial-data/test_data/full_game/items.ron`:
```ron
[
    (name: "iron_ore"),
    (name: "iron_plate"),
    (name: "water"),
]
```

Create `crates/factorial-data/test_data/full_game/recipes.ron`:
```ron
[
    (name: "smelt_iron", inputs: [("iron_ore", 1)], outputs: [("iron_plate", 1)], duration: 60),
]
```

Create `crates/factorial-data/test_data/full_game/buildings.ron`:
```ron
[
    (name: "iron_mine", processor: Source(item: "iron_ore", rate: 2.0)),
    (name: "smelter", processor: Recipe(recipe: "smelt_iron")),
    (name: "water_pump", processor: Source(item: "water", rate: 5.0)),
    (name: "tank", processor: Passthrough),
    (name: "constant_1", processor: Passthrough),
]
```

Create `crates/factorial-data/test_data/full_game/power.ron`:
```ron
(
    generators: [(building: "iron_mine", output: 50.0, priority: "high")],
    consumers: [(building: "smelter", draw: 30.0)],
    storage: [],
)
```

Create `crates/factorial-data/test_data/full_game/fluids.ron`:
```ron
(
    types: ["water"],
    producers: [(building: "water_pump", fluid: "water", rate: 5.0)],
    consumers: [],
    storage: [(building: "tank", fluid: "water", capacity: 1000.0, fill_rate: 10.0, drain_rate: 10.0)],
)
```

Create `crates/factorial-data/test_data/full_game/tech_tree.ron`:
```ron
[
    (
        name: "basic_smelting",
        cost: Points(amount: 100),
        unlocks: [Building("smelter"), Recipe("smelt_iron")],
    ),
]
```

Create `crates/factorial-data/test_data/full_game/logic.ron`:
```ron
(
    circuit_controlled: [
        (building: "smelter", wire: Red, condition: (signal: "iron_ore", op: "gte", value: 10)),
    ],
    constant_combinators: [
        (building: "constant_1", signals: [("iron_ore", 42)]),
    ],
)
```

Add to `loader.rs` tests:

```rust
    #[test]
    fn load_full_game() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/full_game");
        let data = load_game_data(&dir).unwrap();

        assert_eq!(data.registry.item_count(), 3);
        assert_eq!(data.registry.building_count(), 5);

        assert!(data.power_config.is_some());
        assert!(data.fluid_config.is_some());
        assert!(data.tech_tree_config.is_some());
        assert!(data.logic_config.is_some());

        let power = data.power_config.unwrap();
        assert_eq!(power.generators.len(), 1);
        assert_eq!(power.consumers.len(), 1);

        let fluid = data.fluid_config.unwrap();
        assert_eq!(fluid.fluid_types.len(), 1);
        assert_eq!(fluid.producers.len(), 1);
        assert_eq!(fluid.storage.len(), 1);

        let tech = data.tech_tree_config.unwrap();
        assert_eq!(tech.technologies.len(), 1);
        assert_eq!(tech.technologies[0].unlocks.len(), 2);

        let logic = data.logic_config.unwrap();
        assert_eq!(logic.circuit_controlled.len(), 1);
        assert_eq!(logic.constant_combinators.len(), 1);
    }
```

**Step 3: Run tests**

Run: `cargo test --package factorial-data`
Expected: all tests pass

**Step 4: Commit**

```bash
git add crates/factorial-data/
git commit -m "feat(data): module config loading — power, fluid, tech-tree, logic"
```

---

## Task 8: Error Path Tests

**Files:**
- Create: `crates/factorial-data/test_data/errors/` (multiple fixture dirs)
- Modify: `crates/factorial-data/src/loader.rs` (test section)

**Step 1: Create error fixture files**

`crates/factorial-data/test_data/errors/unresolved_item/items.ron`:
```ron
[(name: "iron_ore")]
```
`crates/factorial-data/test_data/errors/unresolved_item/recipes.ron`:
```ron
[(name: "bad", inputs: [("nonexistent", 1)], outputs: [], duration: 60)]
```
`crates/factorial-data/test_data/errors/unresolved_item/buildings.ron`:
```ron
[]
```

`crates/factorial-data/test_data/errors/duplicate_name/items.ron`:
```ron
[(name: "iron_ore"), (name: "iron_ore")]
```
`crates/factorial-data/test_data/errors/duplicate_name/recipes.ron`:
```ron
[]
```
`crates/factorial-data/test_data/errors/duplicate_name/buildings.ron`:
```ron
[]
```

`crates/factorial-data/test_data/errors/missing_items/recipes.ron`:
```ron
[]
```
`crates/factorial-data/test_data/errors/missing_items/buildings.ron`:
```ron
[]
```

`crates/factorial-data/test_data/errors/parse_error/items.ron`:
```
this is not valid ron {{{
```
`crates/factorial-data/test_data/errors/parse_error/recipes.ron`:
```ron
[]
```
`crates/factorial-data/test_data/errors/parse_error/buildings.ron`:
```ron
[]
```

**Step 2: Write error path tests**

Add to `loader.rs` tests:

```rust
    #[test]
    fn error_unresolved_item_in_recipe() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/errors/unresolved_item");
        let result = load_game_data(&dir);
        assert!(matches!(result, Err(DataLoadError::UnresolvedRef { .. })));
        if let Err(DataLoadError::UnresolvedRef { name, .. }) = result {
            assert_eq!(name, "nonexistent");
        }
    }

    #[test]
    fn error_duplicate_item_name() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/errors/duplicate_name");
        let result = load_game_data(&dir);
        assert!(matches!(result, Err(DataLoadError::DuplicateName { .. })));
        if let Err(DataLoadError::DuplicateName { name, .. }) = result {
            assert_eq!(name, "iron_ore");
        }
    }

    #[test]
    fn error_missing_required_file() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/errors/missing_items");
        let result = load_game_data(&dir);
        assert!(matches!(result, Err(DataLoadError::MissingRequired { .. })));
    }

    #[test]
    fn error_parse_error() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/errors/parse_error");
        let result = load_game_data(&dir);
        assert!(matches!(result, Err(DataLoadError::Parse { .. })));
    }
```

**Step 3: Run tests**

Run: `cargo test --package factorial-data`
Expected: all tests pass

**Step 4: Commit**

```bash
git add crates/factorial-data/test_data/errors/ crates/factorial-data/src/loader.rs
git commit -m "test(data): add error path tests — unresolved refs, duplicates, missing files, parse errors"
```

---

## Task 9: JSON Format Equivalence Test

**Files:**
- Create: `crates/factorial-data/test_data/minimal_json/` (3 files)
- Modify: `crates/factorial-data/src/loader.rs` (test section)

**Step 1: Create JSON fixture files**

`crates/factorial-data/test_data/minimal_json/items.json`:
```json
[
    {"name": "iron_ore"},
    {"name": "iron_plate"},
    {"name": "copper_ore"}
]
```

`crates/factorial-data/test_data/minimal_json/recipes.json`:
```json
[
    {"name": "smelt_iron", "inputs": [["iron_ore", 1]], "outputs": [["iron_plate", 1]], "duration": 60}
]
```

`crates/factorial-data/test_data/minimal_json/buildings.json`:
```json
[
    {
        "name": "iron_mine",
        "processor": {"Source": {"item": "iron_ore", "rate": 2.0}},
        "footprint": {"width": 2, "height": 2}
    },
    {
        "name": "smelter",
        "processor": {"Recipe": {"recipe": "smelt_iron"}},
        "footprint": {"width": 3, "height": 3},
        "inventories": {"input_capacity": 200, "output_capacity": 200}
    },
    {
        "name": "chest",
        "processor": {"Demand": {"items": ["iron_plate"]}}
    }
]
```

**Step 2: Write equivalence test**

```rust
    #[test]
    fn format_equivalence_ron_json() {
        let ron_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/minimal_ron");
        let json_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/minimal_json");

        let ron_data = load_game_data(&ron_dir).unwrap();
        let json_data = load_game_data(&json_dir).unwrap();

        assert_eq!(ron_data.registry.item_count(), json_data.registry.item_count());
        assert_eq!(ron_data.registry.recipe_count(), json_data.registry.recipe_count());
        assert_eq!(ron_data.registry.building_count(), json_data.registry.building_count());

        // Name lookups produce same IDs
        assert_eq!(ron_data.registry.item_id("iron_ore"), json_data.registry.item_id("iron_ore"));
        assert_eq!(
            ron_data.building_footprints.len(),
            json_data.building_footprints.len()
        );
    }
```

**Step 3: Run tests**

Run: `cargo test --package factorial-data`
Expected: all tests pass

**Step 4: Commit**

```bash
git add crates/factorial-data/test_data/minimal_json/ crates/factorial-data/src/loader.rs
git commit -m "test(data): add JSON fixtures and format equivalence test"
```

---

## Task 10: Full Integration Test — Load, Build Engine, Run Ticks

**Files:**
- Modify: `crates/factorial-data/src/loader.rs` (test section)

**Step 1: Write the integration test**

```rust
    #[test]
    fn integration_load_build_run() {
        use factorial_core::engine::Engine;
        use factorial_core::sim::SimulationStrategy;

        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/minimal_ron");
        let data = load_game_data(&dir).unwrap();

        // Build engine with loaded registry
        let mut engine = Engine::new_with_registry(SimulationStrategy::Tick, data.registry);

        // Add a mine node using the loaded building type
        let mine_type = engine.registry().unwrap().building_id("iron_mine").unwrap();
        let pending = engine.graph.queue_add_node(mine_type);
        let result = engine.graph.apply_mutations();
        let mine = result.resolve_node(pending).unwrap();

        // Apply loaded processor and inventory config
        let processor = data.building_processors[&mine_type].clone();
        engine.set_processor(mine, processor);
        let (in_cap, out_cap) = data.building_inventories[&mine_type];
        engine.set_input_inventory(mine, factorial_core::item::Inventory::new(1, 1, in_cap));
        engine.set_output_inventory(mine, factorial_core::item::Inventory::new(1, 1, out_cap));

        // Run 10 ticks
        for _ in 0..10 {
            engine.step();
        }

        // Mine should have produced items
        let snaps = engine.snapshot_all_nodes();
        assert_eq!(snaps.len(), 1);
        assert!(!snaps[0].output_contents.is_empty(), "mine should have produced output");
    }
```

**Step 2: Run tests**

Run: `cargo test --package factorial-data -- integration_load_build_run`
Expected: PASS

**Step 3: Run full suite + clippy**

Run: `cargo test --package factorial-data`
Run: `cargo clippy --package factorial-data -- -D warnings`
Run: `cargo fmt --all -- --check`
Expected: all pass

**Step 4: Commit**

```bash
git add crates/factorial-data/src/loader.rs
git commit -m "test(data): add full integration test — load, build engine, run ticks"
```

---

## Task 11: CI Compliance — Clippy, Fmt, Full Workspace Tests

**Step 1: Run workspace-wide checks**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Fix any warnings or failures. Common issues:
- Unused imports in module_config.rs (remove unused module imports)
- Missing `#[allow(dead_code)]` if PowerConfig/FluidConfig build_module methods aren't called from tests
- Type mismatches between schema f64 values and Fixed64

**Step 2: Commit fixes if any**

```bash
git add -A
git commit -m "chore(data): fix clippy warnings and formatting"
```

---

## Summary

| Task | Description | Estimated Lines |
|------|-------------|----------------|
| 1 | Scaffold crate | ~40 |
| 2 | Core schema structs | ~150 |
| 3 | Module schema structs | ~200 |
| 4 | Loader foundation | ~150 |
| 5 | Engine::new_with_registry | ~20 |
| 6 | Core loading pipeline | ~250 |
| 7 | Module config + loading | ~350 |
| 8 | Error path tests | ~60 |
| 9 | JSON equivalence test | ~40 |
| 10 | Full integration test | ~40 |
| 11 | CI compliance | ~10 |
| **Total** | | **~1310** |
