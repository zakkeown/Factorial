# Fluid Networks

The `factorial-fluid` crate models fluid production, consumption, storage, and
pipe transport across independent networks. Each
[tick](../introduction/glossary.md#tick) the module balances supply against
demand per network, computes a **pressure ratio** (a
[`Fixed64`](../introduction/glossary.md#fixed64) value between 0 and 1), and
emits events on state transitions (low pressure / restored) and storage
boundaries (full / empty).

## Key concepts

- Buildings are assigned to fluid networks via
  [`NodeId`](../introduction/glossary.md#nodeid).
- Each network carries a **single fluid type**, identified by an `ItemTypeId`.
  A water network and a steam network are separate objects.
- Per-node fluid specs are stored in the module, not in the core ECS.
- Events fire only on *transitions*, not every tick.

## Creating the module and a network

```rust
use factorial_fluid::{FluidModule, FluidProducer, FluidConsumer, FluidStorage, FluidPipe};
use factorial_core::fixed::Fixed64;
use factorial_core::id::ItemTypeId;

let mut fluid = FluidModule::new();
let water_type = ItemTypeId(100);
let net = fluid.create_network(water_type);   // returns FluidNetworkId
```

`create_network` takes an `ItemTypeId` that identifies the fluid the network
carries. You can query it later via `fluid.network(net).unwrap().fluid_type`.

## Adding producers, consumers, storage, and pipes

```rust
// Pump: 50 units of water per tick
fluid.add_producer(net, pump_node, FluidProducer {
    rate: Fixed64::from_num(50),
});

// Boiler: consumes 30 units per tick
fluid.add_consumer(net, boiler_node, FluidConsumer {
    rate: Fixed64::from_num(30),
});

// Tank: 500 unit capacity, 100 unit/tick fill/drain rate
fluid.add_storage(net, tank_node, FluidStorage {
    capacity: Fixed64::from_num(500),
    current: Fixed64::from_num(0),
    fill_rate: Fixed64::from_num(100),
});

// Pipe: 200 unit throughput capacity
fluid.add_pipe(net, pipe_node, FluidPipe {
    capacity: Fixed64::from_num(200),
});
```

- `add_producer` registers a node that injects fluid per tick.
- `add_consumer` registers a node that draws fluid per tick.
- `add_storage` registers a tank or reservoir. Storage absorbs excess production
  and drains during deficits, both clamped by `fill_rate`.
- `add_pipe` registers a pipe segment. Pipes are currently tracked for network
  membership; throughput limiting is planned for a future release.

## Ticking the fluid module

```rust
let events: Vec<FluidEvent> = fluid.tick(current_tick);
```

Each call to `tick()`:

1. Sums total production from all producers in the network.
2. Sums total demand from all consumers.
3. If production >= demand, pressure is 1.0 and excess fills storage (respecting
   `fill_rate` and capacity).
4. If production < demand, storage drains to cover the deficit (respecting
   `fill_rate` and current level). If a shortfall remains, pressure falls below
   1.0: `pressure = supplied / demand`.
5. Per-consumer fluid consumption for the tick is recorded and queryable via
   `get_consumed_this_tick(network, node)`.
6. `StorageFull` / `StorageEmpty` events are emitted when storage hits a
   boundary.
7. `PressureLow` / `PressureRestored` events are emitted on state transitions
   only.

## Querying pressure

```rust
let p: Option<Fixed64> = fluid.pressure(net);
```

A pressure of `1.0` means all consumer demand is met. Any value below `1.0`
indicates the network is under-supplied and consumers operate at reduced
throughput.

## Events

`FluidEvent` has four variants:

| Event | Fires when |
|-------|-----------|
| `PressureLow { network_id, pressure, tick }` | Network transitions from adequate to low pressure |
| `PressureRestored { network_id, tick }` | Network transitions from low pressure back to fully satisfied |
| `StorageFull { network_id, node, tick }` | A storage node reaches its capacity |
| `StorageEmpty { network_id, node, tick }` | A storage node is completely drained |

Pressure events fire on transitions only. Storage boundary events fire whenever
the condition is met (each tick the boundary is hit).

## Pressure dynamics example

The following excerpt from
`crates/factorial-examples/examples/fluid_network.rs` demonstrates normal
operation, low-pressure from excess demand, and recovery.

```rust
use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::{BuildingTypeId, ItemTypeId};
use factorial_core::sim::SimulationStrategy;
use factorial_fluid::*;

let mut engine = Engine::new(SimulationStrategy::Tick);

let p_pump   = engine.graph.queue_add_node(BuildingTypeId(0));
let p_boiler = engine.graph.queue_add_node(BuildingTypeId(1));
let p_tank   = engine.graph.queue_add_node(BuildingTypeId(2));
let p_pipe   = engine.graph.queue_add_node(BuildingTypeId(3));
let r = engine.graph.apply_mutations();

let pump   = r.resolve_node(p_pump).unwrap();
let boiler = r.resolve_node(p_boiler).unwrap();
let tank   = r.resolve_node(p_tank).unwrap();
let pipe   = r.resolve_node(p_pipe).unwrap();

let water = ItemTypeId(100);
let mut fluid = FluidModule::new();
let net = fluid.create_network(water);

fluid.add_producer(net, pump, FluidProducer {
    rate: Fixed64::from_num(50),
});
fluid.add_consumer(net, boiler, FluidConsumer {
    rate: Fixed64::from_num(30),
});
fluid.add_storage(net, tank, FluidStorage {
    capacity: Fixed64::from_num(500),
    current: Fixed64::from_num(0),
    fill_rate: Fixed64::from_num(100),
});
fluid.add_pipe(net, pipe, FluidPipe {
    capacity: Fixed64::from_num(200),
});

// Phase 1 -- normal: pump 50, boiler 30, excess 20 fills tank
for tick in 1..=8 {
    let events = fluid.tick(tick);
    let pressure = fluid.pressure(net).unwrap();
    let level = fluid.storage.get(&tank).unwrap().current;
    println!("Tick {tick}: pressure={pressure:.2}, tank={level:.1}");
    for event in &events {
        println!("  {event:?}");
    }
}

// Phase 2 -- add a second, larger consumer to create a deficit
let p_boiler2 = engine.graph.queue_add_node(BuildingTypeId(5));
let r = engine.graph.apply_mutations();
let boiler2 = r.resolve_node(p_boiler2).unwrap();

fluid.add_consumer(net, boiler2, FluidConsumer {
    rate: Fixed64::from_num(80),
});

// Total demand: 110, production: 50 => deficit 60, tank drains
for tick in 9..=20 {
    let events = fluid.tick(tick);
    let pressure = fluid.pressure(net).unwrap();
    let level = fluid.storage.get(&tank).unwrap().current;
    println!("Tick {tick}: pressure={pressure:.2}, tank={level:.1}");
    for event in &events {
        println!("  {event:?}");
    }
}

// Phase 3 -- remove the second boiler to restore pressure
fluid.remove_node(boiler2);
for tick in 21..=26 {
    let events = fluid.tick(tick);
    let pressure = fluid.pressure(net).unwrap();
    println!("Tick {tick}: pressure={pressure:.2}");
    for event in &events {
        println!("  {event:?}");
    }
}
```

During Phase 1, the pump produces 50 units/tick while the boiler consumes only
30, so the tank accumulates 20 units/tick and pressure stays at 1.0. In Phase 2,
a second boiler adds 80 units/tick of demand (total 110). The tank drains to
cover the 60-unit deficit; once empty, pressure drops to `50/110` and a
`PressureLow` event fires. In Phase 3, removing the second boiler restores
demand to 30 and a `PressureRestored` event fires. The surplus resumes filling
the tank.

## Removing nodes and networks

```rust
fluid.remove_node(boiler2);   // removes from all networks and spec maps
fluid.remove_network(net);    // removes the entire network
```

`remove_node` clears the node from every network it belongs to and deletes its
producer, consumer, storage, or pipe spec.
