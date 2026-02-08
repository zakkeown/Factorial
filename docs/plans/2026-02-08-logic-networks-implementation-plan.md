# Logic/Circuit Networks Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `factorial-logic` — a framework crate providing wire-based signal networks, combinators, and circuit control for building conditional automation.

**Architecture:** Standalone crate following the `factorial-power` pattern. No changes to factorial-core. The module owns wire network topology, per-node signal sources/consumers, and a tick pipeline that collects signals, evaluates combinators (with one-tick delay), evaluates conditions, and emits transition events. The game layer queries `is_active()` to decide how conditions affect buildings.

**Tech Stack:** Rust 2024 edition, factorial-core types (NodeId, ItemTypeId, Fixed64, Inventory, Ticks), serde, slotmap, bitcode (dev).

**Worktree:** `/Users/zakkeown/Code/Factorial/.worktrees/logic-networks`

**Design doc:** `docs/plans/2026-02-08-logic-networks-design.md`

---

### Task 1: Scaffold the crate

**Files:**
- Create: `crates/factorial-logic/Cargo.toml`
- Create: `crates/factorial-logic/src/lib.rs`
- Create: `crates/factorial-logic/src/combinator.rs`
- Create: `crates/factorial-logic/src/condition.rs`
- Modify: `Cargo.toml` (workspace root — add member)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "factorial-logic"
version = "0.1.0"
edition = "2024"

[dependencies]
factorial-core = { path = "../factorial-core" }
fixed = { workspace = true }
serde = { workspace = true }
slotmap = { workspace = true }

[dev-dependencies]
bitcode = { workspace = true }
```

**Step 2: Create empty source files**

`src/lib.rs`:
```rust
pub mod combinator;
pub mod condition;
```

`src/combinator.rs`:
```rust
//! Combinators for transforming and filtering signals.
```

`src/condition.rs`:
```rust
//! Conditions and circuit control for signal-driven building behavior.
```

**Step 3: Add to workspace**

In root `Cargo.toml`, add `"crates/factorial-logic"` to the `members` array.

**Step 4: Verify it compiles**

Run: `cargo check --package factorial-logic`
Expected: compiles with no errors

**Step 5: Commit**

```bash
git add crates/factorial-logic/ Cargo.toml
git commit -m "feat(logic): scaffold factorial-logic crate"
```

---

### Task 2: Core types — WireNetworkId, WireColor, SignalSet, WireNetwork

**Files:**
- Modify: `crates/factorial-logic/src/lib.rs`

**Step 1: Write the failing test**

Add to `lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(v: f64) -> Fixed64 {
        Fixed64::from_num(v)
    }

    fn make_node_ids(count: usize) -> Vec<NodeId> {
        let mut sm = slotmap::SlotMap::<NodeId, ()>::with_key();
        (0..count).map(|_| sm.insert(())).collect()
    }

    #[test]
    fn wire_network_creation_and_membership() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(2);
        let net = module.create_network(WireColor::Red);

        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[1]);

        let network = module.networks.get(&net).unwrap();
        assert_eq!(network.color, WireColor::Red);
        assert_eq!(network.members.len(), 2);
        assert!(network.members.contains(&nodes[0]));
        assert!(network.members.contains(&nodes[1]));
    }

    #[test]
    fn network_ids_are_unique() {
        let mut module = LogicModule::new();
        let a = module.create_network(WireColor::Red);
        let b = module.create_network(WireColor::Green);
        let c = module.create_network(WireColor::Red);
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn remove_from_network() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(2);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[1]);

        module.remove_from_network(net, nodes[0]);

        let network = module.networks.get(&net).unwrap();
        assert_eq!(network.members.len(), 1);
        assert!(!network.members.contains(&nodes[0]));
        assert!(network.members.contains(&nodes[1]));
    }

    #[test]
    fn remove_network() {
        let mut module = LogicModule::new();
        let net = module.create_network(WireColor::Red);
        assert!(module.networks.contains_key(&net));
        module.remove_network(net);
        assert!(!module.networks.contains_key(&net));
    }

    #[test]
    fn no_duplicate_members() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[0]); // duplicate
        let network = module.networks.get(&net).unwrap();
        assert_eq!(network.members.len(), 1);
    }

    #[test]
    fn empty_network_has_no_signals() {
        let module = LogicModule::new();
        // No networks exist.
        assert!(module.network_signals(WireNetworkId(999)).is_none());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package factorial-logic`
Expected: FAIL — `LogicModule` not defined

**Step 3: Write the implementation**

Add to top of `lib.rs` (above the test module):

```rust
//! Logic/Circuit Networks Module for the Factorial engine.
//!
//! Models wire-based signal networks where buildings share typed signals
//! (ItemTypeId → Fixed64). Supports constant combinators, inventory readers,
//! arithmetic/decider combinators, and per-node circuit control conditions.
//!
//! Signal propagation uses a one-tick delay on combinator outputs to prevent
//! infinite feedback loops and ensure deterministic evaluation order.

pub mod combinator;
pub mod condition;

use std::collections::BTreeMap;

use factorial_core::fixed::{Fixed64, Ticks};
use factorial_core::id::{ItemTypeId, NodeId};
use factorial_core::item::Inventory;
use serde::{Deserialize, Serialize};
use slotmap::SecondaryMap;

use combinator::{ArithmeticCombinator, DeciderCombinator};
use condition::{CircuitControl, Condition, InventoryReader, InventorySource};

// ---------------------------------------------------------------------------
// Signal set
// ---------------------------------------------------------------------------

/// A set of signals: item type → value. Sparse — only non-zero signals stored.
pub type SignalSet = BTreeMap<ItemTypeId, Fixed64>;

// ---------------------------------------------------------------------------
// Wire network types
// ---------------------------------------------------------------------------

/// Identifies a wire network. Cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WireNetworkId(pub u32);

/// Wire color — buildings can connect to one red and one green network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WireColor {
    Red,
    Green,
}

/// A wire network containing member nodes and their merged signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireNetwork {
    pub id: WireNetworkId,
    pub color: WireColor,
    pub members: Vec<NodeId>,
    /// Merged signal set, recomputed each tick.
    pub signals: SignalSet,
}

