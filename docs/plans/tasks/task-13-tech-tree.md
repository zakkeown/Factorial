# Task 13: Tech Tree Module

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 6 — Framework Modules (parallel) |
| **Branch** | `feat/tech-tree` |
| **Depends on** | Tasks 9, 11 (events + serialization) — must be merged to main |
| **Parallel with** | Tasks 12, 14, 15 — separate worktrees |
| **Skill** | `subagent-driven-development` or `executing-plans` |

## Shared File Notes

This task creates a **new crate** (`crates/factorial-tech-tree/`). It modifies the workspace `Cargo.toml` to add the member. If running in parallel with Tasks 12/14, all three modify workspace `Cargo.toml`. Use `claim_file("Cargo.toml")` when merging, or merge sequentially.

## Files

- Create: `crates/factorial-tech-tree/Cargo.toml`
- Create: `crates/factorial-tech-tree/src/lib.rs`
- Modify: workspace `Cargo.toml` — add `"crates/factorial-tech-tree"` to members

## Context

Design doc §10 "Tech Tree Module". Research with prerequisites, multiple cost models, unlock system, infinite research.

Key types:
- `TechTree`: main module struct
- `Technology`: id, prerequisites, cost, unlocks, repeatable flag
- `ResearchCost` enum: Items, Points, Delivery, Rate, ItemRate, Custom
- `Unlock` enum: Building, Recipe, custom
- `CostScaling`: for infinite research
- Events: `ResearchStarted`, `ResearchCompleted`

## Implementation

Key tests:
- Research with prerequisites enforced
- Each cost model works
- Unlock events emitted
- Infinite research scales cost correctly
- Serialization round-trip of tech tree state

## Commit

```bash
cargo test -p factorial-tech-tree && git add -A && git commit -m "feat: tech tree module with prerequisites and cost models"
```

## Verification

- `cargo test -p factorial-tech-tree` — all tests pass
- Prerequisites block research until met
- All 6 cost models produce correct completion
- Infinite research cost scales per `CostScaling`
