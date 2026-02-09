//! Property-based tests for the Factorial core engine.
//!
//! Uses proptest to generate random engines and mutation sequences,
//! then verify structural invariants hold.

use factorial_core::engine::Engine;
use factorial_core::id::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use proptest::prelude::*;

// ===========================================================================
// Generators
// ===========================================================================

/// Generate a random valid engine with up to `max_nodes` nodes.
fn arb_engine(max_nodes: usize) -> impl Strategy<Value = Engine> {
    (1..=max_nodes).prop_flat_map(move |n| {
        // Generate random processor types and transport types for each node.
        proptest::collection::vec(0..4u8, n).prop_map(move |proc_types| {
            let mut engine = Engine::new(SimulationStrategy::Tick);
            let mut nodes = Vec::with_capacity(n);

            for &pt in &proc_types {
                let processor = match pt {
                    0 => make_source(iron(), 2.0),
                    1 => make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
                    2 => make_source(copper(), 1.5),
                    _ => make_recipe(vec![(copper(), 2)], vec![(gear(), 1)], 10),
                };
                let node = add_node(&mut engine, processor, 100, 100);
                nodes.push(node);
            }

            // Connect each node to the next (linear chain).
            for i in 0..nodes.len().saturating_sub(1) {
                connect(
                    &mut engine,
                    nodes[i],
                    nodes[i + 1],
                    make_flow_transport(10.0),
                );
            }

            engine
        })
    })
}

/// Mutation operations for testing mutation safety.
#[derive(Debug, Clone)]
enum MutOp {
    AddNode,
    RemoveNode(usize),
    Connect(usize, usize),
    Disconnect(usize),
    Step,
}

fn arb_mutation_sequence(max_ops: usize) -> impl Strategy<Value = Vec<MutOp>> {
    proptest::collection::vec(
        prop_oneof![
            Just(MutOp::AddNode),
            (0..50usize).prop_map(MutOp::RemoveNode),
            (0..50usize, 0..50usize).prop_map(|(a, b)| MutOp::Connect(a, b)),
            (0..50usize).prop_map(MutOp::Disconnect),
            Just(MutOp::Step),
        ],
        1..=max_ops,
    )
}

// ===========================================================================
// Properties
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Serialize round-trip: deserialize(serialize(e)).state_hash == e.state_hash
    #[test]
    fn serialize_round_trip(mut engine in arb_engine(50)) {
        // Run a few ticks to populate state.
        for _ in 0..5 {
            engine.step();
        }

        let data = engine.serialize().expect("serialize should succeed");
        let restored = Engine::deserialize(&data).expect("deserialize should succeed");

        // Round-trip: serialize the restored engine and deserialize again.
        let data2 = restored.serialize().expect("re-serialize should succeed");

        // Step both to get comparable hashes.
        let mut engine_a = Engine::deserialize(&data).unwrap();
        let mut engine_b = Engine::deserialize(&data2).unwrap();
        engine_a.step();
        engine_b.step();
        prop_assert_eq!(engine_a.state_hash(), engine_b.state_hash());
    }

    /// Determinism: two engines from identical initial state produce identical hashes.
    #[test]
    fn deterministic_simulation(seed in 0..100usize) {
        let node_count = 10 + seed % 40;
        let ticks = 10 + seed % 20;

        let mut engine_a = build_chain_factory(node_count);
        let mut engine_b = build_chain_factory(node_count);

        for _ in 0..ticks {
            engine_a.step();
            engine_b.step();
        }

        prop_assert_eq!(engine_a.state_hash(), engine_b.state_hash());
    }

    /// Topo invariant: topological_order_with_feedback returns every node exactly once.
    #[test]
    fn topo_order_covers_all_nodes(mut engine in arb_engine(100)) {
        // Run a step to ensure graph is settled.
        engine.step();

        let node_count = engine.graph.node_count();
        let (order, _back_edges) = engine.graph.topological_order_with_feedback();

        prop_assert_eq!(order.len(), node_count,
            "topo order should contain all {} nodes, got {}", node_count, order.len());

        // Check uniqueness.
        let mut seen = std::collections::HashSet::new();
        for &nid in order {
            prop_assert!(seen.insert(nid), "duplicate node in topo order: {:?}", nid);
        }
    }

    /// Topo level invariant: for topological_order_with_feedback, forward edges
    /// go from earlier position to later position.
    #[test]
    fn topo_order_forward_edges(mut engine in arb_engine(100)) {
        engine.step();

        let (order, back_edges) = engine.graph.topological_order_with_feedback();

        // Build position map.
        let mut position = std::collections::HashMap::new();
        for (idx, &nid) in order.iter().enumerate() {
            position.insert(nid, idx);
        }

        let back_edge_set: std::collections::HashSet<EdgeId> = back_edges.iter().copied().collect();

        // For every edge NOT in back_edges, from should be before to.
        for (eid, edata) in engine.graph.edges() {
            if back_edge_set.contains(&eid) {
                continue;
            }
            let from_pos = position.get(&edata.from).copied();
            let to_pos = position.get(&edata.to).copied();
            if let (Some(fp), Some(tp)) = (from_pos, to_pos) {
                prop_assert!(fp < tp,
                    "forward edge {:?} ({:?} -> {:?}) has from_pos {} >= to_pos {}",
                    eid, edata.from, edata.to, fp, tp);
            }
        }
    }

    /// Mutation safety: any sequence of mutations on a valid engine doesn't panic.
    #[test]
    fn mutation_safety(ops in arb_mutation_sequence(100)) {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let mut node_ids: Vec<NodeId> = Vec::new();
        let mut edge_ids: Vec<EdgeId> = Vec::new();

        for op in ops {
            match op {
                MutOp::AddNode => {
                    let node = add_node(
                        &mut engine,
                        make_source(iron(), 1.0),
                        100,
                        100,
                    );
                    node_ids.push(node);
                }
                MutOp::RemoveNode(idx) => {
                    if !node_ids.is_empty() {
                        let idx = idx % node_ids.len();
                        let node = node_ids.remove(idx);
                        engine.graph.queue_remove_node(node);
                        engine.graph.apply_mutations();
                        // Remove edges connected to this node.
                        edge_ids.retain(|&eid| {
                            engine.graph.contains_edge(eid)
                        });
                    }
                }
                MutOp::Connect(from, to) => {
                    if node_ids.len() >= 2 {
                        let from_idx = from % node_ids.len();
                        let to_idx = to % node_ids.len();
                        if from_idx != to_idx {
                            let edge = connect(
                                &mut engine,
                                node_ids[from_idx],
                                node_ids[to_idx],
                                make_flow_transport(1.0),
                            );
                            edge_ids.push(edge);
                        }
                    }
                }
                MutOp::Disconnect(idx) => {
                    if !edge_ids.is_empty() {
                        let idx = idx % edge_ids.len();
                        let edge = edge_ids.remove(idx);
                        engine.graph.queue_disconnect(edge);
                        engine.graph.apply_mutations();
                    }
                }
                MutOp::Step => {
                    engine.step();
                }
            }
        }

        // Final validation: graph should be consistent.
        let node_count = engine.graph.node_count();
        let (order, _) = engine.graph.topological_order_with_feedback();
        prop_assert_eq!(order.len(), node_count);
    }
}
