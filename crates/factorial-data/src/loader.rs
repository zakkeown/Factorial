//! Resolution pipeline: reads data files, resolves cross-references, builds registry.
//!
//! Provides format detection (RON/JSON/TOML), file discovery, and deserialization
//! helpers used by the higher-level loading pipeline.

use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::processor::*;
use factorial_core::registry::*;
use factorial_spatial::BuildingFootprint;
use serde::de::DeserializeOwned;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::module_config::*;
use crate::schema::*;

// ===========================================================================
// Errors
// ===========================================================================

/// Errors that can occur during data loading.
#[derive(Debug, thiserror::Error)]
pub enum DataLoadError {
    /// A required data file was not found in the given directory.
    #[error("required file '{file}' not found in {dir}")]
    MissingRequired { file: &'static str, dir: PathBuf },

    /// The file has an extension we don't support.
    #[error("unsupported format for file: {file}")]
    UnsupportedFormat { file: PathBuf },

    /// Two files with the same base name but different formats exist.
    #[error("conflicting formats: {a} and {b}")]
    ConflictingFormats { a: PathBuf, b: PathBuf },

    /// A deserialization error occurred.
    #[error("parse error in {file}: {detail}")]
    Parse { file: PathBuf, detail: String },

    /// A name reference could not be resolved.
    #[error("unresolved {expected_kind} reference '{name}' in {file}")]
    UnresolvedRef {
        file: PathBuf,
        name: String,
        expected_kind: &'static str,
    },

    /// A duplicate name was found.
    #[error("duplicate name '{name}' in {file}")]
    DuplicateName { file: PathBuf, name: String },

    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// ===========================================================================
// Format detection
// ===========================================================================

/// Supported data file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Ron,
    Toml,
    Json,
}

/// Detect the format of a file based on its extension.
pub fn detect_format(path: &Path) -> Result<Format, DataLoadError> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ron") => Ok(Format::Ron),
        Some("toml") => Ok(Format::Toml),
        Some("json") => Ok(Format::Json),
        _ => Err(DataLoadError::UnsupportedFormat {
            file: path.to_path_buf(),
        }),
    }
}

// ===========================================================================
// File discovery
// ===========================================================================

/// Scan a directory for a data file with the given base name (without extension).
///
/// Looks for `{base_name}.ron`, `{base_name}.toml`, and `{base_name}.json`.
/// Returns `Ok(None)` if no file is found, or `Err(ConflictingFormats)` if
/// multiple formats exist for the same base name.
pub fn find_data_file(dir: &Path, base_name: &str) -> Result<Option<PathBuf>, DataLoadError> {
    let extensions = ["ron", "toml", "json"];
    let mut found: Option<PathBuf> = None;

    for ext in &extensions {
        let candidate = dir.join(format!("{base_name}.{ext}"));
        if candidate.exists() {
            if let Some(ref existing) = found {
                return Err(DataLoadError::ConflictingFormats {
                    a: existing.clone(),
                    b: candidate,
                });
            }
            found = Some(candidate);
        }
    }

    Ok(found)
}

/// Like [`find_data_file`], but returns an error if no file is found.
pub fn require_data_file(dir: &Path, base_name: &str) -> Result<PathBuf, DataLoadError> {
    find_data_file(dir, base_name)?.ok_or_else(|| DataLoadError::MissingRequired {
        file: Box::leak(base_name.to_string().into_boxed_str()),
        dir: dir.to_path_buf(),
    })
}

// ===========================================================================
// Deserialization
// ===========================================================================

/// Read a file and deserialize it according to its format (detected from extension).
pub fn deserialize_file<T: DeserializeOwned>(path: &Path) -> Result<T, DataLoadError> {
    let format = detect_format(path)?;
    let content = std::fs::read_to_string(path)?;

    match format {
        Format::Ron => ron::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
        Format::Json => serde_json::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
        Format::Toml => toml::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
    }
}

