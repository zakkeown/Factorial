# Logic/Circuit Networks Design

**Date:** 2026-02-08
**Status:** Draft
**Scope:** New framework crate `factorial-logic`

---

## Overview

A logic network is a group of buildings connected by wires that share signals. Signals are typed values (an `ItemTypeId` + a `Fixed64` count). Each tick, the network collects signals from all connected buildings, sums them, and makes the totals available for any building to read.

Wires are separate from transport edges. A building can have both item connections (transport edges in the production graph) and wire connections (in the logic module) simultaneously. This matters because:

- Wires carry signals, not items — different data model
- Wire networks propagate within a single tick (no latency), unlike transport
- Wire topology can differ completely from item flow topology
- Keeps factorial-core untouched — all logic lives in the new crate

The module follows the same pattern as `factorial-power`: a standalone `LogicModule` struct with its own `tick()` method, own storage (BTreeMaps keyed by NodeId), own event types, and Serialize/Deserialize support.

---

## Signal Model

Signals are the data that flows through wires. A signal is a mapping from `ItemTypeId` to `Fixed64` — "I have 50 iron ore" or "I want 20 copper plates." Using `Fixed64` means signals can represent fractional quantities, percentages, thresholds — whatever the game needs.

```rust
/// A set of signals: item type -> value.
/// Sparse — only non-zero signals are stored.
pub type SignalSet = BTreeMap<ItemTypeId, Fixed64>;
```

---

## Wire Networks

