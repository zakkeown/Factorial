use crate::fixed::{Fixed32, Fixed64, Ticks};
use crate::id::*;
use std::collections::HashMap;

/// Describes a property on an item type.
#[derive(Debug, Clone)]
pub struct PropertyDef {
    pub name: String,
    pub size: PropertySize,
    pub default: PropertyDefault,
}

/// Size/type of a property value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertySize {
    Fixed64,
    Fixed32,
    U32,
    U8,
}

/// Default value for a property.
#[derive(Debug, Clone, Copy)]
pub enum PropertyDefault {
    Fixed64(Fixed64),
    Fixed32(Fixed32),
    U32(u32),
    U8(u8),
}

/// An item type definition in the registry.
#[derive(Debug, Clone)]
pub struct ItemTypeDef {
    pub name: String,
    pub properties: Vec<PropertyDef>,
}

/// A recipe input/output entry.
#[derive(Debug, Clone)]
pub struct RecipeEntry {
    pub item: ItemTypeId,
    pub quantity: u32,
}

/// A recipe definition.
#[derive(Debug, Clone)]
pub struct RecipeDef {
    pub name: String,
    pub inputs: Vec<RecipeEntry>,
    pub outputs: Vec<RecipeEntry>,
    pub duration: Ticks,
}

/// A building template definition.
#[derive(Debug, Clone)]
pub struct BuildingTemplateDef {
    pub name: String,
    pub recipe: Option<RecipeId>,
}

/// Builder for constructing an immutable Registry.
/// Three-phase lifecycle: registration -> mutation -> finalization.
#[derive(Debug)]
pub struct RegistryBuilder {
    items: Vec<ItemTypeDef>,
    item_name_to_id: HashMap<String, ItemTypeId>,
    recipes: Vec<RecipeDef>,
    recipe_name_to_id: HashMap<String, RecipeId>,
    buildings: Vec<BuildingTemplateDef>,
    building_name_to_id: HashMap<String, BuildingTypeId>,
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            item_name_to_id: HashMap::new(),
            recipes: Vec::new(),
            recipe_name_to_id: HashMap::new(),
            buildings: Vec::new(),
            building_name_to_id: HashMap::new(),
        }
    }

    /// Phase 1: Register an item type. Returns its ID.
    pub fn register_item(&mut self, name: &str, properties: Vec<PropertyDef>) -> ItemTypeId {
        let id = ItemTypeId(self.items.len() as u32);
        self.items.push(ItemTypeDef {
            name: name.to_string(),
            properties,
        });
        self.item_name_to_id.insert(name.to_string(), id);
        id
    }

    /// Phase 1: Register a recipe. Returns its ID.
    pub fn register_recipe(
        &mut self,
        name: &str,
        inputs: Vec<RecipeEntry>,
        outputs: Vec<RecipeEntry>,
        duration: Ticks,
    ) -> RecipeId {
        let id = RecipeId(self.recipes.len() as u32);
        self.recipes.push(RecipeDef {
            name: name.to_string(),
            inputs,
            outputs,
            duration,
        });
        self.recipe_name_to_id.insert(name.to_string(), id);
        id
    }

    /// Phase 1: Register a building template. Returns its ID.
    pub fn register_building(&mut self, name: &str, recipe: Option<RecipeId>) -> BuildingTypeId {
        let id = BuildingTypeId(self.buildings.len() as u32);
        self.buildings.push(BuildingTemplateDef {
            name: name.to_string(),
            recipe,
        });
        self.building_name_to_id.insert(name.to_string(), id);
        id
    }

    /// Phase 2: Mutate an existing recipe by name.
    pub fn mutate_recipe<F>(&mut self, name: &str, f: F) -> Result<(), RegistryError>
    where
        F: FnOnce(&mut RecipeDef),
    {
        let id = self
            .recipe_name_to_id
            .get(name)
            .ok_or(RegistryError::NotFound(name.to_string()))?;
        f(&mut self.recipes[id.0 as usize]);
        Ok(())
    }

    /// Phase 2: Mutate an existing building template by name.
    pub fn mutate_building<F>(&mut self, name: &str, f: F) -> Result<(), RegistryError>
    where
        F: FnOnce(&mut BuildingTemplateDef),
    {
        let id = self
            .building_name_to_id
            .get(name)
            .ok_or(RegistryError::NotFound(name.to_string()))?;
        f(&mut self.buildings[id.0 as usize]);
        Ok(())
    }

    /// Lookup item type ID by name.
    pub fn item_id(&self, name: &str) -> Option<ItemTypeId> {
        self.item_name_to_id.get(name).copied()
    }

    /// Lookup recipe ID by name.
    pub fn recipe_id(&self, name: &str) -> Option<RecipeId> {
        self.recipe_name_to_id.get(name).copied()
    }

    /// Returns a reference to a recipe definition by its ID.
    pub fn get_recipe(&self, id: RecipeId) -> Option<&RecipeDef> {
        self.recipes.get(id.0 as usize)
    }

    /// Phase 3: Finalize and build the immutable registry.
    pub fn build(self) -> Result<Registry, RegistryError> {
        // Validate: all recipe item references must exist
        for recipe in &self.recipes {
            for entry in recipe.inputs.iter().chain(recipe.outputs.iter()) {
                if entry.item.0 as usize >= self.items.len() {
                    return Err(RegistryError::InvalidItemRef(entry.item));
                }
            }
        }

        Ok(Registry {
            items: self.items,
            item_name_to_id: self.item_name_to_id,
            recipes: self.recipes,
            recipe_name_to_id: self.recipe_name_to_id,
            buildings: self.buildings,
            building_name_to_id: self.building_name_to_id,
        })
    }
}

