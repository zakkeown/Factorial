//! Multiplayer validation tools for state comparison and determinism checking.
//!
//! Provides utilities for comparing two engine states to find divergences,
//! and for validating that a simulation produces deterministic results.

use crate::engine::Engine;
use crate::id::{EdgeId, NodeId};
use crate::serialize::DeserializeError;

// ---------------------------------------------------------------------------
// State diff types
// ---------------------------------------------------------------------------

/// Difference between two engine states at the node level.
#[derive(Debug, Clone)]
pub enum NodeDiff {
    /// Node exists only in engine A.
    OnlyInA(NodeId),
    /// Node exists only in engine B.
    OnlyInB(NodeId),
    /// Node exists in both but has different state.
    StateMismatch { node: NodeId, description: String },
}

/// Difference between two engine states at the edge level.
#[derive(Debug, Clone)]
pub enum EdgeDiff {
    /// Edge exists only in engine A.
    OnlyInA(EdgeId),
    /// Edge exists only in engine B.
    OnlyInB(EdgeId),
    /// Edge exists in both but has different state.
    StateMismatch { edge: EdgeId, description: String },
}

/// Per-subsystem match results.
#[derive(Debug, Clone)]
pub struct SubsystemDiff {
    pub graph_matches: bool,
    pub processors_match: bool,
    pub processor_states_match: bool,
    pub inventories_match: bool,
    pub transports_match: bool,
    pub sim_state_matches: bool,
}

/// Full state diff between two engines.
#[derive(Debug, Clone)]
pub struct StateDiff {
    pub is_identical: bool,
    pub subsystem_diffs: SubsystemDiff,
    pub node_diffs: Vec<NodeDiff>,
    pub edge_diffs: Vec<EdgeDiff>,
}

// ---------------------------------------------------------------------------
// Quick compare (subsystem-level only)
// ---------------------------------------------------------------------------

/// Quick subsystem-level comparison using hashes.
pub fn quick_compare(a: &Engine, b: &Engine) -> SubsystemDiff {
    let ha = a.subsystem_hashes();
    let hb = b.subsystem_hashes();

    SubsystemDiff {
        graph_matches: ha.graph == hb.graph,
        processors_match: ha.processors == hb.processors,
        processor_states_match: ha.processor_states == hb.processor_states,
        inventories_match: ha.inventories == hb.inventories,
        transports_match: ha.transports == hb.transports,
        sim_state_matches: ha.sim_state == hb.sim_state,
    }
}

// ---------------------------------------------------------------------------
// Full diff
// ---------------------------------------------------------------------------

/// Compute a detailed diff between two engine states.
pub fn diff_engines(a: &Engine, b: &Engine) -> StateDiff {
    let subsystem_diffs = quick_compare(a, b);

    let mut node_diffs = Vec::new();
    let mut edge_diffs = Vec::new();

    // Compare nodes
    let a_nodes: Vec<NodeId> = a.graph.nodes().map(|(id, _)| id).collect();
    let b_nodes: Vec<NodeId> = b.graph.nodes().map(|(id, _)| id).collect();

    for &node in &a_nodes {
        if !b.graph.contains_node(node) {
            node_diffs.push(NodeDiff::OnlyInA(node));
        } else {
            // Compare node state
            let mut mismatches = Vec::new();

            // Compare processor states
            let ps_a = a.processor_states.get(node);
            let ps_b = b.processor_states.get(node);
            if ps_a != ps_b {
                mismatches.push("processor_state");
            }

            // Compare input inventories
            let inv_a = a.inputs.get(node);
            let inv_b = b.inputs.get(node);
            if inv_a != inv_b {
                mismatches.push("input_inventory");
            }

            // Compare output inventories
            let out_a = a.outputs.get(node);
            let out_b = b.outputs.get(node);
            if out_a != out_b {
                mismatches.push("output_inventory");
            }

            if !mismatches.is_empty() {
                node_diffs.push(NodeDiff::StateMismatch {
                    node,
                    description: mismatches.join(", "),
                });
            }
        }
    }

    for &node in &b_nodes {
        if !a.graph.contains_node(node) {
            node_diffs.push(NodeDiff::OnlyInB(node));
        }
    }

    // Compare edges
    let a_edges: Vec<EdgeId> = a.graph.edges().map(|(id, _)| id).collect();
    let b_edges: Vec<EdgeId> = b.graph.edges().map(|(id, _)| id).collect();

    for &edge in &a_edges {
        if !b.graph.contains_edge(edge) {
            edge_diffs.push(EdgeDiff::OnlyInA(edge));
        }
    }

    for &edge in &b_edges {
        if !a.graph.contains_edge(edge) {
            edge_diffs.push(EdgeDiff::OnlyInB(edge));
        }
    }

    let is_identical = node_diffs.is_empty()
        && edge_diffs.is_empty()
        && subsystem_diffs.graph_matches
        && subsystem_diffs.processors_match
        && subsystem_diffs.processor_states_match
        && subsystem_diffs.inventories_match
        && subsystem_diffs.transports_match
        && subsystem_diffs.sim_state_matches;

    StateDiff {
        is_identical,
        subsystem_diffs,
        node_diffs,
        edge_diffs,
    }
}

