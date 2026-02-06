//! Integration test: Dual-Role Nodes (Recipe + Power)
//!
//! Demonstrates that a single engine node can simultaneously act as a
//! FixedRecipe processor (converting steam to water) AND a power producer
//! (via the PowerModule). The PowerModule's `set_producer_capacity()` is
//! called each tick to dynamically update power output based on recipe
//! throughput, showing how game code bridges the two systems.

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::ItemTypeId;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils;
use factorial_power::{PowerModule, PowerProducer};

#[test]
fn dual_role_node_recipe_plus_power() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let steam = ItemTypeId(100);
    let water = ItemTypeId(3);

    // Steam turbine: 1 steam -> 1 water, 1 tick duration.
    // This node acts as a FixedRecipe processor in the engine.
    let turbine = test_utils::add_node(
        &mut engine,
        test_utils::make_recipe(vec![(steam, 1)], vec![(water, 1)], 1),
        100,
        100,
    );

    // Seed the turbine's input inventory with steam so it can run recipes.
    let _ = engine.get_input_inventory_mut(turbine).unwrap().input_slots[0].add(steam, 50);

    // Power module registers the SAME node as a power producer.
    // This is the "dual role" -- the engine sees a recipe processor,
    // while the PowerModule sees a power producer, on the same NodeId.
    let mut power = PowerModule::new();
    let net = power.create_network();
    power.add_producer(
        net,
        turbine,
        PowerProducer {
            capacity: Fixed64::from_num(0), // starts at 0, updated dynamically each tick
        },
    );

    for tick in 0..10u64 {
        engine.step();

        // After each engine step, read how much water the turbine has produced
        // in total. output_quantity returns accumulated output across all ticks.
        let water_out = test_utils::output_quantity(&engine, turbine, water);

        // Set power capacity proportional to total water output (100W per unit).
        // In a real game, you'd track the delta per tick for instantaneous power,
        // but for this test the pattern is the same: read recipe state, update power.
        power.set_producer_capacity(net, turbine, Fixed64::from_num(water_out * 100));

        power.tick(tick);
    }

    // Assert the recipe side: the turbine should have converted steam to water.
    let final_water = test_utils::output_quantity(&engine, turbine, water);
    assert!(
        final_water > 0,
        "turbine should have produced water via its FixedRecipe processor, got {final_water}"
    );

    // Assert the power side: satisfaction should reflect the dynamic capacity updates.
    // Since the turbine produced water and we set capacity = water_out * 100,
    // the producer capacity should be > 0, meaning satisfaction > 0.
    // With no consumers on the network, satisfaction defaults to 1.0 (fully satisfied).
    let satisfaction = power.satisfaction(net).expect("network should exist");
    assert!(
        satisfaction > Fixed64::from_num(0),
        "power satisfaction should be positive, got {satisfaction}"
    );

    // Verify the producer's capacity was actually updated to a non-zero value.
    let producer = power
        .producers
        .get(&turbine)
        .expect("turbine should be registered as producer");
    assert!(
        producer.capacity > Fixed64::from_num(0),
        "producer capacity should have been dynamically updated based on recipe output, got {}",
        producer.capacity
    );

    // The capacity should equal final_water * 100 (the last set_producer_capacity call).
    assert_eq!(
        producer.capacity,
        Fixed64::from_num(final_water * 100),
        "producer capacity should match water output * 100W"
    );
}