/// Deserialize a list from a file. For TOML files, extracts the array at the
/// given `toml_key` from a top-level table. For RON and JSON, deserializes
/// directly as `Vec<T>`.
pub fn deserialize_list<T: DeserializeOwned>(
    path: &Path,
    toml_key: &str,
) -> Result<Vec<T>, DataLoadError> {
    let format = detect_format(path)?;
    let content = std::fs::read_to_string(path)?;

    match format {
        Format::Ron => ron::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
        Format::Json => serde_json::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
        Format::Toml => {
            let table: toml::Value =
                toml::from_str(&content).map_err(|e| DataLoadError::Parse {
                    file: path.to_path_buf(),
                    detail: e.to_string(),
                })?;
            let array = table
                .get(toml_key)
                .ok_or_else(|| DataLoadError::Parse {
                    file: path.to_path_buf(),
                    detail: format!("missing key '{toml_key}' in TOML file"),
                })?
                .clone();
            // Deserialize the array value into Vec<T>.
            array
                .try_into()
                .map_err(|e: toml::de::Error| DataLoadError::Parse {
                    file: path.to_path_buf(),
                    detail: e.to_string(),
                })
        }
    }
}

// ===========================================================================
// Name resolution helpers
// ===========================================================================

/// Look up a name in a map, returning an `UnresolvedRef` error if not found.
pub fn resolve_name<'a, V>(
    map: &'a HashMap<String, V>,
    name: &str,
    file: &Path,
    expected_kind: &'static str,
) -> Result<&'a V, DataLoadError> {
    map.get(name).ok_or_else(|| DataLoadError::UnresolvedRef {
        file: file.to_path_buf(),
        name: name.to_string(),
        expected_kind,
    })
}

