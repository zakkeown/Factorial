# Function Reference

Complete reference for all `extern "C"` functions exported by the
`factorial-ffi` crate. Signatures are given in C. All functions follow
the conventions described in [API Conventions & Safety](conventions.md).

---

## Lifecycle

Functions for creating and destroying engine instances.

### `factorial_create`

```c
FactorialEngine *factorial_create(void);
```

Create a new engine with the **Tick** simulation strategy. Returns a
heap-allocated opaque engine pointer. The caller must eventually free it
with `factorial_destroy()`.

Returns `NULL` on internal error.

See: [The Production Graph](../core-concepts/production-graph.md),
[Determinism & Fixed-Point](../core-concepts/determinism.md)

---

### `factorial_create_delta`

```c
FactorialEngine *factorial_create_delta(uint64_t fixed_timestep);
```

Create a new engine with the **Delta** simulation strategy.
`fixed_timestep` is the number of ticks per fixed simulation step.
Returns a heap-allocated opaque engine pointer.

Returns `NULL` on internal error.

See: [Determinism & Fixed-Point](../core-concepts/determinism.md)

---

### `factorial_destroy`

```c
FactorialResult factorial_destroy(FactorialEngine *engine);
```

Destroy an engine and free its memory. After this call, `engine` is
invalid and must not be used. Also clears the internal event cache.

Returns `FACTORIAL_RESULT_NULL_POINTER` if `engine` is null.

---

## Simulation

Functions that advance the simulation forward in time.

### `factorial_step`

```c
FactorialResult factorial_step(FactorialEngine *engine);
```

Advance the simulation by one tick. Clears the event buffer from the
previous step before executing. Events produced during this step can be
retrieved with `factorial_poll_events()`.

Returns `FACTORIAL_RESULT_POISONED` if the engine is poisoned. Sets the
poisoned flag if an internal panic occurs.

See: [The Production Graph](../core-concepts/production-graph.md)

---

### `factorial_advance`

```c
FactorialResult factorial_advance(FactorialEngine *engine, uint64_t dt);
```

Advance the simulation by `dt` ticks. In Tick mode, `dt` is ignored and
exactly one step runs. In Delta mode, `dt` is accumulated against the
fixed timestep.

Clears the event buffer before executing.

See: [Determinism & Fixed-Point](../core-concepts/determinism.md)

---

## Graph Mutation

Functions for modifying the production graph. Mutations are **queued**
and applied atomically by `factorial_apply_mutations()`.

### `factorial_add_node`

```c
FactorialResult factorial_add_node(
    FactorialEngine *engine,
    uint32_t building_type,
    FfiPendingNodeId *out_pending
);
```

Queue a node addition. `building_type` identifies the building type.
A pending node ID is written to `out_pending`. The real node ID is
assigned when `factorial_apply_mutations()` is called.

See: [The Production Graph](../core-concepts/production-graph.md)

---

### `factorial_remove_node`

```c
FactorialResult factorial_remove_node(
    FactorialEngine *engine,
    FfiNodeId node_id
);
```

Queue a node for removal. The removal takes effect when
`factorial_apply_mutations()` is called.

See: [The Production Graph](../core-concepts/production-graph.md)

---

### `factorial_connect`

```c
FactorialResult factorial_connect(
    FactorialEngine *engine,
    FfiNodeId from_node,
    FfiNodeId to_node,
    FfiPendingEdgeId *out_pending
);
```

Queue an edge connecting `from_node` to `to_node`. A pending edge ID is
written to `out_pending`. The real edge ID is assigned when
`factorial_apply_mutations()` is called.

See: [Transport Strategies](../core-concepts/transport.md)

---

### `factorial_disconnect`

```c
FactorialResult factorial_disconnect(
    FactorialEngine *engine,
    FfiEdgeId edge_id
);
```

Queue an edge for removal (disconnect). The removal takes effect when
`factorial_apply_mutations()` is called.

See: [Transport Strategies](../core-concepts/transport.md)

---

### `factorial_apply_mutations`

```c
FactorialResult factorial_apply_mutations(
    FactorialEngine *engine,
    FfiMutationResult *out_result
);
```