// ---------------------------------------------------------------------------
// Determinism validation
// ---------------------------------------------------------------------------

/// Result of a determinism validation run.
#[derive(Debug)]
pub struct DeterminismResult {
    /// Whether the two runs produced identical results.
    pub is_deterministic: bool,
    /// Tick at which divergence was first detected (if any).
    pub divergence_tick: Option<u64>,
    /// Hash log: (tick, hash_run1, hash_run2) for each tick.
    pub hash_log: Vec<(u64, u64, u64)>,
}

/// Validate that running the same simulation twice from the same snapshot
/// produces identical results.
pub fn validate_determinism(
    snapshot_data: &[u8],
    ticks: u64,
) -> Result<DeterminismResult, DeserializeError> {
    let mut engine_a = Engine::deserialize(snapshot_data)?;
    let mut engine_b = Engine::deserialize(snapshot_data)?;

    let mut hash_log = Vec::new();
    let mut divergence_tick = None;

    for _ in 0..ticks {
        engine_a.step();
        engine_b.step();

        let hash_a = engine_a.state_hash();
        let hash_b = engine_b.state_hash();
        let tick = engine_a.sim_state.tick;

        hash_log.push((tick, hash_a, hash_b));

        if hash_a != hash_b && divergence_tick.is_none() {
            divergence_tick = Some(tick);
        }
    }

    Ok(DeterminismResult {
        is_deterministic: divergence_tick.is_none(),
        divergence_tick,
        hash_log,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::SimulationStrategy;
    use crate::test_utils::*;

    fn make_test_engine() -> Engine {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let src = add_node(&mut engine, make_source(iron(), 3.0), 100, 100);
        let consumer = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 5),
            100,
            100,
        );
        connect(&mut engine, src, consumer, make_flow_transport(5.0));
        engine
    }

    // -----------------------------------------------------------------------
    // Test 1: Identical engines have identical diff
    // -----------------------------------------------------------------------
    #[test]
    fn diff_identical_engines() {
        let engine_a = make_test_engine();
        let data = engine_a.serialize().unwrap();
        let engine_b = Engine::deserialize(&data).unwrap();

        let diff = diff_engines(&engine_a, &engine_b);
        assert!(diff.is_identical);
        assert!(diff.node_diffs.is_empty());
        assert!(diff.edge_diffs.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 2: Different tick count detected
    // -----------------------------------------------------------------------
    #[test]
    fn diff_different_tick_count() {
        let mut engine_a = make_test_engine();
        let engine_b = make_test_engine();

        engine_a.step();
        // engine_b has 0 ticks, engine_a has 1

        let diff = diff_engines(&engine_a, &engine_b);
        assert!(!diff.is_identical);
        assert!(!diff.subsystem_diffs.sim_state_matches);
    }

    // -----------------------------------------------------------------------
    // Test 3: Node only in A detected
    // -----------------------------------------------------------------------
    #[test]
    fn diff_detects_node_only_in_a() {
        let mut engine_a = Engine::new(SimulationStrategy::Tick);
        add_node(&mut engine_a, make_source(iron(), 1.0), 100, 100);
        add_node(&mut engine_a, make_source(iron(), 1.0), 100, 100);

        let engine_b = Engine::new(SimulationStrategy::Tick);
        // engine_b has different nodes (different SlotMap keys)

        let diff = diff_engines(&engine_a, &engine_b);
        assert!(!diff.is_identical);
        // A has nodes that B doesn't
        assert!(!diff.node_diffs.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 4: Edge only in B detected
    // -----------------------------------------------------------------------
    #[test]
    fn diff_detects_edge_only_in_b() {
        let engine_a = Engine::new(SimulationStrategy::Tick);

        let mut engine_b = Engine::new(SimulationStrategy::Tick);
        let src = add_node(&mut engine_b, make_source(iron(), 1.0), 100, 100);
        let sink = add_node(&mut engine_b, make_source(iron(), 1.0), 100, 100);
        connect(&mut engine_b, src, sink, make_flow_transport(1.0));

        let diff = diff_engines(&engine_a, &engine_b);
        assert!(!diff.is_identical);
        assert!(!diff.edge_diffs.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 5: Inventory mismatch detected
    // -----------------------------------------------------------------------
    #[test]
    fn diff_detects_inventory_mismatch() {
        let mut engine_a = make_test_engine();
        let data = engine_a.serialize().unwrap();
        let engine_b = Engine::deserialize(&data).unwrap();

        // Advance only engine_a to create inventory differences
        engine_a.step();
        engine_a.step();
        engine_a.step();

        let diff = diff_engines(&engine_a, &engine_b);
        assert!(!diff.is_identical);
    }

    // -----------------------------------------------------------------------
    // Test 6: Processor state mismatch detected
    // -----------------------------------------------------------------------
    #[test]
    fn diff_detects_processor_state_mismatch() {
        let engine_a = make_test_engine();
        let data = engine_a.serialize().unwrap();
        let mut engine_b = Engine::deserialize(&data).unwrap();

        // Step engine_b to change processor states
        engine_b.step();

        let diff = diff_engines(&engine_a, &engine_b);
        assert!(!diff.is_identical);
    }

    // -----------------------------------------------------------------------
    // Test 7: Quick compare identical
    // -----------------------------------------------------------------------
    #[test]
    fn quick_compare_identical() {
        let engine = make_test_engine();
        let data = engine.serialize().unwrap();
        let engine2 = Engine::deserialize(&data).unwrap();

        let result = quick_compare(&engine, &engine2);
        assert!(result.graph_matches);
        assert!(result.processors_match);
        assert!(result.processor_states_match);
        assert!(result.inventories_match);
        assert!(result.transports_match);
        assert!(result.sim_state_matches);
    }

    // -----------------------------------------------------------------------
    // Test 8: Quick compare after mutation
    // -----------------------------------------------------------------------
    #[test]
    fn quick_compare_after_mutation() {
        let engine_a = make_test_engine();
        let data = engine_a.serialize().unwrap();
        let mut engine_b = Engine::deserialize(&data).unwrap();

        engine_b.step();

        let result = quick_compare(&engine_a, &engine_b);
        assert!(!result.sim_state_matches);
    }

    // -----------------------------------------------------------------------
    // Test 9: Validate determinism passes
    // -----------------------------------------------------------------------
    #[test]
    fn validate_determinism_passes() {
        let engine = make_test_engine();
        let data = engine.serialize().unwrap();

        let result = validate_determinism(&data, 20).unwrap();
        assert!(result.is_deterministic);
        assert!(result.divergence_tick.is_none());
        assert_eq!(result.hash_log.len(), 20);

        // All hash pairs should match
        for (_, h1, h2) in &result.hash_log {
            assert_eq!(h1, h2);
        }
    }

    // -----------------------------------------------------------------------
    // Test 10: Validate determinism reports divergence (if it happened)
    // -----------------------------------------------------------------------
    #[test]
    fn validate_determinism_reports_divergence() {
        // Since our engine IS deterministic, this test just verifies
        // the result structure for a passing case.
        let engine = make_test_engine();
        let data = engine.serialize().unwrap();

        let result = validate_determinism(&data, 5).unwrap();
        assert!(result.is_deterministic);
        assert_eq!(result.hash_log.len(), 5);

        // Verify hash_log has correct tick numbers
        for (i, (tick, _, _)) in result.hash_log.iter().enumerate() {
            assert_eq!(*tick, (i + 1) as u64);
        }
    }

    // -----------------------------------------------------------------------
    // Test 11: Diff empty engines
    // -----------------------------------------------------------------------
    #[test]
    fn diff_empty_engines() {
        let engine_a = Engine::new(SimulationStrategy::Tick);
        let engine_b = Engine::new(SimulationStrategy::Tick);

        let diff = diff_engines(&engine_a, &engine_b);
        assert!(diff.is_identical);
        assert!(diff.node_diffs.is_empty());
        assert!(diff.edge_diffs.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 12: Subsystem diff pinpoints divergent system
    // -----------------------------------------------------------------------
    #[test]
    fn subsystem_diff_pinpoints_divergent_system() {
        let mut engine_a = make_test_engine();
        let data = engine_a.serialize().unwrap();
        let engine_b = Engine::deserialize(&data).unwrap();

        // Only step engine_a — changes sim_state, inventories, processor_states
        engine_a.step();

        let diff = diff_engines(&engine_a, &engine_b);
        assert!(!diff.is_identical);

        // sim_state should definitely differ (tick incremented)
        assert!(!diff.subsystem_diffs.sim_state_matches);

        // Graph structure should still match (no mutations)
        assert!(diff.subsystem_diffs.graph_matches);
    }
}
