# Choose the Right Transport Strategy

**Goal:** Compare all four transport types -- Flow, Item, Batch, and Vehicle -- side by side with identical sources and sinks.
**Prerequisites:** [Transport Strategies](../core-concepts/transport.md), [The Production Graph](../core-concepts/production-graph.md)
**Example:** `crates/factorial-core/examples/transport_showcase.rs`

## Steps

### 1. Create parallel source-sink pairs

```rust
let p_src_flow = engine.graph.queue_add_node(BuildingTypeId(0));
let p_src_item = engine.graph.queue_add_node(BuildingTypeId(0));
let p_src_batch = engine.graph.queue_add_node(BuildingTypeId(0));
let p_src_vehicle = engine.graph.queue_add_node(BuildingTypeId(0));

let p_sink_flow = engine.graph.queue_add_node(BuildingTypeId(1));
let p_sink_item = engine.graph.queue_add_node(BuildingTypeId(1));
let p_sink_batch = engine.graph.queue_add_node(BuildingTypeId(1));
let p_sink_vehicle = engine.graph.queue_add_node(BuildingTypeId(1));
```

Four identical source [nodes](../introduction/glossary.md#node) and four identical sink nodes form parallel lanes, each testing a different [transport strategy](../introduction/glossary.md#transport-strategy).

### 2. Configure identical sources and sinks

```rust
for &src in &sources {
    engine.set_processor(src, Processor::Source(SourceProcessor {
        output_type: ItemTypeId(0),
        base_rate: Fixed64::from_num(5),
        depletion: Depletion::Infinite,
        accumulated: Fixed64::from_num(0),
        initial_properties: None,
    }));
}

for &sink in &sinks {
    engine.set_processor(sink, Processor::Demand(DemandProcessor {
        input_type: ItemTypeId(0),
        base_rate: Fixed64::from_num(10),
        accumulated: Fixed64::from_num(0),
        consumed_total: 0,
        accepted_types: None,
    }));
}
```

Sources produce 5 items per [tick](../introduction/glossary.md#tick); sinks consume up to 10. The transport is the bottleneck.

### 3. Configure the four transport types

```rust
// Flow: continuous rate-based, 5 items/tick.
engine.set_transport(edge_flow, Transport::Flow(FlowTransport {
    rate: Fixed64::from_num(5),
    buffer_capacity: Fixed64::from_num(100),
    latency: 0,
}));

// Item (belt): discrete slots, speed 1, 5 slots, 1 lane.
engine.set_transport(edge_item, Transport::Item(ItemTransport {
    speed: Fixed64::from_num(1),
    slot_count: 5,
    lanes: 1,
}));

// Batch: delivers 10 items every 5 ticks.
engine.set_transport(edge_batch, Transport::Batch(BatchTransport {
    batch_size: 10,
    cycle_time: 5,
}));

// Vehicle: capacity 20, travel time 3 ticks (6-tick round trip).
engine.set_transport(edge_vehicle, Transport::Vehicle(VehicleTransport {
    capacity: 20,
    travel_time: 3,
}));
```

### 4. Run and compare

```rust
for tick in 0..20 {
    engine.step();
    if (tick + 1) % 5 == 0 {
        for (i, &edge) in edges.iter().enumerate() {
            let snap = engine.snapshot_transport(edge);
            // compare utilization and items_in_transit across strategies
        }
    }
}
```

## What's Happening

| Strategy | Behavior | Best For |
|----------|----------|----------|
| **Flow** | Transfers a continuous fractional rate each tick. Items arrive immediately (zero latency). Simplest and cheapest to simulate. | High-throughput pipes, abstract logistics. |
| **Item** | Models a belt with discrete slots. Each slot holds one item and advances one position per tick at the given speed. | Visual conveyor belts with spatial fidelity. |
| **Batch** | Accumulates items and delivers them all at once every `cycle_time` ticks. No items move between deliveries. | Train stops, drone deliveries, periodic transfers. |
| **Vehicle** | Loads up to `capacity` items, travels for `travel_time` ticks, unloads, and returns empty. Round-trip takes `2 * travel_time` ticks. | Trucks, robots, logistics vehicles with travel delay. |

The engine evaluates all four [edges](../introduction/glossary.md#edge) in the same tick. Each transport type has different latency, throughput, and buffering characteristics, so the total delivered to each sink after 20 ticks will differ.

## Variations

- **Latency on Flow:** Set `latency: 2` on `FlowTransport` to add a 2-tick delay before items start arriving.
- **Multi-lane belts:** Increase `ItemTransport::lanes` to 2 for side-by-side belts with doubled throughput.
- **Larger batches:** Increase `batch_size` and `cycle_time` together to model infrequent but large shipments.
- **Mixed strategies:** Use `Flow` for short connections and `Vehicle` for long-distance routes in the same [production graph](../introduction/glossary.md#production-graph). See [Model a Smelting Chain](./smelting-chain.md) for a simpler single-transport setup.
