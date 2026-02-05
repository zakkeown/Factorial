//! Integration tests for the Factorial simulation engine.
//!
//! These tests exercise end-to-end behavior across the full engine pipeline:
//! graph mutations, transport, processing, inventories, serialization, and
//! determinism.

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use factorial_core::transport::*;

// ===========================================================================
// Test 1: Builderment-style chain
// ===========================================================================
//
// Source --FlowTransport--> FixedRecipe --FlowTransport--> Consumer (sink)
// Verify items flow end-to-end across multiple ticks.

#[test]
fn builderment_style_chain() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Source: produces 5 iron per tick.
    let source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);

    // Assembler: 2 iron -> 1 gear, 3 ticks.
    let assembler = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 3),
        100,
        100,
    );

    // Sink: a recipe that consumes gears (large duration so it acts as a sink).
    let sink = add_node(
        &mut engine,
        make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 9999),
        100,
        100,
    );

    // Connect: source -> assembler, assembler -> sink.
    connect(
        &mut engine,
        source,
        assembler,
        make_flow_transport(10.0),
    );
    connect(&mut engine, assembler, sink, make_flow_transport(10.0));

    // Run 20 ticks.
    for _ in 0..20 {
        engine.step();
    }

    // After 20 ticks:
    // - Source produces 5 iron/tick into its output.
    // - FlowTransport moves iron from source output to assembler input.
    // - Assembler consumes 2 iron, works for 3 ticks, produces 1 gear.
    // - FlowTransport moves gears from assembler output to sink input.

    // Verify the assembler has consumed iron (input should have items coming in).
    // The source should have produced items.
    let source_output = output_total(&engine, source);
    let assembler_input = input_total(&engine, assembler);
    let assembler_output = output_total(&engine, assembler);
    let sink_input = input_total(&engine, sink);

    // Items should have flowed through the chain. The exact amounts depend on
    // pipeline timing, but we verify non-trivial flow occurred.
    let total_items_in_system = source_output + assembler_input + assembler_output + sink_input;
    assert!(
        total_items_in_system > 0,
        "items should flow through the chain; total items in system: {total_items_in_system}"
    );

    // The sink should have received at least some gears (the assembler should
    // have completed at least one cycle and the transport delivered them).
    // After 20 ticks: source produces iron tick 1, transport delivers tick 2,
    // assembler starts tick 2, completes tick 4, transport delivers gear tick 5.
    // So by tick 20 we should have multiple gears at the sink.
    let gears_at_sink = input_quantity(&engine, sink, gear());
    assert!(
        gears_at_sink > 0,
        "sink should have received gears, got {gears_at_sink}"
    );

    // Verify the source is still producing (not stalled).
    let source_state = engine.get_processor_state(source).unwrap();
    assert!(
        matches!(source_state, ProcessorState::Working { .. }),
        "source should still be working, got {source_state:?}"
    );
}

// ===========================================================================
// Test 2: Multi-output recipe
// ===========================================================================
//
// Electrolyzer: water -> oxygen + hydrogen (FixedRecipe with 2 outputs).
// Verify both outputs appear. Verify stall when output inventory is full.

#[test]
fn multi_output_recipe() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Water source.
    let water_source = add_node(&mut engine, make_source(water(), 5.0), 100, 100);

    // Electrolyzer: 1 water -> 1 oxygen + 1 hydrogen, 2 ticks.
    // Use a small output capacity to test stalling.
    let electrolyzer_pending = engine.graph.queue_add_node(building());
    let result = engine.graph.apply_mutations();
    let electrolyzer = result.resolve_node(electrolyzer_pending).unwrap();

    engine.set_processor(
        electrolyzer,
        make_recipe(
            vec![(water(), 1)],
            vec![(oxygen(), 1), (hydrogen(), 1)],
            2,
        ),
    );
    engine.set_input_inventory(electrolyzer, simple_inventory(100));
    // Small output: capacity of 4 total items.
    engine.set_output_inventory(electrolyzer, simple_inventory(4));

    // Connect water source to electrolyzer.
    connect(
        &mut engine,
        water_source,
        electrolyzer,
        make_flow_transport(10.0),
    );

    // Run enough ticks for the electrolyzer to complete a cycle.
    // Tick 1: source produces water into output.
    // Tick 2: transport moves water to electrolyzer input.
    // Tick 3: electrolyzer consumes water, starts working (progress=1).
    // Tick 4: electrolyzer completes (progress=2), produces outputs.
    for _ in 0..10 {
        engine.step();
    }

    // Verify both outputs are present in the electrolyzer's output inventory.
    let o2 = output_quantity(&engine, electrolyzer, oxygen());
    let h2 = output_quantity(&engine, electrolyzer, hydrogen());
    assert!(
        o2 > 0,
        "electrolyzer should produce oxygen, got {o2}"
    );
    assert!(
        h2 > 0,
        "electrolyzer should produce hydrogen, got {h2}"
    );

    // Now continue running. The output capacity is 4, so after 2 cycles
    // (4 items total: 2 oxygen + 2 hydrogen) the output should be full
    // and the electrolyzer should stall.
    for _ in 0..50 {
        engine.step();
    }

    let o2_final = output_quantity(&engine, electrolyzer, oxygen());
    let h2_final = output_quantity(&engine, electrolyzer, hydrogen());
    let total_output = o2_final + h2_final;

    // Output should be capped at capacity (4).
    assert!(
        total_output <= 4,
        "output should be capped at capacity 4, got {total_output}"
    );

    // The electrolyzer should be stalled with OutputFull.
    let state = engine.get_processor_state(electrolyzer).unwrap();
    assert!(
        matches!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::OutputFull
            }
        ),
        "electrolyzer should be stalled with OutputFull, got {state:?}"
    );
}

