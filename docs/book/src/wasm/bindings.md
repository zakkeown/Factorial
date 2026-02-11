# WASM Bindings

The `factorial-wasm` crate provides a WebAssembly-compatible API for embedding
the Factorial engine in browser games, sandboxed plugin environments, or any
host that can consume a WASM module.

## Design

The API uses **integer handles** instead of opaque pointers, making it safe
for use across the WASM boundary. A fixed-size handle table (max 16 engines)
maps integer indices to engine instances.

All exported functions use C-compatible signatures with `#[no_mangle]` and
`extern "C"` linkage. Result codes are returned as `i32` constants:

| Code | Meaning               |
|------|-----------------------|
| `0`  | `RESULT_OK`           |
| `1`  | `RESULT_INVALID_HANDLE` |
| `2`  | `RESULT_SERIALIZE_ERROR` |
| `3`  | `RESULT_DESERIALIZE_ERROR` |

## Engine Lifecycle

```text
factorial_engine_create()  -> handle (i32)
factorial_step(handle, n)  -> advance n ticks
factorial_engine_destroy(handle)
```

## Graph Operations

```text
factorial_graph_add_node(handle, building_type) -> node_id (u64)
factorial_graph_add_edge(handle, from, to, transport_type, item_type) -> edge_id (u64)
factorial_graph_remove_node(handle, node_id)
factorial_graph_remove_edge(handle, edge_id)
```

Node and edge IDs are converted to/from `u64` FFI format via `KeyData::from_ffi()`.

## Processor & Transport Configuration

```text
factorial_processor_set_source(handle, node, item_type, rate)
factorial_processor_set_recipe(handle, node, recipe_id)
factorial_processor_set_demand(handle, node, item_type)
factorial_processor_set_passthrough(handle, node)
factorial_processor_enable(handle, node)
factorial_processor_disable(handle, node)

factorial_transport_set_flow(handle, edge, rate, capacity)
factorial_transport_set_item(handle, edge, speed, spacing)
factorial_transport_set_batch(handle, edge, size, interval)
factorial_transport_set_vehicle(handle, edge, capacity, travel_time)
```

## Queries

```text
factorial_query_node_state(handle, node)  -> state snapshot
factorial_query_edge_state(handle, edge)  -> transport state
factorial_query_node_count(handle) -> u32
factorial_query_tick(handle) -> u64
factorial_query_state_hash(handle) -> u64
```

## Events

Events use a **pull-based** model. After each `factorial_step`, the host
reads events from a cache that is cleared on the next step.

```text
factorial_event_count(handle) -> u32
factorial_event_get(handle, index) -> FlatEvent
```

The `FlatEvent` struct is `repr(C)` with fields:

| Field          | Type  | Description                |
|----------------|-------|----------------------------|
| `kind`         | `u32` | Event type discriminant    |
| `tick`         | `u64` | Tick when event occurred   |
| `node`         | `u64` | Related node ID            |
| `edge`         | `u64` | Related edge ID            |
| `item_type`    | `u32` | Item type involved         |
| `quantity`     | `u32` | Quantity involved          |
| `building_type`| `u32` | Building type involved     |
| `from_node`    | `u64` | Source node (edge events)  |
| `to_node`      | `u64` | Target node (edge events)  |

## Serialization

```text
factorial_serialize(handle, out_ptr, out_size) -> result code
factorial_deserialize(ptr, size, out_handle)   -> result code
```

Snapshots use bitcode encoding, the same format as the core crate.

## Logic Networks

```text
factorial_logic_register(handle)
factorial_logic_create_network(handle, color, out_id) -> result code
factorial_logic_add_to_network(handle, network, node)
factorial_logic_set_constant(handle, network, item_type, value)
```

## Memory Management

WASM-specific allocator exports for the host to manage linear memory:

```text
factorial_alloc(size, align) -> ptr
factorial_free(ptr, size, align)
```

## Building

```bash
# Add the WASM target
rustup target add wasm32-unknown-unknown

# Build
cargo build --package factorial-wasm --target wasm32-unknown-unknown --release
```

The resulting `.wasm` file can be loaded by any WASM runtime or bundled
into a web application with `wasm-bindgen` or similar tooling.