/// Immutable registry. Frozen after build(). Thread-safe to share.
#[derive(Debug)]
pub struct Registry {
    items: Vec<ItemTypeDef>,
    item_name_to_id: HashMap<String, ItemTypeId>,
    recipes: Vec<RecipeDef>,
    recipe_name_to_id: HashMap<String, RecipeId>,
    buildings: Vec<BuildingTemplateDef>,
    building_name_to_id: HashMap<String, BuildingTypeId>,
}

impl Registry {
    pub fn get_item(&self, id: ItemTypeId) -> Option<&ItemTypeDef> {
        self.items.get(id.0 as usize)
    }

    pub fn get_recipe(&self, id: RecipeId) -> Option<&RecipeDef> {
        self.recipes.get(id.0 as usize)
    }

    pub fn get_building(&self, id: BuildingTypeId) -> Option<&BuildingTemplateDef> {
        self.buildings.get(id.0 as usize)
    }

    pub fn item_id(&self, name: &str) -> Option<ItemTypeId> {
        self.item_name_to_id.get(name).copied()
    }

    pub fn recipe_id(&self, name: &str) -> Option<RecipeId> {
        self.recipe_name_to_id.get(name).copied()
    }

    pub fn building_id(&self, name: &str) -> Option<BuildingTypeId> {
        self.building_name_to_id.get(name).copied()
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn recipe_count(&self) -> usize {
        self.recipes.len()
    }

    pub fn building_count(&self) -> usize {
        self.buildings.len()
    }

    pub fn item_has_properties(&self, id: ItemTypeId) -> bool {
        self.items
            .get(id.0 as usize)
            .map(|item| !item.properties.is_empty())
            .unwrap_or(false)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid item reference: {0:?}")]
    InvalidItemRef(ItemTypeId),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_builder() -> RegistryBuilder {
        let mut b = RegistryBuilder::new();
        let iron_ore = b.register_item("iron_ore", vec![]);
        let iron_plate = b.register_item("iron_plate", vec![]);
        b.register_recipe(
            "smelt_iron",
            vec![RecipeEntry {
                item: iron_ore,
                quantity: 1,
            }],
            vec![RecipeEntry {
                item: iron_plate,
                quantity: 1,
            }],
            60,
        );
        b.register_building("smelter", b.recipe_id("smelt_iron"));
        b
    }

    #[test]
    fn register_and_build() {
        let builder = setup_builder();
        let reg = builder.build().unwrap();
        assert_eq!(reg.item_count(), 2);
        assert_eq!(reg.recipe_count(), 1);
        assert_eq!(reg.building_count(), 1);
    }

    #[test]
    fn lookup_by_name() {
        let builder = setup_builder();
        let reg = builder.build().unwrap();
        assert!(reg.item_id("iron_ore").is_some());
        assert!(reg.item_id("nonexistent").is_none());
    }

    #[test]
    fn mutate_recipe() {
        let mut builder = setup_builder();
        let chromium = builder.register_item("chromium", vec![]);
        builder
            .mutate_recipe("smelt_iron", |recipe| {
                recipe.inputs.push(RecipeEntry {
                    item: chromium,
                    quantity: 1,
                });
            })
            .unwrap();
        let reg = builder.build().unwrap();
        let recipe = reg
            .get_recipe(reg.recipe_id("smelt_iron").unwrap())
            .unwrap();
        assert_eq!(recipe.inputs.len(), 2);
    }

    #[test]
    fn mutate_nonexistent_fails() {
        let mut builder = setup_builder();
        let result = builder.mutate_recipe("nonexistent", |_| {});
        assert!(result.is_err());
    }

    #[test]
    fn invalid_item_ref_in_recipe_fails() {
        let mut b = RegistryBuilder::new();
        b.register_recipe(
            "bad",
            vec![RecipeEntry {
                item: ItemTypeId(999),
                quantity: 1,
            }],
            vec![],
            60,
        );
        assert!(b.build().is_err());
    }

    #[test]
    fn fungible_item_has_no_properties() {
        let builder = setup_builder();
        let reg = builder.build().unwrap();
        let iron = reg.item_id("iron_ore").unwrap();
        assert!(!reg.item_has_properties(iron));
    }

    #[test]
    fn stateful_item_has_properties() {
        let mut b = RegistryBuilder::new();
        b.register_item(
            "water",
            vec![PropertyDef {
                name: "temperature".to_string(),
                size: PropertySize::Fixed32,
                default: PropertyDefault::Fixed32(Fixed32::from_num(20)),
            }],
        );
        let reg = b.build().unwrap();
        let water = reg.item_id("water").unwrap();
        assert!(reg.item_has_properties(water));
    }

    #[test]
    fn registry_is_immutable_after_build() {
        // Registry has no &mut self methods -- immutability enforced by the type system.
        let builder = setup_builder();
        let reg = builder.build().unwrap();
        // Can only read:
        let _ = reg.get_item(ItemTypeId(0));
        let _ = reg.get_recipe(RecipeId(0));
        let _ = reg.get_building(BuildingTypeId(0));
    }

    // -----------------------------------------------------------------------
    // Error path tests
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_item_ref_error_variant() {
        let mut b = RegistryBuilder::new();
        b.register_recipe(
            "bad_output",
            vec![],
            vec![RecipeEntry {
                item: ItemTypeId(999),
                quantity: 1,
            }],
            60,
        );
        let result = b.build();
        assert!(result.is_err());
        match result {
            Err(RegistryError::InvalidItemRef(id)) => {
                assert_eq!(id, ItemTypeId(999));
                let msg = format!("{}", RegistryError::InvalidItemRef(id));
                assert!(msg.contains("invalid item reference"), "got: {msg}");
            }
            other => panic!("expected InvalidItemRef, got: {other:?}"),
        }
    }

    #[test]
    fn mutate_nonexistent_building_fails() {
        let mut builder = setup_builder();
        let result = builder.mutate_building("nonexistent", |_| {});
        assert!(result.is_err());
        match result {
            Err(RegistryError::NotFound(name)) => {
                assert_eq!(name, "nonexistent");
            }
            other => panic!("expected NotFound, got: {other:?}"),
        }
    }

    #[test]
    fn mutate_building_succeeds() {
        let mut builder = setup_builder();
        builder
            .mutate_building("smelter", |b| {
                b.recipe = None;
            })
            .unwrap();
        let reg = builder.build().unwrap();
        let smelter_id = reg.building_id("smelter").unwrap();
        let smelter = reg.get_building(smelter_id).unwrap();
        assert!(smelter.recipe.is_none());
    }

    #[test]
    fn registry_get_nonexistent_returns_none() {
        let builder = setup_builder();
        let reg = builder.build().unwrap();
        assert!(reg.get_item(ItemTypeId(999)).is_none());
        assert!(reg.get_recipe(RecipeId(999)).is_none());
        assert!(reg.get_building(BuildingTypeId(999)).is_none());
        assert!(reg.building_id("nonexistent").is_none());
        assert!(reg.recipe_id("nonexistent").is_none());
    }

    #[test]
    fn registry_item_has_properties_nonexistent_returns_false() {
        let builder = setup_builder();
        let reg = builder.build().unwrap();
        assert!(!reg.item_has_properties(ItemTypeId(999)));
    }

    #[test]
    fn empty_registry_builds_successfully() {
        let b = RegistryBuilder::new();
        let reg = b.build().unwrap();
        assert_eq!(reg.item_count(), 0);
        assert_eq!(reg.recipe_count(), 0);
        assert_eq!(reg.building_count(), 0);
    }
}