Apply all queued graph mutations atomically. The `out_result` struct is
populated with arrays mapping pending IDs to their real IDs:

```c
typedef struct {
    const FfiIdPair *added_nodes;
    uint32_t added_node_count;
    const FfiIdPair *added_edges;
    uint32_t added_edge_count;
} FfiMutationResult;

typedef struct {
    uint64_t pending_id;
    uint64_t real_id;
} FfiIdPair;
```

The pointers in `FfiMutationResult` are valid until the next call to
`factorial_apply_mutations()` or `factorial_destroy()`.

See: [The Production Graph](../core-concepts/production-graph.md)

---

## Processor Configuration

Functions for assigning processors (production logic) to nodes.

### `factorial_set_source`

```c
FactorialResult factorial_set_source(
    FactorialEngine *engine,
    FfiNodeId node_id,
    uint32_t item_type,
    int64_t rate
);
```

Set a node's processor to **Source**. The node will produce items of
`item_type` at the given `rate`. The `rate` parameter is raw Fixed64 bits
(Q32.32 format); shift an integer left by 32 to construct it from C
(e.g., `(int64_t)5 << 32` for a rate of 5).

The source has infinite depletion by default.

See: [Processors](../core-concepts/processors.md),
[Determinism & Fixed-Point](../core-concepts/determinism.md)

---

### `factorial_set_fixed_processor`

```c
FactorialResult factorial_set_fixed_processor(
    FactorialEngine *engine,
    FfiNodeId node_id,
    const FfiRecipe *recipe
);
```

Set a node's processor to **FixedRecipe**. The recipe defines inputs,
outputs, and duration:

```c
typedef struct {
    uint32_t item_type;
    uint32_t quantity;
} FfiItemStack;

typedef struct {
    uint32_t input_count;
    const FfiItemStack *inputs;
    uint32_t output_count;
    const FfiItemStack *outputs;
    uint32_t duration;
} FfiRecipe;
```

The `inputs` and `outputs` arrays must remain valid for the duration of
the call. The library copies the data internally.

See: [Processors](../core-concepts/processors.md)

---

## Transport Configuration

Functions for assigning transport strategies to edges.

### `factorial_set_flow_transport`

```c
FactorialResult factorial_set_flow_transport(
    FactorialEngine *engine,
    FfiEdgeId edge_id,
    int64_t rate
);
```

Set an edge's transport to **FlowTransport** with the given `rate`
(raw Fixed64 bits, Q32.32). Uses a default buffer capacity of 1000 and
zero latency.

See: [Transport Strategies](../core-concepts/transport.md)

---

### `factorial_set_item_transport`

```c
FactorialResult factorial_set_item_transport(
    FactorialEngine *engine,
    FfiEdgeId edge_id,
    int64_t speed,
    uint32_t slot_count,
    uint8_t lanes
);
```

Set an edge's transport to **ItemTransport** (conveyor belt). `speed` is
raw Fixed64 bits (Q32.32). `slot_count` is the number of item slots on
the belt. `lanes` is the number of parallel lanes.

See: [Transport Strategies](../core-concepts/transport.md)

---

### `factorial_set_batch_transport`

```c
FactorialResult factorial_set_batch_transport(
    FactorialEngine *engine,
    FfiEdgeId edge_id,
    uint32_t batch_size,
    uint32_t cycle_time
);
```

Set an edge's transport to **BatchTransport**. Items are moved in
batches of `batch_size` every `cycle_time` ticks.

See: [Transport Strategies](../core-concepts/transport.md)

---

### `factorial_set_vehicle_transport`

```c
FactorialResult factorial_set_vehicle_transport(
    FactorialEngine *engine,
    FfiEdgeId edge_id,
    uint32_t capacity,
    uint32_t travel_time
);
```

Set an edge's transport to **VehicleTransport**. A vehicle carries up to
`capacity` items and takes `travel_time` ticks per trip.

See: [Transport Strategies](../core-concepts/transport.md)

---

## Inventory Configuration

Functions for configuring node inventories.

### `factorial_set_input_capacity`

```c
FactorialResult factorial_set_input_capacity(
    FactorialEngine *engine,
    FfiNodeId node_id,
    uint32_t capacity
);
```

