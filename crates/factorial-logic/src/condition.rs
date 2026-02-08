//! Conditions and circuit control for signal-driven building behavior.

use factorial_core::id::NodeId;
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SignalSet;
    use crate::combinator::SignalSelector;
    use factorial_core::fixed::Fixed64;
    use factorial_core::id::ItemTypeId;

    fn fixed(v: f64) -> Fixed64 {
        Fixed64::from_num(v)
    }

    fn iron() -> ItemTypeId {
        ItemTypeId(0)
    }

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
