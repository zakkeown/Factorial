# Task 12: Production Statistics Module

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 6 — Framework Modules (parallel) |
| **Branch** | `feat/stats` |
| **Depends on** | Tasks 9, 11 (events + serialization) — must be merged to main |
| **Parallel with** | Tasks 13, 14, 15 — separate worktrees |
| **Skill** | `subagent-driven-development` or `executing-plans` |

## Shared File Notes

This task creates a **new crate** (`crates/factorial-stats/`). It modifies the workspace `Cargo.toml` to add the member. If running in parallel with Tasks 13/14, all three modify workspace `Cargo.toml`. Use `claim_file("Cargo.toml")` when merging, or merge sequentially.

No overlap with other module task files — each is a separate crate.

## Files

- Create: `crates/factorial-stats/Cargo.toml`
- Create: `crates/factorial-stats/src/lib.rs`
- Modify: workspace `Cargo.toml` — add `"crates/factorial-stats"` to members

## Context

Design doc §10 "Production Statistics Module". Per-node, per-edge, per-item-type throughput over configurable time windows. Listens to core events.

Key types:
- `ProductionStats`: main module struct
- Per-node: production rate, consumption rate, idle ratio, stall ratio, uptime
- Per-edge: throughput, utilization
- Global: total production/consumption per item type
- Historical ring buffer per metric

## Implementation

Key tests:
- Rates computed correctly from events
- Rolling averages over configurable windows
- Historical data in ring buffer
- Idle/stall ratios match building state

## Commit

```bash
cargo test -p factorial-stats && git add -A && git commit -m "feat: production statistics module"
```

## Verification

- `cargo test -p factorial-stats` — all tests pass
- Throughput rates match expected values
- Rolling windows produce correct averages
- Ring buffer wraps correctly
