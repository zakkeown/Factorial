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
}
