# Memory Layout

This page describes how Factorial organizes data in memory for cache-friendly access and predictable performance. Understanding the layout is useful when profiling, extending the engine, or reasoning about performance characteristics of large factories.

---

## Struct-of-Arrays (SoA) design

Traditional object-oriented engines store all data for a building in a single struct (Array-of-Structs). Factorial inverts this: each component type is stored in its own `SecondaryMap` keyed by [NodeId](../introduction/glossary.md#node) or [EdgeId](../introduction/glossary.md#edge). This is the Struct-of-Arrays (SoA) pattern.

The `Engine` struct declares separate maps for each per-node component:

```rust
pub(crate) processors:        SecondaryMap<NodeId, Processor>,
pub(crate) processor_states:  SecondaryMap<NodeId, ProcessorState>,
pub(crate) inputs:            SecondaryMap<NodeId, Inventory>,
pub(crate) outputs:           SecondaryMap<NodeId, Inventory>,
pub(crate) modifiers:         SecondaryMap<NodeId, Vec<Modifier>>,
pub(crate) junctions:         SecondaryMap<NodeId, Junction>,
pub(crate) junction_states:   SecondaryMap<NodeId, JunctionState>,
```

Per-edge state follows the same pattern:

```rust
pub(crate) transports:        SecondaryMap<EdgeId, Transport>,
pub(crate) transport_states:  SecondaryMap<EdgeId, TransportState>,
pub(crate) edge_budgets:      SecondaryMap<EdgeId, u32>,
```

Additionally, the `ComponentStorage` struct holds opt-in component maps for module systems:

```rust
pub struct ComponentStorage {
    pub inventories:      SecondaryMap<NodeId, Inventory>,
    pub power_consumers:  SecondaryMap<NodeId, PowerConsumer>,
    pub power_producers:  SecondaryMap<NodeId, PowerProducer>,
}
```

### Why SoA matters

When the engine runs the **process** phase, it iterates over `processors` and `processor_states` together but does not touch `transports`. When it runs the **transport** phase, it iterates over `transports` and `transport_states` but does not touch `processors`. SoA ensures that each phase loads only the data it needs into CPU cache lines, avoiding pollution from unrelated fields.

For a 5,000-node factory, the processor evaluation phase touches roughly `5,000 * (sizeof(Processor) + sizeof(ProcessorState))` bytes of contiguous memory. With Array-of-Structs, it would also pull in inventory data, modifier lists, and junction state on every cache miss -- wasting cache capacity.

---

## Arena allocation with generational indices

### SlotMap as an arena

Both `NodeId` and `EdgeId` are [SlotMap](https://docs.rs/slotmap) keys created via `new_key_type!`:

```rust
new_key_type! {
    pub struct NodeId;
    pub struct EdgeId;
}
```

The [production graph](../introduction/glossary.md#production-graph) stores node and edge data in `SlotMap<NodeId, NodeData>` and `SlotMap<EdgeId, EdgeData>`. A `SlotMap` is a generational arena allocator:

- **O(1) insert**: appends to the end of a backing `Vec` or reuses a slot from the free list.
- **O(1) remove**: marks the slot as free and increments its generation counter. No compaction, no shifting.
- **O(1) lookup**: indexes directly into the backing `Vec` by the key's slot index.
- **Generation checks**: each key carries a generation counter. If a key's generation does not match the slot's current generation, the lookup fails gracefully. This prevents use-after-free bugs where a stale `NodeId` from a removed building accidentally references a newly created building that reused the same slot.

`SecondaryMap` (used for all component storage) piggybacks on the same key space. It maintains its own backing array indexed by the same slot indices, so lookups remain O(1) without hashing.

### No garbage collector

Rust's ownership model provides deterministic deallocation. When a node is removed:

1. The graph's `SlotMap` marks the node slot as free (O(1)).
2. Each `SecondaryMap` removes its entry for that `NodeId` (O(1) each).
3. Adjacent edges are queued for removal and processed in the same mutation batch.

There is no mark-and-sweep, no reference counting overhead, and no GC pauses. Deallocation cost is proportional to the number of components attached to the removed node, not to the total number of nodes in the graph.

---

## Pre-allocated ring buffers for events

The [event system](../core-concepts/events.md) uses bounded ring buffers (`EventBuffer`) rather than growable `Vec` collections. Each of the 12 event kinds has its own ring buffer with a fixed capacity (default: 1,024):

```rust
pub struct EventBuffer {
    events: Vec<Option<Event>>,  // pre-allocated to capacity
    head: usize,                 // write position (wraps around)
    len: usize,                  // current count
    total_written: u64,          // lifetime counter
}
```

Key properties:

- **Bounded memory**: the buffer never grows beyond its initial capacity. A factory that produces millions of events per second uses the same amount of memory as one that produces ten.
- **No allocation on emit**: `EventBuffer::push` writes into the pre-allocated `Vec` at the current `head` index and advances the write pointer. When the buffer is full, the oldest event is silently overwritten.
- **Suppression eliminates the buffer entirely**: calling `EventBus::suppress(kind)` drops the buffer for that event kind. Subsequent emissions for that kind are no-ops with zero cost.
- **Lazy allocation**: buffers are not created until the first event of a given kind is emitted, so unused event kinds consume no memory.

The `EventBus` holds one buffer per kind in a fixed-size array:

```rust
pub struct EventBus {
    buffers: [Option<EventBuffer>; 12],
    suppressed: [bool; 12],
    // ...
}
```

This design guarantees that the event system contributes zero allocations to the per-tick cost.

---

## Belt slot data in flat arrays

[ItemTransport](../introduction/glossary.md#transport-strategy) models Factorio-style conveyor belts with individually tracked slots. Each belt's state is a `BeltState`:

```rust
pub struct BeltState {
    pub slots: Vec<Option<ItemTypeId>>,
}
```

The `slots` vector is pre-allocated at belt creation time to `lanes * slot_count` entries. For a single-lane belt with 50 slots, this is a contiguous array of 50 `Option<ItemTypeId>` values (50 * 8 = 400 bytes on 64-bit platforms).

During the transport phase, belt advancement scans from the end of the array toward the front, shifting items forward one position. This sequential access pattern is highly cache-friendly: the entire belt fits in a few cache lines, and the access pattern is predictable enough for hardware prefetchers.

No allocations occur during belt ticking. The flat array is reused tick after tick.

---

## Summary of allocation characteristics

| Component | Allocation timing | Per-tick allocation |
|-----------|------------------|---------------------|
| `SlotMap` (graph nodes/edges) | On node/edge creation | None (reuses free slots) |
| `SecondaryMap` (components) | On component attach | None |
| `EventBuffer` (ring buffers) | On first emit per kind | None |
| `BeltState` (belt slots) | On belt creation | None |
| `VecDeque` (mutation queue) | On first queued mutation | None (reuses capacity) |
| Topological sort order | On graph mutation | Rebuilt from existing `Vec` |

The only allocations that can occur during a tick are:

1. **Graph mutations** (pre-tick phase only): adding a node may grow the `SlotMap` backing array. This is bounded by the number of mutations per tick and happens at most once.
2. **Event delivery** (post-tick phase): reactive handlers return `Vec<EventMutation>`, which may allocate. This is outside the core simulation phases.

Phases 2 (Transport) and 3 (Process) -- the dominant phases -- are allocation-free.

---

## Next steps

- [Performance Model](performance.md) -- tick budgets, profiling, and benchmarks.
- [Design Decisions & Trade-offs](decisions.md) -- why these data structures were chosen.