/// Check whether a name already exists in a map, returning a `DuplicateName`
/// error if so.
pub fn check_duplicate<V>(
    map: &HashMap<String, V>,
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

// ===========================================================================
// GameData and loading pipeline
// ===========================================================================

/// Aggregated game data loaded from data files. Contains the built
/// registry plus per-building metadata (footprints, processors, inventories)
/// and optional module configurations.
pub struct GameData {
    pub registry: Registry,
    pub building_footprints: BTreeMap<BuildingTypeId, BuildingFootprint>,
    pub building_processors: BTreeMap<BuildingTypeId, Processor>,
    pub building_inventories: BTreeMap<BuildingTypeId, (u32, u32)>,
    pub power_config: Option<PowerConfig>,
    pub fluid_config: Option<FluidConfig>,
    pub tech_tree_config: Option<TechTreeConfig>,
    pub logic_config: Option<LogicConfig>,
}

/// Load all game data from a directory of data files.
///
/// The directory must contain `items`, `recipes`, and `buildings` files
/// (in RON, JSON, or TOML format). It may optionally contain `power`,
/// `fluids`, `tech_tree`, and `logic` files for module configuration.
///
/// # Errors
///
/// Returns `DataLoadError` if required files are missing, data cannot be
/// parsed, or name references cannot be resolved.
pub fn load_game_data(dir: &Path) -> Result<GameData, DataLoadError> {
    // ------------------------------------------------------------------
    // 1. Discover required and optional files
    // ------------------------------------------------------------------
    let items_path = require_data_file(dir, "items")?;
    let recipes_path = require_data_file(dir, "recipes")?;
    let buildings_path = require_data_file(dir, "buildings")?;

    let power_path = find_data_file(dir, "power")?;
    let fluids_path = find_data_file(dir, "fluids")?;
    let tech_tree_path = find_data_file(dir, "tech_tree")?;
    let logic_path = find_data_file(dir, "logic")?;

    // ------------------------------------------------------------------
    // 2. Load items -> build name-to-ID map, register in builder
    // ------------------------------------------------------------------
    let items_data: Vec<ItemData> = deserialize_list(&items_path, "items")?;
    let mut builder = RegistryBuilder::new();
    let mut item_names: HashMap<String, ItemTypeId> = HashMap::new();

    for item in &items_data {
        check_duplicate(&item_names, &item.name, &items_path)?;
        let id = builder.register_item(&item.name, vec![]);
        item_names.insert(item.name.clone(), id);
    }

    // ------------------------------------------------------------------
    // 3. Load recipes -> resolve item names, register in builder
    // ------------------------------------------------------------------
    let recipes_data: Vec<RecipeData> = deserialize_list(&recipes_path, "recipes")?;
    let mut recipe_names: HashMap<String, RecipeId> = HashMap::new();

    for recipe in &recipes_data {
        check_duplicate(&recipe_names, &recipe.name, &recipes_path)?;

        let inputs: Vec<RecipeEntry> = recipe
            .inputs
            .iter()
            .map(|(name, qty)| {
                let id = resolve_name(&item_names, name, &recipes_path, "item")?;
                Ok(RecipeEntry {
                    item: *id,
                    quantity: *qty,
                })
            })
            .collect::<Result<Vec<_>, DataLoadError>>()?;

        let outputs: Vec<RecipeEntry> = recipe
            .outputs
            .iter()
            .map(|(name, qty)| {
                let id = resolve_name(&item_names, name, &recipes_path, "item")?;
                Ok(RecipeEntry {
                    item: *id,
                    quantity: *qty,
                })
            })
            .collect::<Result<Vec<_>, DataLoadError>>()?;

        let id = builder.register_recipe(&recipe.name, inputs, outputs, recipe.duration);
        recipe_names.insert(recipe.name.clone(), id);
    }

    // ------------------------------------------------------------------
    // 4. Load buildings -> resolve references, build processors, register
    // ------------------------------------------------------------------
    let buildings_data: Vec<BuildingData> = deserialize_list(&buildings_path, "buildings")?;
    let mut building_names: HashMap<String, BuildingTypeId> = HashMap::new();
    let mut building_footprints: BTreeMap<BuildingTypeId, BuildingFootprint> = BTreeMap::new();
    let mut building_processors: BTreeMap<BuildingTypeId, Processor> = BTreeMap::new();
    let mut building_inventories: BTreeMap<BuildingTypeId, (u32, u32)> = BTreeMap::new();

    for bld in &buildings_data {
        check_duplicate(&building_names, &bld.name, &buildings_path)?;

        // Determine the recipe reference for the registry (Some only for Recipe processors).
        let recipe_ref = match &bld.processor {
            ProcessorData::Recipe { recipe } => {
                let rid = resolve_name(&recipe_names, recipe, &buildings_path, "recipe")?;
                Some(*rid)
            }
            _ => None,
        };

        let building_id = builder.register_building(&bld.name, recipe_ref);
        building_names.insert(bld.name.clone(), building_id);

        // Build processor from ProcessorData.
        let processor = resolve_processor(
            &bld.processor,
            &item_names,
            &recipe_names,
            &builder,
            &buildings_path,
        )?;
        building_processors.insert(building_id, processor);

        // Footprint.
        building_footprints.insert(
            building_id,
            BuildingFootprint {
                width: bld.footprint.width,
                height: bld.footprint.height,
            },
        );

        // Inventory capacities.
        building_inventories.insert(
            building_id,
            (
                bld.inventories.input_capacity,
                bld.inventories.output_capacity,
            ),
        );
    }

    // ------------------------------------------------------------------
    // 5. Build the registry
    // ------------------------------------------------------------------
    let registry = builder.build().map_err(|e| DataLoadError::Parse {
        file: buildings_path.clone(),
        detail: e.to_string(),
    })?;

    // ------------------------------------------------------------------
    // 6. Optional module configs
    // ------------------------------------------------------------------
    let power_config = match power_path {
        Some(path) => Some(crate::module_config::load_power_config(
            &path,
            &building_names,
        )?),
        None => None,
    };

    let fluid_config = match fluids_path {
        Some(path) => Some(crate::module_config::load_fluid_config(
            &path,
            &item_names,
            &building_names,
        )?),
        None => None,
    };

    let tech_tree_config = match tech_tree_path {
        Some(path) => Some(crate::module_config::load_tech_tree_config(
            &path,
            &item_names,
            &recipe_names,
            &building_names,
        )?),
        None => None,
    };

    let logic_config = match logic_path {
        Some(path) => Some(crate::module_config::load_logic_config(
            &path,
            &item_names,
            &building_names,
        )?),
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

/// Resolve a `ProcessorData` (from schema) into a `Processor` (engine type).
fn resolve_processor(
    data: &ProcessorData,
    item_names: &HashMap<String, ItemTypeId>,
    recipe_names: &HashMap<String, RecipeId>,
    builder: &RegistryBuilder,
    file: &Path,
) -> Result<Processor, DataLoadError> {
    match data {
        ProcessorData::Source { item, rate } => {
            let item_id = resolve_name(item_names, item, file, "item")?;
            Ok(Processor::Source(SourceProcessor {
                output_type: *item_id,
                base_rate: Fixed64::from_num(*rate),
                depletion: Depletion::Infinite,
                accumulated: Fixed64::from_num(0),
                initial_properties: None,
            }))
        }
        ProcessorData::Recipe { recipe } => {
            let recipe_id = resolve_name(recipe_names, recipe, file, "recipe")?;
            let recipe_def =
                builder
                    .get_recipe(*recipe_id)
                    .ok_or_else(|| DataLoadError::UnresolvedRef {
                        file: file.to_path_buf(),
                        name: recipe.clone(),
                        expected_kind: "recipe",
                    })?;
            Ok(Processor::Fixed(FixedRecipe {
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
            }))
        }
        ProcessorData::Demand { items } => {
            let resolved: Vec<ItemTypeId> = items
                .iter()
                .map(|name| {
                    let id = resolve_name(item_names, name, file, "item")?;
                    Ok(*id)
                })
                .collect::<Result<Vec<_>, DataLoadError>>()?;

            let first = resolved.first().ok_or_else(|| DataLoadError::Parse {
                file: file.to_path_buf(),
                detail: "Demand processor must have at least one item".to_string(),
            })?;

            Ok(Processor::Demand(DemandProcessor {
                input_type: *first,
                base_rate: Fixed64::from_num(1),
                accumulated: Fixed64::from_num(0),
                consumed_total: 0,
                accepted_types: if resolved.len() > 1 {
                    Some(resolved)
                } else {
                    None
                },
            }))
        }
        ProcessorData::Passthrough => Ok(Processor::Passthrough),
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temporary directory with a unique name for test isolation.
    fn make_test_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "factorial_data_test_{suffix}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Clean up a test directory.
    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    // -----------------------------------------------------------------------
    // detect_format
    // -----------------------------------------------------------------------

    #[test]
    fn detect_format_ron() {
        assert_eq!(detect_format(Path::new("items.ron")).unwrap(), Format::Ron);
    }

    #[test]
    fn detect_format_toml() {
        assert_eq!(
            detect_format(Path::new("items.toml")).unwrap(),
            Format::Toml
        );
    }

    #[test]
    fn detect_format_json() {
        assert_eq!(
            detect_format(Path::new("items.json")).unwrap(),
            Format::Json
        );
    }

    #[test]
    fn detect_format_unsupported() {
        let result = detect_format(Path::new("items.yaml"));
        assert!(matches!(
            result,
            Err(DataLoadError::UnsupportedFormat { .. })
        ));
    }

    #[test]
    fn detect_format_no_extension() {
        let result = detect_format(Path::new("items"));
        assert!(matches!(
            result,
            Err(DataLoadError::UnsupportedFormat { .. })
        ));
    }

    // -----------------------------------------------------------------------
    // find_data_file
    // -----------------------------------------------------------------------

    #[test]
    fn find_data_file_found_ron() {
        let dir = make_test_dir("find_ron");
        fs::write(dir.join("items.ron"), "[]").unwrap();

        let result = find_data_file(&dir, "items").unwrap();
        assert_eq!(result, Some(dir.join("items.ron")));

        cleanup(&dir);
    }

    #[test]
    fn find_data_file_found_json() {
        let dir = make_test_dir("find_json");
        fs::write(dir.join("items.json"), "[]").unwrap();

        let result = find_data_file(&dir, "items").unwrap();
        assert_eq!(result, Some(dir.join("items.json")));

        cleanup(&dir);
    }

    #[test]
    fn find_data_file_found_toml() {
        let dir = make_test_dir("find_toml");
        fs::write(dir.join("items.toml"), "").unwrap();

        let result = find_data_file(&dir, "items").unwrap();
        assert_eq!(result, Some(dir.join("items.toml")));

        cleanup(&dir);
    }

    #[test]
    fn find_data_file_missing() {
        let dir = make_test_dir("find_missing");

        let result = find_data_file(&dir, "items").unwrap();
        assert_eq!(result, None);

        cleanup(&dir);
    }

    #[test]
    fn find_data_file_conflict() {
        let dir = make_test_dir("find_conflict");
        fs::write(dir.join("items.ron"), "[]").unwrap();
        fs::write(dir.join("items.json"), "[]").unwrap();

        let result = find_data_file(&dir, "items");
        assert!(matches!(
            result,
            Err(DataLoadError::ConflictingFormats { .. })
        ));

        cleanup(&dir);
    }

    // -----------------------------------------------------------------------
    // require_data_file
    // -----------------------------------------------------------------------

    #[test]
    fn require_data_file_found() {
        let dir = make_test_dir("require_found");
        fs::write(dir.join("items.ron"), "[]").unwrap();

        let result = require_data_file(&dir, "items").unwrap();
        assert_eq!(result, dir.join("items.ron"));

        cleanup(&dir);
    }

    #[test]
    fn require_data_file_missing() {
        let dir = make_test_dir("require_missing");

        let result = require_data_file(&dir, "items");
        assert!(matches!(result, Err(DataLoadError::MissingRequired { .. })));

        cleanup(&dir);
    }

    // -----------------------------------------------------------------------
    // deserialize_file
    // -----------------------------------------------------------------------

    #[test]
    fn deserialize_file_ron() {
        let dir = make_test_dir("deser_ron");
        let path = dir.join("items.ron");
        fs::write(&path, r#"[(name: "iron_ore"), (name: "copper_ore")]"#).unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_file(&path).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "iron_ore");

        cleanup(&dir);
    }

    #[test]
    fn deserialize_file_json() {
        let dir = make_test_dir("deser_json");
        let path = dir.join("items.json");
        fs::write(&path, r#"[{"name": "iron_ore"}, {"name": "copper_ore"}]"#).unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_file(&path).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "iron_ore");

        cleanup(&dir);
    }

    #[test]
    fn deserialize_file_toml() {
        let dir = make_test_dir("deser_toml");
        let path = dir.join("items.toml");
        fs::write(
            &path,
            r#"
[[items]]
name = "iron_ore"

[[items]]
name = "copper_ore"
"#,
        )
        .unwrap();

        let wrapper: crate::schema::TomlItems = deserialize_file(&path).unwrap();
        assert_eq!(wrapper.items.len(), 2);
        assert_eq!(wrapper.items[0].name, "iron_ore");

        cleanup(&dir);
    }

    #[test]
    fn deserialize_file_parse_error() {
        let dir = make_test_dir("deser_parse_err");
        let path = dir.join("bad.ron");
        fs::write(&path, "this is not valid RON {{{").unwrap();

        let result: Result<Vec<crate::schema::ItemData>, _> = deserialize_file(&path);
        assert!(matches!(result, Err(DataLoadError::Parse { .. })));

        cleanup(&dir);
    }

    #[test]
    fn deserialize_file_unsupported_format() {
        let dir = make_test_dir("deser_unsupported");
        let path = dir.join("items.yaml");
        fs::write(&path, "").unwrap();

        let result: Result<Vec<crate::schema::ItemData>, _> = deserialize_file(&path);
        assert!(matches!(
            result,
            Err(DataLoadError::UnsupportedFormat { .. })
        ));

        cleanup(&dir);
    }

    // -----------------------------------------------------------------------
    // deserialize_list
    // -----------------------------------------------------------------------

    #[test]
    fn deserialize_list_ron() {
        let dir = make_test_dir("list_ron");
        let path = dir.join("items.ron");
        fs::write(&path, r#"[(name: "iron_ore"), (name: "copper_ore")]"#).unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_list(&path, "items").unwrap();
        assert_eq!(items.len(), 2);

        cleanup(&dir);
    }

    #[test]
    fn deserialize_list_json() {
        let dir = make_test_dir("list_json");
        let path = dir.join("items.json");
        fs::write(&path, r#"[{"name": "iron_ore"}, {"name": "copper_ore"}]"#).unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_list(&path, "items").unwrap();
        assert_eq!(items.len(), 2);

        cleanup(&dir);
    }

    #[test]
    fn deserialize_list_toml() {
        let dir = make_test_dir("list_toml");
        let path = dir.join("items.toml");
        fs::write(
            &path,
            r#"
[[items]]
name = "iron_ore"

[[items]]
name = "copper_ore"
"#,
        )
        .unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_list(&path, "items").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "iron_ore");

        cleanup(&dir);
    }

    #[test]
    fn deserialize_list_toml_missing_key() {
        let dir = make_test_dir("list_toml_missing");
        let path = dir.join("items.toml");
        fs::write(&path, r#"foo = "bar""#).unwrap();

        let result: Result<Vec<crate::schema::ItemData>, _> = deserialize_list(&path, "items");
        assert!(matches!(result, Err(DataLoadError::Parse { .. })));

        cleanup(&dir);
    }

    // -----------------------------------------------------------------------
    // resolve_name / check_duplicate
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_name_found() {
        let mut map = HashMap::new();
        map.insert("iron_ore".to_string(), 42u32);

        let val = resolve_name(&map, "iron_ore", Path::new("items.ron"), "item").unwrap();
        assert_eq!(*val, 42);
    }

    #[test]
    fn resolve_name_missing() {
        let map: HashMap<String, u32> = HashMap::new();

        let result = resolve_name(&map, "iron_ore", Path::new("items.ron"), "item");
        assert!(matches!(
            result,
            Err(DataLoadError::UnresolvedRef { ref name, expected_kind: "item", .. }) if name == "iron_ore"
        ));
    }

    #[test]
    fn check_duplicate_no_dup() {
        let map: HashMap<String, u32> = HashMap::new();
        assert!(check_duplicate(&map, "iron_ore", Path::new("items.ron")).is_ok());
    }

    #[test]
    fn check_duplicate_has_dup() {
        let mut map = HashMap::new();
        map.insert("iron_ore".to_string(), 42u32);

        let result = check_duplicate(&map, "iron_ore", Path::new("items.ron"));
        assert!(matches!(
            result,
            Err(DataLoadError::DuplicateName { ref name, .. }) if name == "iron_ore"
        ));
    }

    // -----------------------------------------------------------------------
    // Error display messages
    // -----------------------------------------------------------------------

    #[test]
    fn error_display_messages() {
        let e = DataLoadError::MissingRequired {
            file: "items",
            dir: PathBuf::from("/data"),
        };
        assert!(format!("{e}").contains("items"));
        assert!(format!("{e}").contains("/data"));

        let e = DataLoadError::UnsupportedFormat {
            file: PathBuf::from("items.yaml"),
        };
        assert!(format!("{e}").contains("items.yaml"));

        let e = DataLoadError::ConflictingFormats {
            a: PathBuf::from("items.ron"),
            b: PathBuf::from("items.json"),
        };
        let msg = format!("{e}");
        assert!(msg.contains("items.ron"));
        assert!(msg.contains("items.json"));

        let e = DataLoadError::Parse {
            file: PathBuf::from("bad.ron"),
            detail: "syntax error".to_string(),
        };
        assert!(format!("{e}").contains("bad.ron"));
        assert!(format!("{e}").contains("syntax error"));

        let e = DataLoadError::UnresolvedRef {
            file: PathBuf::from("buildings.ron"),
            name: "iron_ore".to_string(),
            expected_kind: "item",
        };
        let msg = format!("{e}");
        assert!(msg.contains("iron_ore"));
        assert!(msg.contains("item"));

        let e = DataLoadError::DuplicateName {
            file: PathBuf::from("items.ron"),
            name: "iron_ore".to_string(),
        };
        assert!(format!("{e}").contains("iron_ore"));
    }

    // -----------------------------------------------------------------------
    // Io error conversion
    // -----------------------------------------------------------------------

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let data_err: DataLoadError = io_err.into();
        assert!(matches!(data_err, DataLoadError::Io(_)));
        assert!(format!("{data_err}").contains("file not found"));
    }

    // -----------------------------------------------------------------------
    // load_game_data integration
    // -----------------------------------------------------------------------

    #[test]
    fn load_minimal_ron() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/minimal_ron");
        let data = load_game_data(&dir).unwrap();
        assert_eq!(data.registry.item_count(), 3);
        assert_eq!(data.registry.recipe_count(), 1);
        assert_eq!(data.registry.building_count(), 3);
        assert!(data.registry.item_id("iron_ore").is_some());
        assert!(data.registry.building_id("iron_mine").is_some());
        let mine_id = data.registry.building_id("iron_mine").unwrap();
        let fp = data.building_footprints.get(&mine_id).unwrap();
        assert_eq!(fp.width, 2);
        assert_eq!(fp.height, 2);
        assert_eq!(data.building_processors.len(), 3);
        assert!(data.power_config.is_none());
    }

    // -----------------------------------------------------------------------
    // Error path tests
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // Format equivalence
    // -----------------------------------------------------------------------

    #[test]
    fn format_equivalence_ron_json() {
        let ron_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/minimal_ron");
        let json_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/minimal_json");
        let ron_data = load_game_data(&ron_dir).unwrap();
        let json_data = load_game_data(&json_dir).unwrap();
        assert_eq!(
            ron_data.registry.item_count(),
            json_data.registry.item_count()
        );
        assert_eq!(
            ron_data.registry.recipe_count(),
            json_data.registry.recipe_count()
        );
        assert_eq!(
            ron_data.registry.building_count(),
            json_data.registry.building_count()
        );
        assert_eq!(
            ron_data.registry.item_id("iron_ore"),
            json_data.registry.item_id("iron_ore")
        );
        assert_eq!(
            ron_data.building_footprints.len(),
            json_data.building_footprints.len()
        );
    }

    // -----------------------------------------------------------------------
    // Full integration test
    // -----------------------------------------------------------------------

    #[test]
    fn integration_load_build_run() {
        use factorial_core::engine::Engine;
        use factorial_core::sim::SimulationStrategy;

        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/minimal_ron");
        let data = load_game_data(&dir).unwrap();

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
        assert!(
            !snaps[0].output_contents.is_empty(),
            "mine should have produced output"
        );
    }

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
        let tech = data.tech_tree_config.unwrap();
        assert_eq!(tech.technologies.len(), 1);
        assert_eq!(tech.technologies[0].unlocks.len(), 2);
        let logic = data.logic_config.unwrap();
        assert_eq!(logic.circuit_controlled.len(), 1);
        assert_eq!(logic.constant_combinators.len(), 1);
    }
}
