# Task 8: Simulation Loop

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 5a — Engine (sequential) |
| **Branch** | `main` (commit directly) |
| **Depends on** | Tasks 6, 7 (transport + processors) — must be merged to main |
| **Parallel with** | None |
| **Skill** | `subagent-driven-development` |

## Files

- Create: `crates/factorial-core/src/engine.rs`
- Create: `crates/factorial-core/src/sim.rs`
- Modify: `crates/factorial-core/src/lib.rs` — add `pub mod engine; pub mod sim;`

## Context

Design doc §3. The `Engine` struct ties everything together. Six-phase simulation step: pre-tick → transport → process → component → post-tick → bookkeeping. Three strategies: tick, delta, event.

Key types:
- `Engine`: owns graph, component storage, registry ref, event buffers, simulation state
- `SimulationStrategy` trait/enum: `Tick`, `Delta`, `Event`
- `SimState`: current tick, queued mutations, RNG state

The engine orchestrates:
1. Pre-tick: apply queued mutations, inject player actions
2. Transport: move items along edges (grouped by transport variant)
3. Process: buildings consume inputs, advance recipes, produce outputs (topological order)
4. Component: module-registered systems run
5. Post-tick: deliver buffered events, reactive handlers enqueue mutations
6. Bookkeeping: update stats, state hash

## Implementation

Key tests:
- Single tick: source → transport → consumer chain works
- Multi-tick: recipe completes after duration ticks
- Delta mode: advance(dt) runs correct number of fixed steps
- Queued mutation: add node mid-tick applies next tick
- Processing order: topological order upstream-before-downstream
- Determinism: same inputs = same state hash

## Commit

```bash
cargo test -p factorial-core -- engine && git add -A && git commit -m "feat: simulation engine with tick/delta/event strategies"
```

## Verification

- `cargo test -p factorial-core -- engine` — all tests pass
- Full source → transport → process pipeline works in single tick
- Determinism test: two runs from identical state produce identical hash
- Delta mode runs correct number of sub-ticks
