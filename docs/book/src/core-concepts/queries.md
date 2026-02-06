# Queries

Queries are **cheap, read-only** functions that inspect the current state of the
simulation. They never modify the engine and are safe to call at any point -- including
every frame from a rendering loop. Use queries to drive UI, debug overlays, analytics
dashboards, and automated testing.

## Node snapshots

The primary query for rendering is `snapshot_node()`, which returns a complete
[snapshot](../introduction/glossary.md#snapshot) of a single
[node](../introduction/glossary.md#node):

```rust
// From crates/factorial-core/examples/events_and_queries.rs

// Query a single node.
if let Some(snap) = engine.snapshot_node(assembler) {
    println!(
        "Node {:?}: state={:?}, progress={:.2}",
        snap.id, snap.processor_state, snap.progress
    );
    println!("  Input:  {:?}", snap.input_contents);
    println!("  Output: {:?}", snap.output_contents);
}
```

A `NodeSnapshot` contains:

| Field | Type | Description |
|---|---|---|
| `id` | `NodeId` | The node's unique identifier |
| `building_type` | `BuildingTypeId` | The [building type](../introduction/glossary.md#building-type-id) this node was created from |
| `processor_state` | `ProcessorState` | Current state: `Idle`, `Working`, or `Stalled` |
| `progress` | `Fixed64` | Crafting progress as a fraction (0.0 to 1.0) |
| `input_contents` | `Vec<(ItemTypeId, u32)>` | Items currently in the input [inventory](../introduction/glossary.md#inventory) |
| `output_contents` | `Vec<(ItemTypeId, u32)>` | Items currently in the output inventory |
| `input_edges` | `Vec<EdgeId>` | [Edges](../introduction/glossary.md#edge) feeding into this node |
| `output_edges` | `Vec<EdgeId>` | Edges leaving this node |

### Bulk snapshot

To query every node at once:

```rust
// From crates/factorial-core/examples/events_and_queries.rs

let snapshots = engine.snapshot_all_nodes();
for snap in &snapshots {
    let name = match snap.building_type.0 {
        0 => "Mine",
        1 => "Assembler",
        _ => "Unknown",
    };
    println!(
        "  {}: state={:?}, progress={:.2}",
        name, snap.processor_state, snap.progress
    );
}
```

`snapshot_all_nodes()` iterates all nodes in the [production graph](../introduction/glossary.md#production-graph)
and collects their snapshots into a `Vec<NodeSnapshot>`.

## Transport snapshots

Query the state of a [transport](../introduction/glossary.md#transport-strategy) edge:

```rust
// From crates/factorial-core/examples/events_and_queries.rs

if let Some(tsnap) = engine.snapshot_transport(belt) {
    println!(
        "  utilization={:.2}, items_in_transit={}",
        tsnap.utilization, tsnap.items_in_transit
    );
}
```

A `TransportSnapshot` contains:

| Field | Type | Description |
|---|---|---|
| `id` | `EdgeId` | The edge's unique identifier |
| `from` | `NodeId` | Source node |
| `to` | `NodeId` | Destination node |
| `utilization` | `Fixed64` | How full the transport is (0.0 to 1.0) |
| `items_in_transit` | `u32` | Number of items currently being carried |

## Processor progress

Query the crafting progress of a specific node as a fraction between 0.0 and 1.0:

```rust
// From crates/factorial-core/examples/events_and_queries.rs

let progress = engine
    .get_processor_progress(assembler)
    .unwrap_or(Fixed64::ZERO);
println!("  Assembler progress: {:.2}", progress);
```

Returns `Some(progress)` when the [processor](../introduction/glossary.md#processor) is
in the `Working` state with a Fixed recipe. Returns `None` if the node does not exist
or the processor is not a Fixed type. For Source and Demand processors, progress is
always zero.

## Edge utilization

Query how full a transport edge is:

```rust
let utilization = engine.get_edge_utilization(edge_id);
```

Returns a [fixed-point](../introduction/glossary.md#fixed-point) value between 0.0
(empty) and 1.0 (full). Useful for rendering belt fullness indicators or optimizing
factory throughput.

## Graph counts

Quick counts that do not allocate:

```rust
let nodes = engine.node_count();   // total nodes in the graph
let edges = engine.edge_count();   // total edges in the graph
```

## Adjacency queries

Get the edges connected to a specific node:

```rust
let input_edges: &[EdgeId] = engine.get_inputs(node_id);
let output_edges: &[EdgeId] = engine.get_outputs(node_id);
```

These return slices -- no allocation, no copying. Use them to walk the graph from a
specific node.

## Node diagnostics

For debugging, `diagnose_node()` returns detailed diagnostic information about a node,
including its current [stall](../introduction/glossary.md#stall) reason (if any),
processor configuration, and inventory state:

```rust
if let Some(diag) = engine.diagnose_node(node_id) {
    println!("Stall reason: {:?}", diag.stall_reason);
    println!("Input utilization: {:?}", diag.input_utilization);
    println!("Output utilization: {:?}", diag.output_utilization);
}
```

## Complete query API reference

| Method | Returns | Allocates? | Description |
|---|---|---|---|
| `snapshot_node(node)` | `Option<NodeSnapshot>` | Yes (Vec) | Full snapshot of one node |
| `snapshot_all_nodes()` | `Vec<NodeSnapshot>` | Yes (Vec) | Snapshots of all nodes |
| `snapshot_transport(edge)` | `Option<TransportSnapshot>` | No | Snapshot of one transport edge |
| `get_processor_progress(node)` | `Option<Fixed64>` | No | Crafting progress (0.0--1.0) |
| `get_edge_utilization(edge)` | `Option<Fixed64>` | No | Transport fullness (0.0--1.0) |
| `node_count()` | `usize` | No | Total node count |
| `edge_count()` | `usize` | No | Total edge count |
| `get_inputs(node)` | `&[EdgeId]` | No | Incoming edges for a node |
| `get_outputs(node)` | `&[EdgeId]` | No | Outgoing edges for a node |
| `diagnose_node(node)` | `Option<DiagnosticInfo>` | Yes | Detailed node diagnostics |

All query methods take `&self` -- they require only an immutable reference to the engine.
You can safely interleave queries with rendering code without holding a mutable borrow.
