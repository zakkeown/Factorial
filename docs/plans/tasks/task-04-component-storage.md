# Task 4: Component Storage (SoA)

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 2 — Core Types (parallel) |
| **Branch** | `feat/components` |
| **Depends on** | Task 2 (IDs + registry) — must be merged to main |
| **Parallel with** | Task 3 (Item Storage) — separate worktree |
| **Skill** | `subagent-driven-development` or `executing-plans` |

## Shared File Notes

This task adds `pub mod component;` to `lib.rs`. If running in parallel with Task 3, both branches will modify `lib.rs`. This is a trivial additive conflict — resolve at merge time. Use `claim_file("crates/factorial-core/src/lib.rs")` when merging.

**Important:** This task imports from `crate::item::Inventory` (Task 3). If Task 3 hasn't merged yet, you need Task 3's `item.rs` available. Options:
1. Wait for Task 3 to merge first (safest)
2. Cherry-pick Task 3's commit into this branch
3. Create a minimal `Inventory` stub for compilation, replace at merge

Recommended: Merge Task 3 first since it's smaller, then rebase this branch.

## Files

- Create: `crates/factorial-core/src/component.rs`
- Modify: `crates/factorial-core/src/lib.rs` — add `pub mod component;`

## Context

Design doc §9 "Component Storage". SoA layout with `SlotMap` per component type. Each component type gets its own contiguous storage array.

## Step 1: Write component storage and tests

`crates/factorial-core/src/component.rs`:

```rust
use crate::id::NodeId;
use crate::item::Inventory;
use crate::fixed::Fixed64;
use slotmap::SlotMap;

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

/// SoA component storage. Each component type has its own SlotMap.
#[derive(Debug)]
pub struct ComponentStorage {
    pub inventories: SlotMap<NodeId, Inventory>,
    pub power_consumers: SlotMap<NodeId, PowerConsumer>,
    pub power_producers: SlotMap<NodeId, PowerProducer>,
}

impl ComponentStorage {
    pub fn new() -> Self {
        Self {
            inventories: SlotMap::with_key(),
            power_consumers: SlotMap::with_key(),
            power_producers: SlotMap::with_key(),
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
        storage.power_consumers.insert(node, PowerConsumer { demand: Fixed64::from_num(90) });
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
            storage.power_consumers.insert(node, PowerConsumer {
                demand: Fixed64::from_num(100),
            });
        }
        let total_demand: Fixed64 = storage.power_consumers
            .values()
            .map(|pc| pc.demand)
            .fold(Fixed64::from_num(0), |acc, d| acc + d);
        assert_eq!(total_demand, Fixed64::from_num(500));
    }
}
```

## Step 2: Update lib.rs, run tests, commit

```bash
cargo test -p factorial-core && git add -A && git commit -m "feat: SoA component storage with SlotMap"
```

## Verification

- `cargo test -p factorial-core -- component::tests` — all tests pass
- `ComponentStorage::remove_node` cleans all component types
- SlotMap iteration works for bulk queries