Wire networks are groups of buildings connected by wires. Every building on the same wire network can read the merged signal set — the sum of all signals contributed by every node on that network. Two wire colors (Red/Green) let buildings participate in two independent signal networks simultaneously.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WireNetworkId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WireColor {
    Red,
    Green,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireNetwork {
    pub id: WireNetworkId,
    pub color: WireColor,
    pub members: Vec<NodeId>,
    /// Merged signal set, recomputed each tick.
    pub signals: SignalSet,
}
```

Each tick, the module iterates every wire network, collects the signal output from each member node, sums them into the merged `SignalSet`, and stores it. Any node can then read the merged signals from its network(s) to make decisions.

---

## Signal Sources

Buildings contribute signals to their wire network through signal sources. Three kinds cover the range from simple to fully programmable.

### Constant Combinator

Outputs a fixed set of signals every tick. The game dev configures it once. Useful for thresholds ("I want 200 iron plates") or control flags.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantCombinator {
    pub signals: SignalSet,
    pub enabled: bool,
}
```

### Inventory Reader

Reads a building's actual input or output inventory and emits signals for what's in it. This is the bridge between the production graph and the logic network — "this chest has 47 iron ore" becomes a signal automatically.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventorySource {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReader {
    pub target_node: NodeId,
    pub source: InventorySource,
}
```

### Arithmetic Combinator

Reads signals from its wire network, performs an operation, and outputs the result as a new signal.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalSelector {
    /// A specific signal from the network.
    Signal(ItemTypeId),
    /// A constant value.
    Constant(Fixed64),
    /// The sum of all signals ("each" equivalent).
    Each,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArithmeticCombinator {
    pub left: SignalSelector,
    pub op: ArithmeticOp,
    pub right: SignalSelector,
    pub output: ItemTypeId,
}
```

### Decider Combinator

Like an arithmetic combinator but conditional — it only outputs signals when a condition is met.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeciderOutput {
    /// Output a specific signal with value 1 when condition is true.
    One(ItemTypeId),
    /// Pass through the input signal's value when condition is true.
    InputCount(ItemTypeId),
    /// Pass through all input signals when condition is true.
    Everything,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeciderCombinator {
    pub condition: Condition,
    pub output: DeciderOutput,
}
```

---

## Signal Consumers — Conditions & Circuit Control

Buildings read signals from their wire network and change behavior based on conditions. A condition is a predicate that evaluates to true or false each tick.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Gt,
    Lt,
    Eq,
    Gte,
    Lte,
    Ne,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub left: SignalSelector,
    pub op: ComparisonOp,
    pub right: SignalSelector,
}
```

Conditions drive circuit control. The module computes a `bool` per node per tick that the game layer can query:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitControl {
    pub condition: Condition,
    pub wire_color: WireColor,
    /// Result of evaluating the condition this tick.
    pub active: bool,
    /// Whether the control was active last tick (for transition detection).
    pub was_active: bool,
}
```

The game layer decides what "active" means for each building — disable an inserter, stop a splitter's output, gate a recipe. The logic module computes signals and evaluates conditions; the game interprets the results.

---

## Tick Pipeline

The logic module's `tick()` runs once per game tick in five steps:

1. **Collect** — For each wire network, iterate members and collect their signal outputs (constant values, inventory reads, combinator results from *last* tick).
2. **Merge** — Sum all collected signals per network into the merged `SignalSet`.
3. **Evaluate combinators** — Run arithmetic and decider combinators against the new merged signals, storing their outputs for next tick's collection.
4. **Evaluate conditions** — For each `CircuitControl`, evaluate its condition against the merged signals and update `active`.
5. **Emit events** — Fire `CircuitActivated`/`CircuitDeactivated` events on transitions.

The one-tick delay on combinator outputs (step 1 uses last tick's results, step 3 computes new results) prevents infinite feedback loops and keeps evaluation deterministic regardless of iteration order.

---

## Events

Events follow the same transition-only pattern as the power module:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicEvent {
    /// A node's circuit control transitioned from inactive to active.
    CircuitActivated { node: NodeId, tick: Ticks },
    /// A node's circuit control transitioned from active to inactive.
    CircuitDeactivated { node: NodeId, tick: Ticks },
    /// A wire network's merged signals changed from last tick (opt-in, can be noisy).
    NetworkSignalsChanged { network: WireNetworkId, tick: Ticks },
}
```

---

## LogicModule Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicModule {
    // Topology
    pub networks: BTreeMap<WireNetworkId, WireNetwork>,

    // Per-node signal sources
    pub constants: BTreeMap<NodeId, ConstantCombinator>,
    pub inventory_readers: BTreeMap<NodeId, InventoryReader>,
    pub arithmetic_combinators: BTreeMap<NodeId, ArithmeticCombinator>,
    pub decider_combinators: BTreeMap<NodeId, DeciderCombinator>,

    // Per-node signal consumption
    pub circuit_controls: BTreeMap<NodeId, CircuitControl>,

    // Internal: combinator outputs from last tick (for one-tick delay)
    combinator_outputs: BTreeMap<NodeId, SignalSet>,

    // Internal: previous network signals (for change detection)
    prev_signals: BTreeMap<WireNetworkId, SignalSet>,

    next_network_id: u32,
}
```

## Public API

```rust
impl LogicModule {
    pub fn new() -> Self

    // --- Network management ---
    pub fn create_network(&mut self, color: WireColor) -> WireNetworkId
    pub fn remove_network(&mut self, id: WireNetworkId)
    pub fn add_to_network(&mut self, network: WireNetworkId, node: NodeId)
    pub fn remove_from_network(&mut self, network: WireNetworkId, node: NodeId)

    // --- Signal sources ---
    pub fn set_constant(&mut self, node: NodeId, signals: SignalSet, enabled: bool)
    pub fn set_inventory_reader(&mut self, node: NodeId, target: NodeId, source: InventorySource)
    pub fn set_arithmetic(&mut self, node: NodeId, combinator: ArithmeticCombinator)
    pub fn set_decider(&mut self, node: NodeId, combinator: DeciderCombinator)

    // --- Signal consumption ---
    pub fn set_circuit_control(&mut self, node: NodeId, condition: Condition, wire_color: WireColor)

    // --- Queries ---
    pub fn is_active(&self, node: NodeId) -> Option<bool>
    pub fn network_signals(&self, network: WireNetworkId) -> Option<&SignalSet>

    // --- Cleanup ---
    pub fn remove_node(&mut self, node: NodeId)

    // --- Tick ---
    pub fn tick(
        &mut self,
        inputs: &SecondaryMap<NodeId, Inventory>,
        outputs: &SecondaryMap<NodeId, Inventory>,
        current_tick: Ticks,
    ) -> Vec<LogicEvent>
}
```

---

## Crate Structure

```
crates/factorial-logic/
├── Cargo.toml
└── src/
    ├── lib.rs          # LogicModule, WireNetwork, tick pipeline, events, public API
    ├── combinator.rs   # ArithmeticCombinator, DeciderCombinator, SignalSelector, ops
    └── condition.rs    # Condition, ComparisonOp, CircuitControl, evaluation logic
```

### Dependencies

- `factorial-core` — for `NodeId`, `ItemTypeId`, `Fixed64`, `Inventory`, `Ticks`
- `serde` — derive Serialize/Deserialize on all public types
- `slotmap` — for `SecondaryMap` in the `tick()` signature
- `bitcode` (dev-dependency) — for serde round-trip test assertions

---

## Tests

1. `signal_merge_sums_correctly` — two nodes contribute signals, merged set has correct totals
2. `constant_combinator_contributes_signals` — constant outputs appear in merged set
3. `disabled_constant_contributes_nothing` — disabled constant is skipped
4. `inventory_reader_reads_node_inventory` — reader emits signals matching inventory contents
5. `arithmetic_combinator_add` — addition produces correct output signal
6. `arithmetic_combinator_multiply` — multiplication produces correct output signal
7. `arithmetic_combinator_divide` — division produces correct output signal
8. `arithmetic_combinator_each` — Each selector sums all signals
9. `decider_combinator_passes_when_true` — outputs signals only when condition met
10. `decider_combinator_blocks_when_false` — no output when condition not met
11. `decider_output_everything` — passes through all input signals
12. `circuit_control_evaluates_condition` — `is_active()` returns correct bool
13. `one_tick_delay_on_combinators` — combinator output from tick N appears in network on tick N+1
14. `event_fires_on_transition_only` — activated/deactivated events fire once, not every tick
15. `two_wire_colors_independent` — red and green networks on same node don't interfere
16. `remove_node_cleans_all_state` — removing a node clears it from networks, sources, controls
17. `remove_network_cleans_members` — removing a network clears merged signals and membership
18. `serde_round_trip` — full module serializes and deserializes correctly
19. `empty_network_has_no_signals` — network with no signal sources has empty SignalSet
20. `comparison_ops_all_variants` — each ComparisonOp evaluates correctly

---

## Usage Example

```rust
use factorial_logic::*;

// Create the logic module alongside the engine.
let mut logic = LogicModule::new();

// Create a red wire network.
let red_net = logic.create_network(WireColor::Red);

// Connect a chest and a constant combinator to the network.
logic.add_to_network(red_net, chest_node);
logic.add_to_network(red_net, combinator_node);

// The chest broadcasts its inventory contents.
logic.set_inventory_reader(chest_node, chest_node, InventorySource::Output);

// The constant combinator outputs a threshold: "iron ore = 200".
let mut threshold = SignalSet::new();
threshold.insert(iron_ore_id, Fixed64::from_num(200));
logic.set_constant(combinator_node, threshold, true);

// An inserter checks: "iron ore signal > 200" to decide whether to run.
logic.add_to_network(red_net, inserter_node);
logic.set_circuit_control(
    inserter_node,
    Condition {
        left: SignalSelector::Signal(iron_ore_id),
        op: ComparisonOp::Gt,
        right: SignalSelector::Constant(Fixed64::from_num(200)),
    },
    WireColor::Red,
);

// Game loop
loop {
    engine.step();
    let events = logic.tick(&engine.inputs, &engine.outputs, engine.sim_state.tick);

    // Check inserter's circuit control.
    if logic.is_active(inserter_node) == Some(true) {
        // Inserter is enabled — game layer allows it to operate.
    }
}
```

---

## Implementation Order

```
1. Core types and module struct (lib.rs)
   ├── 1a. WireNetworkId, WireColor, WireNetwork, SignalSet
   ├── 1b. LogicModule struct, new(), network management API
   └── 1c. remove_node cleanup

2. Combinators (combinator.rs)
   ├── 2a. SignalSelector, ArithmeticOp, ArithmeticCombinator
   ├── 2b. DeciderOutput, DeciderCombinator
   └── 2c. Evaluation functions for both combinator types

3. Conditions (condition.rs)
   ├── 3a. ComparisonOp, Condition, evaluation
   ├── 3b. CircuitControl struct
   └── 3c. InventorySource, InventoryReader

4. Tick pipeline (lib.rs)
   ├── 4a. Signal collection from constants, inventory readers, combinator outputs
   ├── 4b. Signal merging per network
   ├── 4c. Combinator evaluation (with one-tick delay storage)
   ├── 4d. Condition evaluation and CircuitControl update
   └── 4e. Event emission on transitions

5. Tests
   └── All 20 tests listed above
```

Estimated scope: ~800-1000 lines including tests.
