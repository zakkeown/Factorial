//! Stress and endurance tests for the Factorial engine.
//!
//! These are marked `#[ignore]` for nightly CI runs. Run with:
//!   cargo test --package factorial-core -- --ignored

use factorial_core::engine::Engine;
use factorial_core::id::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;

/// Build a 50k-node factory, run 1000 ticks, verify hash is deterministic.
#[test]
#[ignore]
fn test_50k_node_factory_1000_ticks() {
    let mut engine_a = build_large_factory(50_000);
    let mut engine_b = build_large_factory(50_000);

    for _ in 0..1000 {
        engine_a.step();
        engine_b.step();
    }

    assert_eq!(
        engine_a.state_hash(),
        engine_b.state_hash(),
        "50k-node factory should be deterministic after 1000 ticks"
    );
}

/// Run a medium factory for 100,000 ticks.
/// Verify no panics and final hash is deterministic.
/// (100k ticks at ~1.6ms/step ≈ ~3 min per engine in release mode.)
#[test]
#[ignore]
fn test_endurance_100k_ticks() {
    let mut engine_a = build_large_factory(5_000);
    let mut engine_b = build_large_factory(5_000);

    for _ in 0..100_000 {
        engine_a.step();
    }
    for _ in 0..100_000 {
        engine_b.step();
    }

    assert_eq!(
        engine_a.state_hash(),
        engine_b.state_hash(),
        "5k-node factory should be deterministic after 100k ticks"
    );
}

/// Add/remove 500 nodes per tick for 200 ticks.
/// Verify graph stays consistent (no dangling edges, correct node count).
#[test]
#[ignore]
fn test_mutation_storm() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let mut all_nodes: Vec<NodeId> = Vec::new();

    for tick in 0..200 {
        // Add 500 nodes.
        let mut new_nodes = Vec::new();
        for _ in 0..500 {
            let node = add_node(&mut engine, make_source(iron(), 1.0), 50, 50);
            new_nodes.push(node);
        }

        // Connect new nodes to each other in pairs.
        for pair in new_nodes.chunks(2) {
            if pair.len() == 2 {
                connect(&mut engine, pair[0], pair[1], make_flow_transport(5.0));
            }
        }

        // Also connect some new nodes to old nodes if available.
        if !all_nodes.is_empty() {
            for (i, &new_node) in new_nodes.iter().enumerate().take(10) {
                let old_idx = (tick * 10 + i) % all_nodes.len();
                if engine.graph.contains_node(all_nodes[old_idx]) {
                    connect(
                        &mut engine,
                        all_nodes[old_idx],
                        new_node,
                        make_flow_transport(3.0),
                    );
                }
            }
        }

        all_nodes.extend(&new_nodes);

        // Step the engine.
        engine.step();

        // Remove 500 of the oldest nodes (if we have enough).
        if all_nodes.len() > 1000 {
            let to_remove: Vec<NodeId> = all_nodes.drain(..500).collect();
            for node in to_remove {
                if engine.graph.contains_node(node) {
                    engine.graph.queue_remove_node(node);
                }
            }
            engine.graph.apply_mutations();
        }

        // Verify consistency: no dangling edges.
        for (_eid, edata) in engine.graph.edges() {
            assert!(
                engine.graph.contains_node(edata.from),
                "dangling edge source at tick {tick}"
            );
            assert!(
                engine.graph.contains_node(edata.to),
                "dangling edge destination at tick {tick}"
            );
        }
    }

    // Final check: topo order covers all nodes.
    let (order, _) = engine.graph.topological_order_with_feedback();
    assert_eq!(order.len(), engine.graph.node_count());
}
