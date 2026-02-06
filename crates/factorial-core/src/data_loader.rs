//! Data-driven registry loading from JSON.
//!
//! Feature-gated behind `data-loader`. Provides JSON deserialization into
//! [`RegistryBuilder`] for game content defined in data files.

use crate::fixed::{Fixed32, Fixed64};
use crate::registry::{
    PropertyDef, PropertyDefault, PropertySize, RecipeEntry, RegistryBuilder, RegistryError,
};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during data loading.
#[derive(Debug, thiserror::Error)]
pub enum DataLoadError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("registry error: {0}")]
    Registry(#[from] RegistryError),
    #[error("unknown item reference: {0}")]
    UnknownItemRef(String),
    #[error("unknown recipe reference: {0}")]
    UnknownRecipeRef(String),
}

// ---------------------------------------------------------------------------
// JSON data structures
// ---------------------------------------------------------------------------

/// Top-level registry data structure for JSON deserialization.
#[derive(Debug, serde::Deserialize)]
pub struct RegistryData {
    #[serde(default)]
    pub items: Vec<ItemData>,
    #[serde(default)]
    pub recipes: Vec<RecipeData>,
    #[serde(default)]
    pub buildings: Vec<BuildingData>,
}

/// JSON representation of an item type.
#[derive(Debug, serde::Deserialize)]
pub struct ItemData {
    pub name: String,
    #[serde(default)]
    pub properties: Vec<PropertyData>,
}

/// JSON representation of a property definition.
#[derive(Debug, serde::Deserialize)]
pub struct PropertyData {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: String, // "fixed64", "fixed32", "u32", "u8"
    #[serde(default)]
    pub default: Option<f64>,
}

/// JSON representation of a recipe.
#[derive(Debug, serde::Deserialize)]
pub struct RecipeData {
    pub name: String,
    #[serde(default)]
    pub inputs: Vec<RecipeEntryData>,
    #[serde(default)]
    pub outputs: Vec<RecipeEntryData>,
    pub duration: u64,
}

/// JSON representation of a recipe input/output entry.
#[derive(Debug, serde::Deserialize)]
pub struct RecipeEntryData {
    pub item: String, // references item by name
    pub quantity: u32,
}

/// JSON representation of a building template.
#[derive(Debug, serde::Deserialize)]
pub struct BuildingData {
    pub name: String,
    pub recipe: Option<String>, // references recipe by name
}

// ---------------------------------------------------------------------------
// Loading functions
// ---------------------------------------------------------------------------

/// Load a registry from a JSON string.
pub fn load_registry_json(json: &str) -> Result<RegistryBuilder, DataLoadError> {
    let data: RegistryData = serde_json::from_str(json)?;
    build_registry(data)
}

/// Load a registry from JSON bytes.
pub fn load_registry_json_bytes(bytes: &[u8]) -> Result<RegistryBuilder, DataLoadError> {
    let data: RegistryData = serde_json::from_slice(bytes)?;
    build_registry(data)
}

fn parse_property(prop: &PropertyData) -> PropertyDef {
    let (size, default) = match prop.prop_type.as_str() {
        "fixed64" => {
            let val = prop.default.unwrap_or(0.0);
            (
                PropertySize::Fixed64,
                PropertyDefault::Fixed64(Fixed64::from_num(val)),
            )
        }
        "fixed32" => {
            let val = prop.default.unwrap_or(0.0);
            (
                PropertySize::Fixed32,
                PropertyDefault::Fixed32(Fixed32::from_num(val)),
            )
        }
        "u32" => {
            let val = prop.default.unwrap_or(0.0) as u32;
            (PropertySize::U32, PropertyDefault::U32(val))
        }
        "u8" => {
            let val = prop.default.unwrap_or(0.0) as u8;
            (PropertySize::U8, PropertyDefault::U8(val))
        }
        _ => {
            // Default to u32 for unknown types
            let val = prop.default.unwrap_or(0.0) as u32;
            (PropertySize::U32, PropertyDefault::U32(val))
        }
    };

    PropertyDef {
        name: prop.name.clone(),
        size,
        default,
    }
}