Set the input inventory capacity for a node. Creates an inventory with
one input slot and one output slot, each with the given capacity.

---

### `factorial_set_output_capacity`

```c
FactorialResult factorial_set_output_capacity(
    FactorialEngine *engine,
    FfiNodeId node_id,
    uint32_t capacity
);
```

Set the output inventory capacity for a node. Creates an inventory with
one input slot and one output slot, each with the given capacity.

---

## Queries

Read-only functions for inspecting engine state. These take
`const FactorialEngine *` and do not modify the engine.

### `factorial_node_count`

```c
FactorialResult factorial_node_count(
    const FactorialEngine *engine,
    uint32_t *out_count
);
```

Write the number of nodes in the graph to `out_count`.

See: [Queries](../core-concepts/queries.md)

---

### `factorial_edge_count`

```c
FactorialResult factorial_edge_count(
    const FactorialEngine *engine,
    uint32_t *out_count
);
```

Write the number of edges in the graph to `out_count`.

See: [Queries](../core-concepts/queries.md)

---

### `factorial_get_tick`

```c
FactorialResult factorial_get_tick(
    const FactorialEngine *engine,
    uint64_t *out_tick
);
```

Write the current simulation tick counter to `out_tick`.

See: [Queries](../core-concepts/queries.md),
[Determinism & Fixed-Point](../core-concepts/determinism.md)

---

### `factorial_get_state_hash`

```c
FactorialResult factorial_get_state_hash(
    const FactorialEngine *engine,
    uint64_t *out_hash
);
```

Write a deterministic hash of the entire engine state to `out_hash`.
Useful for desync detection in multiplayer scenarios. Two engines that
have processed identical inputs will produce identical hashes.

See: [Queries](../core-concepts/queries.md),
[Determinism & Fixed-Point](../core-concepts/determinism.md)

---

### `factorial_get_processor_state`

```c
FactorialResult factorial_get_processor_state(
    const FactorialEngine *engine,
    FfiNodeId node_id,
    FfiProcessorInfo *out_info
);
```

Write the processor state for the given node to `out_info`:

```c
typedef enum {
    FFI_PROCESSOR_STATE_IDLE = 0,
    FFI_PROCESSOR_STATE_WORKING = 1,
    FFI_PROCESSOR_STATE_STALLED_MISSING_INPUTS = 2,
    FFI_PROCESSOR_STATE_STALLED_OUTPUT_FULL = 3,
    FFI_PROCESSOR_STATE_STALLED_NO_POWER = 4,
    FFI_PROCESSOR_STATE_STALLED_DEPLETED = 5,
} FfiProcessorState;

typedef struct {
    FfiProcessorState state;
    uint32_t progress;
} FfiProcessorInfo;
```

Returns `FACTORIAL_RESULT_NODE_NOT_FOUND` if `node_id` does not exist.

See: [Processors](../core-concepts/processors.md),
[Queries](../core-concepts/queries.md)

---

## Inventory Queries

### `factorial_get_input_inventory_count`

```c
FactorialResult factorial_get_input_inventory_count(
    const FactorialEngine *engine,
    FfiNodeId node_id,
    uint32_t *out_count
);
```

Write the total item count across all input inventory slots for the
given node to `out_count`.

Returns `FACTORIAL_RESULT_NODE_NOT_FOUND` if `node_id` does not exist.

See: [Queries](../core-concepts/queries.md)

---

### `factorial_get_output_inventory_count`

```c
FactorialResult factorial_get_output_inventory_count(
    const FactorialEngine *engine,
    FfiNodeId node_id,
    uint32_t *out_count
);
```

Write the total item count across all output inventory slots for the
given node to `out_count`.

Returns `FACTORIAL_RESULT_NODE_NOT_FOUND` if `node_id` does not exist.

See: [Queries](../core-concepts/queries.md)

---

## Events

### `factorial_poll_events`

```c
FactorialResult factorial_poll_events(
    const FactorialEngine *engine,
    FfiEventBuffer *out_buffer
);
```

Retrieve all events buffered since the last step. The `out_buffer`
struct is populated with a pointer to an engine-owned array and its
count:

