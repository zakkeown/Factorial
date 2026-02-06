use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ItemTypeId(pub u32);

/// Identifies a building template in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BuildingTypeId(pub u32);

/// Identifies a recipe in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub u32);

/// Identifies a property on an item type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PropertyId(pub u16);

/// Identifies a modifier applied to a building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    #[test]
    fn recipe_id_equality_and_copy() {
        let a = RecipeId(0);
        let b = RecipeId(0);
        let c = RecipeId(1);
        assert_eq!(a, b);
        assert_ne!(a, c);
        let d = a; // Copy
        assert_eq!(a, d);
    }

    #[test]
    fn property_id_ordering() {
        let a = PropertyId(1);
        let b = PropertyId(2);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn modifier_id_equality() {
        let a = ModifierId(10);
        let b = ModifierId(10);
        let c = ModifierId(20);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn pending_node_id_equality() {
        let a = PendingNodeId(0);
        let b = PendingNodeId(0);
        let c = PendingNodeId(1);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn pending_edge_id_equality() {
        let a = PendingEdgeId(0);
        let b = PendingEdgeId(0);
        let c = PendingEdgeId(1);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn ids_debug_format() {
        let item = ItemTypeId(42);
        let debug = format!("{item:?}");
        assert!(debug.contains("42"), "got: {debug}");

        let building = BuildingTypeId(7);
        let debug = format!("{building:?}");
        assert!(debug.contains("7"), "got: {debug}");
    }

    #[test]
    fn item_type_id_ordering() {
        let a = ItemTypeId(1);
        let b = ItemTypeId(2);
        assert!(a < b);
    }

    #[test]
    fn building_type_id_hashable() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(BuildingTypeId(0), "furnace");
        map.insert(BuildingTypeId(1), "assembler");
        assert_eq!(map[&BuildingTypeId(0)], "furnace");
        assert_eq!(map.len(), 2);
    }
}