fn build_registry(data: RegistryData) -> Result<RegistryBuilder, DataLoadError> {
    let mut builder = RegistryBuilder::new();

    // Phase 1: Register all items
    for item in &data.items {
        let properties: Vec<PropertyDef> = item.properties.iter().map(parse_property).collect();
        builder.register_item(&item.name, properties);
    }

    // Phase 2: Register all recipes (resolve item refs by name)
    for recipe in &data.recipes {
        let mut inputs = Vec::new();
        for entry in &recipe.inputs {
            let item_id = builder
                .item_id(&entry.item)
                .ok_or_else(|| DataLoadError::UnknownItemRef(entry.item.clone()))?;
            inputs.push(RecipeEntry {
                item: item_id,
                quantity: entry.quantity,
            });
        }

        let mut outputs = Vec::new();
        for entry in &recipe.outputs {
            let item_id = builder
                .item_id(&entry.item)
                .ok_or_else(|| DataLoadError::UnknownItemRef(entry.item.clone()))?;
            outputs.push(RecipeEntry {
                item: item_id,
                quantity: entry.quantity,
            });
        }

        builder.register_recipe(&recipe.name, inputs, outputs, recipe.duration);
    }

    // Phase 3: Register all buildings (resolve recipe refs by name)
    for building in &data.buildings {
        let recipe_id = match &building.recipe {
            Some(name) => {
                let id = builder
                    .recipe_id(name)
                    .ok_or_else(|| DataLoadError::UnknownRecipeRef(name.clone()))?;
                Some(id)
            }
            None => None,
        };
        builder.register_building(&building.name, recipe_id);
    }

    Ok(builder)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_empty_json() {
        let json = r#"{"items": [], "recipes": [], "buildings": []}"#;
        let builder = load_registry_json(json).unwrap();
        let reg = builder.build().unwrap();
        assert_eq!(reg.item_count(), 0);
        assert_eq!(reg.recipe_count(), 0);
        assert_eq!(reg.building_count(), 0);
    }

    #[test]
    fn load_items_only() {
        let json = r#"{"items": [{"name": "iron_ore"}, {"name": "copper_ore"}]}"#;
        let builder = load_registry_json(json).unwrap();
        let reg = builder.build().unwrap();
        assert_eq!(reg.item_count(), 2);
        assert!(reg.item_id("iron_ore").is_some());
        assert!(reg.item_id("copper_ore").is_some());
    }

    #[test]
    fn load_full_registry() {
        let json = r#"{
            "items": [
                {"name": "iron_ore"},
                {"name": "iron_plate"}
            ],
            "recipes": [
                {
                    "name": "smelt_iron",
                    "inputs": [{"item": "iron_ore", "quantity": 1}],
                    "outputs": [{"item": "iron_plate", "quantity": 1}],
                    "duration": 60
                }
            ],
            "buildings": [
                {"name": "smelter", "recipe": "smelt_iron"}
            ]
        }"#;
        let builder = load_registry_json(json).unwrap();
        let reg = builder.build().unwrap();
        assert_eq!(reg.item_count(), 2);
        assert_eq!(reg.recipe_count(), 1);
        assert_eq!(reg.building_count(), 1);
    }

    #[test]
    fn load_recipe_references_item_by_name() {
        let json = r#"{
            "items": [{"name": "ore"}, {"name": "plate"}],
            "recipes": [{
                "name": "smelt",
                "inputs": [{"item": "ore", "quantity": 2}],
                "outputs": [{"item": "plate", "quantity": 1}],
                "duration": 30
            }]
        }"#;
        let builder = load_registry_json(json).unwrap();
        let reg = builder.build().unwrap();
        let recipe = reg.get_recipe(reg.recipe_id("smelt").unwrap()).unwrap();
        assert_eq!(recipe.inputs.len(), 1);
        assert_eq!(recipe.inputs[0].quantity, 2);
        assert_eq!(recipe.outputs.len(), 1);
    }

    #[test]
    fn load_building_references_recipe_by_name() {
        let json = r#"{
            "items": [{"name": "a"}, {"name": "b"}],
            "recipes": [{"name": "r1", "inputs": [{"item": "a", "quantity": 1}], "outputs": [{"item": "b", "quantity": 1}], "duration": 10}],
            "buildings": [{"name": "b1", "recipe": "r1"}, {"name": "b2", "recipe": null}]
        }"#;
        let builder = load_registry_json(json).unwrap();
        let reg = builder.build().unwrap();
        let b1 = reg.get_building(reg.building_id("b1").unwrap()).unwrap();
        assert!(b1.recipe.is_some());
        let b2 = reg.get_building(reg.building_id("b2").unwrap()).unwrap();
        assert!(b2.recipe.is_none());
    }

    #[test]
    fn load_unknown_item_fails() {
        let json = r#"{
            "items": [{"name": "ore"}],
            "recipes": [{"name": "bad", "inputs": [{"item": "nonexistent", "quantity": 1}], "outputs": [], "duration": 10}]
        }"#;
        let result = load_registry_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DataLoadError::UnknownItemRef(_)));
    }

    #[test]
    fn load_unknown_recipe_fails() {
        let json = r#"{
            "items": [],
            "recipes": [],
            "buildings": [{"name": "b1", "recipe": "nonexistent"}]
        }"#;
        let result = load_registry_json(json);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DataLoadError::UnknownRecipeRef(_)
        ));
    }

    #[test]
    fn load_item_with_properties() {
        let json = r#"{
            "items": [{
                "name": "water",
                "properties": [
                    {"name": "temperature", "type": "fixed32", "default": 20.0},
                    {"name": "pressure", "type": "u32", "default": 101}
                ]
            }]
        }"#;
        let builder = load_registry_json(json).unwrap();
        let reg = builder.build().unwrap();
        let water = reg.item_id("water").unwrap();
        assert!(reg.item_has_properties(water));
    }

    #[test]
    fn load_invalid_json_fails() {
        let result = load_registry_json("not valid json {{{");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataLoadError::JsonParse(_)));
    }

    #[test]
    fn load_builds_successfully() {
        let json = r#"{
            "items": [{"name": "a"}, {"name": "b"}, {"name": "c"}],
            "recipes": [
                {"name": "r1", "inputs": [{"item": "a", "quantity": 1}], "outputs": [{"item": "b", "quantity": 2}], "duration": 5},
                {"name": "r2", "inputs": [{"item": "b", "quantity": 1}], "outputs": [{"item": "c", "quantity": 1}], "duration": 10}
            ],
            "buildings": [
                {"name": "assembler", "recipe": "r1"},
                {"name": "refinery", "recipe": "r2"},
                {"name": "storage", "recipe": null}
            ]
        }"#;
        let builder = load_registry_json(json).unwrap();
        let reg = builder.build().unwrap();
        assert_eq!(reg.item_count(), 3);
        assert_eq!(reg.recipe_count(), 2);
        assert_eq!(reg.building_count(), 3);
    }
}
