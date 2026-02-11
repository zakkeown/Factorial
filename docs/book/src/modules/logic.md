# Logic Networks

The `factorial-logic` crate adds wire-based signal networks to the Factorial
engine, enabling Factorio-style combinators and circuit control.

## Concepts

**Wire networks** connect buildings via colored wires (`Red` or `Green`).
Each tick, signals from all members of a network are merged (summed) into a
single `SignalSet` -- a sparse map of `ItemTypeId` to `Fixed64` values.

**Signal sources** feed values into a network:

- **Constant combinators** -- output a fixed set of signals when enabled.
- **Inventory readers** -- read a node's input or output inventory and emit
  item quantities as signals.
- **Combinator outputs** -- the result of arithmetic or decider operations
  (delayed by one tick to prevent infinite feedback loops).

**Signal sinks** react to network signals:

- **Circuit controls** -- evaluate a condition against network signals and
  enable or disable a building accordingly.

## Combinators

### Arithmetic combinator

Reads two values (each selected by signal name, constant, or the "Each" wildcard),
applies an operation (Add, Subtract, Multiply, Divide, Modulo), and writes the
result to an output signal.

### Decider combinator

Evaluates a condition (`left op right` where `op` is one of `>`, `<`, `=`,
`>=`, `<=`, `!=`). When the condition is true, outputs one of:

- **One** -- a constant `1` on the output signal.
- **InputCount** -- the input value of the output signal.
- **Everything** -- all input signals that satisfy the condition.

## Tick Pipeline

The logic module runs during the **Component** phase of the engine tick:

1. Collect signals from constants, inventory readers, and last-tick combinator outputs.
2. Merge signals per network (duplicate items are summed).
3. Evaluate combinators and store outputs for next tick.
4. Evaluate circuit controls and update building active state.
5. Emit events: `CircuitActivated`, `CircuitDeactivated`, `NetworkSignalsChanged`.

## Events

| Event                    | Emitted when                                        |
|--------------------------|-----------------------------------------------------|
| `CircuitActivated`       | A circuit control transitions a building to active  |
| `CircuitDeactivated`     | A circuit control transitions a building to inactive|
| `NetworkSignalsChanged`  | The merged signal set of a network changes          |

## Example

```rust,ignore
use factorial_logic::{LogicModule, WireColor};

// Register the module with the engine.
let mut logic = LogicModule::new();

// Create a red wire network.
let net = logic.create_network(WireColor::Red);

// Attach a constant combinator to the network.
logic.add_constant(net, item_id, Fixed64::from_num(10));

// Attach a circuit control to a building.
logic.add_circuit_control(node_id, net, condition);
```
