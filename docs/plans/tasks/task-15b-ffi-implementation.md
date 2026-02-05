# Task 15b: C FFI Layer Implementation

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 6 — Framework Modules (parallel) |
| **Branch** | `feat/ffi` |
| **Depends on** | Task 11 (serialization), Task 15a (FFI skeleton) — must be merged to main |
| **Parallel with** | Tasks 12, 13, 14 — separate worktrees |
| **Skill** | `subagent-driven-development` or `executing-plans` |

## Key Optimization

In the original plan, T15 (FFI) ran after T12-14 (framework modules). By splitting T15 into skeleton (15a) + implementation (15b), the core FFI functions can now run **in parallel** with the framework modules. This turns a 3-way parallel window into 4-way.

Module-specific FFI functions (stats queries, tech tree commands, power network info) can be added in Task 16 or as a follow-up after the framework modules merge.

## Shared File Notes

This task only modifies files inside `crates/factorial-ffi/`. No conflicts with T12-14 which each have their own crate directories. The workspace `Cargo.toml` was already updated in T15a.

## Files

- Modify: `crates/factorial-ffi/src/lib.rs` — implement all FFI functions
- Modify: `crates/factorial-ffi/Cargo.toml` — may need additional deps

## Context

Design doc §11 "C API Design". First-class C API with `extern "C"` functions. Pull-based event delivery. Bulk queries return pointers into engine-owned memory. `cbindgen` generates headers.

Key functions:
- `factorial_create`, `factorial_destroy`, `factorial_step`
- `factorial_add_node`, `factorial_remove_node`, `factorial_connect`
- `factorial_get_processor_state`, `factorial_get_inventory_count`
- `factorial_snapshot_nodes`, `factorial_query_belt_items`
- `factorial_poll_events_*`
- `factorial_serialize`, `factorial_deserialize`, `factorial_free_buffer`

Error handling: `FactorialResult` status codes, `catch_unwind` at FFI boundary.

## Implementation

Key tests:
- Create/destroy engine lifecycle
- Add node and step produces expected state
- Serialize/deserialize through C API
- Poll events returns correct data
- Null pointer inputs handled gracefully

## Commit

```bash
cargo test -p factorial-ffi && git add -A && git commit -m "feat: C FFI layer with cbindgen header generation"
```

## Verification

- `cargo test -p factorial-ffi` — all tests pass
- `cbindgen` generates valid C header
- Null pointer inputs return error codes (no panic)
- `catch_unwind` wraps all extern functions
