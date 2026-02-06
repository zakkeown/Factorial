# Transport Strategies

A [transport strategy](../introduction/glossary.md#transport-strategy) determines how
items move along an [edge](../introduction/glossary.md#edge) between two
[nodes](../introduction/glossary.md#node) in the
[production graph](../introduction/glossary.md#production-graph). Each edge has exactly
one transport strategy, assigned via `engine.set_transport(edge_id, transport)`.

Like [processors](../introduction/glossary.md#processor), transports use **enum dispatch**
(not trait objects). Edges are grouped by transport variant and processed in homogeneous
batches during the transport phase for cache locality.

## The four transport types

### Flow

Continuous rate-based flow. Models pipes and conveyors in games like Builderment or
Satisfactory.

| Field | Type | Description |
|---|---|---|
| `rate` | `Fixed64` | Items per tick (fractional via [fixed-point](../introduction/glossary.md#fixed-point)) |
| `buffer_capacity` | `Fixed64` | Maximum buffered amount before back-pressure kicks in |
| `latency` | `u32` | Ticks delay before items appear at the destination |

Items flow at a fixed rate each tick. A buffer accumulates fractional items between
ticks. When `latency` is greater than zero, items accepted into the buffer do not
begin delivering until the latency period expires.

```rust
// From crates/factorial-core/examples/transport_showcase.rs

// Flow: continuous rate-based, 5 items/tick, no latency.
engine.set_transport(
    edge_flow,
    Transport::Flow(FlowTransport {
        rate: Fixed64::from_num(5),
        buffer_capacity: Fixed64::from_num(100),
        latency: 0,
    }),
);
```

### Item

Discrete belt with individually tracked slots. Models Factorio-style conveyor belts.

| Field | Type | Description |
|---|---|---|
| `speed` | `Fixed64` | Slots advanced per tick (fractional via fixed-point) |
| `slot_count` | `u32` | Total number of slots on the belt |
| `lanes` | `u8` | Number of parallel lanes (typically 1 or 2) |

Each slot holds one item. Items advance from the input end (high index) toward the
output end (slot 0) each tick. When the output slot is occupied, it delivers to the
destination node. When the input slot is free, it accepts a new item from the source.
Back-pressure propagates naturally: a full belt cannot accept new items.

```rust
// From crates/factorial-core/examples/transport_showcase.rs

// Item (belt): discrete slots, speed 1, 5 slots, 1 lane.
engine.set_transport(
    edge_item,
    Transport::Item(ItemTransport {
        speed: Fixed64::from_num(1),
        slot_count: 5,
        lanes: 1,
    }),
);
```

### Batch

Discrete chunks delivered per cycle. Models train loads, courier pallets, and periodic
bulk deliveries.

| Field | Type | Description |
|---|---|---|
| `batch_size` | `u32` | Maximum items per batch delivery |
| `cycle_time` | `u32` | Ticks per batch cycle |

Items accumulate in a pending buffer (up to `batch_size`). When the cycle timer reaches
`cycle_time`, all pending items are delivered at once and the cycle resets. Partial
batches are delivered if fewer items than `batch_size` were available.

```rust
// From crates/factorial-core/examples/transport_showcase.rs

// Batch: delivers 10 items every 5 ticks.
engine.set_transport(
    edge_batch,
    Transport::Batch(BatchTransport {
        batch_size: 10,
        cycle_time: 5,
    }),
);
```

### Vehicle

Vehicle with capacity and travel time. Models trucks, drones, and other round-trip
carriers.

| Field | Type | Description |
|---|---|---|
| `capacity` | `u32` | Maximum items the vehicle can carry |
| `travel_time` | `u32` | Ticks for one trip (source to destination) |

The vehicle lifecycle:

1. At source: load items (up to `capacity`).
2. Travel toward destination (`travel_time` ticks).
3. At destination: deliver cargo and begin the return trip.
4. Return to source (`travel_time` ticks).
5. Back at source: ready for the next load.

The full round trip takes `2 * travel_time` ticks, so effective throughput is
`capacity / (2 * travel_time)` items per tick.

```rust
// From crates/factorial-core/examples/transport_showcase.rs

// Vehicle: capacity 20, travel time 3 ticks (6-tick round trip).
engine.set_transport(
    edge_vehicle,
    Transport::Vehicle(VehicleTransport {
        capacity: 20,
        travel_time: 3,
    }),
);
```

## Comparison table

| Strategy | Throughput model | Latency | Back-pressure | Best for |
|---|---|---|---|---|
| **Flow** | Continuous `rate` items/tick | Configurable (0+) | Buffer capacity limit | Pipes, simple conveyors |
| **Item** | Depends on `speed` and `slot_count` | `slot_count / speed` ticks | Full belt blocks insertion | Factorio-style belts |
| **Batch** | `batch_size / cycle_time` items/tick (avg) | `cycle_time` ticks | Pending buffer fills to `batch_size` | Train loads, pallets |
| **Vehicle** | `capacity / (2 * travel_time)` items/tick | `travel_time` ticks | Single vehicle serializes loads | Trucks, drones |

## Transport state

Each transport variant has a corresponding state struct that tracks mutable per-tick
data. State is created automatically via `TransportState::new_for(transport)` when you
call `engine.set_transport()`.

| Transport | State struct | Key fields |
|---|---|---|
| `Flow` | `FlowState` | `buffered`, `latency_remaining` |
| `Item` | `BeltState` | `slots` (flat array of `Option<ItemTypeId>`) |
| `Batch` | `BatchState` | `progress`, `pending` |
| `Vehicle` | `VehicleState` | `position`, `cargo`, `returning` |

## Full example

The `transport_showcase` example creates four parallel source-to-sink chains, each using
a different transport strategy, and compares delivery after 20 ticks:

```rust
// From crates/factorial-core/examples/transport_showcase.rs

// Configure all sources identically: 5 items/tick.
for &src in &sources {
    engine.set_processor(
        src,
        Processor::Source(SourceProcessor {
            output_type: ItemTypeId(0),
            base_rate: Fixed64::from_num(5),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0),
            initial_properties: None,
        }),
    );
}

// Sinks consume items (demand processor).
for &sink in &sinks {
    engine.set_processor(
        sink,
        Processor::Demand(DemandProcessor {
            input_type: ItemTypeId(0),
            base_rate: Fixed64::from_num(10),
            accumulated: Fixed64::from_num(0),
            consumed_total: 0,
            accepted_types: None,
        }),
    );
}

// Run and compare.
for tick in 0..20 {
    engine.step();

    if (tick + 1) % 5 == 0 {
        for (i, &edge) in edges.iter().enumerate() {
            let snap = engine.snapshot_transport(edge);
            if let Some(snap) = snap {
                println!(
                    "  {:12}: utilization={:.2}, items_in_transit={}",
                    labels[i], snap.utilization, snap.items_in_transit
                );
            }
        }
    }
}
```
