# Task 7: Processor System

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 4 — Strategies (parallel) |
| **Branch** | `feat/processors` |
| **Depends on** | Task 5 (production graph) — must be merged to main |
| **Parallel with** | Task 6 (Transport) — separate worktree |
| **Skill** | `subagent-driven-development` or `executing-plans` |

## Shared File Notes

This task adds `pub mod processor;` to `lib.rs`. If running in parallel with Task 6, both branches modify `lib.rs`. Trivial additive conflict — resolve at merge time. Use `claim_file("crates/factorial-core/src/lib.rs")` when merging.

No overlap with Task 6's files — `processor.rs` and `transport/` are completely independent modules.

## Files

- Create: `crates/factorial-core/src/processor.rs`
- Modify: `crates/factorial-core/src/lib.rs` — add `pub mod processor;`

## Context

Design doc §5. Enum dispatch: `Source`, `Fixed`, `Property`, `Custom`. Modifier system with canonical stacking order.

Key types:
- `Processor` enum
- `SourceProcessor`: output type, base rate, depletion
- `FixedRecipe`: inputs/outputs/duration, multi-output support
- `PropertyProcessor`: transforms on item properties
- `CustomProcessor`: trait object escape hatch
- `ProcessorState`: Idle, Working { progress }, Stalled { reason }
- `Modifier`: speed/productivity/efficiency with Fixed64 values
- `ProcessContext`: constrained view for CustomProcessor callbacks

## Implementation

Key tests:
- FixedRecipe: consumes inputs, produces outputs after duration, stalls when output full
- SourceProcessor: produces at base rate, depletion (infinite, finite, decaying)
- PropertyProcessor: transforms property values
- Modifiers: speed affects duration, productivity gives bonus output
- Modifier stacking order is canonical (sorted by ModifierId)

## Commit

```bash
cargo test -p factorial-core -- processor && git add -A && git commit -m "feat: processor system with source, fixed, property, custom and modifiers"
```

## Verification

- `cargo test -p factorial-core -- processor` — all tests pass
- FixedRecipe completes after `duration` ticks
- Modifiers apply in canonical order (sorted by ModifierId)
- Stall detection works (output full, missing inputs)
