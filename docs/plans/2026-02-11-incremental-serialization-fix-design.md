# Incremental Serialization Fix Design

**Date:** 2026-02-11
**Status:** Approved
**Scope:** Fix 3 issues preventing incremental serialization from working, add benchmarks

## Background

The incremental serialization infrastructure (partitioned snapshots, dirty partition flags, `serialize_incremental` API) was built in a prior iteration. However, three issues prevent it from delivering any actual performance benefit:

1. `serialize_incremental()` eagerly serializes all 5 partition blobs, then picks — no work is saved
2. `phase_bookkeeping()` unconditionally marks 4/5 partitions dirty every tick
3. No benchmark proves the optimization works

## Change 1: Lazy `serialize_incremental` + Public `PartitionedSnapshot`

### Problem

`serialize_incremental()` builds fresh bitcode blobs for all 5 partitions (cloning all engine state), then decides which to keep. The baseline is also re-deserialized from raw bytes on every save.

### Fix

- Make `PartitionedSnapshot` public
- Change `serialize_incremental` signature: takes `Option<&PartitionedSnapshot>`, returns `Result<PartitionedSnapshot, SerializeError>`
- Add `PartitionedSnapshot::to_bytes()` and `PartitionedSnapshot::from_bytes()` for wire format conversion
- Only serialize dirty partitions; clone clean blobs from baseline
- `serialize_partitioned()` also returns `PartitionedSnapshot` instead of `Vec<u8>`

#### New API

```rust
// PartitionedSnapshot is now public
pub struct PartitionedSnapshot {
    pub header: PartitionedSnapshotHeader,
    pub(crate) partitions: [Vec<u8>; 5],
}

impl PartitionedSnapshot {
    pub fn to_bytes(&self) -> Result<Vec<u8>, SerializeError>;
    pub fn from_bytes(data: &[u8]) -> Result<Self, DeserializeError>;
}

impl Engine {
    pub fn serialize_partitioned(&self) -> Result<PartitionedSnapshot, SerializeError>;

    pub fn serialize_incremental(
        &mut self,
        baseline: Option<&PartitionedSnapshot>,
    ) -> Result<PartitionedSnapshot, SerializeError>;

    pub fn deserialize_partitioned(snapshot: &PartitionedSnapshot) -> Result<Self, DeserializeError>;
}
```

#### Usage Pattern

```rust
// First save
let mut baseline = engine.serialize_partitioned()?;

// Game loop
loop {
    engine.step();
    if should_autosave() {
        baseline = engine.serialize_incremental(Some(&baseline))?;
        write_to_disk(&baseline.to_bytes()?);
    }
}

// Loading
let snapshot = PartitionedSnapshot::from_bytes(&read_from_disk()?)?;
let engine = Engine::deserialize_partitioned(&snapshot)?;
```

#### Internal: `serialize_single_partition` helper

```rust
fn serialize_single_partition(&self, index: usize) -> Result<Vec<u8>, SerializeError> {
    match index {
        0 => bitcode::serialize(&GraphPartition { ... }),
        1 => bitcode::serialize(&ProcessorPartition { ... }),
        2 => bitcode::serialize(&InventoryPartition { ... }),
        3 => bitcode::serialize(&TransportPartition { ... }),
        4 => bitcode::serialize(&JunctionPartition { ... }),
        _ => unreachable!(),
    }
}
```

### Files Modified

| File | Changes |
|------|---------|
| `crates/factorial-core/src/serialize.rs` | Make `PartitionedSnapshot` public, add `to_bytes`/`from_bytes`, add `serialize_single_partition`, rewrite `serialize_incremental`/`serialize_partitioned`/`deserialize_partitioned` |

## Change 2: Fix Partition Dirty Tracking

### Problem

`phase_bookkeeping()` (engine.rs:1571-1579) unconditionally marks Graph, Processors, Inventories, and Transports dirty every tick. This means after any `step()`, incremental serialization always re-serializes 4/5 partitions regardless of actual changes.

### Fix: Two Parts

#### Part A: Auto-inference in DirtyTracker

When `mark_node()`, `mark_edge()`, or `mark_graph()` is called, automatically set corresponding partition flags:

```
mark_node()  → sets Processors + Inventories dirty
mark_edge()  → sets Transports dirty
mark_graph() → sets Graph dirty
```

This handles the public API path (`set_processor`, `set_input_inventory`, etc.).

#### Part B: Pipeline phase marking + bookkeeping fix

Add coarse-grained partition marking at the end of internal pipeline phases:

- `phase_transport()`: mark `TRANSPORTS` + `INVENTORIES` if any transport work was done
- `phase_process()`: mark `PROCESSORS` + `INVENTORIES` if any node was processed
- `phase_component()`: mark `JUNCTIONS` if any junction ran
- `phase_bookkeeping()`: mark only `GRAPH` (for `sim_state.tick` and `last_state_hash`)

Remove the blanket `mark_partition` calls from `phase_bookkeeping()`.

#### Steady-State Behavior

At steady state (active factory), all 4 main partitions are still correctly marked dirty. When the factory is paused or empty, only `GRAPH` gets flagged — and incremental saves skip the other 4 partitions entirely.

### Files Modified

| File | Changes |
|------|---------|
| `crates/factorial-core/src/dirty.rs` | Add auto-inference in `mark_node`, `mark_edge`, `mark_graph` |
| `crates/factorial-core/src/engine.rs` | Add partition marks in `phase_transport`, `phase_process`, `phase_component`; fix `phase_bookkeeping` |

## Change 3: Benchmarks

Add to `crates/factorial-core/benches/sim_bench.rs`:

1. **`serialize_incremental_steady_state`** — Medium factory, step + incremental save. All partitions dirty. Validates no overhead vs full.

2. **`serialize_incremental_paused`** — Medium factory, no step between saves. Only Graph dirty. Shows the win: 1/5 partitions encoded.

3. **`serialize_incremental_config_change`** — Medium factory at rest, swap one processor. 1-2 partitions dirty. Shows partial-dirty scenario.

Update existing `serialize_incremental_with_baseline` to use new `&PartitionedSnapshot` API.

### Files Modified

| File | Changes |
|------|---------|
| `crates/factorial-core/benches/sim_bench.rs` | Add 3 new benchmark cases, update existing incremental bench |

## Tests

Existing tests cover the core behavior. New/updated tests:

1. `incremental_skips_clean_partitions` — Verify that only dirty partition blobs differ from baseline
2. `partitioned_snapshot_to_from_bytes` — Round-trip `to_bytes`/`from_bytes`
3. `paused_engine_only_graph_dirty` — After step with no nodes, only Graph partition flagged
4. `auto_inference_mark_node` — `mark_node` sets Processors + Inventories
5. `auto_inference_mark_edge` — `mark_edge` sets Transports
6. `auto_inference_mark_graph` — `mark_graph` sets Graph

## Implementation Order

```
1. DirtyTracker auto-inference (dirty.rs)
2. Pipeline partition marking + bookkeeping fix (engine.rs)
3. Public PartitionedSnapshot + lazy serialize_incremental (serialize.rs)
4. Update existing tests for new API
5. Add new tests
6. Benchmarks (sim_bench.rs)
7. Run CI: cargo test, clippy, fmt, coverage
```

Estimated: ~150 lines changed, ~80 lines new tests, ~40 lines benchmarks.
