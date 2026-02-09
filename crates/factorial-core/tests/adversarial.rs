//! Adversarial input tests for the Factorial engine.
//!
//! Tests edge cases that should either return errors or be handled gracefully
//! without panics.

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;

/// Self-loop: edge from node to itself.
/// Should be handled gracefully (no panic).
#[test]
fn self_loop_edge() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let node = add_node(&mut engine, make_source(iron(), 2.0), 100, 100);

    // Connect node to itself.
    let pending = engine.graph.queue_connect(node, node);
    let result = engine.graph.apply_mutations();
    let edge = result.resolve_edge(pending).unwrap();
    engine.set_transport(edge, make_flow_transport(10.0));

    // Should not panic on step.
    for _ in 0..20 {
        engine.step();
    }
}

/// Zero-capacity inventory with items arriving.
#[test]
fn zero_capacity_inventory() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let source = add_node(&mut engine, make_source(iron(), 10.0), 100, 100);
    let sink = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
        0, // zero input capacity
        0, // zero output capacity
    );
    connect(&mut engine, source, sink, make_flow_transport(10.0));

    // Should not panic even though capacity is 0.
    for _ in 0..20 {
        engine.step();
    }
}

/// Zero-duration recipe.
#[test]
fn zero_duration_recipe() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);
    let processor = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 0), // zero duration
        100,
        100,
    );
    connect(&mut engine, source, processor, make_flow_transport(10.0));

    // Should not panic.
    for _ in 0..20 {
        engine.step();
    }
}

/// Recipe that produces its own input item (feedback without explicit cycle).
#[test]
fn recipe_produces_own_input() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let node = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 1)], vec![(iron(), 2)], 3), // produces what it consumes
        100,
        100,
    );

    // Seed with some iron in input.
    if let Some(inv) = engine.get_input_inventory_mut(node)
        && let Some(slot) = inv.input_slots.first_mut()
    {
        let _ = slot.add(iron(), 10);
    }

    // Should not panic.
    for _ in 0..50 {
        engine.step();
    }
}

/// Negative Fixed64 rate on a source processor.
#[test]
fn negative_source_rate() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let _node = add_node(
        &mut engine,
        Processor::Source(SourceProcessor {
            output_type: iron(),
            base_rate: Fixed64::from_num(-5),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0),
            initial_properties: None,
        }),
        100,
        100,
    );

    // Should not panic.
    for _ in 0..20 {
        engine.step();
    }
}

/// 1000 modifiers stacked on a single node.
#[test]
fn many_modifiers_stacked() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let node = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 10),
        100,
        100,
    );

    // Stack 1000 speed modifiers.
    let mods: Vec<Modifier> = (0..1000)
        .map(|_| Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(Fixed64::from_num(1.01)),
            stacking: StackingRule::Multiplicative,
        })
        .collect();
    engine.set_modifiers(node, mods);

    // Seed input.
    if let Some(inv) = engine.get_input_inventory_mut(node)
        && let Some(slot) = inv.input_slots.first_mut()
    {
        let _ = slot.add(iron(), 1000);
    }

    // Should not panic.
    for _ in 0..20 {
        engine.step();
    }
}

/// Disconnected graph: 3 isolated subgraphs, verify all process correctly.
#[test]
fn disconnected_graph() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Create 3 independent chains.
    let mut chains: Vec<Vec<NodeId>> = Vec::new();
    for _ in 0..3 {
        let source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);
        let middle = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 3),
            100,
            100,
        );
        let sink = add_node(
            &mut engine,
            make_recipe(vec![(gear(), 1)], vec![(copper(), 1)], 5),
            100,
            100,
        );
        connect(&mut engine, source, middle, make_flow_transport(10.0));
        connect(&mut engine, middle, sink, make_flow_transport(10.0));
        chains.push(vec![source, middle, sink]);
    }

    // Run and verify all subgraphs produce output.
    for _ in 0..50 {
        engine.step();
    }

    // All sources should have produced items.
    for chain in &chains {
        let source_output = output_total(&engine, chain[0]);
        assert!(
            source_output > 0 || output_total(&engine, chain[1]) > 0,
            "disconnected subgraph should still process"
        );
    }
}

/// Duplicate edges between same node pair.
#[test]
fn duplicate_edges() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);
    let sink = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
        100,
        100,
    );

    // Add 5 duplicate edges between the same pair.
    for _ in 0..5 {
        connect(&mut engine, source, sink, make_flow_transport(10.0));
    }

    assert_eq!(engine.graph.edge_count(), 5);

    // Should not panic.
    for _ in 0..20 {
        engine.step();
    }
}

/// Remove a node that has active transport edges.
/// Verify edges are cleaned up properly.
#[test]
fn remove_node_with_active_edges() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let a = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);
    let b = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
        100,
        100,
    );
    let c = add_node(
        &mut engine,
        make_recipe(vec![(gear(), 1)], vec![(copper(), 1)], 5),
        100,
        100,
    );

    let edge_ab = connect(&mut engine, a, b, make_flow_transport(10.0));
    let edge_bc = connect(&mut engine, b, c, make_flow_transport(10.0));

    // Warm up so transport state is populated.
    for _ in 0..10 {
        engine.step();
    }

    // Remove the middle node.
    engine.graph.queue_remove_node(b);
    engine.graph.apply_mutations();

    // Edges connected to b should be gone.
    assert!(!engine.graph.contains_edge(edge_ab));
    assert!(!engine.graph.contains_edge(edge_bc));
    assert!(!engine.graph.contains_node(b));

    // Should not panic on further steps.
    for _ in 0..10 {
        engine.step();
    }
}

/// Deserialize truncated/corrupted bytes.
/// Verify Err, not panic.
#[test]
fn deserialize_corrupted_bytes() {
    // Empty bytes.
    assert!(Engine::deserialize(&[]).is_err());
    assert!(Engine::deserialize_partitioned(&[]).is_err());

    // Too short for header.
    assert!(Engine::deserialize(&[0x01, 0x02, 0x03]).is_err());
    assert!(Engine::deserialize_partitioned(&[0x01, 0x02, 0x03]).is_err());

    // Random garbage.
    let garbage: Vec<u8> = (0..1024).map(|i| (i * 37 + 13) as u8).collect();
    assert!(Engine::deserialize(&garbage).is_err());
    assert!(Engine::deserialize_partitioned(&garbage).is_err());

    // Valid header but truncated body: serialize a real engine, then truncate.
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let _node = add_node(&mut engine, make_source(iron(), 1.0), 100, 100);
    engine.step();

    let data = engine.serialize().unwrap();
    if data.len() > 10 {
        let truncated = &data[..data.len() / 2];
        assert!(Engine::deserialize(truncated).is_err());
    }

    let pdata = engine.serialize_partitioned().unwrap();
    if pdata.len() > 10 {
        let truncated = &pdata[..pdata.len() / 2];
        assert!(Engine::deserialize_partitioned(truncated).is_err());
    }
}
