//! Fluid network example: production, consumption, storage, and pressure.
//!
//! Creates a fluid network carrying water, with a pump (producer), a
//! boiler (consumer), a storage tank, and a pipe. Demonstrates normal
//! operation and low-pressure scenarios.
//!
//! Run with: `cargo run -p factorial-examples --example fluid_network`

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::{BuildingTypeId, ItemTypeId};
use factorial_core::sim::SimulationStrategy;
use factorial_fluid::*;

fn main() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Create nodes to represent buildings in the fluid network.
    let p_pump = engine.graph.queue_add_node(BuildingTypeId(0));
    let p_boiler = engine.graph.queue_add_node(BuildingTypeId(1));
    let p_tank = engine.graph.queue_add_node(BuildingTypeId(2));
    let p_pipe = engine.graph.queue_add_node(BuildingTypeId(3));
    let r = engine.graph.apply_mutations();

    let pump = r.resolve_node(p_pump).unwrap();
    let boiler = r.resolve_node(p_boiler).unwrap();
    let tank = r.resolve_node(p_tank).unwrap();
    let pipe = r.resolve_node(p_pipe).unwrap();

    // --- Create a fluid module and water network ---

    let water_type = ItemTypeId(100); // water fluid type
    let mut fluid = FluidModule::new();
    let net = fluid.create_network(water_type);

    // Pump: produces 50 units of water per tick.
    fluid.add_producer(net, pump, FluidProducer {
        rate: Fixed64::from_num(50),
    });

    // Boiler: consumes 30 units of water per tick.
    fluid.add_consumer(net, boiler, FluidConsumer {
        rate: Fixed64::from_num(30),
    });

    // Storage tank: holds up to 500 units, fill/drain rate 100/tick.
    fluid.add_storage(net, tank, FluidStorage {
        capacity: Fixed64::from_num(500),
        current: Fixed64::from_num(0),
        fill_rate: Fixed64::from_num(100),
    });

    // Pipe: capacity 200 units throughput.
    fluid.add_pipe(net, pipe, FluidPipe {
        capacity: Fixed64::from_num(200),
    });

    // --- Scenario 1: Normal operation (production > consumption) ---

    println!("=== Scenario 1: Normal operation (pump: 50, boiler: 30) ===\n");

    for tick in 1..=8 {
        let events = fluid.tick(tick);
        let pressure = fluid.pressure(net).unwrap();
        let tank_level = fluid.storage.get(&tank).unwrap().current;
        let consumed = fluid.get_consumed_this_tick(net, boiler);

        println!(
            "Tick {}: pressure={:.2}, tank={:.1}, boiler consumed={:.1}",
            tick, pressure, tank_level, consumed
        );
        for event in &events {
            println!("  Event: {:?}", event);
        }
    }

    // --- Scenario 2: Low pressure (increase consumption) ---

    println!("\n=== Scenario 2: Low pressure (add second boiler) ===\n");

    // Add a second, larger consumer to create a deficit.
    let p_boiler2 = engine.graph.queue_add_node(BuildingTypeId(5));
    let r = engine.graph.apply_mutations();
    let boiler2 = r.resolve_node(p_boiler2).unwrap();

    fluid.add_consumer(net, boiler2, FluidConsumer {
        rate: Fixed64::from_num(80),
    });

    println!("Added second boiler consuming 80/tick.");
    println!("Total demand: 110/tick, production: 50/tick.\n");

    for tick in 9..=20 {
        let events = fluid.tick(tick);
        let pressure = fluid.pressure(net).unwrap();
        let tank_level = fluid.storage.get(&tank).unwrap().current;

        println!(
            "Tick {}: pressure={:.2}, tank={:.1}",
            tick, pressure, tank_level
        );
        for event in &events {
            println!("  Event: {:?}", event);
        }
    }

    // --- Scenario 3: Restored pressure ---

    println!("\n=== Scenario 3: Restore pressure (remove second boiler) ===\n");

    fluid.remove_node(boiler2);
    println!("Removed second boiler. Demand back to 30/tick.\n");

    for tick in 21..=26 {
        let events = fluid.tick(tick);
        let pressure = fluid.pressure(net).unwrap();
        let tank_level = fluid.storage.get(&tank).unwrap().current;

        println!(
            "Tick {}: pressure={:.2}, tank={:.1}",
            tick, pressure, tank_level
        );
        for event in &events {
            println!("  Event: {:?}", event);
        }
    }

    println!("\nFluid network demo complete.");
}
