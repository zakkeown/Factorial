//! Events and queries example: passive event listeners and the query API.
//!
//! Sets up a small factory (source -> processor), registers passive event
//! listeners, runs a few ticks, and demonstrates the snapshot/query APIs.
//!
//! Run with: `cargo run -p factorial-core --example events_and_queries`

use std::cell::RefCell;
use std::rc::Rc;

use factorial_core::engine::Engine;
use factorial_core::event::{Event, EventKind};
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::item::Inventory;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::*;

fn main() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // --- Build a small factory: Mine -> Assembler ---

    let p_mine = engine.graph.queue_add_node(BuildingTypeId(0));
    let p_assembler = engine.graph.queue_add_node(BuildingTypeId(1));
    let r = engine.graph.apply_mutations();
    let mine = r.resolve_node(p_mine).unwrap();
    let assembler = r.resolve_node(p_assembler).unwrap();

    let p_belt = engine.graph.queue_connect(mine, assembler);
    let r = engine.graph.apply_mutations();
    let belt = r.resolve_edge(p_belt).unwrap();

    // Mine: produces 3 iron ore per tick.
    engine.set_processor(
        mine,
        Processor::Source(SourceProcessor {
            output_type: ItemTypeId(0),
            base_rate: Fixed64::from_num(3),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0),
            initial_properties: None,
        }),
    );

    // Assembler: 2 iron ore -> 1 iron gear, takes 3 ticks.
    engine.set_processor(
        assembler,
        Processor::Fixed(FixedRecipe {
            inputs: vec![RecipeInput {
                item_type: ItemTypeId(0),
                quantity: 2,
            }],
            outputs: vec![RecipeOutput {
                item_type: ItemTypeId(1),
                quantity: 1,
            }],
            duration: 3,
        }),
    );

    // Inventories and transport.
    for node in [mine, assembler] {
        engine.set_input_inventory(node, Inventory::new(1, 1, 100));
        engine.set_output_inventory(node, Inventory::new(1, 1, 100));
    }

    engine.set_transport(
        belt,
        Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(5),
            buffer_capacity: Fixed64::from_num(100),
            latency: 0,
        }),
    );

    // --- Register passive event listeners ---

    // Track produced items via a shared counter.
    let produced_count = Rc::new(RefCell::new(0u32));
    let counter = produced_count.clone();
    engine.on_passive(
        EventKind::ItemProduced,
        Box::new(move |event| {
            if let Event::ItemProduced { quantity, .. } = event {
                *counter.borrow_mut() += quantity;
            }
        }),
    );

    // Track recipe completions.
    let recipe_completions = Rc::new(RefCell::new(0u32));
    let completions = recipe_completions.clone();
    engine.on_passive(
        EventKind::RecipeCompleted,
        Box::new(move |_event| {
            *completions.borrow_mut() += 1;
        }),
    );

    // --- Run simulation and use query APIs ---

    println!("Running events & queries demo...\n");

    for tick in 0..10 {
        engine.step();

        // Use snapshot_all_nodes() to query the full state.
        let snapshots = engine.snapshot_all_nodes();

        // Use get_processor_progress() for the assembler.
        let progress = engine
            .get_processor_progress(assembler)
            .unwrap_or(Fixed64::ZERO);

        println!("=== Tick {} ===", tick + 1);
        for snap in &snapshots {
            let name = match snap.building_type.0 {
                0 => "Mine",
                1 => "Assembler",
                _ => "Unknown",
            };
            println!(
                "  {}: state={:?}, progress={:.2}",
                name, snap.processor_state, snap.progress
            );
        }
        println!("  Assembler progress (raw query): {:.2}", progress);
        println!(
            "  Total produced: {}, Recipe completions: {}",
            produced_count.borrow(),
            recipe_completions.borrow()
        );
        println!();
    }

    // --- Final query: transport snapshot ---

    if let Some(tsnap) = engine.snapshot_transport(belt) {
        println!("Belt transport snapshot:");
        println!(
            "  utilization={:.2}, items_in_transit={}",
            tsnap.utilization, tsnap.items_in_transit
        );
    }

    println!("\nFinal state hash: {}", engine.state_hash());
}
