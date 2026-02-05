use crate::id::*;
use serde::{Serialize, Deserialize};

slotmap::new_key_type! {
    /// Identifies a specific item instance (for stateful items).
    pub struct InstanceId;
}

/// A stack of fungible items (no properties). Just a counter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_type: ItemTypeId,
    pub quantity: u32,
}

/// Inventory slot that holds either fungible counts or instance references.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
    pub fn add(&mut self, item_type: ItemTypeId, quantity: u32) -> u32 {
        let current_total: u32 = self.stacks.iter().map(|s| s.quantity).sum();
        let space = self.capacity.saturating_sub(current_total);
        let to_add = quantity.min(space);
        let overflow = quantity - to_add;

        if to_add > 0 {
            if let Some(stack) = self.stacks.iter_mut().find(|s| s.item_type == item_type) {
                stack.quantity += to_add;
            } else {
                self.stacks.push(ItemStack { item_type, quantity: to_add });
            }
        }

        overflow
    }

    /// Remove fungible items. Returns the amount actually removed.
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
}

/// Inventory for a building node. Multiple input/output slots.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
        slot.add(iron, 5);
        let removed = slot.remove(iron, 10);
        assert_eq!(removed, 5);
        assert_eq!(slot.quantity(iron), 0);
    }

    #[test]
    fn inventory_slot_multiple_types() {
        let mut slot = InventorySlot::new(100);
        let iron = ItemTypeId(0);
        let copper = ItemTypeId(1);
        slot.add(iron, 30);
        slot.add(copper, 20);
        assert_eq!(slot.total(), 50);
        assert_eq!(slot.quantity(iron), 30);
        assert_eq!(slot.quantity(copper), 20);
    }

    #[test]
    fn inventory_has_space() {
        let mut slot = InventorySlot::new(10);
        assert!(slot.has_space());
        slot.add(ItemTypeId(0), 10);
        assert!(!slot.has_space());
    }

    #[test]
    fn inventory_multiple_slots() {
        let inv = Inventory::new(2, 1, 50);
        assert_eq!(inv.input_slots.len(), 2);
        assert_eq!(inv.output_slots.len(), 1);
    }
}