impl WireNetwork {
    pub fn new(id: WireNetworkId, color: WireColor) -> Self {
        Self {
            id,
            color,
            members: Vec::new(),
            signals: SignalSet::new(),
        }
    }

    pub fn add_member(&mut self, node: NodeId) {
        if !self.members.contains(&node) {
            self.members.push(node);
        }
    }

    pub fn remove_member(&mut self, node: NodeId) {
        self.members.retain(|n| *n != node);
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events emitted by the logic module on state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicEvent {
    /// A node's circuit control transitioned from inactive to active.
    CircuitActivated { node: NodeId, tick: Ticks },
    /// A node's circuit control transitioned from active to inactive.
    CircuitDeactivated { node: NodeId, tick: Ticks },
    /// A wire network's merged signals changed from last tick.
    NetworkSignalsChanged { network: WireNetworkId, tick: Ticks },
}

// ---------------------------------------------------------------------------
// Constant combinator
// ---------------------------------------------------------------------------

/// Outputs a fixed set of signals every tick when enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantCombinator {
    pub signals: SignalSet,
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Logic module
// ---------------------------------------------------------------------------

/// Manages all wire networks, signal sources, and circuit controls.
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

impl Default for LogicModule {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicModule {
    pub fn new() -> Self {
        Self {
            networks: BTreeMap::new(),
            constants: BTreeMap::new(),
            inventory_readers: BTreeMap::new(),
            arithmetic_combinators: BTreeMap::new(),
            decider_combinators: BTreeMap::new(),
            circuit_controls: BTreeMap::new(),
            combinator_outputs: BTreeMap::new(),
            prev_signals: BTreeMap::new(),
            next_network_id: 0,
        }
    }

    // --- Network management ---

    pub fn create_network(&mut self, color: WireColor) -> WireNetworkId {
        let id = WireNetworkId(self.next_network_id);
        self.next_network_id += 1;
        self.networks.insert(id, WireNetwork::new(id, color));
        id
    }

    pub fn remove_network(&mut self, id: WireNetworkId) {
        self.networks.remove(&id);
        self.prev_signals.remove(&id);
    }

    pub fn add_to_network(&mut self, network: WireNetworkId, node: NodeId) {
        if let Some(net) = self.networks.get_mut(&network) {
            net.add_member(node);
        }
    }

    pub fn remove_from_network(&mut self, network: WireNetworkId, node: NodeId) {
        if let Some(net) = self.networks.get_mut(&network) {
            net.remove_member(node);
        }
    }

    // --- Queries ---

    pub fn network_signals(&self, network: WireNetworkId) -> Option<&SignalSet> {
        self.networks.get(&network).map(|n| &n.signals)
    }

    pub fn is_active(&self, node: NodeId) -> Option<bool> {
        self.circuit_controls.get(&node).map(|c| c.active)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package factorial-logic`
Expected: all 6 tests PASS

**Step 5: Commit**

```bash
git add crates/factorial-logic/src/lib.rs
git commit -m "feat(logic): add core types, wire networks, and LogicModule struct"
```

---

### Task 3: Combinators — ArithmeticCombinator, DeciderCombinator, evaluation

**Files:**
- Modify: `crates/factorial-logic/src/combinator.rs`

**Step 1: Write the failing tests**

Add to `combinator.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use factorial_core::id::ItemTypeId;

    fn fixed(v: f64) -> Fixed64 {
        Fixed64::from_num(v)
    }

    fn iron() -> ItemTypeId { ItemTypeId(0) }
    fn copper() -> ItemTypeId { ItemTypeId(1) }
    fn steel() -> ItemTypeId { ItemTypeId(2) }

    fn signals_with(items: &[(ItemTypeId, f64)]) -> SignalSet {
        items.iter().map(|&(id, v)| (id, fixed(v))).collect()
    }

    #[test]
    fn resolve_signal_selector() {
        let signals = signals_with(&[(iron(), 50.0), (copper(), 30.0)]);

        assert_eq!(resolve_selector(&SignalSelector::Signal(iron()), &signals), fixed(50.0));
        assert_eq!(resolve_selector(&SignalSelector::Signal(steel()), &signals), fixed(0.0));
        assert_eq!(resolve_selector(&SignalSelector::Constant(fixed(42.0)), &signals), fixed(42.0));
        assert_eq!(resolve_selector(&SignalSelector::Each, &signals), fixed(80.0)); // 50 + 30
    }

    #[test]
    fn arithmetic_combinator_add() {
        let signals = signals_with(&[(iron(), 50.0), (copper(), 30.0)]);
        let combinator = ArithmeticCombinator {
            left: SignalSelector::Signal(iron()),
            op: ArithmeticOp::Add,
            right: SignalSelector::Signal(copper()),
            output: steel(),
        };
        let result = evaluate_arithmetic(&combinator, &signals);
        assert_eq!(result.get(&steel()), Some(&fixed(80.0)));
    }

    #[test]
    fn arithmetic_combinator_multiply() {
        let signals = signals_with(&[(iron(), 10.0)]);
        let combinator = ArithmeticCombinator {
            left: SignalSelector::Signal(iron()),
            op: ArithmeticOp::Multiply,
            right: SignalSelector::Constant(fixed(3.0)),
            output: steel(),
        };
        let result = evaluate_arithmetic(&combinator, &signals);
        assert_eq!(result.get(&steel()), Some(&fixed(30.0)));
    }

    #[test]
    fn arithmetic_combinator_divide() {
        let signals = signals_with(&[(iron(), 100.0)]);
        let combinator = ArithmeticCombinator {
            left: SignalSelector::Signal(iron()),
            op: ArithmeticOp::Divide,
            right: SignalSelector::Constant(fixed(4.0)),
            output: steel(),
        };
        let result = evaluate_arithmetic(&combinator, &signals);
        assert_eq!(result.get(&steel()), Some(&fixed(25.0)));
    }

    #[test]
    fn arithmetic_combinator_divide_by_zero() {
        let signals = signals_with(&[(iron(), 100.0)]);
        let combinator = ArithmeticCombinator {
            left: SignalSelector::Signal(iron()),
            op: ArithmeticOp::Divide,
            right: SignalSelector::Constant(fixed(0.0)),
            output: steel(),
        };
        let result = evaluate_arithmetic(&combinator, &signals);
        // Divide by zero produces zero (safe default).
        assert_eq!(result.get(&steel()), Some(&fixed(0.0)));
    }

    #[test]
    fn arithmetic_combinator_modulo() {
        let signals = signals_with(&[(iron(), 10.0)]);
        let combinator = ArithmeticCombinator {
            left: SignalSelector::Signal(iron()),
            op: ArithmeticOp::Modulo,
            right: SignalSelector::Constant(fixed(3.0)),
            output: steel(),
        };
        let result = evaluate_arithmetic(&combinator, &signals);
        assert_eq!(result.get(&steel()), Some(&fixed(1.0)));
    }

    #[test]
    fn arithmetic_combinator_subtract() {
        let signals = signals_with(&[(iron(), 50.0), (copper(), 30.0)]);
        let combinator = ArithmeticCombinator {
            left: SignalSelector::Signal(iron()),
            op: ArithmeticOp::Subtract,
            right: SignalSelector::Signal(copper()),
            output: steel(),
        };
        let result = evaluate_arithmetic(&combinator, &signals);
        assert_eq!(result.get(&steel()), Some(&fixed(20.0)));
    }

    #[test]
    fn arithmetic_combinator_each() {
        let signals = signals_with(&[(iron(), 10.0), (copper(), 20.0), (steel(), 30.0)]);
        let combinator = ArithmeticCombinator {
            left: SignalSelector::Each,
            op: ArithmeticOp::Add,
            right: SignalSelector::Constant(fixed(0.0)),
            output: iron(),
        };
        let result = evaluate_arithmetic(&combinator, &signals);
        // Each sums all: 10 + 20 + 30 = 60, then + 0 = 60
        assert_eq!(result.get(&iron()), Some(&fixed(60.0)));
    }

    #[test]
    fn decider_combinator_passes_when_true() {
        let signals = signals_with(&[(iron(), 100.0)]);
        let combinator = DeciderCombinator {
            condition: Condition {
                left: SignalSelector::Signal(iron()),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(50.0)),
            },
            output: DeciderOutput::One(steel()),
        };
        let result = evaluate_decider(&combinator, &signals);
        assert_eq!(result.get(&steel()), Some(&fixed(1.0)));
    }

    #[test]
    fn decider_combinator_blocks_when_false() {
        let signals = signals_with(&[(iron(), 10.0)]);
        let combinator = DeciderCombinator {
            condition: Condition {
                left: SignalSelector::Signal(iron()),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(50.0)),
            },
            output: DeciderOutput::One(steel()),
        };
        let result = evaluate_decider(&combinator, &signals);
        assert!(result.is_empty());
    }

    #[test]
    fn decider_output_input_count() {
        let signals = signals_with(&[(iron(), 75.0)]);
        let combinator = DeciderCombinator {
            condition: Condition {
                left: SignalSelector::Signal(iron()),
                op: ComparisonOp::Gte,
                right: SignalSelector::Constant(fixed(50.0)),
            },
            output: DeciderOutput::InputCount(iron()),
        };
        let result = evaluate_decider(&combinator, &signals);
        assert_eq!(result.get(&iron()), Some(&fixed(75.0)));
    }

    #[test]
    fn decider_output_everything() {
        let signals = signals_with(&[(iron(), 10.0), (copper(), 20.0)]);
        let combinator = DeciderCombinator {
            condition: Condition {
                left: SignalSelector::Each,
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(0.0)),
            },
            output: DeciderOutput::Everything,
        };
        let result = evaluate_decider(&combinator, &signals);
        assert_eq!(result.get(&iron()), Some(&fixed(10.0)));
        assert_eq!(result.get(&copper()), Some(&fixed(20.0)));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package factorial-logic`
Expected: FAIL — types and functions not defined

**Step 3: Write the implementation**

In `combinator.rs`:

```rust
//! Combinators for transforming and filtering signals.

use std::collections::BTreeMap;

use factorial_core::fixed::Fixed64;
use factorial_core::id::ItemTypeId;
use serde::{Deserialize, Serialize};

use crate::condition::{Condition, ComparisonOp, evaluate_condition};
use crate::SignalSet;

// ---------------------------------------------------------------------------
// Signal selector
// ---------------------------------------------------------------------------

/// Selects a value from the current signal set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalSelector {
    /// A specific signal from the network.
    Signal(ItemTypeId),
    /// A constant value.
    Constant(Fixed64),
    /// The sum of all signals ("each" equivalent).
    Each,
}

/// Resolve a signal selector against a signal set.
pub fn resolve_selector(selector: &SignalSelector, signals: &SignalSet) -> Fixed64 {
    let zero = Fixed64::from_num(0);
    match selector {
        SignalSelector::Signal(id) => signals.get(id).copied().unwrap_or(zero),
        SignalSelector::Constant(v) => *v,
        SignalSelector::Each => signals.values().fold(zero, |acc, &v| acc + v),
    }
}

// ---------------------------------------------------------------------------
// Arithmetic combinator
// ---------------------------------------------------------------------------

/// Arithmetic operation for combinators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

/// Reads signals, performs an arithmetic operation, outputs the result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArithmeticCombinator {
    pub left: SignalSelector,
    pub op: ArithmeticOp,
    pub right: SignalSelector,
    pub output: ItemTypeId,
}

/// Apply an arithmetic operation. Division/modulo by zero returns zero.
fn apply_op(left: Fixed64, op: ArithmeticOp, right: Fixed64) -> Fixed64 {
    let zero = Fixed64::from_num(0);
    match op {
        ArithmeticOp::Add => left + right,
        ArithmeticOp::Subtract => left - right,
        ArithmeticOp::Multiply => left * right,
        ArithmeticOp::Divide => {
            if right == zero { zero } else { left / right }
        }
        ArithmeticOp::Modulo => {
            if right == zero { zero } else { left % right }
        }
    }
}

/// Evaluate an arithmetic combinator against a signal set.
/// Returns a signal set containing the single output signal.
pub fn evaluate_arithmetic(combinator: &ArithmeticCombinator, signals: &SignalSet) -> SignalSet {
    let left = resolve_selector(&combinator.left, signals);
    let right = resolve_selector(&combinator.right, signals);
    let result = apply_op(left, combinator.op, right);
    let mut output = SignalSet::new();
    output.insert(combinator.output, result);
    output
}

// ---------------------------------------------------------------------------
// Decider combinator
// ---------------------------------------------------------------------------

/// What the decider combinator outputs when its condition is true.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeciderOutput {
    /// Output a specific signal with value 1 when condition is true.
    One(ItemTypeId),
    /// Pass through the input signal's value when condition is true.
    InputCount(ItemTypeId),
    /// Pass through all input signals when condition is true.
    Everything,
}

