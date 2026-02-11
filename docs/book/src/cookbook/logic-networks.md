# Logic Networks

**Goal:** Wire buildings together with signal networks so they react to production state -- threshold activation, periodic clocks, item counting, conditional routing, and signal-driven recipe switching.
**Prerequisites:** [Logic Networks module](../modules/logic.md), [The Production Graph](../core-concepts/production-graph.md)
**Example:** `crates/factorial-integration-tests/tests/logic_networks.rs`

## Recipe 1: Clock Circuit

A constant combinator increments a counter each tick via an arithmetic combinator. When the counter exceeds a threshold, a decider combinator outputs a pulse.

### Setup

```rust
use factorial_logic::{LogicModuleBridge, WireColor, SignalSet};
use factorial_logic::combinator::*;
use factorial_logic::condition::*;

let mut engine = Engine::new(SimulationStrategy::Tick);
engine.register_module(Box::new(LogicModuleBridge::new()));

// Create nodes for the clock components.
let clock_const = add_node(&mut engine, make_passthrough(), 10, 10);
let clock_counter = add_node(&mut engine, make_passthrough(), 10, 10);
let clock_output = add_node(&mut engine, make_passthrough(), 10, 10);
```

### Wire the network

```rust
{
    let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();
    let red = bridge.logic_mut().create_network(WireColor::Red);
    bridge.logic_mut().add_to_network(red, clock_const);
    bridge.logic_mut().add_to_network(red, clock_counter);
    bridge.logic_mut().add_to_network(red, clock_output);

    // Constant combinator: emit "1" on the clock signal each tick.
    let mut signals = SignalSet::new();
    signals.insert(clock_signal, Fixed64::from_num(1));
    bridge.logic_mut().set_constant(clock_const, signals, true);

    // Arithmetic combinator: adds the clock signal to itself (accumulates).
    bridge.logic_mut().set_arithmetic_combinator(
        clock_counter,
        ArithmeticCombinator {
            left: SignalSelector::Signal(clock_signal),
            op: ArithmeticOp::Add,
            right: SignalSelector::Signal(clock_signal),
            output_signal: clock_signal,
        },
    );

    // Decider combinator: outputs 1 when the accumulated signal exceeds 10.
    bridge.logic_mut().set_decider_combinator(
        clock_output,
        DeciderCombinator {
            condition: Condition {
                left: SignalSelector::Signal(clock_signal),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(Fixed64::from_num(10)),
            },
            output_signal: pulse_signal,
            output_mode: DeciderOutput::One,
        },
    );
}
```

### Tick and observe

```rust
for _ in 0..20 {
    engine.step();
}

let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
let active = bridge.logic().is_active(clock_output);
// The decider fires once the accumulated signal crosses the threshold.
```

## Recipe 2: Item Counter

An inventory reader broadcasts item counts from a building's input inventory. Other buildings on the network can see how many items are buffered.

```rust
let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();
let red = bridge.logic_mut().create_network(WireColor::Red);
bridge.logic_mut().add_to_network(red, chest);
bridge.logic_mut().add_to_network(red, display_node);

// The chest reads its own input inventory and emits item counts as signals.
bridge.logic_mut().set_inventory_reader(chest, chest, InventorySource::Input);

// After a few ticks, query the merged signals on the network:
let signals = bridge.logic().network_signals(red).unwrap();
let iron_count = signals.get(&iron_plate);
// iron_count reflects how many iron plates are in the chest's input inventory.
```

Every tick the inventory reader re-scans the inventory and updates the signal. The merged `SignalSet` on the wire network is the sum of all sources -- if multiple readers are on the same network, their counts add up.

## Recipe 3: Conditional Routing

Enable or disable a building based on an item threshold. When the signal condition is false, the building's circuit control sets `active = false`, which can be checked by the engine to pause processing.

```rust
{
    let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();
    let red = bridge.logic_mut().create_network(WireColor::Red);
    bridge.logic_mut().add_to_network(red, warehouse);
    bridge.logic_mut().add_to_network(red, assembler);

    // Read the warehouse inventory.
    bridge.logic_mut().set_inventory_reader(
        warehouse, warehouse, InventorySource::Output,
    );

    // Assembler activates only when iron_plate > 50 on the red network.
    bridge.logic_mut().set_circuit_control(
        assembler,
        Condition {
            left: SignalSelector::Signal(iron_plate),
            op: ComparisonOp::Gt,
            right: SignalSelector::Constant(Fixed64::from_num(50)),
        },
        WireColor::Red,
    );
}

// Run the simulation.
for _ in 0..100 {
    engine.step();
}

let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
let assembler_enabled = bridge.logic().is_active(assembler);
// true when warehouse has > 50 iron plates, false otherwise.
```

