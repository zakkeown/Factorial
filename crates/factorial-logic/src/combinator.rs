//! Combinators for transforming and filtering signals.

use factorial_core::fixed::Fixed64;
use factorial_core::id::ItemTypeId;
use serde::{Deserialize, Serialize};

use crate::SignalSet;
use crate::condition::{Condition, evaluate_condition};

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
            if right == zero {
                zero
            } else {
                left / right
            }
        }
        ArithmeticOp::Modulo => {
            if right == zero {
                zero
            } else {
                left % right
            }
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::ComparisonOp;
    use factorial_core::id::ItemTypeId;

    fn fixed(v: f64) -> Fixed64 {
        Fixed64::from_num(v)
    }

    fn iron() -> ItemTypeId {
        ItemTypeId(0)
    }
    fn copper() -> ItemTypeId {
        ItemTypeId(1)
    }
    fn steel() -> ItemTypeId {
        ItemTypeId(2)
    }

    fn signals_with(items: &[(ItemTypeId, f64)]) -> SignalSet {
        items.iter().map(|&(id, v)| (id, fixed(v))).collect()
    }

    #[test]
    fn resolve_signal_selector() {
        let signals = signals_with(&[(iron(), 50.0), (copper(), 30.0)]);

        assert_eq!(
            resolve_selector(&SignalSelector::Signal(iron()), &signals),
            fixed(50.0)
        );
        assert_eq!(
            resolve_selector(&SignalSelector::Signal(steel()), &signals),
            fixed(0.0)
        );
        assert_eq!(
            resolve_selector(&SignalSelector::Constant(fixed(42.0)), &signals),
            fixed(42.0)
        );
        assert_eq!(
            resolve_selector(&SignalSelector::Each, &signals),
            fixed(80.0)
        );
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