// ===========================================================================
// Test 3: Belt with inserters
// ===========================================================================
//
// Source --ItemTransport belt--> Destination building.
// Verify items are picked up and delivered correctly across ticks.

#[test]
fn belt_with_inserters() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Source: produces 1 iron per tick.
    let source = add_node(&mut engine, make_source(iron(), 1.0), 100, 100);

    // Destination: recipe that needs iron.
    let dest = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
        100,
        100,
    );

    // Connect with an ItemTransport belt: 5 slots, speed 1, 1 lane.
    let belt_edge = connect(
        &mut engine,
        source,
        dest,
        Transport::Item(ItemTransport {
            speed: fixed(1.0),
            slot_count: 5,
            lanes: 1,
        }),
    );

    // Run enough ticks for items to traverse the belt.
    // Tick 1: source produces iron.
    // Tick 2: transport picks up item, places at input slot (slot 4).
    // Ticks 3-6: item advances through slots 3, 2, 1, 0 then delivered.
    // So items start arriving at dest around tick 6-7.
    for _ in 0..20 {
        engine.step();
    }

    // Verify items have arrived at the destination.
    let iron_at_dest = input_quantity(&engine, dest, iron());
    assert!(
        iron_at_dest > 0 || output_quantity(&engine, dest, gear()) > 0,
        "items should have been delivered via belt; iron at dest: {iron_at_dest}, gears produced: {}",
        output_quantity(&engine, dest, gear())
    );

    // Verify the belt has items in transit (it should be partially full).
    let belt_state = engine.get_transport_state(belt_edge).unwrap();
    if let TransportState::Item(bs) = belt_state {
        // The belt state should be accessible and valid (slot count matches config).
        assert_eq!(
            bs.slots.len(),
            5,
            "belt should have 5 slots, got {}",
            bs.slots.len()
        );
        // After 20 ticks with speed 1 and continuous input, belt should have
        // items flowing through it (some slots occupied).
        let occupied = bs.occupied_count();
        assert!(
            occupied > 0,
            "belt should have items in transit after 20 ticks, got {occupied} occupied slots"
        );
    } else {
        panic!("expected ItemTransport state");
    }

    // The destination should have produced some gears by now
    // (it has a 5-tick recipe and items have been arriving since ~tick 7).
    let gears = output_quantity(&engine, dest, gear());
    assert!(
        gears > 0,
        "destination should have produced gears by tick 20, got {gears}"
    );
}

// ===========================================================================
// Test 4: Modifiers
// ===========================================================================
//
// Speed module on assembler (FixedRecipe). Verify reduced base_duration
// effect -- recipe completes faster with a Speed modifier.