### Events

The logic module emits `CircuitActivated` and `CircuitDeactivated` events on transitions, so the game UI can show when a building flips state:

```rust
let events = bridge.last_events();
for event in events {
    match event {
        LogicEvent::CircuitActivated { node, tick } => { /* show green light */ }
        LogicEvent::CircuitDeactivated { node, tick } => { /* show red light */ }
        _ => {}
    }
}
```

## Recipe 4: Priority Splitter

Route items to the highest-priority consumer by reading downstream inventories and using circuit controls to enable only the preferred path.

```rust
// Setup: source -> splitter -> consumer_a (high priority)
//                           -> consumer_b (low priority)

{
    let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();
    let red = bridge.logic_mut().create_network(WireColor::Red);
    bridge.logic_mut().add_to_network(red, consumer_a);
    bridge.logic_mut().add_to_network(red, consumer_b);

    // Read consumer_a's input inventory.
    bridge.logic_mut().set_inventory_reader(
        consumer_a, consumer_a, InventorySource::Input,
    );

    // consumer_b only activates when consumer_a has > 80 items (i.e., nearly full).
    bridge.logic_mut().set_circuit_control(
        consumer_b,
        Condition {
            left: SignalSelector::Signal(iron_plate),
            op: ComparisonOp::Gt,
            right: SignalSelector::Constant(Fixed64::from_num(80)),
        },
        WireColor::Red,
    );
}
```

Items flow to `consumer_a` first. Once its inventory exceeds 80, the circuit control enables `consumer_b` as overflow. When `consumer_a` drains below the threshold, `consumer_b` deactivates and items route back to the primary consumer.

## Recipe 5: Signal-Driven Recipe Switching

Use circuit signals to switch a `MultiRecipe` processor between recipes at runtime. When a condition fires, the logic bridge sets `pending_switch` on the processor.

```rust
use factorial_logic::condition::CircuitAction;

{
    let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();
    let red = bridge.logic_mut().create_network(WireColor::Red);
    bridge.logic_mut().add_to_network(red, sensor_node);
    bridge.logic_mut().add_to_network(red, factory_node);

    // Sensor reads inventory and broadcasts item counts.
    bridge.logic_mut().set_inventory_reader(
        sensor_node, sensor_node, InventorySource::Output,
    );

    // When iron_plate > 100, switch factory_node to recipe index 1.
    bridge.logic_mut().set_circuit_control_with_action(
        factory_node,
        Condition {
            left: SignalSelector::Signal(iron_plate),
            op: ComparisonOp::Gt,
            right: SignalSelector::Constant(Fixed64::from_num(100)),
        },
        WireColor::Red,
        CircuitAction::SwitchRecipe { recipe_index: 1 },
    );
}
```

The `MultiRecipe` processor respects its `RecipeSwitchPolicy`: with `CompleteFirst` (default), the switch happens after the current craft finishes. With `CancelImmediate`, it switches immediately. The bridge only triggers on the rising edge (inactive -> active transition) to avoid repeated switches.

## What's Happening

The logic module runs during the engine's **Component** phase (phase 4). Each tick:

1. Signal sources (constants, inventory readers, combinator outputs) feed values into wire networks.
2. Signals are merged per network -- duplicate item types are summed.
3. Combinators evaluate and store their outputs for next tick (one-tick delay prevents feedback loops).
4. Circuit controls evaluate conditions and update their `active` state.
5. The bridge applies `CircuitAction::SwitchRecipe` actions to `MultiRecipe` processors on rising edges.
6. Events fire on state transitions: `CircuitActivated`, `CircuitDeactivated`, `NetworkSignalsChanged`.

## Variations

- **Dual-wire logic:** Use both `Red` and `Green` networks on the same nodes for independent signal channels (e.g., one for item counts, one for control signals).
- **Feedback loops:** Connect a combinator's output back to its own network for accumulating counters or PID controllers. The one-tick delay ensures deterministic behavior.
- **Combining with power:** Use circuit controls to shut down low-priority consumers when a power signal drops below a threshold, complementing the power module's built-in priority system.
- **Multi-condition switching:** Chain multiple `set_circuit_control_with_action` calls on different nodes to create state machines that switch between several recipes based on different signal conditions.