/// Reads signals, evaluates a condition, conditionally outputs signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeciderCombinator {
    pub condition: Condition,
    pub output: DeciderOutput,
}

/// Evaluate a decider combinator against a signal set.
/// Returns an empty set if the condition is false.
pub fn evaluate_decider(combinator: &DeciderCombinator, signals: &SignalSet) -> SignalSet {
    if !evaluate_condition(&combinator.condition, signals) {
        return SignalSet::new();
    }
    let one = Fixed64::from_num(1);
    match &combinator.output {
        DeciderOutput::One(id) => {
            let mut out = SignalSet::new();
            out.insert(*id, one);
            out
        }
        DeciderOutput::InputCount(id) => {
            let mut out = SignalSet::new();
            let val = signals.get(id).copied().unwrap_or(Fixed64::from_num(0));
            out.insert(*id, val);
            out
        }
        DeciderOutput::Everything => signals.clone(),
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package factorial-logic`
Expected: FAIL — `condition.rs` types not yet defined. That's expected. Proceed to Task 4.

**Step 5: Skip commit — Task 3 and 4 must be done together since they depend on each other.**

---

### Task 4: Conditions — ComparisonOp, Condition, CircuitControl, InventoryReader

**Files:**
- Modify: `crates/factorial-logic/src/condition.rs`

**Step 1: Write the failing tests**

Add to `condition.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::combinator::SignalSelector;
    use crate::SignalSet;
    use factorial_core::id::ItemTypeId;

    fn fixed(v: f64) -> Fixed64 {
        Fixed64::from_num(v)
    }

    fn iron() -> ItemTypeId { ItemTypeId(0) }
    fn copper() -> ItemTypeId { ItemTypeId(1) }

    fn signals_with(items: &[(ItemTypeId, f64)]) -> SignalSet {
        items.iter().map(|&(id, v)| (id, fixed(v))).collect()
    }

    #[test]
    fn comparison_ops_all_variants() {
        let signals = signals_with(&[(iron(), 50.0)]);

        // Gt
        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Gt,
            right: SignalSelector::Constant(fixed(40.0)),
        };
        assert!(evaluate_condition(&cond, &signals));

        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Gt,
            right: SignalSelector::Constant(fixed(50.0)),
        };
        assert!(!evaluate_condition(&cond, &signals));

        // Lt
        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Lt,
            right: SignalSelector::Constant(fixed(60.0)),
        };
        assert!(evaluate_condition(&cond, &signals));

        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Lt,
            right: SignalSelector::Constant(fixed(50.0)),
        };
        assert!(!evaluate_condition(&cond, &signals));

        // Eq
        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Eq,
            right: SignalSelector::Constant(fixed(50.0)),
        };
        assert!(evaluate_condition(&cond, &signals));

        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Eq,
            right: SignalSelector::Constant(fixed(49.0)),
        };
        assert!(!evaluate_condition(&cond, &signals));

        // Gte
        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Gte,
            right: SignalSelector::Constant(fixed(50.0)),
        };
        assert!(evaluate_condition(&cond, &signals));

        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Gte,
            right: SignalSelector::Constant(fixed(51.0)),
        };
        assert!(!evaluate_condition(&cond, &signals));

        // Lte
        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Lte,
            right: SignalSelector::Constant(fixed(50.0)),
        };
        assert!(evaluate_condition(&cond, &signals));

        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Lte,
            right: SignalSelector::Constant(fixed(49.0)),
        };
        assert!(!evaluate_condition(&cond, &signals));

        // Ne
        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Ne,
            right: SignalSelector::Constant(fixed(40.0)),
        };
        assert!(evaluate_condition(&cond, &signals));

        let cond = Condition {
            left: SignalSelector::Signal(iron()),
            op: ComparisonOp::Ne,
            right: SignalSelector::Constant(fixed(50.0)),
        };
        assert!(!evaluate_condition(&cond, &signals));
    }

    #[test]
    fn circuit_control_evaluates_condition() {
        let signals = signals_with(&[(iron(), 100.0)]);

        let mut control = CircuitControl {
            condition: Condition {
                left: SignalSelector::Signal(iron()),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(50.0)),
            },
            wire_color: WireColor::Red,
            active: false,
            was_active: false,
        };

        update_circuit_control(&mut control, &signals);
        assert!(control.active);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package factorial-logic`
Expected: FAIL — types not defined

**Step 3: Write the implementation**

In `condition.rs`:

```rust
//! Conditions and circuit control for signal-driven building behavior.