```c
typedef struct {
    const FfiEvent *events;
    uint32_t count;
} FfiEventBuffer;
```

The buffer is valid until the next call to `factorial_step()`,
`factorial_advance()`, or `factorial_destroy()`. Do not free it.

Each event is a flat `repr(C)` struct:

```c
typedef enum {
    FFI_EVENT_KIND_ITEM_PRODUCED = 0,
    FFI_EVENT_KIND_ITEM_CONSUMED = 1,
    FFI_EVENT_KIND_RECIPE_STARTED = 2,
    FFI_EVENT_KIND_RECIPE_COMPLETED = 3,
    FFI_EVENT_KIND_BUILDING_STALLED = 4,
    FFI_EVENT_KIND_BUILDING_RESUMED = 5,
    FFI_EVENT_KIND_ITEM_DELIVERED = 6,
    FFI_EVENT_KIND_TRANSPORT_FULL = 7,
    FFI_EVENT_KIND_NODE_ADDED = 8,
    FFI_EVENT_KIND_NODE_REMOVED = 9,
    FFI_EVENT_KIND_EDGE_ADDED = 10,
    FFI_EVENT_KIND_EDGE_REMOVED = 11,
} FfiEventKind;

typedef struct {
    FfiEventKind kind;
    uint64_t tick;
    FfiNodeId node;
    FfiEdgeId edge;
    uint32_t item_type;
    uint32_t quantity;
    uint32_t building_type;
    FfiNodeId from_node;
    FfiNodeId to_node;
} FfiEvent;
```

Fields that are not applicable to a given event kind are set to 0.

See: [Events](../core-concepts/events.md)

---

## Poison Inspection

### `factorial_is_poisoned`

```c
bool factorial_is_poisoned(const FactorialEngine *engine);
```

Return `true` if the engine is poisoned (a previous panic left it in an
inconsistent state). Returns `false` if `engine` is null.

See: [API Conventions & Safety -- Poisoned Flag](conventions.md#poisoned-flag)

---

### `factorial_clear_poison`

```c
FactorialResult factorial_clear_poison(FactorialEngine *engine);
```

Clear the poisoned flag, allowing the engine to be used again. Use with
caution -- if the panic corrupted internal state, subsequent behavior is
undefined.

See: [API Conventions & Safety -- Poisoned Flag](conventions.md#poisoned-flag)

---

## Serialization

Functions for saving and restoring engine state.

### `factorial_serialize`

```c
FactorialResult factorial_serialize(
    const FactorialEngine *engine,
    FfiByteBuffer *out_buffer
);
```

Serialize the entire engine state to a binary buffer. The caller
receives an `FfiByteBuffer`:

```c
typedef struct {
    uint8_t *data;
    size_t len;
} FfiByteBuffer;
```

The buffer is heap-allocated by the library. The caller **must** free it
with `factorial_free_buffer()` when done. On error, `data` is set to
null and `len` to 0.

See: [Serialization](../core-concepts/serialization.md)

---

### `factorial_deserialize`

```c
FactorialResult factorial_deserialize(
    const uint8_t *data,
    size_t len,
    FactorialEngine **out_engine
);
```

Deserialize an engine from a binary buffer. `data` must point to `len`
valid bytes (typically read from a file or network). On success, a new
engine pointer is written to `out_engine`. The caller takes ownership
and must eventually call `factorial_destroy()`.

On failure, `*out_engine` is set to null and
`FACTORIAL_RESULT_DESERIALIZE_ERROR` is returned.

See: [Serialization](../core-concepts/serialization.md)

---

### `factorial_free_buffer`

```c
FactorialResult factorial_free_buffer(FfiByteBuffer buffer);
```

Free a byte buffer previously returned by `factorial_serialize()`. After
this call, the buffer's `data` pointer is invalid.

Passing a buffer with a null `data` pointer is safe (no-op, returns
`FACTORIAL_RESULT_OK`). Passing a buffer not obtained from
`factorial_serialize()` is undefined behavior.

See: [API Conventions & Safety -- Buffer Ownership](conventions.md#buffer-ownership)
