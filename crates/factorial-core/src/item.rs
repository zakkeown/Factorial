use crate::id::*;
use crate::fixed::Fixed64;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

slotmap::new_key_type! {
    /// Identifies a specific item instance (for stateful items).
    pub struct InstanceId;
}

/// A stack of fungible items with optional per-instance properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_type: ItemTypeId,
    pub quantity: u32,
    /// Per-instance properties (e.g., temperature, quality).
    /// Empty by default. Game code sets properties via processors or modules.
    #[serde(default)]
    pub properties: BTreeMap<PropertyId, Fixed64>,
}

impl ItemStack {
    pub fn new(item_type: ItemTypeId, quantity: u32) -> Self {
        Self {
            item_type,
            quantity,
            properties: BTreeMap::new(),
        }
    }

    pub fn set_property(&mut self, id: PropertyId, value: Fixed64) {
        self.properties.insert(id, value);
    }

    pub fn get_property(&self, id: PropertyId) -> Option<Fixed64> {
        self.properties.get(&id).copied()
    }
}

/// Inventory slot that holds either fungible counts or instance references.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InventorySlot {
    /// Fungible item stacks keyed by item type.
    pub stacks: Vec<ItemStack>,
    pub capacity: u32,
}

impl InventorySlot {
    pub fn new(capacity: u32) -> Self {
        Self {
            stacks: Vec::new(),
            capacity,
        }
    }

    /// Add fungible items. Returns the amount that didn't fit.
    #[must_use = "overflow count indicates items that did not fit"]
    pub fn add(&mut self, item_type: ItemTypeId, quantity: u32) -> u32 {
        let current_total: u32 = self.stacks.iter().map(|s| s.quantity).sum();
        let space = self.capacity.saturating_sub(current_total);
        let to_add = quantity.min(space);
        let overflow = quantity - to_add;

        if to_add > 0 {
            if let Some(stack) = self.stacks.iter_mut().find(|s| s.item_type == item_type) {
                stack.quantity += to_add;
            } else {
                self.stacks.push(ItemStack::new(item_type, to_add));
            }
        }

        overflow
    }

    /// Remove fungible items. Returns the amount actually removed.
    #[must_use = "returns the quantity actually removed, which may be less than requested"]
    pub fn remove(&mut self, item_type: ItemTypeId, quantity: u32) -> u32 {
        if let Some(stack) = self.stacks.iter_mut().find(|s| s.item_type == item_type) {
            let to_remove = quantity.min(stack.quantity);
            stack.quantity -= to_remove;
            if stack.quantity == 0 {
                self.stacks.retain(|s| s.quantity > 0);
            }
            to_remove
        } else {
            0
        }
    }

    /// Get quantity of a specific item type.
    pub fn quantity(&self, item_type: ItemTypeId) -> u32 {
        self.stacks.iter()
            .find(|s| s.item_type == item_type)
            .map(|s| s.quantity)
            .unwrap_or(0)
    }

    /// Total items across all types.
    pub fn total(&self) -> u32 {
        self.stacks.iter().map(|s| s.quantity).sum()
    }

    /// Check if inventory has room for more items.
    pub fn has_space(&self) -> bool {
        self.total() < self.capacity
    }

    /// Check if inventory has room for a specific quantity.
    pub fn has_space_for(&self, quantity: u32) -> bool {
        self.total() + quantity <= self.capacity
    }

    /// Get properties of a stack by item type.
    pub fn get_properties(&self, item_type: ItemTypeId) -> Option<&BTreeMap<PropertyId, Fixed64>> {
        self.stacks
            .iter()
            .find(|s| s.item_type == item_type)
            .map(|s| &s.properties)
    }

    /// Set a single property on a typed stack. Returns false if the item type is not present.
    pub fn set_stack_property(&mut self, item_type: ItemTypeId, property: PropertyId, value: Fixed64) -> bool {
        if let Some(stack) = self.stacks.iter_mut().find(|s| s.item_type == item_type) {
            stack.set_property(property, value);
            true
        } else {
            false
        }
    }

    /// Like `add()` but merges properties onto the stack (incoming overrides existing).
    pub fn add_with_properties(&mut self, item_type: ItemTypeId, quantity: u32, properties: &BTreeMap<PropertyId, Fixed64>) -> u32 {
        let current_total: u32 = self.stacks.iter().map(|s| s.quantity).sum();
        let space = self.capacity.saturating_sub(current_total);
        let to_add = quantity.min(space);
        let overflow = quantity - to_add;

        if to_add > 0 {
            if let Some(stack) = self.stacks.iter_mut().find(|s| s.item_type == item_type) {
                stack.quantity += to_add;
                // Merge properties: incoming overrides existing.
                for (&prop, &val) in properties {
                    stack.properties.insert(prop, val);
                }
            } else {
                let mut stack = ItemStack::new(item_type, to_add);
                stack.properties = properties.clone();
                self.stacks.push(stack);
            }
        }

        overflow
    }
}

/// Inventory for a building node. Multiple input/output slots.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Inventory {
    pub input_slots: Vec<InventorySlot>,
    pub output_slots: Vec<InventorySlot>,
}