use factorial_core::fixed::Fixed64;
use factorial_core::id::{ItemTypeId, NodeId};
use serde::{Deserialize, Serialize};

use crate::combinator::{SignalSelector, resolve_selector};
use crate::{SignalSet, WireColor};

// ---------------------------------------------------------------------------
// Comparison operations
// ---------------------------------------------------------------------------

/// Comparison operator for conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Gt,
    Lt,
    Eq,
    Gte,
    Lte,
    Ne,
}

// ---------------------------------------------------------------------------
// Condition
// ---------------------------------------------------------------------------

/// A predicate evaluated against wire network signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub left: SignalSelector,
    pub op: ComparisonOp,
    pub right: SignalSelector,
}

/// Evaluate a condition against a signal set.
pub fn evaluate_condition(condition: &Condition, signals: &SignalSet) -> bool {
    let left = resolve_selector(&condition.left, signals);
    let right = resolve_selector(&condition.right, signals);
    match condition.op {
        ComparisonOp::Gt => left > right,
        ComparisonOp::Lt => left < right,
        ComparisonOp::Eq => left == right,
        ComparisonOp::Gte => left >= right,
        ComparisonOp::Lte => left <= right,
        ComparisonOp::Ne => left != right,
    }
}

// ---------------------------------------------------------------------------
// Circuit control
// ---------------------------------------------------------------------------

