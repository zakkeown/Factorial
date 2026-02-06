//! Junction system for routing items between transport edges.
//!
//! Junctions sit on nodes and control how items flow between incoming
//! and outgoing edges. They are processed during the component phase
//! in topological order.

use crate::fixed::Fixed64;
use crate::id::ItemTypeId;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Split/Merge policies
// ---------------------------------------------------------------------------

/// How a splitter distributes items across outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitPolicy {
    /// Distribute evenly across outputs in round-robin fashion.
    RoundRobin,
    /// Send to the first output with capacity, in order.
    Priority,
    /// Attempt to send equal amounts to all outputs simultaneously.
    EvenSplit,
}

/// How a merger selects items from inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergePolicy {
    /// Pull from inputs in round-robin fashion.
    RoundRobin,
    /// Pull from the first input with items, in order.
    Priority,
}

// ---------------------------------------------------------------------------
// Junction configurations
// ---------------------------------------------------------------------------

/// Configuration for an inserter junction (picks up items and places them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InserterConfig {
    /// Items per tick the inserter can move.
    pub speed: Fixed64,
    /// Maximum items to pick up at once.
    pub stack_size: u32,
    /// Optional item type filter (None = accept all).
    pub filter: Option<ItemTypeId>,
}

/// Configuration for a splitter junction (distributes items across outputs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitterConfig {
    /// How to distribute items.
    pub policy: SplitPolicy,
    /// Optional item type filter.
    pub filter: Option<ItemTypeId>,
}

/// Configuration for a merger junction (combines items from inputs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergerConfig {
    /// How to select input items.
    pub policy: MergePolicy,
}

/// A junction attached to a node, controlling item routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Junction {
    Inserter(InserterConfig),
    Splitter(SplitterConfig),
    Merger(MergerConfig),
}

// ---------------------------------------------------------------------------
// Junction runtime state
// ---------------------------------------------------------------------------

/// Runtime state for a junction, persisted across ticks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JunctionState {
    /// Current round-robin index for RoundRobin policies.
    pub round_robin_index: usize,
    /// Fractional item accumulator for sub-1 speeds.
    pub accumulated: Fixed64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test 1: inserter_config_defaults
    // -----------------------------------------------------------------------
    #[test]
    fn inserter_config_defaults() {
        let config = InserterConfig {
            speed: Fixed64::from_num(2),
            stack_size: 5,
            filter: None,
        };
        assert_eq!(config.speed, Fixed64::from_num(2));
        assert_eq!(config.stack_size, 5);
        assert!(config.filter.is_none());
    }

    // -----------------------------------------------------------------------
    // Test 2: splitter_roundrobin
    // -----------------------------------------------------------------------
    #[test]
    fn splitter_roundrobin() {
        let config = SplitterConfig {
            policy: SplitPolicy::RoundRobin,
            filter: None,
        };
        assert_eq!(config.policy, SplitPolicy::RoundRobin);
    }

    // -----------------------------------------------------------------------
    // Test 3: splitter_priority
    // -----------------------------------------------------------------------
    #[test]
    fn splitter_priority() {
        let config = SplitterConfig {
            policy: SplitPolicy::Priority,
            filter: None,
        };
        assert_eq!(config.policy, SplitPolicy::Priority);
    }

    // -----------------------------------------------------------------------
    // Test 4: splitter_evensplit
    // -----------------------------------------------------------------------
    #[test]
    fn splitter_evensplit() {
        let config = SplitterConfig {
            policy: SplitPolicy::EvenSplit,
            filter: None,
        };
        assert_eq!(config.policy, SplitPolicy::EvenSplit);
    }

    // -----------------------------------------------------------------------
    // Test 5: merger_roundrobin
    // -----------------------------------------------------------------------
    #[test]
    fn merger_roundrobin() {
        let config = MergerConfig {
            policy: MergePolicy::RoundRobin,
        };
        assert_eq!(config.policy, MergePolicy::RoundRobin);
    }

    // -----------------------------------------------------------------------
    // Test 6: merger_priority
    // -----------------------------------------------------------------------
    #[test]
    fn merger_priority() {
        let config = MergerConfig {
            policy: MergePolicy::Priority,
        };
        assert_eq!(config.policy, MergePolicy::Priority);
    }

    // -----------------------------------------------------------------------
    // Test 7: junction_state_default
    // -----------------------------------------------------------------------
    #[test]
    fn junction_state_default() {
        let state = JunctionState::default();
        assert_eq!(state.round_robin_index, 0);
        assert_eq!(state.accumulated, Fixed64::from_num(0));
    }

    // -----------------------------------------------------------------------
    // Test 8: junction_enum_inserter
    // -----------------------------------------------------------------------
    #[test]
    fn junction_enum_inserter() {
        let junction = Junction::Inserter(InserterConfig {
            speed: Fixed64::from_num(3),
            stack_size: 10,
            filter: None,
        });
        assert!(matches!(junction, Junction::Inserter(_)));
        if let Junction::Inserter(config) = &junction {
            assert_eq!(config.speed, Fixed64::from_num(3));
            assert_eq!(config.stack_size, 10);
        }
    }

    // -----------------------------------------------------------------------
    // Test 9: junction_enum_splitter
    // -----------------------------------------------------------------------
    #[test]
    fn junction_enum_splitter() {
        let junction = Junction::Splitter(SplitterConfig {
            policy: SplitPolicy::EvenSplit,
            filter: Some(ItemTypeId(42)),
        });
        assert!(matches!(junction, Junction::Splitter(_)));
        if let Junction::Splitter(config) = &junction {
            assert_eq!(config.policy, SplitPolicy::EvenSplit);
            assert_eq!(config.filter, Some(ItemTypeId(42)));
        }
    }

    // -----------------------------------------------------------------------
    // Test 10: junction_enum_merger
    // -----------------------------------------------------------------------
    #[test]
    fn junction_enum_merger() {
        let junction = Junction::Merger(MergerConfig {
            policy: MergePolicy::Priority,
        });
        assert!(matches!(junction, Junction::Merger(_)));
        if let Junction::Merger(config) = &junction {
            assert_eq!(config.policy, MergePolicy::Priority);
        }
    }

    // -----------------------------------------------------------------------
    // Test 11: junction_serde_roundtrip
    // -----------------------------------------------------------------------
    #[test]
    fn junction_serde_roundtrip() {
        let junction = Junction::Splitter(SplitterConfig {
            policy: SplitPolicy::RoundRobin,
            filter: Some(ItemTypeId(7)),
        });
        let data = bitcode::serialize(&junction).expect("serialize junction");
        let restored: Junction = bitcode::deserialize(&data).expect("deserialize junction");
        assert_eq!(junction, restored);

        let state = JunctionState {
            round_robin_index: 3,
            accumulated: Fixed64::from_num(5),
        };
        let data = bitcode::serialize(&state).expect("serialize state");
        let restored: JunctionState = bitcode::deserialize(&data).expect("deserialize state");
        assert_eq!(state, restored);
    }

    // -----------------------------------------------------------------------
    // Test 12: junction_state_persists_index
    // -----------------------------------------------------------------------
    #[test]
    fn junction_state_persists_index() {
        let mut state = JunctionState::default();
        assert_eq!(state.round_robin_index, 0);

        state.round_robin_index = 42;
        assert_eq!(state.round_robin_index, 42);

        // Verify it persists across clone.
        let cloned = state.clone();
        assert_eq!(cloned.round_robin_index, 42);
    }
}
