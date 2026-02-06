# Save, Load, and Migrate State

**Goal:** Serialize the full engine state to bytes, deserialize it into a new engine, and verify the round trip preserves determinism.
**Prerequisites:** [Serialization](../core-concepts/serialization.md), [Determinism & Fixed-Point](../core-concepts/determinism.md)
**Example:** `crates/factorial-core/examples/save_load.rs`

## Steps

### 1. Build and run a factory

```rust
fn build_factory() -> Engine {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    let p_mine = engine.graph.queue_add_node(BuildingTypeId(0));
    let p_smelter = engine.graph.queue_add_node(BuildingTypeId(1));
    let r = engine.graph.apply_mutations();
    let mine = r.resolve_node(p_mine).unwrap();
    let smelter = r.resolve_node(p_smelter).unwrap();

    let p_belt = engine.graph.queue_connect(mine, smelter);
    let r = engine.graph.apply_mutations();
    let belt = r.resolve_edge(p_belt).unwrap();

    // configure processors, inventories, transport (same as smelting-chain recipe)
    engine
}

let mut engine = build_factory();
for _ in 0..10 {
    engine.step();
}
```

Run the factory for 10 [ticks](../introduction/glossary.md#tick) to build up state in [inventories](../introduction/glossary.md#inventory), processor progress, and transport buffers.

### 2. Serialize to bytes

```rust
let bytes = engine.serialize().expect("serialization should succeed");
println!("Serialized to {} bytes", bytes.len());
```

`Engine::serialize()` captures the complete engine state: the [production graph](../introduction/glossary.md#production-graph), all [processors](../introduction/glossary.md#processor), inventories, transport state, and the current tick counter. The output is a compact binary format.

### 3. Deserialize into a new engine

```rust
let mut restored = Engine::deserialize(&bytes).expect("deserialization should succeed");
assert_eq!(engine.sim_state.tick, restored.sim_state.tick);
```

The restored engine has identical state. The tick counter, all node states, and inventory contents match the original at the moment of serialization.

### 4. Verify determinism with state hashes

```rust
engine.step();
restored.step();

let hash_original = engine.state_hash();
let hash_restored = restored.state_hash();
assert_eq!(hash_original, hash_restored);
```

Running one more tick on both engines produces the same [state hash](../introduction/glossary.md#state-hash). This proves the round trip is lossless -- the restored engine is bit-for-bit identical to the original. [Fixed-point](../introduction/glossary.md#fixed-point) arithmetic guarantees this across platforms.

## What's Happening

Serialization captures every piece of mutable state in the engine: the graph topology (nodes and edges), processor configurations and progress, inventory contents, transport buffers and in-transit items, and the simulation tick counter. Deserialization reconstructs the engine from these bytes, including recomputing any derived structures (topological sort order, etc.). The [state hash](../introduction/glossary.md#state-hash) is a u64 computed from all mutable state, so comparing hashes is a fast way to verify two engines are in sync.

## Variations

- **File-based saves:** Write the `bytes` to disk with `std::fs::write()` and read them back with `std::fs::read()`. Add a version header to support migration.
- **Autosave:** Serialize periodically (e.g., every 300 ticks) to implement automatic save points.
- **Incremental serialization:** For large factories, use the partitioned serialization API to save only changed regions. See [Serialization](../core-concepts/serialization.md) for details.
- **Migration:** When adding new fields to the engine in a game update, implement a migration function that reads the old format, patches the data, and produces the new format.
- **Multiplayer sync:** Combine serialization with [state hash comparison](./multiplayer.md) to detect and recover from desync.
