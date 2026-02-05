# Task 10: Query API

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 5b — Engine Systems (parallel) |
| **Branch** | `feat/query` |
| **Depends on** | Task 8 (simulation loop) — must be merged to main |
| **Parallel with** | Task 9 (Events) — separate worktree |
| **Skill** | `subagent-driven-development` or `executing-plans` |

## Shared File Notes

Both this task and Task 9 modify `engine.rs` and `lib.rs`. See Task 9 notes for merge strategy. The safer option is running T9 and T10 sequentially in one session.

## Files

- Create: `crates/factorial-core/src/query.rs`
- Modify: `crates/factorial-core/src/engine.rs` — add query methods
- Modify: `crates/factorial-core/src/lib.rs` — add `pub mod query;`

## Context

Design doc §7. Read-only access to simulation state for rendering and UI. Single-node queries, bulk queries, graph introspection.

Key methods on `Engine`:
- `get_processor_progress(node) -> Fixed64`
- `get_processor_state(node) -> ProcessorState`
- `get_inventory(node, slot) -> &InventorySlot`
- `get_transport_state(edge) -> TransportSnapshot`
- `get_edge_utilization(edge) -> Fixed64`
- `snapshot_all_nodes() -> &[NodeSnapshot]`
- `node_count()`, `edge_count()`
- `get_inputs(node)`, `get_outputs(node)`

## Implementation

Key tests:
- Query returns correct processor state
- Inventory query matches actual contents
- Bulk snapshot covers all nodes
- Graph introspection returns correct counts and adjacency

## Commit

```bash
cargo test -p factorial-core -- query && git add -A && git commit -m "feat: query API for reading simulation state"
```

## Verification

- `cargo test -p factorial-core -- query` — all tests pass
- All query methods return correct data
- Snapshot covers every node in the graph
- No mutation possible through query API (read-only)
