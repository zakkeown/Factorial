# Determinism & Fixed-Point

Factorial is **fully deterministic**: given identical inputs, two engine instances
produce bit-for-bit identical state on every platform. This guarantee enables
multiplayer lockstep, replay systems, and save/load without state drift.

## Why determinism matters

- **Multiplayer lockstep**: Each client runs its own simulation. Only player *inputs*
  are transmitted over the network. If the simulation is deterministic, all clients
  stay in sync without needing to send full state. A single desync corrupts the game.

- **Replay**: Record the stream of player actions. Replay them on the same engine
  version and get the exact same outcome, tick for tick.

- **Save/load**: Serialize the engine state at any [tick](../introduction/glossary.md#tick),
  deserialize it later, and continue the simulation with no drift. The
  [state hash](../introduction/glossary.md#state-hash) before save matches the state
  hash after load.

## Fixed-point arithmetic

IEEE 754 floating-point (`f32`, `f64`) is **not** deterministic across platforms.
Different CPUs, compilers, and optimization levels can produce different results for the
same operation. This is unacceptable for lockstep simulation.

Factorial uses [fixed-point](../introduction/glossary.md#fixed-point) arithmetic from
the `fixed` crate:

| Type | Alias | Format | Range | Precision |
|---|---|---|---|---|
| `Fixed64` | `I32F32` | Q32.32 | Roughly +/- 2 billion | 1/4,294,967,296 |
| `Fixed32` | `I16F16` | Q16.16 | Roughly +/- 32,768 | 1/65,536 |

`Fixed64` (Q32.32) is the workhorse type used for all simulation arithmetic: production
rates, accumulator fractions, transport rates, modifier values, and timing. `Fixed32`
(Q16.16) is used for compact storage where full precision is unnecessary (item
properties, etc.).

Fixed-point operations are pure integer arithmetic under the hood -- addition,
subtraction, multiplication, and division on integers. No floating-point hardware is
involved. This guarantees identical results on x86, ARM, and WASM.

```rust
use factorial_core::fixed::{Fixed64, f64_to_fixed64};

// Convert from f64 at initialization (not in sim loop).
let rate = Fixed64::from_num(2.5);
let modifier = Fixed64::from_num(1.5);

// All simulation arithmetic uses Fixed64.
let effective_rate = rate * modifier; // 3.75 exactly

// Convert back to f64 only for display/FFI.
let display_value: f64 = effective_rate.to_num();
```

**Important**: Convert from `f64` only during initialization. Convert back to `f64` only
for display or FFI. Never use `f64` in the simulation loop.

## Topological evaluation order

Determinism requires a **stable** evaluation order. Factorial evaluates nodes in
[topological order](../core-concepts/production-graph.md#topological-ordering) computed
by Kahn's algorithm. This means:

1. Upstream producers are always evaluated before their downstream consumers.
2. The order is deterministic -- the same graph always produces the same order.
3. The order is recomputed only when the graph structure changes (add/remove node or edge).

For nodes at the same topological level (no dependency between them), the order is
determined by the SlotMap's internal key ordering, which is stable and deterministic.

## Queued mutations

Direct mutation of the [production graph](../introduction/glossary.md#production-graph)
during a tick would introduce order-dependent behavior (the result depends on *when* during
the tick the mutation happens). Factorial prevents this by **queuing** all structural
mutations:

1. Game code calls `queue_add_node()`, `queue_connect()`, `queue_remove_node()`, etc.
2. Nothing changes until `apply_mutations()` is called.
3. `apply_mutations()` applies all queued changes atomically at a defined point (the
   pre-tick phase).

This ensures that the graph structure is constant throughout the transport and process
phases, eliminating a class of non-determinism.

## State hashing

The engine computes a deterministic `u64` hash of the entire simulation state after
every tick. Two engines with identical inputs produce identical
[state hashes](../introduction/glossary.md#state-hash):

```rust
// From crates/factorial-core/examples/multiplayer_desync.rs

// Create two identical engines.
let mut engine_a = Engine::new(SimulationStrategy::Tick);
let mut engine_b = Engine::new(SimulationStrategy::Tick);

setup_factory(&mut engine_a);
setup_factory(&mut engine_b);

// Run both for 10 ticks.
for _ in 0..10 {
    engine_a.step();
    engine_b.step();
}

let hash_a = engine_a.state_hash();
let hash_b = engine_b.state_hash();
assert_eq!(hash_a, hash_b, "identical inputs produce identical state");
```

The hash covers all simulation state: the graph structure, processor configurations and
states, inventory contents (including item properties), transport states, and the tick
counter.

## Multiplayer desync detection

In a multiplayer game, each client compares its state hash against the authoritative hash
after every tick (or every N ticks). If hashes diverge, a desync has occurred.

```rust
// From crates/factorial-core/examples/multiplayer_desync.rs

// Apply a divergent operation to engine B only.
engine_b.set_modifiers(
    smelter_b,
    vec![Modifier {
        id: ModifierId(0),
        kind: ModifierKind::Speed(Fixed64::from_num(2)),
        stacking: StackingRule::default(),
    }],
);

// Run both for 5 more ticks.
for _ in 0..5 {
    engine_a.step();
    engine_b.step();
}

let hash_a = engine_a.state_hash();
let hash_b = engine_b.state_hash();
assert_ne!(hash_a, hash_b, "divergent inputs produce different state");
```

### Subsystem hashes

When a desync is detected, use `subsystem_hashes()` to pinpoint *which* subsystem
diverged:

```rust
let hashes = engine.subsystem_hashes();
// SubsystemHashes {
//     graph: u64,
//     processors: u64,
//     processor_states: u64,
//     inventories: u64,
//     transports: u64,
//     sim_state: u64,
// }
```

Compare each field between the two engines to narrow down the cause. For example, if
`processors` differs but `graph` matches, the desync is in processor configuration, not
graph structure.

## Summary of determinism guarantees

| Mechanism | What it prevents |
|---|---|
| Fixed-point arithmetic (`Fixed64`, `Fixed32`) | Platform-dependent floating-point results |
| Topological evaluation order | Order-dependent node processing |
| Queued mutations | Mid-tick graph changes |
| Canonical modifier sorting (by `ModifierId`) | Insertion-order-dependent modifier stacking |
| State hashing (`state_hash()`) | Undetected desync between clients |
| Subsystem hashing (`subsystem_hashes()`) | Inability to diagnose desync root cause |
