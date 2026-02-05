# Task 6: Transport Strategies

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 4 — Strategies (parallel) |
| **Branch** | `feat/transport` |
| **Depends on** | Task 5 (production graph) — must be merged to main |
| **Parallel with** | Task 7 (Processors) — separate worktree |
| **Skill** | `subagent-driven-development` or `executing-plans` |

## Shared File Notes

This task adds `pub mod transport;` to `lib.rs`. If running in parallel with Task 7, both branches modify `lib.rs`. Trivial additive conflict — resolve at merge time. Use `claim_file("crates/factorial-core/src/lib.rs")` when merging.

No overlap with Task 7's files — `transport.rs` and `processor.rs` are completely independent modules.

## Files

- Create: `crates/factorial-core/src/transport.rs` (module root)
- Create: `crates/factorial-core/src/transport/flow.rs`
- Create: `crates/factorial-core/src/transport/item.rs`
- Create: `crates/factorial-core/src/transport/batch.rs`
- Create: `crates/factorial-core/src/transport/vehicle.rs`
- Modify: `crates/factorial-core/src/lib.rs` — add `pub mod transport;`

## Context

Design doc §4. Enum dispatch with `Flow`, `Item`, `Batch`, `Vehicle`, `Custom` variants. Each variant processed in homogeneous batches for cache locality.

Key types:
- `Transport` enum (the dispatch enum from the design doc)
- `FlowTransport`: rate + buffer + latency
- `ItemTransport`: belt with slot array, lanes, speed
- `BatchTransport`: batch_size + cycle_time
- `VehicleTransport`: capacity + travel_time + schedule

Transport state is stored externally in typed arenas (design doc §9 "Transport State Storage"):
- `FlowState`: rate buffer, tiny
- `BeltState`: flat slot array, pre-allocated
- `BatchState`: simple counter
- `VehicleState`: position, cargo

Each transport implements an `advance` method called during the transport phase.

## Implementation

Key tests:
- FlowTransport: items flow at declared rate, respect capacity, latency delay
- ItemTransport: items advance through belt slots, back up when full
- BatchTransport: discrete chunks per cycle
- VehicleTransport: travel time, loading, cargo

## Commit

```bash
cargo test -p factorial-core -- transport && git add -A && git commit -m "feat: transport strategies (flow, item, batch, vehicle)"
```

## Verification

- `cargo test -p factorial-core -- transport` — all tests pass
- Each transport variant advances items correctly
- Back-pressure works (items stop when destination full)
- State is stored in external arenas, not inline