/// Per-node circuit control: evaluates a condition and stores the result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitControl {
    pub condition: Condition,
    pub wire_color: WireColor,
    /// Result of evaluating the condition this tick.
    pub active: bool,
    /// Whether the control was active last tick (for transition detection).
    pub was_active: bool,
}

/// Update a circuit control's active state from the given signals.
pub fn update_circuit_control(control: &mut CircuitControl, signals: &SignalSet) {
    control.was_active = control.active;
    control.active = evaluate_condition(&control.condition, signals);
}

// ---------------------------------------------------------------------------
// Inventory reader
// ---------------------------------------------------------------------------

/// Which inventory to read signals from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventorySource {
    Input,
    Output,
}

/// Reads a building's inventory and emits signals for each item type present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReader {
    pub target_node: NodeId,
    pub source: InventorySource,
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package factorial-logic`
Expected: all tests in combinator.rs and condition.rs PASS

**Step 5: Commit**

```bash
git add crates/factorial-logic/src/combinator.rs crates/factorial-logic/src/condition.rs
git commit -m "feat(logic): add combinators, conditions, and circuit control types"
```

---

### Task 5: Signal source API and remove_node cleanup

**Files:**
- Modify: `crates/factorial-logic/src/lib.rs`

**Step 1: Write the failing tests**

Add to the existing `tests` module in `lib.rs`:

```rust
    #[test]
    fn set_constant_and_query() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);

        let mut signals = SignalSet::new();
        signals.insert(ItemTypeId(0), fixed(100.0));
        module.set_constant(nodes[0], signals.clone(), true);

        assert!(module.constants.contains_key(&nodes[0]));
        assert_eq!(module.constants[&nodes[0]].signals, signals);
        assert!(module.constants[&nodes[0]].enabled);
    }

    #[test]
    fn set_inventory_reader() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(2);

        module.set_inventory_reader(nodes[0], nodes[1], InventorySource::Output);

        assert!(module.inventory_readers.contains_key(&nodes[0]));
        assert_eq!(module.inventory_readers[&nodes[0]].target_node, nodes[1]);
        assert_eq!(module.inventory_readers[&nodes[0]].source, InventorySource::Output);
    }

    #[test]
    fn set_circuit_control() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);

        module.set_circuit_control(
            nodes[0],
            Condition {
                left: SignalSelector::Signal(ItemTypeId(0)),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(50.0)),
            },
            WireColor::Red,
        );

        assert!(module.circuit_controls.contains_key(&nodes[0]));
        assert_eq!(module.is_active(nodes[0]), Some(false));
    }

    #[test]
    fn remove_node_cleans_all_state() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);
        let net = module.create_network(WireColor::Red);

        module.add_to_network(net, nodes[0]);

        let mut signals = SignalSet::new();
        signals.insert(ItemTypeId(0), fixed(100.0));
        module.set_constant(nodes[0], signals, true);
        module.set_inventory_reader(nodes[0], nodes[0], InventorySource::Input);
        module.set_arithmetic(nodes[0], ArithmeticCombinator {
            left: SignalSelector::Constant(fixed(1.0)),
            op: ArithmeticOp::Add,
            right: SignalSelector::Constant(fixed(2.0)),
            output: ItemTypeId(0),
        });
        module.set_decider(nodes[0], DeciderCombinator {
            condition: Condition {
                left: SignalSelector::Constant(fixed(1.0)),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(0.0)),
            },
            output: DeciderOutput::One(ItemTypeId(0)),
        });
        module.set_circuit_control(
            nodes[0],
            Condition {
                left: SignalSelector::Constant(fixed(1.0)),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(0.0)),
            },
            WireColor::Red,
        );

        module.remove_node(nodes[0]);

        assert!(!module.constants.contains_key(&nodes[0]));
        assert!(!module.inventory_readers.contains_key(&nodes[0]));
        assert!(!module.arithmetic_combinators.contains_key(&nodes[0]));
        assert!(!module.decider_combinators.contains_key(&nodes[0]));
        assert!(!module.circuit_controls.contains_key(&nodes[0]));
        assert!(!module.combinator_outputs.contains_key(&nodes[0]));
        let network = module.networks.get(&net).unwrap();
        assert!(!network.members.contains(&nodes[0]));
        assert!(module.is_active(nodes[0]).is_none());
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package factorial-logic`
Expected: FAIL — `set_constant`, `set_inventory_reader`, etc. not defined

**Step 3: Write the implementation**

Add these methods to the `impl LogicModule` block in `lib.rs`:

```rust
    // --- Signal sources ---

    pub fn set_constant(&mut self, node: NodeId, signals: SignalSet, enabled: bool) {
        self.constants.insert(node, ConstantCombinator { signals, enabled });
    }

    pub fn set_inventory_reader(&mut self, node: NodeId, target: NodeId, source: InventorySource) {
        self.inventory_readers.insert(node, InventoryReader { target_node: target, source });
    }

    pub fn set_arithmetic(&mut self, node: NodeId, combinator: ArithmeticCombinator) {
        self.arithmetic_combinators.insert(node, combinator);
    }

    pub fn set_decider(&mut self, node: NodeId, combinator: DeciderCombinator) {
        self.decider_combinators.insert(node, combinator);
    }

    // --- Signal consumption ---

    pub fn set_circuit_control(&mut self, node: NodeId, condition: Condition, wire_color: WireColor) {
        self.circuit_controls.insert(node, CircuitControl {
            condition,
            wire_color,
            active: false,
            was_active: false,
        });
    }

    // --- Cleanup ---

    pub fn remove_node(&mut self, node: NodeId) {
        self.constants.remove(&node);
        self.inventory_readers.remove(&node);
        self.arithmetic_combinators.remove(&node);
        self.decider_combinators.remove(&node);
        self.circuit_controls.remove(&node);
        self.combinator_outputs.remove(&node);
        for network in self.networks.values_mut() {
            network.remove_member(node);
        }
    }
