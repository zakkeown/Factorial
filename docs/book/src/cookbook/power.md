# Add Power with Brownout Handling

**Goal:** Set up a power grid with producers, priority-based consumers, battery storage, and handle brownout when demand exceeds supply.
**Prerequisites:** [Power Networks](../modules/power.md), [The Production Graph](../core-concepts/production-graph.md)
**Example:** `crates/factorial-examples/examples/power_network.rs`

## Steps

### 1. Create buildings and the power module

```rust
let p0 = engine.graph.queue_add_node(BuildingTypeId(0)); // power plant
let p1 = engine.graph.queue_add_node(BuildingTypeId(1)); // assembler
let p2 = engine.graph.queue_add_node(BuildingTypeId(2)); // smelter
let p3 = engine.graph.queue_add_node(BuildingTypeId(3)); // lamp
let p4 = engine.graph.queue_add_node(BuildingTypeId(4)); // battery
let r = engine.graph.apply_mutations();

let power_plant = r.resolve_node(p0).unwrap();
let assembler = r.resolve_node(p1).unwrap();
let smelter = r.resolve_node(p2).unwrap();
let lamp = r.resolve_node(p3).unwrap();
let battery = r.resolve_node(p4).unwrap();

let mut power = PowerModule::new();
let net = power.create_network();
```

The `PowerModule` is a side-car module that operates alongside the core engine. [Nodes](../introduction/glossary.md#node) are created in the [production graph](../introduction/glossary.md#production-graph) as usual, then registered with the power module.

### 2. Add producers, consumers, and storage

```rust
power.add_producer(net, power_plant, PowerProducer {
    capacity: Fixed64::from_num(100),
});

power.add_consumer_with_priority(net, assembler,
    PowerConsumer { demand: Fixed64::from_num(40) }, PowerPriority::High);
power.add_consumer_with_priority(net, smelter,
    PowerConsumer { demand: Fixed64::from_num(40) }, PowerPriority::Medium);
power.add_consumer_with_priority(net, lamp,
    PowerConsumer { demand: Fixed64::from_num(40) }, PowerPriority::Low);

power.add_storage(net, battery, PowerStorage {
    capacity: Fixed64::from_num(200),
    charge: Fixed64::from_num(100),
    charge_rate: Fixed64::from_num(50),
});
```

Total demand is 120W but the power plant only supplies 100W. The battery covers the 20W deficit. Consumers have different priorities: when power is scarce, high-priority buildings are served first.

### 3. Tick the power module and check satisfaction

```rust
for tick in 1..=5 {
    let events = power.tick(tick);
    let satisfaction = power.satisfaction(net).unwrap();
    let charge = power.storage.get(&battery).unwrap().charge;
    // satisfaction is 1.0 while battery has charge to cover the deficit
}
```

### 4. Trigger a brownout

```rust
power.set_producer_capacity(net, power_plant, Fixed64::from_num(30));

for tick in 6..=12 {
    let events = power.tick(tick);
    let satisfaction = power.satisfaction(net).unwrap();
    let asm_sat = power.get_consumer_satisfaction(net, assembler).unwrap_or(Fixed64::ZERO);
    let lamp_sat = power.get_consumer_satisfaction(net, lamp).unwrap_or(Fixed64::ZERO);
    // High-priority assembler keeps running; low-priority lamp shuts down first
}
```

Reducing the power plant to 30W creates a severe deficit. The battery drains and eventually the network enters brownout. The power module sheds load by priority: the lamp (low) loses power first, then the smelter (medium), while the assembler (high) is last to be affected.

### 5. Recover from brownout

```rust
power.set_producer_capacity(net, power_plant, Fixed64::from_num(150));

for tick in 13..=18 {
    let events = power.tick(tick);
    // satisfaction returns to 1.0, excess power recharges the battery
}
```

## What's Happening

The power module runs a satisfaction algorithm each tick: it sums production capacity plus available storage discharge, then distributes power to consumers in priority order. When total supply cannot meet total demand, `satisfaction` drops below 1.0 and the module emits brownout events. Buildings receiving insufficient power will [stall](../introduction/glossary.md#stall) with `NoPower`. Excess production beyond demand is routed to storage up to its `charge_rate`.

## Variations

- **Multiple networks:** Call `power.create_network()` multiple times for isolated power grids (e.g., separate base zones).
- **Solar with day/night:** Vary `set_producer_capacity()` over time to simulate intermittent generation.
- **No battery:** Omit `add_storage()` for a grid with no buffer -- brownout is immediate when demand exceeds production.
- **Integrating with the engine:** Use power satisfaction to scale [processor](../introduction/glossary.md#processor) speed via [modifiers](../introduction/glossary.md#modifier), so underpowered buildings run slower instead of stopping entirely.
