# Power Networks

The `factorial-power` crate models electrical power production, consumption, and
storage across independent networks. Each
[tick](../introduction/glossary.md#tick) the module balances supply against
demand per network, computes a **satisfaction ratio** (a
[`Fixed64`](../introduction/glossary.md#fixed64) value between 0 and 1), and
emits events on state transitions such as brownout or recovery.

## Key concepts

- Every building that participates in the power system is identified by a
  [`NodeId`](../introduction/glossary.md#nodeid) obtained from the
  [production graph](../core-concepts/production-graph.md).
- Buildings are organized into **power networks**. Each network balances
  independently; a brownout in one network does not affect another.
- Per-node power specs (producer capacity, consumer demand, storage state) are
  owned by `PowerModule`, not by the core ECS.
- Events fire only on *transitions*, not every tick.

## Creating the module and a network

```rust
use factorial_power::{PowerModule, PowerProducer, PowerConsumer, PowerStorage, PowerPriority};
use factorial_core::fixed::Fixed64;

let mut power = PowerModule::new();
let net = power.create_network();   // returns PowerNetworkId
```

`create_network()` allocates a new `PowerNetworkId` with an auto-incrementing
counter. You can hold multiple networks simultaneously; they are fully
independent.

## Adding producers, consumers, and storage

```rust
// 100 W power plant
power.add_producer(net, power_plant_node, PowerProducer {
    capacity: Fixed64::from_num(100),
});

// 40 W consumer at default (Medium) priority
power.add_consumer(net, smelter_node, PowerConsumer {
    demand: Fixed64::from_num(40),
});

// Battery: 200 J capacity, starting at 100 J, 50 W charge/discharge rate
power.add_storage(net, battery_node, PowerStorage {
    capacity: Fixed64::from_num(200),
    charge: Fixed64::from_num(100),
    charge_rate: Fixed64::from_num(50),
});
```

- `add_producer` registers a node that injects watts into the network.
- `add_consumer` registers a node that draws watts. Defaults to
  `PowerPriority::Medium`.
- `add_storage` registers a battery or accumulator. Storage absorbs excess
  production and discharges during deficits, both clamped by `charge_rate`.

## Priority-based allocation

`PowerPriority` has three variants:

| Variant  | Allocation order |
|----------|-----------------|
| `High`   | First -- receives power before all others (e.g. life support) |
| `Medium` | Default |
| `Low`    | Last -- receives power only after High and Medium are satisfied |

Register a consumer with an explicit priority via `add_consumer_with_priority`:

```rust
power.add_consumer_with_priority(
    net,
    assembler_node,
    PowerConsumer { demand: Fixed64::from_num(40) },
    PowerPriority::High,
);
```

During a deficit, the tick algorithm sorts consumers by descending priority and
allocates available watts in that order. High-priority consumers are fully
satisfied before Medium consumers receive any power, and so on. Each consumer's
individual satisfaction ratio is stored and queryable via
`get_consumer_satisfaction(network, node)`.

## Ticking the power module

```rust
let events: Vec<PowerEvent> = power.tick(current_tick);
```

Each call to `tick()`:

1. Sums total production across all producers in the network.
2. Sums total demand across all consumers.
3. If production >= demand, satisfaction is 1.0 and excess charges storage
   (respecting `charge_rate` and capacity).
4. If production < demand, storage discharges to cover the deficit (respecting
   `charge_rate` and current charge). If a shortfall remains, available power is
   distributed to consumers in priority order.
5. Per-consumer satisfaction ratios are recorded.
6. Events are emitted on state transitions only (brownout or restored).

## Querying satisfaction

```rust
// Network-wide satisfaction (0.0 to 1.0)
let sat: Option<Fixed64> = power.satisfaction(net);

// Per-consumer satisfaction
let asm_sat: Option<Fixed64> = power.get_consumer_satisfaction(net, assembler_node);
```

A satisfaction of `1.0` means all demand is met. Any value below `1.0`
indicates partial power -- buildings can use this ratio to scale their
processing speed.

## Dynamic producer capacity

Producer output can change at runtime (e.g. a steam turbine whose output varies
with temperature):

```rust
power.set_producer_capacity(net, turbine_node, Fixed64::from_num(75));
```

The new capacity takes effect on the next `tick()`.

## Events

`PowerEvent` has two variants:

| Event | Fires when |
|-------|-----------|
| `PowerGridBrownout { network_id, deficit, tick }` | Network transitions from satisfied to under-powered |
| `PowerGridRestored { network_id, tick }` | Network transitions from brownout back to fully satisfied |

Events fire on *transitions* only. If a network remains in brownout for 10
ticks, only one `PowerGridBrownout` event is emitted (on the first tick).

## Brownout scenario

The following excerpt from
`crates/factorial-examples/examples/power_network.rs` demonstrates a three-phase
scenario: normal operation with battery backup, brownout under reduced
production, and recovery.

```rust
use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::BuildingTypeId;
use factorial_core::sim::SimulationStrategy;
use factorial_power::*;

let mut engine = Engine::new(SimulationStrategy::Tick);

// Create graph nodes for buildings
let p0 = engine.graph.queue_add_node(BuildingTypeId(0));
let p1 = engine.graph.queue_add_node(BuildingTypeId(1));
let p2 = engine.graph.queue_add_node(BuildingTypeId(2));
let p3 = engine.graph.queue_add_node(BuildingTypeId(3));
let p4 = engine.graph.queue_add_node(BuildingTypeId(4));
let r = engine.graph.apply_mutations();

let power_plant = r.resolve_node(p0).unwrap();
let assembler   = r.resolve_node(p1).unwrap();
let smelter     = r.resolve_node(p2).unwrap();
let lamp        = r.resolve_node(p3).unwrap();
let battery     = r.resolve_node(p4).unwrap();

let mut power = PowerModule::new();
let net = power.create_network();

power.add_producer(net, power_plant, PowerProducer {
    capacity: Fixed64::from_num(100),
});
power.add_consumer_with_priority(
    net, assembler,
    PowerConsumer { demand: Fixed64::from_num(40) },
    PowerPriority::High,
);
power.add_consumer_with_priority(
    net, smelter,
    PowerConsumer { demand: Fixed64::from_num(40) },
    PowerPriority::Medium,
);
power.add_consumer_with_priority(
    net, lamp,
    PowerConsumer { demand: Fixed64::from_num(40) },
    PowerPriority::Low,
);
power.add_storage(net, battery, PowerStorage {
    capacity: Fixed64::from_num(200),
    charge: Fixed64::from_num(100),
    charge_rate: Fixed64::from_num(50),
});

// Phase 1 -- battery covers the 20 W deficit (120 W demand, 100 W production)
for tick in 1..=5 {
    let events = power.tick(tick);
    let satisfaction = power.satisfaction(net).unwrap();
    let charge = power.storage.get(&battery).unwrap().charge;
    println!("Tick {tick}: satisfaction={satisfaction:.2}, charge={charge:.1}");
    for event in &events {
        println!("  {event:?}");
    }
}

// Phase 2 -- reduce production to 30 W to trigger brownout
power.set_producer_capacity(net, power_plant, Fixed64::from_num(30));
for tick in 6..=12 {
    let events = power.tick(tick);
    let sat = power.satisfaction(net).unwrap();
    println!("Tick {tick}: satisfaction={sat:.2}");
    for event in &events {
        println!("  {event:?}");
    }
}

// Phase 3 -- restore production to recover
power.set_producer_capacity(net, power_plant, Fixed64::from_num(150));
for tick in 13..=18 {
    let events = power.tick(tick);
    let sat = power.satisfaction(net).unwrap();
    println!("Tick {tick}: satisfaction={sat:.2}");
    for event in &events {
        println!("  {event:?}");
    }
}
```

During Phase 1, the battery discharges 20 J per tick to cover the gap between
100 W production and 120 W demand. In Phase 2, production drops to 30 W and the
battery drains quickly; once empty, a `PowerGridBrownout` event fires and the
high-priority assembler receives power first while the low-priority lamp is
starved. In Phase 3, production jumps to 150 W, the network recovers, and a
`PowerGridRestored` event fires. The 30 W surplus recharges the battery.

## Removing nodes and networks

```rust
power.remove_node(lamp);       // removes from all networks and spec maps
power.remove_network(net);     // removes the entire network
```

`remove_node` clears the node from every network it belongs to and deletes its
producer, consumer, or storage spec. `remove_network` deletes the network
struct but does not remove per-node specs, which may be shared across networks.