#[test]
fn modifier_speed_effect() {
    // Run two identical factories: one without modifier, one with 2x speed.
    // Compare how many items each produces over the same number of ticks.

    fn build_factory(speed_modifier: Option<Fixed64>) -> Engine {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Source: infinite iron at 10/tick.
        let source = add_node(&mut engine, make_source(iron(), 10.0), 100, 100);

        // Assembler: 1 iron -> 1 gear, 10 ticks.
        let assembler = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 10),
            100,
            1000,
        );

        // Apply speed modifier if requested.
        if let Some(speed) = speed_modifier {
            engine.set_modifiers(
                assembler,
                vec![Modifier {
                    id: ModifierId(0),
                    kind: ModifierKind::Speed(speed),
                }],
            );
        }

        // Connect source to assembler.
        connect(&mut engine, source, assembler, make_flow_transport(20.0));

        engine
    }

    let mut base_engine = build_factory(None);
    let mut fast_engine = build_factory(Some(fixed(2.0)));

    // Run both for 100 ticks.
    for _ in 0..100 {
        base_engine.step();
        fast_engine.step();
    }

    // Count gears produced by each.
    // Find the assembler node (second node added).
    let base_nodes: Vec<_> = base_engine.graph.nodes().map(|(id, _)| id).collect();
    let fast_nodes: Vec<_> = fast_engine.graph.nodes().map(|(id, _)| id).collect();

    let base_assembler = base_nodes[1];
    let fast_assembler = fast_nodes[1];

    let base_gears = output_quantity(&base_engine, base_assembler, gear());
    let fast_gears = output_quantity(&fast_engine, fast_assembler, gear());

    // The fast assembler (2x speed) should produce more gears than the base.
    // Base: 10 tick recipe, so ~10 gears in 100 ticks (after pipeline warmup).
    // Fast: 5 tick effective recipe, so ~20 gears in 100 ticks.
    assert!(
        fast_gears > base_gears,
        "2x speed assembler should produce more gears: fast={fast_gears}, base={base_gears}"
    );

    // Verify approximately 2x throughput (with some tolerance for pipeline warmup).
    // The fast assembler should produce roughly double.
    assert!(
        fast_gears >= base_gears + (base_gears / 3),
        "fast assembler should produce significantly more: fast={fast_gears}, base={base_gears}"
    );
}

// ===========================================================================
// Test 5: Serialize round-trip
// ===========================================================================
//
// Build a factory, run 50 ticks, serialize, deserialize, run 50 more ticks,
// compare state hash with a fresh run that runs 100 ticks straight.

#[test]
fn serialize_round_trip_determinism() {
    fn build_factory() -> Engine {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        let source = add_node(&mut engine, make_source(iron(), 3.0), 100, 100);
        let assembler = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 5),
            100,
            100,
        );
        let sink = add_node(
            &mut engine,
            make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 9999),
            100,
            100,
        );

        connect(&mut engine, source, assembler, make_flow_transport(10.0));
        connect(&mut engine, assembler, sink, make_flow_transport(10.0));

        engine
    }

    // Path A: run 100 ticks straight.
    let mut straight = build_factory();
    for _ in 0..100 {
        straight.step();
    }
    let straight_hash = straight.state_hash();

    // Path B: run 50, serialize, deserialize, run 50 more.
    let mut split = build_factory();
    for _ in 0..50 {
        split.step();
    }

    let serialized = split.serialize().expect("serialize should succeed");
    let mut restored =
        Engine::deserialize(&serialized).expect("deserialize should succeed");

    // Verify the restored engine has the same state hash as the split engine at tick 50.
    assert_eq!(
        restored.state_hash(),
        split.state_hash(),
        "restored engine should have same hash as original at tick 50"
    );

    // Run 50 more ticks on the restored engine.
    for _ in 0..50 {
        restored.step();
    }

    // Compare final state hashes.
    assert_eq!(
        restored.state_hash(),
        straight_hash,
        "serialized round-trip (50+50) should match straight run (100). \
         restored={}, straight={}",
        restored.state_hash(),
        straight_hash,
    );

    // Also verify tick counts match.
    assert_eq!(
        restored.sim_state.tick, straight.sim_state.tick,
        "tick counts should match: restored={}, straight={}",
        restored.sim_state.tick, straight.sim_state.tick,
    );
}

// ===========================================================================
// Test 6: Determinism
// ===========================================================================
//
// Run same factory twice from same initial state for 100 ticks, verify
// identical tick-by-tick state hashes.

