# Factorial Engine Enhancements Design

**Date:** 2026-02-05
**Status:** Approved
**Scope:** 9 items across 5 work groups, touching all 5 crates

## Work Groups & Dependencies

```
A (infrastructure) ──→ B (test coverage)
                  ──→ C (determinism)      [parallel]
                  ──→ D (serialization)    [parallel]
                  ──→ E (FFI)              [parallel]
```

| Group | Items | Theme |
|-------|-------|-------|
| **A** | 5, 6 | Infrastructure — verify clippy, extract test helpers |
| **B** | 1, 2, 7 | Test coverage — Delta, adversarial graph, Vehicle/Batch integration |
| **C** | 9 | Determinism — BTreeMap in power module |
| **D** | 3 | Serialization — version field in snapshot format |
| **E** | 4, 8 | FFI — poisoned flag + full configuration surface |

---

## Group A: Infrastructure

### A1: Verify Clippy Warnings (Item 5)

Run `cargo clippy --workspace --all-targets` including with `test-utils` feature flag once it exists. If warnings surface under specific feature combinations, fix them. If clean, drop this item.

### A2: Extract Shared Test Helpers (Item 6)

Create `crates/factorial-core/src/test_utils.rs` gated behind a feature flag:

```rust
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils { ... }
```

Add to `Cargo.toml`:

```toml
[features]
test-utils = []
```

**Helpers to extract** (currently duplicated in `tests/integration.rs` and `benches/sim_bench.rs`):

- `fixed(f64) -> Fixed64`
- Item constructors: `iron()`, `copper()`, `gear()`, `water()`
- `building() -> BuildingTypeId`
- `simple_inventory(capacity) -> Inventory`
- `make_source(item, rate) -> Processor`
- `make_recipe(inputs, outputs, duration) -> Processor`
- `make_flow_transport(rate) -> Transport`
- `add_node(engine, processor, input_cap, output_cap) -> NodeId`
- `connect(engine, from, to, transport) -> EdgeId`

Plus query helpers currently only in `integration.rs`:

- `output_quantity()`, `input_quantity()`, `input_total()`, `output_total()`

Both `integration.rs` and `sim_bench.rs` import from `factorial_core::test_utils`. Benchmarks enable the feature via `[dev-dependencies]`.

---

## Group B: Test Coverage

### B1: Delta Simulation Strategy Tests (Item 1)

New tests in `engine.rs` using `test_utils` helpers:

- **Delta accumulates fractional ticks** — advance by 0.5 twice, verify same result as advance by 1.0
- **Delta sub-tick no-op** — advance by less than one tick's worth, verify no processing occurs
- **Delta multi-tick catchup** — advance by 3.5, verify 3 full ticks process and 0.5 remains in accumulator
- **Delta zero dt** — advance by 0, verify no state change
- **Delta determinism** — two engines with identical setup and identical `advance()` calls produce identical state hashes

### B2: Adversarial Graph Mutation Tests (Item 2)

New tests in `graph.rs`:

- **Self-loop rejected** — `connect(A, A)` returns `GraphError::CycleDetected` (or new `SelfLoop` variant)
- **Duplicate edge handling** — `connect(A, B)` twice, verify behavior (error or idempotent)
- **Remove non-existent node** — returns appropriate error, no panic
- **Remove non-existent edge** — returns appropriate error, no panic
- **Remove node with active edges** — verify all connected edges cleaned up (both inbound and outbound)
- **Mutation during iteration safety** — queued mutations don't corrupt in-progress topo sort

### B3: VehicleTransport & BatchTransport Integration Tests (Item 7)

New tests in `tests/integration.rs`:

- **Batch chain** — Source → Batch belt → Processor, verify items arrive in discrete chunks
- **Vehicle round-trip** — Source → Vehicle → Processor, verify travel time and capacity constraints in a running engine
- **Mixed transport factory** — Chain using Flow, Item, Batch, and Vehicle transports together, verify all nodes receive correct inputs

~15 new tests total across Group B.

---

## Group C: Determinism

### C1: Replace HashMap with BTreeMap in Power Module (Item 9)

In `crates/factorial-power/src/lib.rs`, replace all four `HashMap` usages with `BTreeMap`:

- `PowerModule::networks: BTreeMap<PowerNetworkId, PowerNetwork>`
- `PowerModule::producers: BTreeMap<NodeId, PowerProducer>`
- `PowerModule::consumers: BTreeMap<NodeId, PowerConsumer>`
- `PowerModule::storage: BTreeMap<NodeId, PowerStorage>`

