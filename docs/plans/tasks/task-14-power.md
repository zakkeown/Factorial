# Task 14: Power Networks Module

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 6 — Framework Modules (parallel) |
| **Branch** | `feat/power` |
| **Depends on** | Tasks 9, 11 (events + serialization) — must be merged to main |
| **Parallel with** | Tasks 12, 13, 15 — separate worktrees |
| **Skill** | `subagent-driven-development` or `executing-plans` |

## Shared File Notes

This task creates a **new crate** (`crates/factorial-power/`). It modifies the workspace `Cargo.toml` to add the member. If running in parallel with Tasks 12/13, all three modify workspace `Cargo.toml`. Use `claim_file("Cargo.toml")` when merging, or merge sequentially.

## Files

- Create: `crates/factorial-power/Cargo.toml`
- Create: `crates/factorial-power/src/lib.rs`
- Modify: workspace `Cargo.toml` — add `"crates/factorial-power"` to members

## Context

Design doc §10 "Power Networks Module". Producer/consumer/storage balance per network. Satisfaction ratio affects building performance.

Key types:
- `PowerNetwork`: producers, consumers, storage node lists, satisfaction ratio
- `PowerModule`: manages all networks
- Tick logic: sum production/demand, balance with storage, compute satisfaction
- Events: `PowerGridBrownout`, `PowerGridRestored`

## Implementation

Key tests:
- Balanced network: satisfaction = 1.0
- Under-powered: satisfaction < 1.0
- Storage charges/discharges correctly
- Brownout event emitted on deficit
- Restored event emitted on recovery

## Commit

```bash
cargo test -p factorial-power && git add -A && git commit -m "feat: power networks module with satisfaction balancing"
```

## Verification

- `cargo test -p factorial-power` — all tests pass
- Balanced network satisfaction is exactly `Fixed64::from_num(1)`
- Storage absorbs excess and releases during deficit
- Events fire on state transitions (not every tick)
