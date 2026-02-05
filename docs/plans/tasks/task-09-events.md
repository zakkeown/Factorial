# Task 9: Event System

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 5b — Engine Systems (parallel) |
| **Branch** | `feat/events` |
| **Depends on** | Task 8 (simulation loop) — must be merged to main |
| **Parallel with** | Task 10 (Query API) — separate worktree |
| **Skill** | `subagent-driven-development` or `executing-plans` |

## Shared File Notes

Both this task and Task 10 modify `engine.rs` and `lib.rs`. These are **non-trivial** merge conflicts since both add methods to `Engine` and fields to the struct. Strategy:

1. Merge whichever finishes first to main
2. Second branch rebases onto updated main via `rebase_assist`
3. Use `detect_advanced_conflicts` before merging the second branch

Alternative: assign one session to do both T9 and T10 sequentially, avoiding the conflict entirely.

## Files

- Create: `crates/factorial-core/src/event.rs`
- Modify: `crates/factorial-core/src/engine.rs` — integrate event bus
- Modify: `crates/factorial-core/src/lib.rs` — add `pub mod event;`

## Context

Design doc §6. Typed events emitted in post-tick phase. Pre-allocated ring buffers per event type. Passive listeners (read-only) and reactive handlers (enqueue mutations for next tick).

Key types:
- Core event types: `ItemProduced`, `ItemConsumed`, `RecipeStarted`, `RecipeCompleted`, `BuildingStalled`, `BuildingResumed`, `ItemDelivered`, `TransportFull`, `NodeAdded`, `NodeRemoved`, etc.
- `EventBuffer<T>`: pre-allocated ring buffer for a specific event type
- `EventBus`: collection of typed event buffers
- Subscriber registration: `on::<T>(handler)`
- Event suppression: `suppress_event::<T>()`

## Implementation

Key tests:
- Events emitted during correct phases
- Ring buffer wraps correctly, drops oldest
- Passive listeners receive events in registration order
- Reactive handlers enqueue mutations for next tick (one-tick delay)
- Suppressed events have zero allocation cost
- Event counts match expected production

## Commit

```bash
cargo test -p factorial-core -- event && git add -A && git commit -m "feat: event system with typed ring buffers and subscriber ordering"
```

## Verification

- `cargo test -p factorial-core -- event` — all tests pass
- Ring buffer wraps and evicts oldest entries
- Reactive handler mutations appear on next tick (not current)
- Suppressed events don't allocate