```

Also add these imports at the top of `lib.rs`:

```rust
use combinator::{ArithmeticCombinator, DeciderCombinator, SignalSelector, ArithmeticOp};
use condition::{CircuitControl, Condition, ComparisonOp, InventoryReader, InventorySource};
```

And re-export in the test module:

```rust
    use crate::combinator::*;
    use crate::condition::*;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package factorial-logic`
Expected: all tests PASS

**Step 5: Commit**

```bash
git add crates/factorial-logic/src/lib.rs
git commit -m "feat(logic): add signal source API and remove_node cleanup"
```

---

### Task 6: Tick pipeline — signal collection, merging, evaluation, events

**Files:**
- Modify: `crates/factorial-logic/src/lib.rs`

This is the core of the module. The tick pipeline runs in 5 steps:
1. Collect signals from constants, inventory readers, and last-tick combinator outputs
2. Merge signals per network
3. Evaluate arithmetic and decider combinators, store outputs for next tick
4. Evaluate circuit controls
5. Emit transition events

**Step 1: Write the failing tests**

Add to the `tests` module in `lib.rs`:

```rust
    use factorial_core::item::{Inventory, InventorySlot};

    fn make_inventories() -> (SecondaryMap<NodeId, Inventory>, SecondaryMap<NodeId, Inventory>) {
        (SecondaryMap::new(), SecondaryMap::new())
    }

    fn make_inventory_with(node: NodeId, item: ItemTypeId, qty: u32) -> SecondaryMap<NodeId, Inventory> {
        let mut map = SecondaryMap::new();
        let mut inv = Inventory::new(1, 1, 1000);
        inv.input_slots[0].add(item, qty);
        map.insert(node, inv);
        map
    }

    fn make_output_inventory_with(node: NodeId, item: ItemTypeId, qty: u32) -> SecondaryMap<NodeId, Inventory> {
        let mut map = SecondaryMap::new();
        let mut inv = Inventory::new(1, 1, 1000);
        inv.output_slots[0].add(item, qty);
        map.insert(node, inv);
        map
    }

    #[test]
    fn signal_merge_sums_correctly() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(2);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[1]);

        let iron = ItemTypeId(0);
        let copper = ItemTypeId(1);

        let mut s1 = SignalSet::new();
        s1.insert(iron, fixed(30.0));
        s1.insert(copper, fixed(10.0));
        module.set_constant(nodes[0], s1, true);

        let mut s2 = SignalSet::new();
        s2.insert(iron, fixed(20.0));
        module.set_constant(nodes[1], s2, true);

        let (inputs, outputs) = make_inventories();
        module.tick(&inputs, &outputs, 1);

        let merged = module.network_signals(net).unwrap();
        assert_eq!(merged.get(&iron), Some(&fixed(50.0)));
        assert_eq!(merged.get(&copper), Some(&fixed(10.0)));
    }

    #[test]
    fn constant_combinator_contributes_signals() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);

        let iron = ItemTypeId(0);
        let mut signals = SignalSet::new();
        signals.insert(iron, fixed(42.0));
        module.set_constant(nodes[0], signals, true);

        let (inputs, outputs) = make_inventories();
        module.tick(&inputs, &outputs, 1);

        let merged = module.network_signals(net).unwrap();
        assert_eq!(merged.get(&iron), Some(&fixed(42.0)));
    }

    #[test]
    fn disabled_constant_contributes_nothing() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);

        let iron = ItemTypeId(0);
        let mut signals = SignalSet::new();
        signals.insert(iron, fixed(42.0));
        module.set_constant(nodes[0], signals, false);

        let (inputs, outputs) = make_inventories();
        module.tick(&inputs, &outputs, 1);

        let merged = module.network_signals(net).unwrap();
        assert!(merged.is_empty());
    }

    #[test]
    fn inventory_reader_reads_node_inventory() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);

        let iron = ItemTypeId(0);
        module.set_inventory_reader(nodes[0], nodes[0], InventorySource::Input);

        let inputs = make_inventory_with(nodes[0], iron, 47);
        let outputs = SecondaryMap::new();
        module.tick(&inputs, &outputs, 1);

        let merged = module.network_signals(net).unwrap();
        assert_eq!(merged.get(&iron), Some(&fixed(47.0)));
    }

    #[test]
    fn inventory_reader_reads_output_inventory() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);

        let iron = ItemTypeId(0);
        module.set_inventory_reader(nodes[0], nodes[0], InventorySource::Output);

        let inputs = SecondaryMap::new();
        let outputs = make_output_inventory_with(nodes[0], iron, 33);
        module.tick(&inputs, &outputs, 1);

        let merged = module.network_signals(net).unwrap();
        assert_eq!(merged.get(&iron), Some(&fixed(33.0)));
    }

    #[test]
    fn one_tick_delay_on_combinators() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(2);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[1]);

        let iron = ItemTypeId(0);
        let steel = ItemTypeId(2);

        // Node 0: constant outputting iron=10
        let mut s = SignalSet::new();
        s.insert(iron, fixed(10.0));
        module.set_constant(nodes[0], s, true);

        // Node 1: arithmetic combinator: iron * 2 -> steel
        module.set_arithmetic(nodes[1], ArithmeticCombinator {
            left: SignalSelector::Signal(iron),
            op: ArithmeticOp::Multiply,
            right: SignalSelector::Constant(fixed(2.0)),
            output: steel,
        });

        let (inputs, outputs) = make_inventories();

        // Tick 1: combinator sees iron=10, computes steel=20, but output
        // won't appear in network until tick 2.
        module.tick(&inputs, &outputs, 1);
        let merged = module.network_signals(net).unwrap();
        assert_eq!(merged.get(&iron), Some(&fixed(10.0)));
        assert!(merged.get(&steel).is_none() || merged.get(&steel) == Some(&fixed(0.0)));

        // Tick 2: combinator's output from tick 1 now appears in network.
        module.tick(&inputs, &outputs, 2);
        let merged = module.network_signals(net).unwrap();
        assert_eq!(merged.get(&iron), Some(&fixed(10.0)));
        assert_eq!(merged.get(&steel), Some(&fixed(20.0)));
    }

    #[test]
    fn circuit_control_evaluates_from_tick() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(2);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[1]);

        let iron = ItemTypeId(0);
        let mut s = SignalSet::new();
        s.insert(iron, fixed(100.0));
        module.set_constant(nodes[0], s, true);

        module.set_circuit_control(
            nodes[1],
            Condition {
                left: SignalSelector::Signal(iron),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(50.0)),
            },
            WireColor::Red,
        );

        let (inputs, outputs) = make_inventories();
        module.tick(&inputs, &outputs, 1);

        assert_eq!(module.is_active(nodes[1]), Some(true));
    }

    #[test]
    fn event_fires_on_transition_only() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(2);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[1]);

        let iron = ItemTypeId(0);
        let mut s = SignalSet::new();
        s.insert(iron, fixed(100.0));
        module.set_constant(nodes[0], s, true);

        module.set_circuit_control(
            nodes[1],
            Condition {
                left: SignalSelector::Signal(iron),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(50.0)),
            },
            WireColor::Red,
        );

        let (inputs, outputs) = make_inventories();

        // Tick 1: transition to active — event.
        let events = module.tick(&inputs, &outputs, 1);
        let activated: Vec<_> = events.iter().filter(|e| matches!(e, LogicEvent::CircuitActivated { .. })).collect();
        assert_eq!(activated.len(), 1);

        // Tick 2: still active — no event.
        let events = module.tick(&inputs, &outputs, 2);
        let activated: Vec<_> = events.iter().filter(|e| matches!(e, LogicEvent::CircuitActivated { .. })).collect();
        assert_eq!(activated.len(), 0);

        // Disable the constant — condition becomes false.
        module.constants.get_mut(&nodes[0]).unwrap().enabled = false;

        // Tick 3: transition to inactive — event.
        let events = module.tick(&inputs, &outputs, 3);
        let deactivated: Vec<_> = events.iter().filter(|e| matches!(e, LogicEvent::CircuitDeactivated { .. })).collect();
        assert_eq!(deactivated.len(), 1);

        // Tick 4: still inactive — no event.
        let events = module.tick(&inputs, &outputs, 4);
        let deactivated: Vec<_> = events.iter().filter(|e| matches!(e, LogicEvent::CircuitDeactivated { .. })).collect();
        assert_eq!(deactivated.len(), 0);
    }

    #[test]
    fn two_wire_colors_independent() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(3);
        let red_net = module.create_network(WireColor::Red);
        let green_net = module.create_network(WireColor::Green);

        let iron = ItemTypeId(0);
        let copper = ItemTypeId(1);

        // Node 0 on red: iron=100
        module.add_to_network(red_net, nodes[0]);
        let mut s1 = SignalSet::new();
        s1.insert(iron, fixed(100.0));
        module.set_constant(nodes[0], s1, true);

        // Node 1 on green: copper=200
        module.add_to_network(green_net, nodes[1]);
        let mut s2 = SignalSet::new();
        s2.insert(copper, fixed(200.0));
        module.set_constant(nodes[1], s2, true);

        // Node 2 checks red network for iron > 50
        module.add_to_network(red_net, nodes[2]);
        module.set_circuit_control(
            nodes[2],
            Condition {
                left: SignalSelector::Signal(iron),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(50.0)),
            },
            WireColor::Red,
        );

        let (inputs, outputs) = make_inventories();
        module.tick(&inputs, &outputs, 1);

        // Red has iron=100, green has copper=200 — they don't mix.
        let red_signals = module.network_signals(red_net).unwrap();
        assert_eq!(red_signals.get(&iron), Some(&fixed(100.0)));
        assert!(red_signals.get(&copper).is_none());

        let green_signals = module.network_signals(green_net).unwrap();
        assert_eq!(green_signals.get(&copper), Some(&fixed(200.0)));
        assert!(green_signals.get(&iron).is_none());

        // Circuit control on node 2 reads red, so it's active.
        assert_eq!(module.is_active(nodes[2]), Some(true));
    }

    #[test]
    fn remove_network_cleans_signals() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(1);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);

        let iron = ItemTypeId(0);
        let mut s = SignalSet::new();
        s.insert(iron, fixed(42.0));
        module.set_constant(nodes[0], s, true);

        let (inputs, outputs) = make_inventories();
        module.tick(&inputs, &outputs, 1);
        assert!(module.network_signals(net).is_some());

        module.remove_network(net);
        assert!(module.network_signals(net).is_none());
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package factorial-logic`
Expected: FAIL — `tick()` not defined

**Step 3: Write the tick pipeline**

Add the `tick()` method to `impl LogicModule` in `lib.rs`. Also add a helper to read inventory signals.

```rust
    /// Advance all logic networks by one tick.
    ///
    /// 1. Collect signals from constants, inventory readers, last-tick combinator outputs
    /// 2. Merge signals per network
    /// 3. Evaluate combinators, store outputs for next tick
    /// 4. Evaluate circuit controls
    /// 5. Emit transition events
    pub fn tick(
        &mut self,
        inputs: &SecondaryMap<NodeId, Inventory>,
        outputs: &SecondaryMap<NodeId, Inventory>,
        current_tick: Ticks,
    ) -> Vec<LogicEvent> {
        let zero = Fixed64::from_num(0);
        let mut events = Vec::new();

        // --- Step 1 & 2: Collect and merge signals per network ---
        for network in self.networks.values_mut() {
            let mut merged = SignalSet::new();

            for &node in &network.members {
                // Constant combinator
                if let Some(constant) = self.constants.get(&node) {
                    if constant.enabled {
                        for (&item, &value) in &constant.signals {
                            *merged.entry(item).or_insert(zero) += value;
                        }
                    }
                }

                // Inventory reader
                if let Some(reader) = self.inventory_readers.get(&node) {
                    let inv = match reader.source {
                        InventorySource::Input => inputs.get(reader.target_node),
                        InventorySource::Output => outputs.get(reader.target_node),
                    };
                    if let Some(inv) = inv {
                        let slots = match reader.source {
                            InventorySource::Input => &inv.input_slots,
                            InventorySource::Output => &inv.output_slots,
                        };
                        for slot in slots {
                            for stack in &slot.stacks {
                                if stack.quantity > 0 {
                                    *merged.entry(stack.item_type).or_insert(zero) +=
                                        Fixed64::from_num(stack.quantity);
                                }
                            }
                        }
                    }
                }

                // Combinator outputs from last tick (one-tick delay)
                if let Some(prev_output) = self.combinator_outputs.get(&node) {
                    for (&item, &value) in prev_output {
                        *merged.entry(item).or_insert(zero) += value;
                    }
                }
            }

            network.signals = merged;
        }

        // --- Step 3: Evaluate combinators, store new outputs for next tick ---
        let mut new_combinator_outputs: BTreeMap<NodeId, SignalSet> = BTreeMap::new();

        // Find which network each combinator node belongs to, get those signals.
        // A node could be on multiple networks; use the first one found.
        // (In practice, combinators read from their network.)
        let node_to_network_signals: BTreeMap<NodeId, SignalSet> = {
            let mut map = BTreeMap::new();
            for network in self.networks.values() {
                for &node in &network.members {
                    // If a node is on multiple networks, merge all signals.
                    // This handles the case where a node is on red and green.
                    let entry = map.entry(node).or_insert_with(SignalSet::new);
                    for (&item, &value) in &network.signals {
                        *entry.entry(item).or_insert(zero) += value;
                    }
                }
            }
            map
        };

        let empty_signals = SignalSet::new();

        for (&node, combinator) in &self.arithmetic_combinators {
            let signals = node_to_network_signals.get(&node).unwrap_or(&empty_signals);
            let output = combinator::evaluate_arithmetic(combinator, signals);
            new_combinator_outputs.insert(node, output);
        }

        for (&node, combinator) in &self.decider_combinators {
            let signals = node_to_network_signals.get(&node).unwrap_or(&empty_signals);
            let output = combinator::evaluate_decider(combinator, signals);
            if !output.is_empty() {
                new_combinator_outputs.insert(node, output);
            }
        }

        self.combinator_outputs = new_combinator_outputs;

        // --- Step 4: Evaluate circuit controls ---
        for (&node, control) in self.circuit_controls.iter_mut() {
            // Find the signals for this control's wire color.
            let mut control_signals = SignalSet::new();
            for network in self.networks.values() {
                if network.color == control.wire_color && network.members.contains(&node) {
                    for (&item, &value) in &network.signals {
                        *control_signals.entry(item).or_insert(zero) += value;
                    }
                }
            }
            condition::update_circuit_control(control, &control_signals);
        }

        // --- Step 5: Emit transition events ---
        for (&node, control) in &self.circuit_controls {
            if control.active && !control.was_active {
                events.push(LogicEvent::CircuitActivated {
                    node,
                    tick: current_tick,
                });
            } else if !control.active && control.was_active {
                events.push(LogicEvent::CircuitDeactivated {
                    node,
                    tick: current_tick,
                });
            }
        }

        // NetworkSignalsChanged events
        for (id, network) in &self.networks {
            let changed = match self.prev_signals.get(id) {
                Some(prev) => prev != &network.signals,
                None => !network.signals.is_empty(),
            };
            if changed {
                events.push(LogicEvent::NetworkSignalsChanged {
                    network: *id,
                    tick: current_tick,
                });
            }
        }

        // Store current signals for next tick's change detection.
        self.prev_signals = self
            .networks
            .iter()
            .map(|(&id, net)| (id, net.signals.clone()))
            .collect();

        events
    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package factorial-logic`
Expected: all tests PASS

**Step 5: Commit**

```bash
git add crates/factorial-logic/src/lib.rs
git commit -m "feat(logic): implement tick pipeline with signal collection, merging, and events"
```

---

### Task 7: Serde round-trip test

**Files:**
- Modify: `crates/factorial-logic/src/lib.rs`

**Step 1: Write the test**

Add to `tests` module in `lib.rs`:

```rust
    #[test]
    fn serde_round_trip() {
        let mut module = LogicModule::new();
        let nodes = make_node_ids(3);
        let net = module.create_network(WireColor::Red);
        module.add_to_network(net, nodes[0]);
        module.add_to_network(net, nodes[1]);

        let iron = ItemTypeId(0);
        let mut s = SignalSet::new();
        s.insert(iron, fixed(100.0));
        module.set_constant(nodes[0], s, true);

        module.set_circuit_control(
            nodes[1],
            Condition {
                left: SignalSelector::Signal(iron),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(fixed(50.0)),
            },
            WireColor::Red,
        );

        module.set_arithmetic(nodes[2], ArithmeticCombinator {
            left: SignalSelector::Signal(iron),
            op: ArithmeticOp::Multiply,
            right: SignalSelector::Constant(fixed(2.0)),
            output: ItemTypeId(1),
        });

        // Tick once to populate internal state.
        let (inputs, outputs) = make_inventories();
        module.tick(&inputs, &outputs, 1);

        // Serialize and deserialize.
        let data = bitcode::serialize(&module).expect("serialize");
        let restored: LogicModule = bitcode::deserialize(&data).expect("deserialize");

        assert_eq!(restored.networks.len(), module.networks.len());
        assert_eq!(restored.constants.len(), module.constants.len());
        assert_eq!(restored.circuit_controls.len(), module.circuit_controls.len());
        assert_eq!(restored.arithmetic_combinators.len(), module.arithmetic_combinators.len());
        assert_eq!(
            restored.network_signals(net).unwrap().get(&iron),
            module.network_signals(net).unwrap().get(&iron),
        );
    }
```

**Step 2: Run test**

Run: `cargo test --package factorial-logic -- serde_round_trip`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/factorial-logic/src/lib.rs
git commit -m "test(logic): add serde round-trip test"
```

---

### Task 8: Clippy, fmt, and final verification

**Files:** None new — this is a quality gate.

**Step 1: Run clippy**

Run: `cargo clippy --package factorial-logic --all-targets -- -D warnings`
Expected: no warnings. If any, fix them.

**Step 2: Run fmt check**

Run: `cargo fmt --all -- --check`
Expected: no formatting issues. If any, run `cargo fmt --all` and commit.

**Step 3: Run full workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass including the new factorial-logic tests.

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "chore(logic): fix clippy warnings and formatting"
```

(Skip commit if no changes needed.)

---

### Task 9: Final review and merge preparation

**Step 1: Review test count**

Run: `cargo test --package factorial-logic -- --list 2>&1 | grep "test " | wc -l`
Expected: ~20+ tests

**Step 2: Verify all design doc tests are covered**

Cross-check against the 20 tests listed in the design doc. All should be present.

**Step 3: Use superpowers:finishing-a-development-branch to decide on merge/PR.**
