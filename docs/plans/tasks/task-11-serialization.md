# Task 11: Serialization & Snapshots

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 5c — Serialization (sequential) |
| **Branch** | `main` (commit directly) |
| **Depends on** | Tasks 9, 10 (events + query) — must be merged to main |
| **Parallel with** | None |
| **Skill** | `subagent-driven-development` |

## Files

- Create: `crates/factorial-core/src/serialize.rs`
- Modify: `crates/factorial-core/src/engine.rs` — add serialize/deserialize methods
- Modify: `crates/factorial-core/src/lib.rs` — add `pub mod serialize;`

## Context

Design doc §8. Binary format via `bitcode`. Versioned sections. Module serialization hooks. Snapshot ring buffer for undo/replay.

Key types:
- `SnapshotHeader`: magic number, core version, tick count
- `engine.serialize() -> Vec<u8>`
- `Engine::deserialize(data, registry) -> Result<Engine>`
- `engine.state_hash() -> u64` (incremental)
- `engine.subsystem_hashes() -> SubsystemHashes`
- Module serialization registration: `engine.register_serializer(name, impl)`
- Snapshot ring buffer with configurable capacity

## Implementation

Key tests:
- Round-trip: serialize → deserialize → same state hash
- NodeId stability across round-trips
- Version mismatch produces explicit error
- Module serialization hooks called correctly
- State hash changes when state changes
- State hash identical for identical state
- Snapshot ring buffer evicts oldest

## Commit

```bash
cargo test -p factorial-core -- serialize && git add -A && git commit -m "feat: serialization with bitcode, state hashing, and snapshot ring buffer"
```

## Verification

- `cargo test -p factorial-core -- serialize` — all tests pass
- Round-trip preserves state hash
- Version mismatch returns `Err`, not panic
- Snapshot ring buffer has correct capacity behavior
