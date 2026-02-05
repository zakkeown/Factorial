# Task 2: ID Types & Registry

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 1 — Foundation (sequential) |
| **Branch** | `main` (commit directly) |
| **Depends on** | Task 1 (workspace + fixed-point) |
| **Parallel with** | None |
| **Skill** | `subagent-driven-development` |

## Files

- Create: `crates/factorial-core/src/id.rs`
- Create: `crates/factorial-core/src/registry.rs`
- Modify: `crates/factorial-core/src/lib.rs` — add `pub mod id; pub mod registry;`

## Context

Design doc §2 "Registry". Three-phase lifecycle: registration → mutation → finalization. After `build()`, the registry is immutable. IDs are cheap copy types used throughout.

## Step 1: Write failing tests for ID types

`crates/factorial-core/src/id.rs`:

```rust
use serde::{Serialize, Deserialize};
use slotmap::new_key_type;

new_key_type! {
    /// Identifies a node (building) in the production graph.
    pub struct NodeId;

    /// Identifies an edge (transport link) in the production graph.
    pub struct EdgeId;

    /// Identifies a junction (splitter/merger/inserter) in the graph.
    pub struct JunctionId;
}

/// Identifies an item type in the registry. Cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemTypeId(pub u32);

/// Identifies a building template in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildingTypeId(pub u32);

/// Identifies a recipe in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub u32);

/// Identifies a property on an item type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PropertyId(pub u16);

/// Identifies a modifier applied to a building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModifierId(pub u32);

/// A pending node ID returned from queued mutations. Resolves to NodeId on apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PendingNodeId(pub u64);

/// A pending edge ID returned from queued mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PendingEdgeId(pub u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_type_id_equality() {
        let a = ItemTypeId(0);
        let b = ItemTypeId(0);
        let c = ItemTypeId(1);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn building_type_id_copy() {
        let a = BuildingTypeId(5);
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn ids_are_hashable() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(ItemTypeId(0), "iron_ore");
        map.insert(ItemTypeId(1), "iron_plate");
        assert_eq!(map[&ItemTypeId(0)], "iron_ore");
    }
}
```

## Step 2: Run tests to verify they pass

```bash
cargo test -p factorial-core -- id::tests
```

Expected: PASS

## Step 3: Write registry with builder pattern

`crates/factorial-core/src/registry.rs`:

```rust
use crate::fixed::{Fixed64, Fixed32, Ticks};
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
/// Three-phase lifecycle: registration → mutation → finalization.
pub struct RegistryBuilder {
    items: Vec<ItemTypeDef>,
    item_name_to_id: HashMap<String, ItemTypeId>,
    recipes: Vec<RecipeDef>,
    recipe_name_to_id: HashMap<String, RecipeId>,
    buildings: Vec<BuildingTemplateDef>,
    building_name_to_id: HashMap<String, BuildingTypeId>,
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
    pub fn register_recipe(&mut self, name: &str, inputs: Vec<RecipeEntry>, outputs: Vec<RecipeEntry>, duration: Ticks) -> RecipeId {
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
        let id = self.recipe_name_to_id.get(name)
            .ok_or(RegistryError::NotFound(name.to_string()))?;
        f(&mut self.recipes[id.0 as usize]);
        Ok(())
    }

    /// Phase 2: Mutate an existing building template by name.
    pub fn mutate_building<F>(&mut self, name: &str, f: F) -> Result<(), RegistryError>
    where
        F: FnOnce(&mut BuildingTemplateDef),
    {
        let id = self.building_name_to_id.get(name)
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
        self.items.get(id.0 as usize)
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
            vec![RecipeEntry { item: iron_ore, quantity: 1 }],
            vec![RecipeEntry { item: iron_plate, quantity: 1 }],
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
        builder.mutate_recipe("smelt_iron", |recipe| {
            recipe.inputs.push(RecipeEntry { item: chromium, quantity: 1 });
        }).unwrap();
        let reg = builder.build().unwrap();
        let recipe = reg.get_recipe(reg.recipe_id("smelt_iron").unwrap()).unwrap();
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
            vec![RecipeEntry { item: ItemTypeId(999), quantity: 1 }],
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
        b.register_item("water", vec![
            PropertyDef {
                name: "temperature".to_string(),
                size: PropertySize::Fixed32,
                default: PropertyDefault::Fixed32(Fixed32::from_num(20)),
            },
        ]);
        let reg = b.build().unwrap();
        let water = reg.item_id("water").unwrap();
        assert!(reg.item_has_properties(water));
    }

    #[test]
    fn registry_is_immutable_after_build() {
        // Registry has no &mut self methods — immutability enforced by the type system.
        let builder = setup_builder();
        let reg = builder.build().unwrap();
        // Can only read:
        let _ = reg.get_item(ItemTypeId(0));
        let _ = reg.get_recipe(RecipeId(0));
        let _ = reg.get_building(BuildingTypeId(0));
    }
}
```

## Step 4: Add `thiserror` dependency and update lib.rs

Add to `crates/factorial-core/Cargo.toml` dependencies:

```toml
thiserror = "2"
```

Update `crates/factorial-core/src/lib.rs`:

```rust
pub mod fixed;
pub mod id;
pub mod registry;
```

## Step 5: Run tests

```bash
cargo test -p factorial-core
```

Expected: All tests PASS.

## Step 6: Commit

```bash
git add -A && git commit -m "feat: ID types and registry with builder pattern"
```

## Verification

- `cargo test -p factorial-core` — all tests pass
- `RegistryBuilder::new().build()` compiles and returns `Ok(Registry)`
- IDs are `Copy + Eq + Hash`
- `thiserror` errors have display messages
