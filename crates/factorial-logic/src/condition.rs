//! Conditions and circuit control for signal-driven building behavior.

use factorial_core::id::NodeId;
use serde::{Deserialize, Serialize};

use crate::WireColor;
use crate::combinator::SignalSelector;

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

/// A predicate evaluated against wire network signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub left: SignalSelector,
    pub op: ComparisonOp,
    pub right: SignalSelector,
}

/// Per-node circuit control: evaluates a condition and stores the result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitControl {
    pub condition: Condition,
    pub wire_color: WireColor,
    pub active: bool,
    pub was_active: bool,
}

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
