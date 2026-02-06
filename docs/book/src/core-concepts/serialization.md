# Serialization

Factorial provides built-in binary serialization for the complete engine state. Save,
load, snapshot ring buffers for undo/replay, and incremental serialization for dirty
tracking are all supported out of the box.

## Basic save and load

Serialize the engine to a `Vec<u8>` with `engine.serialize()` and restore it with
`Engine::deserialize()`:

```rust
// From crates/factorial-core/examples/save_load.rs

// Serialize the engine state to bytes.
let bytes = engine.serialize().expect("serialization should succeed");
println!("Serialized to {} bytes", bytes.len());

// Deserialize into a new engine.
let mut restored = Engine::deserialize(&bytes).expect("deserialization should succeed");
```

After deserialization, the restored engine has identical simulation state. Running
both engines for one more [tick](../introduction/glossary.md#tick) produces identical
[state hashes](../introduction/glossary.md#state-hash):

```rust
// From crates/factorial-core/examples/save_load.rs

engine.step();
restored.step();

let hash_original = engine.state_hash();
let hash_restored = restored.state_hash();
assert_eq!(hash_original, hash_restored,
    "hashes should match after save/load round trip");
```

## Binary format

Factorial uses **bitcode** for serialization -- a compact binary codec that is
significantly smaller than JSON or bincode. A populated engine with multiple nodes,
edges, processors, inventories, and transports typically serializes to well under
10 KB.

The `EventBus` is excluded from serialization because it contains closures (which cannot
be serialized). After deserialization, a fresh `EventBus` is created and subscribers must
be re-registered.

## Versioning

Every serialized blob starts with a `SnapshotHeader` that contains:

| Field | Type | Description |
|---|---|---|
| `magic` | `u32` | Magic number (`0xFAC70001`) for format detection |
| `version` | `u32` | Format version (currently `2`) |
| `tick` | `u64` | Tick count when the snapshot was taken |

Deserialization validates the header before attempting to decode the payload:

- **Future version** (header version > current): returns `DeserializeError::FutureVersion`.
- **Past version** (header version < current): returns `DeserializeError::UnsupportedVersion`.
- **Invalid magic**: returns `DeserializeError::InvalidMagic`.

For forward migration, use `Engine::deserialize_with_migrations()` which accepts a
`MigrationRegistry` and applies registered migrations to bring older snapshots up to
the current format version.

## Module hooks

Registered simulation modules are excluded from the serialized snapshot (they may contain
non-serializable state like closures or external handles). After deserialization, the
`modules` vec is empty. Re-register your modules before resuming the simulation:

```rust
let mut engine = Engine::deserialize(&bytes)?;
engine.register_module(my_power_module);
engine.register_module(my_fluid_module);
// Now safe to call engine.step()
```

## Dirty tracking

The engine tracks which parts of the state have changed since the last query via
`DirtyTracker`. This is useful for rendering systems that need to know whether to
re-read a [node](../introduction/glossary.md#node) or
[edge](../introduction/glossary.md#edge):

```rust
// Check if any state has changed since the last mark_clean().
if engine.is_dirty() {
    // Re-read snapshots for rendering.
    let snapshots = engine.snapshot_all_nodes();
    render_factory(&snapshots);

    // Mark the engine as clean.
    engine.mark_clean();
}
```

The dirty tracker provides granular queries:

- `is_dirty()` -- returns `true` if any node, edge, or graph structure changed.
- `is_node_dirty(node)` -- returns `true` if a specific node's state changed.
- `is_edge_dirty(edge)` -- returns `true` if a specific edge's state changed.
- `mark_clean()` -- resets all per-tick dirty flags.

## Partitioned serialization

For large engines, Factorial supports **partitioned** serialization that splits the
state into five independent partitions:

| Index | Partition | Contents |
|---|---|---|
| 0 | Graph | [Production graph](../introduction/glossary.md#production-graph), sim state, strategy, pause flag |
| 1 | Processors | [Processor](../introduction/glossary.md#processor) configs, states, [modifiers](../introduction/glossary.md#modifier) |
| 2 | Inventories | Input and output [inventories](../introduction/glossary.md#inventory) |
| 3 | Transports | [Transport](../introduction/glossary.md#transport-strategy) configs and states |
| 4 | Junctions | [Junction](../introduction/glossary.md#junction) configs and states |

Partitioned snapshots use a separate magic number (`0xFAC70002`) to distinguish them
from legacy snapshots.

### Incremental serialization

When combined with dirty tracking, incremental serialization re-serializes only the
partitions that have changed, copying clean partitions from a baseline:

```rust
// First snapshot: serialize everything (no baseline).
let baseline = engine.serialize_incremental(None)?;

// After some ticks, only dirty partitions are re-serialized.
engine.step();
let incremental = engine.serialize_incremental(Some(&baseline))?;
```

`serialize_incremental()` clears the dirty partition flags after serialization. If no
baseline is provided, all partitions are serialized (equivalent to
`serialize_partitioned()`).

### Format detection

Use `Engine::detect_snapshot_format()` to determine which format a blob uses before
attempting to deserialize:

```rust
match Engine::detect_snapshot_format(&data) {
    SnapshotFormat::Legacy => {
        let engine = Engine::deserialize(&data)?;
    }
    SnapshotFormat::Partitioned => {
        let engine = Engine::deserialize_partitioned(&data)?;
    }
    SnapshotFormat::Unknown => {
        return Err("unrecognized snapshot format");
    }
}
```

## Snapshot ring buffer

For undo/replay, use the `SnapshotRingBuffer` to maintain a fixed-capacity history of
serialized engine states:

```rust
let mut buffer = SnapshotRingBuffer::new(10); // keep up to 10 snapshots

// After each tick, take a snapshot.
engine.step();
engine.take_snapshot(&mut buffer)?;

// Restore a previous state (0 = oldest, len-1 = newest).
if let Some(restored) = Engine::restore_snapshot(&buffer, 0)? {
    // `restored` is the engine state from the oldest snapshot.
}

// Get the most recent snapshot.
if let Some(latest) = buffer.latest() {
    println!("Latest snapshot: tick={}, {} bytes", latest.tick, latest.data.len());
}
```

When the buffer is full, the oldest snapshot is evicted automatically. The ring buffer
tracks total snapshots taken (including evicted) via `total_taken()`.

## API summary

| Operation | Method | Returns |
|---|---|---|
| Serialize (legacy) | `engine.serialize()` | `Result<Vec<u8>, SerializeError>` |
| Deserialize (legacy) | `Engine::deserialize(&bytes)` | `Result<Engine, DeserializeError>` |
| Deserialize with migrations | `Engine::deserialize_with_migrations(&bytes, &registry)` | `Result<Engine, DeserializeError>` |
| Serialize (partitioned) | `engine.serialize_partitioned()` | `Result<Vec<u8>, SerializeError>` |
| Serialize (incremental) | `engine.serialize_incremental(baseline)` | `Result<Vec<u8>, SerializeError>` |
| Deserialize (partitioned) | `Engine::deserialize_partitioned(&bytes)` | `Result<Engine, DeserializeError>` |
| Detect format | `Engine::detect_snapshot_format(&bytes)` | `SnapshotFormat` |
| Take snapshot | `engine.take_snapshot(&mut buffer)` | `Result<(), SerializeError>` |
| Restore snapshot | `Engine::restore_snapshot(&buffer, index)` | `Result<Option<Engine>, DeserializeError>` |
| Check dirty | `engine.is_dirty()` | `bool` |
| Mark clean | `engine.mark_clean()` | -- |
