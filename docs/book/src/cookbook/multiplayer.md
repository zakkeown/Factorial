# Detect Multiplayer Desync

**Goal:** Run two identical engines in lockstep, compare state hashes each tick, and detect when a divergent operation causes desync.
**Prerequisites:** [Determinism & Fixed-Point](../core-concepts/determinism.md), [Serialization](../core-concepts/serialization.md)
**Example:** `crates/factorial-core/examples/multiplayer_desync.rs`

## Steps

### 1. Create two identical engines

```rust
let mut engine_a = Engine::new(SimulationStrategy::Tick);
let mut engine_b = Engine::new(SimulationStrategy::Tick);

fn setup_factory(engine: &mut Engine) -> (NodeId, NodeId) {
    let p_mine = engine.graph.queue_add_node(BuildingTypeId(0));
    let p_smelter = engine.graph.queue_add_node(BuildingTypeId(1));
    let r = engine.graph.apply_mutations();
    let mine = r.resolve_node(p_mine).unwrap();
    let smelter = r.resolve_node(p_smelter).unwrap();

    // configure processors, inventories, transport identically
    (mine, smelter)
}

let (_mine_a, _smelter_a) = setup_factory(&mut engine_a);
let (_mine_b, smelter_b) = setup_factory(&mut engine_b);
```

Both engines are constructed with the exact same operations in the same order. Because the engine uses [fixed-point](../introduction/glossary.md#fixed-point) arithmetic, identical inputs always produce identical outputs regardless of platform.

### 2. Run both in lockstep and compare hashes

```rust
for tick in 0..10 {
    engine_a.step();
    engine_b.step();

    if (tick + 1) % 5 == 0 {
        println!(
            "Tick {}: A={}, B={}",
            tick + 1, engine_a.state_hash(), engine_b.state_hash()
        );
    }
}

let hash_a = engine_a.state_hash();
let hash_b = engine_b.state_hash();
assert_eq!(hash_a, hash_b, "identical inputs produce identical state");
```

The [state hash](../introduction/glossary.md#state-hash) is a deterministic u64 computed from all mutable engine state. After 10 identical [ticks](../introduction/glossary.md#tick), both hashes match.

### 3. Apply a divergent operation

```rust
engine_b.set_modifiers(
    smelter_b,
    vec![Modifier {
        id: ModifierId(0),
        kind: ModifierKind::Speed(Fixed64::from_num(2)),
        stacking: StackingRule::default(),
    }],
);
```

Only engine B receives a 2x speed [modifier](../introduction/glossary.md#modifier) on its smelter. This simulates a client applying an operation that the other client did not receive (e.g., a dropped network packet).

### 4. Detect the desync

```rust
for tick in 10..15 {
    engine_a.step();
    engine_b.step();

    println!(
        "Tick {}: A={}, B={}",
        tick + 1, engine_a.state_hash(), engine_b.state_hash()
    );
}

let hash_a = engine_a.state_hash();
let hash_b = engine_b.state_hash();
assert_ne!(hash_a, hash_b, "divergent inputs produce different state");
println!("Desync detected!");
```

After the divergent operation, every subsequent tick produces different hashes. The desync is detected immediately on the next hash comparison.

## What's Happening

Factorial's determinism guarantee means that two engines receiving the same sequence of inputs will always produce the same state. In a multiplayer architecture, each client runs its own engine and applies the same operations (received via a shared command log or lockstep protocol). Periodically, clients exchange state hashes. If hashes diverge, a desync has occurred -- meaning one client applied an operation the other did not, or applied operations in a different order.

The [state hash](../introduction/glossary.md#state-hash) covers all mutable state: [node](../introduction/glossary.md#node) configurations, [inventory](../introduction/glossary.md#inventory) contents, [processor](../introduction/glossary.md#processor) progress, transport buffers, and the tick counter. Even a single bit of difference (e.g., one extra item in an inventory) produces a completely different hash.

## Variations

- **Hash exchange frequency:** Compare hashes every tick for strict lockstep, or every N ticks for more relaxed sync with lower bandwidth.
- **Desync recovery:** When a desync is detected, use [serialization](./serialization.md) to send the authoritative engine state from the host to the desynced client.
- **Command log replay:** Record all player commands with their tick numbers. On desync, replay the command log from the last known-good state to find which command diverged.
- **Checksum subsets:** Hash subregions of the [production graph](../introduction/glossary.md#production-graph) independently to narrow down which part of the factory desynced.
- **Determinism testing:** Run the same scenario on different platforms (x86, ARM, WASM) and verify hashes match. Fixed-point arithmetic guarantees this.
