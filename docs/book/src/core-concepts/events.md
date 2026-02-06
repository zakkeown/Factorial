# Events

Factorial provides a typed event system for reacting to simulation activity. Events are
emitted during the simulation phases (transport, process, component) and delivered in
batch during the post-[tick](../introduction/glossary.md#tick) phase. Each event type
has its own pre-allocated ring buffer with configurable capacity.

## The 12 event types

Events are variants of the `Event` enum. Every event carries the `tick` at which it
occurred.

### Production events

| Event | Fields | When emitted |
|---|---|---|
| `ItemProduced` | `node`, `item_type`, `quantity`, `tick` | A [processor](../introduction/glossary.md#processor) produces items into a node's output [inventory](../introduction/glossary.md#inventory) |
| `ItemConsumed` | `node`, `item_type`, `quantity`, `tick` | A processor consumes items from a node's input inventory |
| `RecipeStarted` | `node`, `tick` | A Fixed processor begins a new crafting cycle |
| `RecipeCompleted` | `node`, `tick` | A Fixed processor finishes a crafting cycle |

### Building state events

| Event | Fields | When emitted |
|---|---|---|
| `BuildingStalled` | `node`, `reason`, `tick` | A processor transitions to the [Stalled](../introduction/glossary.md#stall) state |
| `BuildingResumed` | `node`, `tick` | A previously stalled processor resumes operation |

### Transport events

| Event | Fields | When emitted |
|---|---|---|
| `ItemDelivered` | `edge`, `quantity`, `tick` | Items arrive at the destination end of an [edge](../introduction/glossary.md#edge) |
| `TransportFull` | `edge`, `tick` | A transport cannot accept more items (back-pressure) |

### Graph events

| Event | Fields | When emitted |
|---|---|---|
| `NodeAdded` | `node`, `building_type`, `tick` | A [node](../introduction/glossary.md#node) is added to the [production graph](../introduction/glossary.md#production-graph) |
| `NodeRemoved` | `node`, `tick` | A node is removed from the graph |
| `EdgeAdded` | `edge`, `from`, `to`, `tick` | An edge is added between two nodes |
| `EdgeRemoved` | `edge`, `tick` | An edge is removed from the graph |

## Passive listeners

Passive listeners receive events **read-only**. Use them for UI updates, audio triggers,
analytics, and logging. They cannot modify simulation state.

Register a passive listener with `engine.on_passive()`:

```rust
// From crates/factorial-core/examples/events_and_queries.rs

use std::cell::RefCell;
use std::rc::Rc;

// Track produced items via a shared counter.
let produced_count = Rc::new(RefCell::new(0u32));
let counter = produced_count.clone();
engine.on_passive(
    EventKind::ItemProduced,
    Box::new(move |event| {
        if let Event::ItemProduced { quantity, .. } = event {
            *counter.borrow_mut() += quantity;
        }
    }),
);

// Track recipe completions.
let recipe_completions = Rc::new(RefCell::new(0u32));
let completions = recipe_completions.clone();
engine.on_passive(
    EventKind::RecipeCompleted,
    Box::new(move |_event| {
        *completions.borrow_mut() += 1;
    }),
);
```

Within the same priority level, passive listeners are called in registration order.

## Reactive handlers

Reactive handlers receive events and return **mutations** to enqueue for the next tick.
Use them for automated factory logic -- auto-building, chain reactions, or resource
management.

Register a reactive handler with `engine.on_reactive()`:

```rust
// When a recipe completes, queue the node for removal.
engine.on_reactive(
    EventKind::RecipeCompleted,
    Box::new(|event| {
        if let Event::RecipeCompleted { node, .. } = event {
            vec![EventMutation::RemoveNode { node: *node }]
        } else {
            vec![]
        }
    }),
);
```

The `EventMutation` enum supports four operations:

- `AddNode { building_type }` -- queue a new node.
- `RemoveNode { node }` -- queue a node for removal.
- `Connect { from, to }` -- queue an edge between two nodes.
- `Disconnect { edge }` -- queue an edge for removal.

Mutations from reactive handlers are collected during event delivery and applied during
the **pre-tick** phase of the *next* tick, preserving determinism.

## Priority and filtering

For fine-grained control over subscriber ordering and event selection, use the filtered
registration methods:

```rust
engine.event_bus.on_passive_filtered(
    EventKind::ItemProduced,
    SubscriberPriority::Pre,  // runs before Normal and Post subscribers
    Some(Box::new(|e| {
        // Only fire for quantities > 5
        matches!(e, Event::ItemProduced { quantity, .. } if *quantity > 5)
    })),
    Box::new(move |_event| {
        // handle high-volume production
    }),
);
```

Three priority levels control execution order:

| Priority | Runs |
|---|---|
| `Pre` | First |
| `Normal` (default) | Second |
| `Post` | Last |

Within the same priority, subscribers run in registration order. The optional filter
predicate skips events that return `false`, avoiding unnecessary work.

## Event suppression

Suppress an event type entirely to eliminate its allocation and recording cost:

```rust
engine.suppress_event(EventKind::ItemProduced);
```

Suppressed events are never buffered, never delivered, and have **zero runtime cost**.
The ring buffer for a suppressed event type is dropped immediately. This is useful for
high-frequency events (like `ItemProduced`) in production builds where you do not need
UI telemetry.

## Pull-based polling for FFI

The event bus exposes read-only access to event buffers, enabling pull-based polling
from FFI consumers that cannot use Rust closures:

```rust
// Check how many events of a kind are buffered.
let count = engine.event_bus.buffered_count(EventKind::ItemProduced);

// Read the buffer directly.
if let Some(buffer) = engine.event_bus.buffer(EventKind::ItemProduced) {
    for event in buffer.iter() {
        // Process event from C/FFI side.
    }
}

// Lifetime counters (including events dropped from the ring buffer).
let total = engine.event_bus.total_emitted(EventKind::ItemProduced);
```

The ring buffer has a configurable capacity (default 1024 per event type). When full,
the oldest events are dropped. Use `total_emitted()` and `dropped_count()` to detect
when events are being lost.

## Event delivery lifecycle

Each `engine.step()` follows this sequence:

1. **Pre-tick**: Apply queued graph mutations (including mutations from reactive handlers).
2. **Transport**: Move items along edges; emit `ItemDelivered`, `TransportFull`.
3. **Process**: Run processors; emit `ItemProduced`, `ItemConsumed`, `RecipeStarted`, `RecipeCompleted`, `BuildingStalled`, `BuildingResumed`.
4. **Component**: Module-registered systems run.
5. **Post-tick**: Deliver all buffered events to subscribers. Reactive handler mutations are collected.
6. **Bookkeeping**: Update tick counter, compute [state hash](../introduction/glossary.md#state-hash).

Events from step N are delivered in step N's post-tick. Reactive mutations from step N
are applied in step N+1's pre-tick. This one-tick delay is by design -- it guarantees
that event handlers never mutate the graph mid-tick, preserving determinism.
