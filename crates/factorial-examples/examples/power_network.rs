//! Power network example: production, consumption, storage, and brownout.
//!
//! Creates an engine with several nodes, sets up a power network with
//! producers, consumers (at different priorities), and a storage node.
//! Demonstrates normal operation, brownout detection, and recovery.
//!
//! Run with: `cargo run -p factorial-examples --example power_network`

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::BuildingTypeId;
use factorial_core::sim::SimulationStrategy;
use factorial_power::*;

fn main() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Create nodes in the production graph to represent buildings.
    // These NodeIds are used to register buildings in the power module.
    let p0 = engine.graph.queue_add_node(BuildingTypeId(0)); // power plant
    let p1 = engine.graph.queue_add_node(BuildingTypeId(1)); // assembler (high priority)
    let p2 = engine.graph.queue_add_node(BuildingTypeId(2)); // smelter (medium priority)
    let p3 = engine.graph.queue_add_node(BuildingTypeId(3)); // lamp (low priority)
    let p4 = engine.graph.queue_add_node(BuildingTypeId(4)); // battery
    let r = engine.graph.apply_mutations();

    let power_plant = r.resolve_node(p0).unwrap();
    let assembler = r.resolve_node(p1).unwrap();
    let smelter = r.resolve_node(p2).unwrap();
    let lamp = r.resolve_node(p3).unwrap();
    let battery = r.resolve_node(p4).unwrap();

    // --- Create a power module and network ---

    let mut power = PowerModule::new();
    let net = power.create_network();

    // Add a producer: 100W power plant.
    power.add_producer(
        net,
        power_plant,
        PowerProducer {
            capacity: Fixed64::from_num(100),
        },
    );

    // Add consumers with different priorities.
    power.add_consumer_with_priority(
        net,
        assembler,
        PowerConsumer {
            demand: Fixed64::from_num(40),
        },
        PowerPriority::High,
    );
    power.add_consumer_with_priority(
        net,
        smelter,
        PowerConsumer {
            demand: Fixed64::from_num(40),
        },
        PowerPriority::Medium,
    );
    power.add_consumer_with_priority(
        net,
        lamp,
        PowerConsumer {
            demand: Fixed64::from_num(40),
        },
        PowerPriority::Low,
    );

    // Add a battery with 200J capacity, 50W charge/discharge rate.
    power.add_storage(
        net,
        battery,
        PowerStorage {
            capacity: Fixed64::from_num(200),
            charge: Fixed64::from_num(100),
            charge_rate: Fixed64::from_num(50),
        },
    );

    // --- Scenario 1: Demand slightly exceeds production ---
    // Total demand: 120W, production: 100W, deficit: 20W.
    // Battery covers the 20W deficit.

    println!("=== Scenario 1: Battery covers small deficit ===\n");

    for tick in 1..=5 {
        let events = power.tick(tick);
        let satisfaction = power.satisfaction(net).unwrap();
        let charge = power.storage.get(&battery).unwrap().charge;

        println!(
            "Tick {}: satisfaction={:.2}, battery charge={:.1}",
            tick, satisfaction, charge
        );
        for event in &events {
            println!("  Event: {:?}", event);
        }
    }

    // --- Scenario 2: Brownout (demand >> production) ---
    // Reduce production to trigger brownout.

    println!("\n=== Scenario 2: Brownout (reduced production) ===\n");

    power.set_producer_capacity(net, power_plant, Fixed64::from_num(30));
    println!("Reduced power plant to 30W.");
    println!("Total demand: 120W, production: 30W.\n");

    for tick in 6..=12 {
        let events = power.tick(tick);
        let satisfaction = power.satisfaction(net).unwrap();
        let charge = power.storage.get(&battery).unwrap().charge;

        // Check per-consumer satisfaction.
        let asm_sat = power
            .get_consumer_satisfaction(net, assembler)
            .unwrap_or(Fixed64::ZERO);
        let smelt_sat = power
            .get_consumer_satisfaction(net, smelter)
            .unwrap_or(Fixed64::ZERO);
        let lamp_sat = power
            .get_consumer_satisfaction(net, lamp)
            .unwrap_or(Fixed64::ZERO);

        println!(
            "Tick {}: network={:.2}, asm={:.2}, smelt={:.2}, lamp={:.2}, battery={:.1}",
            tick, satisfaction, asm_sat, smelt_sat, lamp_sat, charge
        );
        for event in &events {
            println!("  Event: {:?}", event);
        }
    }

    // --- Scenario 3: Recovery ---

    println!("\n=== Scenario 3: Recovery (production restored) ===\n");

    power.set_producer_capacity(net, power_plant, Fixed64::from_num(150));
    println!("Restored power plant to 150W (excess charges battery).\n");

    for tick in 13..=18 {
        let events = power.tick(tick);
        let satisfaction = power.satisfaction(net).unwrap();
        let charge = power.storage.get(&battery).unwrap().charge;

        println!(
            "Tick {}: satisfaction={:.2}, battery charge={:.1}",
            tick, satisfaction, charge
        );
        for event in &events {
            println!("  Event: {:?}", event);
        }
    }

    println!("\nPower network demo complete.");
}