impl Inventory {
    pub fn new(input_count: usize, output_count: usize, capacity: u32) -> Self {
        Self {
            input_slots: (0..input_count).map(|_| InventorySlot::new(capacity)).collect(),
            output_slots: (0..output_count).map(|_| InventorySlot::new(capacity)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_slot_add_and_remove() {
        let mut slot = InventorySlot::new(100);
        let iron = ItemTypeId(0);
        let overflow = slot.add(iron, 50);
        assert_eq!(overflow, 0);
        assert_eq!(slot.quantity(iron), 50);

        let removed = slot.remove(iron, 30);
        assert_eq!(removed, 30);
        assert_eq!(slot.quantity(iron), 20);
    }

    #[test]
    fn inventory_slot_overflow() {
        let mut slot = InventorySlot::new(10);
        let iron = ItemTypeId(0);
        let overflow = slot.add(iron, 15);
        assert_eq!(overflow, 5);
        assert_eq!(slot.quantity(iron), 10);
    }

    #[test]
    fn inventory_slot_remove_more_than_available() {
        let mut slot = InventorySlot::new(100);
        let iron = ItemTypeId(0);
        let _ = slot.add(iron, 5);
        let removed = slot.remove(iron, 10);
        assert_eq!(removed, 5);
        assert_eq!(slot.quantity(iron), 0);
    }

    #[test]
    fn inventory_slot_multiple_types() {
        let mut slot = InventorySlot::new(100);
        let iron = ItemTypeId(0);
        let copper = ItemTypeId(1);
        let _ = slot.add(iron, 30);
        let _ = slot.add(copper, 20);
        assert_eq!(slot.total(), 50);
        assert_eq!(slot.quantity(iron), 30);
        assert_eq!(slot.quantity(copper), 20);
    }

    #[test]
    fn inventory_has_space() {
        let mut slot = InventorySlot::new(10);
        assert!(slot.has_space());
        let _ = slot.add(ItemTypeId(0), 10);
        assert!(!slot.has_space());
    }

    #[test]
    fn inventory_multiple_slots() {
        let inv = Inventory::new(2, 1, 50);
        assert_eq!(inv.input_slots.len(), 2);
        assert_eq!(inv.output_slots.len(), 1);
    }

    #[test]
    fn inventory_slot_get_properties() {
        let mut slot = InventorySlot::new(100);
        let iron = ItemTypeId(0);
        let temp = PropertyId(0);
        let mut props = BTreeMap::new();
        props.insert(temp, Fixed64::from_num(70));
        slot.add_with_properties(iron, 5, &props);

        let retrieved = slot.get_properties(iron);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().get(&temp).copied(), Some(Fixed64::from_num(70)));

        // Non-existent type returns None.
        assert!(slot.get_properties(ItemTypeId(99)).is_none());
    }

    #[test]
    fn inventory_slot_set_stack_property() {
        let mut slot = InventorySlot::new(100);
        let iron = ItemTypeId(0);
        let temp = PropertyId(0);
        let _ = slot.add(iron, 10);
        assert!(slot.set_stack_property(iron, temp, Fixed64::from_num(95)));
        let props = slot.get_properties(iron).unwrap();
        assert_eq!(props.get(&temp).copied(), Some(Fixed64::from_num(95)));

        // Non-existent type returns false.
        assert!(!slot.set_stack_property(ItemTypeId(99), temp, Fixed64::from_num(50)));
    }

    #[test]
    fn inventory_slot_add_with_properties() {
        let mut slot = InventorySlot::new(100);
        let iron = ItemTypeId(0);
        let temp = PropertyId(0);
        let quality = PropertyId(1);

        // First add with temperature.
        let mut props1 = BTreeMap::new();
        props1.insert(temp, Fixed64::from_num(70));
        let overflow = slot.add_with_properties(iron, 5, &props1);
        assert_eq!(overflow, 0);
        assert_eq!(slot.quantity(iron), 5);

        // Second add with different temperature — should override.
        let mut props2 = BTreeMap::new();
        props2.insert(temp, Fixed64::from_num(56));
        props2.insert(quality, Fixed64::from_num(3));
        let overflow = slot.add_with_properties(iron, 3, &props2);
        assert_eq!(overflow, 0);
        assert_eq!(slot.quantity(iron), 8);

        let retrieved = slot.get_properties(iron).unwrap();
        assert_eq!(retrieved.get(&temp).copied(), Some(Fixed64::from_num(56)));
        assert_eq!(retrieved.get(&quality).copied(), Some(Fixed64::from_num(3)));
    }

    #[test]
    fn inventory_slot_add_with_properties_overflow() {
        let mut slot = InventorySlot::new(5);
        let iron = ItemTypeId(0);
        let mut props = BTreeMap::new();
        props.insert(PropertyId(0), Fixed64::from_num(100));
        let overflow = slot.add_with_properties(iron, 8, &props);
        assert_eq!(overflow, 3);
        assert_eq!(slot.quantity(iron), 5);
    }

    #[test]
    fn item_stack_with_properties() {
        use crate::id::PropertyId;
        use crate::fixed::Fixed64;

        let mut stack = ItemStack {
            item_type: ItemTypeId(0),
            quantity: 10,
            properties: Default::default(),
        };

        let temp = PropertyId(0);
        stack.set_property(temp, Fixed64::from_num(95));
        assert_eq!(stack.get_property(temp), Some(Fixed64::from_num(95)));
        assert_eq!(stack.get_property(PropertyId(1)), None);
    }
}
