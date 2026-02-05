# Task 5: Production Graph

> **For Claude:** Use `superpowers:test-driven-development` for implementation.

| Field | Value |
|-------|-------|
| **Phase** | 3 — Graph (sequential) |
| **Branch** | `main` (commit directly) |
| **Depends on** | Tasks 2, 3, 4 — all must be merged to main |
| **Parallel with** | Task 15a (FFI skeleton) — background task |
| **Skill** | `subagent-driven-development` |

## Files

- Create: `crates/factorial-core/src/graph.rs`
- Modify: `crates/factorial-core/src/lib.rs` — add `pub mod graph;`

## Context

Design doc §2 "Production Graph". Nodes (buildings), edges (transport links), junctions. Topological ordering cached and invalidated on mutation. Mutations are queued and applied during pre-tick.

This is the **largest single module**. Take care with the implementation.

## Step 1: Write production graph types and tests

Key types:

- `NodeData`: building type, current state
- `EdgeData`: source/dest nodes, transport type
- `TopologicalGraph`: the default graph implementation
- Queued mutation system

The graph stores nodes in a `SlotMap<NodeId, NodeData>`, edges in `SlotMap<EdgeId, EdgeData>`, and adjacency lists for fast traversal.

Topological sort uses Kahn's algorithm. The sort result is cached and a `dirty` flag marks when it needs recomputation.

Key tests:
- Add/remove nodes
- Connect edges
- Topological sort correctness
- Queued mutations
- Cycle detection
- Adjacent node queries

## Step 2: Run tests, commit

```bash
cargo test -p factorial-core -- graph::tests && git add -A && git commit -m "feat: production graph with topological sort and queued mutations"
```

## Verification

- `cargo test -p factorial-core -- graph::tests` — all tests pass
- Kahn's algorithm produces correct topological order
- Cycle detection returns error on cyclic graphs
- Queued mutations apply atomically
- Adjacency queries return correct neighbors