#[test]
fn determinism_identical_runs() {
    fn build_and_run() -> Vec<u64> {
        let mut engine = Engine::new(SimulationStrategy::Tick);

        // Build a non-trivial factory: source -> assembler -> sink, with
        // a second source and assembler feeding into the same sink.
        let iron_source = add_node(&mut engine, make_source(iron(), 3.0), 100, 100);
        let copper_source = add_node(&mut engine, make_source(copper(), 2.0), 100, 100);

        let iron_assembler = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 5),
            100,
            100,
        );
        let copper_assembler = add_node(
            &mut engine,
            make_recipe(vec![(copper(), 1)], vec![(gear(), 1)], 3),
            100,
            100,
        );

        // Sink that consumes gears.
        let sink = add_node(
            &mut engine,
            make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 9999),
            200,
            100,
        );

        // Connect iron chain.
        connect(
            &mut engine,
            iron_source,
            iron_assembler,
            make_flow_transport(10.0),
        );
        connect(
            &mut engine,
            iron_assembler,
            sink,
            make_flow_transport(10.0),
        );

        // Connect copper chain.
        connect(
            &mut engine,
            copper_source,
            copper_assembler,
            make_flow_transport(10.0),
        );
        connect(
            &mut engine,
            copper_assembler,
            sink,
            make_flow_transport(10.0),
        );

        // Run 100 ticks, recording hash each tick.
        let mut hashes = Vec::with_capacity(100);
        for _ in 0..100 {
            engine.step();
            hashes.push(engine.state_hash());
        }
        hashes
    }

    let run1 = build_and_run();
    let run2 = build_and_run();

    assert_eq!(
        run1.len(),
        run2.len(),
        "both runs should have 100 hashes"
    );

    for (tick, (h1, h2)) in run1.iter().zip(run2.iter()).enumerate() {
        assert_eq!(
            h1, h2,
            "state hashes diverged at tick {}: run1={h1}, run2={h2}",
            tick + 1,
        );
    }

    // Also verify hashes are not all the same (the simulation should evolve).
    let unique_hashes: std::collections::HashSet<u64> = run1.iter().copied().collect();
    assert!(
        unique_hashes.len() > 1,
        "state hashes should change between ticks, but all {} hashes are identical",
        run1.len()
    );
}

// ===========================================================================
// Batch transport chain
// ===========================================================================
#[test]
fn batch_transport_chain() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);
    let consumer = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 999)], vec![(gear(), 1)], 9999),
        100,
        100,
    );
    connect(&mut engine, source, consumer, make_batch_transport(10, 5));
    for _ in 0..20 {
        engine.step();
    }
    let consumer_input = input_quantity(&engine, consumer, iron());
    assert!(
        consumer_input > 0,
        "consumer should have received iron via batch transport, got {consumer_input}"
    );
}

// ===========================================================================
// Vehicle transport round-trip
// ===========================================================================
#[test]
fn vehicle_transport_round_trip() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let source = add_node(&mut engine, make_source(iron(), 10.0), 100, 100);
    let consumer = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 999)], vec![(gear(), 1)], 9999),
        200,
        100,
    );
    connect(&mut engine, source, consumer, make_vehicle_transport(20, 3));
    for _ in 0..20 {
        engine.step();
    }
    let consumer_input = input_quantity(&engine, consumer, iron());
    assert!(
        consumer_input > 0,
        "consumer should have received iron via vehicle transport, got {consumer_input}"
    );
    assert!(
        consumer_input <= 80,
        "vehicle shouldn't exceed capacity * trips; got {consumer_input}"
    );
}

// ===========================================================================
// Mixed transport factory
// ===========================================================================
#[test]
fn mixed_transport_factory() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let iron_source = add_node(&mut engine, make_source(iron(), 5.0), 100, 100);
    let copper_source = add_node(&mut engine, make_source(copper(), 5.0), 100, 100);
    let assembler = add_node(
        &mut engine,
        make_recipe(vec![(iron(), 1), (copper(), 1)], vec![(gear(), 1)], 3),
        100,
        100,
    );
    let buffer = add_node(
        &mut engine,
        make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 9999),
        100,
        100,
    );
    let sink = add_node(
        &mut engine,
        make_recipe(vec![(gear(), 999)], vec![(iron(), 1)], 9999),
        100,
        100,
    );
    connect(&mut engine, iron_source, assembler, make_flow_transport(10.0));
    connect(&mut engine, copper_source, assembler, make_item_transport(5));
    connect(&mut engine, assembler, buffer, make_batch_transport(5, 3));
    connect(&mut engine, buffer, sink, make_vehicle_transport(10, 2));
    for _ in 0..50 {
        engine.step();
    }
    let total_items = input_total(&engine, assembler)
        + output_total(&engine, assembler)
        + input_total(&engine, buffer)
        + output_total(&engine, buffer)
        + input_total(&engine, sink);
    assert!(
        total_items > 0,
        "items should flow through mixed transport factory; total items in system: {total_items}"
    );
}