**Prerequisites:** `PowerNetworkId` must implement `Ord`. Add `#[derive(Ord, PartialOrd)]` if missing.

**New test:**

- **Deterministic power distribution** — Create a network with multiple consumers exceeding supply, run `balance()` twice with identical setup, assert identical per-consumer satisfaction ratios

~30 lines modified, 1 new test.

---

## Group D: Serialization Version Field

### D1: Add Version to Snapshot Format (Item 3)

In `crates/factorial-core/src/serialize.rs`:

```rust
const SNAPSHOT_VERSION: u32 = 1;

#[derive(Encode, Decode)]
struct VersionedSnapshot {
    version: u32,
    data: Vec<u8>,
}
```

**Serialize path:** Encode engine state, wrap in `VersionedSnapshot { version: SNAPSHOT_VERSION, data }`, encode wrapper.

**Deserialize path:** Decode outer `VersionedSnapshot`. If `version == SNAPSHOT_VERSION`, decode inner data. If `version > SNAPSHOT_VERSION`, return `SerializeError::FutureVersion`. If `version < SNAPSHOT_VERSION`, return `SerializeError::UnsupportedVersion(version)` (migration hook for later).

**New tests:**

- **Round-trip preserves version** — serialize then deserialize, verify success
- **Future version rejected** — craft snapshot with `version: 99`, verify `FutureVersion` error
- **Version field present** — serialize, decode only outer wrapper, assert `version == 1`

~50 lines new code, 3 new tests. Breaking change to serialized format (acceptable pre-release).

---

## Group E: FFI Enhancements

### E1: Poisoned Flag (Item 4)

Add `poisoned: bool` to the FFI engine wrapper:

```rust
pub struct FfiEngine {
    inner: Engine,
    poisoned: bool,
}
```

- On panic catch: set `poisoned = true`, return error code
- On every subsequent FFI call: check `poisoned` first, return `FACTORIAL_ERROR_POISONED`
- New functions:
  - `factorial_is_poisoned(engine) -> bool`
  - `factorial_clear_poison(engine)` — for recovery attempts

3 new tests: poisoned sets on panic, blocks subsequent calls, clears correctly.

### E2: Full Configuration FFI Functions (Item 8)

15 new FFI functions:

**Registry/Item types:**

- `factorial_register_item(engine, name_ptr, name_len, out_id)`
- `factorial_register_building(engine, name_ptr, name_len, out_id)`
- `factorial_finalize_registry(engine)`

**Processor setup:**

- `factorial_set_source(engine, node_id, item_id, rate)`
- `factorial_set_fixed_processor(engine, node_id, recipe_ptr)`
- `factorial_set_property_processor(engine, node_id, ...)`

**Transport setup:**

- `factorial_set_flow_transport(engine, edge_id, rate)`
- `factorial_set_item_transport(engine, edge_id, speed)`
- `factorial_set_batch_transport(engine, edge_id, batch_size, interval)`
- `factorial_set_vehicle_transport(engine, edge_id, capacity, travel_time)`

**Inventory:**

- `factorial_set_input_capacity(engine, node_id, capacity)`
- `factorial_set_output_capacity(engine, node_id, capacity)`
- `factorial_get_inventory_item(engine, node_id, slot, out_stack)`

**Recipe C struct:**

```c
typedef struct {
    uint32_t input_count;
    FactorialItemStack* inputs;
    uint32_t output_count;
    FactorialItemStack* outputs;
    int64_t duration;  // Fixed64 raw value
} FactorialRecipe;
```

All functions follow existing patterns: null-check pointers, `catch_unwind`, check poisoned flag, return error codes. ~20 new tests.

---

## Summary

| Group | New/Modified Code | New Tests |
|-------|-------------------|-----------|
| A | `test_utils.rs` + feature flag, update imports | 0 (refactor) |
| B | Delta tests, adversarial graph tests, integration tests | ~15 |
| C | HashMap → BTreeMap in power module | 1 |
| D | VersionedSnapshot wrapper in serialize.rs | 3 |
| E | Poisoned flag + 15 FFI functions | ~23 |
| **Total** | | **~42 new tests** |

Post-implementation target: **~306 tests** (264 existing + 42 new), 0 clippy warnings.
