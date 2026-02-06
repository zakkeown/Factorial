use crate::fixed::Fixed64;
use crate::id::NodeId;
use crate::item::Inventory;
use slotmap::SecondaryMap;

/// Power consumer component.
#[derive(Debug, Clone)]
pub struct PowerConsumer {
    pub demand: Fixed64,
}

/// Power producer component.
#[derive(Debug, Clone)]
pub struct PowerProducer {
    pub output: Fixed64,
}

/// SoA component storage. Each component type has its own SecondaryMap
/// keyed by NodeId, providing O(1) access with contiguous storage.
#[derive(Debug)]
pub struct ComponentStorage {
    pub inventories: SecondaryMap<NodeId, Inventory>,
    pub power_consumers: SecondaryMap<NodeId, PowerConsumer>,
    pub power_producers: SecondaryMap<NodeId, PowerProducer>,
}

impl ComponentStorage {
    pub fn new() -> Self {
        Self {
            inventories: SecondaryMap::new(),
            power_consumers: SecondaryMap::new(),
            power_producers: SecondaryMap::new(),
        }
    }

    /// Remove all components for a given node.
    pub fn remove_node(&mut self, node: NodeId) {
        self.inventories.remove(node);
        self.power_consumers.remove(node);
        self.power_producers.remove(node);
    }
}

impl Default for ComponentStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    #[test]
    fn insert_and_get_inventory() {
        let mut storage = ComponentStorage::new();
        let mut nodes: SlotMap<NodeId, ()> = SlotMap::with_key();
        let node = nodes.insert(());
        storage.inventories.insert(node, Inventory::new(1, 1, 50));
        assert!(storage.inventories.contains_key(node));
    }

    #[test]
    fn remove_node_cleans_all_components() {
        let mut storage = ComponentStorage::new();
        let mut nodes: SlotMap<NodeId, ()> = SlotMap::with_key();
        let node = nodes.insert(());
        storage.inventories.insert(node, Inventory::new(1, 1, 50));
        storage
            .power_consumers
            .insert(node, PowerConsumer { demand: Fixed64::from_num(90) });
        storage.remove_node(node);
        assert!(!storage.inventories.contains_key(node));
        assert!(!storage.power_consumers.contains_key(node));
    }

    #[test]
    fn iterate_all_power_consumers() {
        let mut storage = ComponentStorage::new();
        let mut nodes: SlotMap<NodeId, ()> = SlotMap::with_key();
        for _ in 0..5 {
            let node = nodes.insert(());
            storage.power_consumers.insert(
                node,
                PowerConsumer {
                    demand: Fixed64::from_num(100),
                },
            );
        }
        let total_demand: Fixed64 = storage
            .power_consumers
            .values()
            .map(|pc| pc.demand)
            .fold(Fixed64::from_num(0), |acc, d| acc + d);
        assert_eq!(total_demand, Fixed64::from_num(500));
    }

    #[test]
    fn component_storage_default() {
        let storage = ComponentStorage::default();
        assert!(storage.inventories.is_empty());
        assert!(storage.power_consumers.is_empty());
        assert!(storage.power_producers.is_empty());
    }

    #[test]
    fn remove_node_nonexistent_is_noop() {
        let mut storage = ComponentStorage::new();
        let mut nodes: SlotMap<NodeId, ()> = SlotMap::with_key();
        let node = nodes.insert(());
        nodes.remove(node); // Remove from slotmap but keep the key
        // Should not panic
        storage.remove_node(node);
    }

    #[test]
    fn power_producer_stored_and_retrieved() {
        let mut storage = ComponentStorage::new();
        let mut nodes: SlotMap<NodeId, ()> = SlotMap::with_key();
        let node = nodes.insert(());
        storage.power_producers.insert(
            node,
            PowerProducer {
                output: Fixed64::from_num(500),
            },
        );
        assert!(storage.power_producers.contains_key(node));
        assert_eq!(storage.power_producers[node].output, Fixed64::from_num(500));
    }

    #[test]
    fn remove_node_only_affects_target() {
        let mut storage = ComponentStorage::new();
        let mut nodes: SlotMap<NodeId, ()> = SlotMap::with_key();
        let node_a = nodes.insert(());
        let node_b = nodes.insert(());
        storage.inventories.insert(node_a, Inventory::new(1, 1, 50));
        storage.inventories.insert(node_b, Inventory::new(1, 1, 50));
        storage.remove_node(node_a);
        assert!(!storage.inventories.contains_key(node_a));
        assert!(storage.inventories.contains_key(node_b));
    }
}
