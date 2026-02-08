//! Combinators for transforming and filtering signals.

use factorial_core::fixed::Fixed64;
use factorial_core::id::ItemTypeId;
use serde::{Deserialize, Serialize};

/// Selects a value from the current signal set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalSelector {
    Signal(ItemTypeId),
    Constant(Fixed64),
    Each,
}

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

/// What the decider combinator outputs when its condition is true.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeciderOutput {
    One(ItemTypeId),
    InputCount(ItemTypeId),
    Everything,
}

/// Reads signals, evaluates a condition, conditionally outputs signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeciderCombinator {
    pub condition: crate::condition::Condition,
    pub output: DeciderOutput,
}
