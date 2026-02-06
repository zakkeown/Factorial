# Pipe Fluids Between Buildings

**Goal:** Create a fluid network with a pump, boiler, storage tank, and pipe, and handle low-pressure scenarios.
**Prerequisites:** [Fluid Networks](../modules/fluid.md), [The Production Graph](../core-concepts/production-graph.md)
**Example:** `crates/factorial-examples/examples/fluid_network.rs`

## Steps

### 1. Create buildings and the fluid module

```rust
let p_pump = engine.graph.queue_add_node(BuildingTypeId(0));
let p_boiler = engine.graph.queue_add_node(BuildingTypeId(1));
let p_tank = engine.graph.queue_add_node(BuildingTypeId(2));
let p_pipe = engine.graph.queue_add_node(BuildingTypeId(3));
let r = engine.graph.apply_mutations();

let pump = r.resolve_node(p_pump).unwrap();
let boiler = r.resolve_node(p_boiler).unwrap();
let tank = r.resolve_node(p_tank).unwrap();
let pipe = r.resolve_node(p_pipe).unwrap();

let water_type = ItemTypeId(100);
let mut fluid = FluidModule::new();
let net = fluid.create_network(water_type);
```

The `FluidModule` manages fluid networks separately from the core item-based [production graph](../introduction/glossary.md#production-graph). Each network carries a single fluid type, identified by an [ItemTypeId](../introduction/glossary.md#item-type-id).

### 2. Add producers, consumers, storage, and pipes

```rust
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
```

The pump produces 50 units per [tick](../introduction/glossary.md#tick), the boiler consumes 30, and the surplus flows into the storage tank. The pipe defines maximum throughput capacity for the network segment.

### 3. Run under normal conditions

```rust
for tick in 1..=8 {
    let events = fluid.tick(tick);
    let pressure = fluid.pressure(net).unwrap();
    let tank_level = fluid.storage.get(&tank).unwrap().current;
    let consumed = fluid.get_consumed_this_tick(net, boiler);
    // pressure stays at 1.0, tank fills with surplus (20 units/tick)
}
```

### 4. Create a low-pressure scenario

```rust
let p_boiler2 = engine.graph.queue_add_node(BuildingTypeId(5));
let r = engine.graph.apply_mutations();
let boiler2 = r.resolve_node(p_boiler2).unwrap();

fluid.add_consumer(net, boiler2, FluidConsumer {
    rate: Fixed64::from_num(80),
});
// Total demand: 110/tick, production: 50/tick -- deficit of 60/tick
```

Adding a second boiler pushes total demand above production. The storage tank drains to cover the deficit, and once empty, pressure drops below 1.0.

### 5. Restore pressure

```rust
fluid.remove_node(boiler2);
// Demand drops back to 30/tick, surplus refills the tank
```

## What's Happening

The fluid module calculates network pressure each tick as the ratio of available supply (production plus storage discharge) to total demand. When pressure is 1.0, all consumers receive their full requested rate. When pressure drops, consumers receive a proportionally reduced flow. The storage tank acts as a buffer: surplus production fills it, and deficits drain it. Once the tank is empty and demand still exceeds production, pressure falls below 1.0 and consumers are starved.

## Variations

- **Multiple fluid types:** Create separate networks for water, oil, and steam -- each with its own pressure dynamics.
- **Pipe throughput limits:** Reduce `FluidPipe::capacity` to model narrow pipes that bottleneck flow even when production is sufficient.
- **No storage:** Omit the tank to see immediate pressure drops whenever demand exceeds production.
- **Integrating with recipes:** Use fluid consumption as an input to a [processor](../introduction/glossary.md#processor) recipe -- for example, a boiler that consumes water and produces steam items.
- **Power interaction:** Combine with the [Power module](./power.md) so that pumps require electricity to operate.
