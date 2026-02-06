//! Transport showcase: all four transport strategies side by side.
//!
//! Creates four parallel chains from identical source patterns, each using
//! a different transport strategy (Flow, Item, Batch, Vehicle). Runs 20
//! ticks and compares how many items each strategy delivered.
//!
//! Run with: `cargo run -p factorial-core --example transport_showcase`

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::item::Inventory;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::*;

fn main() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // --- Create 8 nodes: 4 sources and 4 sinks ---

    let p_src_flow = engine.graph.queue_add_node(BuildingTypeId(0));
    let p_src_item = engine.graph.queue_add_node(BuildingTypeId(0));
    let p_src_batch = engine.graph.queue_add_node(BuildingTypeId(0));
    let p_src_vehicle = engine.graph.queue_add_node(BuildingTypeId(0));

    let p_sink_flow = engine.graph.queue_add_node(BuildingTypeId(1));
    let p_sink_item = engine.graph.queue_add_node(BuildingTypeId(1));
    let p_sink_batch = engine.graph.queue_add_node(BuildingTypeId(1));
    let p_sink_vehicle = engine.graph.queue_add_node(BuildingTypeId(1));

    let r = engine.graph.apply_mutations();

    let src_flow = r.resolve_node(p_src_flow).unwrap();
    let src_item = r.resolve_node(p_src_item).unwrap();
    let src_batch = r.resolve_node(p_src_batch).unwrap();
    let src_vehicle = r.resolve_node(p_src_vehicle).unwrap();

    let sink_flow = r.resolve_node(p_sink_flow).unwrap();
    let sink_item = r.resolve_node(p_sink_item).unwrap();
    let sink_batch = r.resolve_node(p_sink_batch).unwrap();
    let sink_vehicle = r.resolve_node(p_sink_vehicle).unwrap();

    // --- Connect each source to its corresponding sink ---

    let p_edge_flow = engine.graph.queue_connect(src_flow, sink_flow);
    let p_edge_item = engine.graph.queue_connect(src_item, sink_item);
    let p_edge_batch = engine.graph.queue_connect(src_batch, sink_batch);
    let p_edge_vehicle = engine.graph.queue_connect(src_vehicle, sink_vehicle);

    let r = engine.graph.apply_mutations();

    let edge_flow = r.resolve_edge(p_edge_flow).unwrap();
    let edge_item = r.resolve_edge(p_edge_item).unwrap();
    let edge_batch = r.resolve_edge(p_edge_batch).unwrap();
    let edge_vehicle = r.resolve_edge(p_edge_vehicle).unwrap();

    // --- Configure all sources identically: 5 items/tick ---

    let sources = [src_flow, src_item, src_batch, src_vehicle];
    let sinks = [sink_flow, sink_item, sink_batch, sink_vehicle];

    for &src in &sources {
        engine.set_processor(
            src,
            Processor::Source(SourceProcessor {
                output_type: ItemTypeId(0),
                base_rate: Fixed64::from_num(5),
                depletion: Depletion::Infinite,
                accumulated: Fixed64::from_num(0),
                initial_properties: None,
            }),
        );
    }

    // Sinks consume items (demand processor).
    for &sink in &sinks {
        engine.set_processor(
            sink,
            Processor::Demand(DemandProcessor {
                input_type: ItemTypeId(0),
                base_rate: Fixed64::from_num(10),
                accumulated: Fixed64::from_num(0),
                consumed_total: 0,
                accepted_types: None,
            }),
        );
    }

    // --- Set up inventories ---

    for &node in sources.iter().chain(sinks.iter()) {
        engine.set_input_inventory(node, Inventory::new(1, 1, 200));
        engine.set_output_inventory(node, Inventory::new(1, 1, 200));
    }

    // --- Configure four different transport strategies ---

    // 1. Flow: continuous rate-based, 5 items/tick, no latency.
    engine.set_transport(
        edge_flow,
        Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(5),
            buffer_capacity: Fixed64::from_num(100),
            latency: 0,
        }),
    );

    // 2. Item (belt): discrete slots, speed 1, 5 slots, 1 lane.
    engine.set_transport(
        edge_item,
        Transport::Item(ItemTransport {
            speed: Fixed64::from_num(1),
            slot_count: 5,
            lanes: 1,
        }),
    );

    // 3. Batch: delivers 10 items every 5 ticks.
    engine.set_transport(
        edge_batch,
        Transport::Batch(BatchTransport {
            batch_size: 10,
            cycle_time: 5,
        }),
    );

    // 4. Vehicle: capacity 20, travel time 3 ticks (6-tick round trip).
    engine.set_transport(
        edge_vehicle,
        Transport::Vehicle(VehicleTransport {
            capacity: 20,
            travel_time: 3,
        }),
    );

    // --- Run 20 ticks ---

    println!("Running 20 ticks with four transport strategies...\n");

    let labels = ["Flow", "Item (Belt)", "Batch", "Vehicle"];
    let edges = [edge_flow, edge_item, edge_batch, edge_vehicle];

    for tick in 0..20 {
        engine.step();

        if (tick + 1) % 5 == 0 {
            println!("=== After tick {} ===", tick + 1);
            for (i, &edge) in edges.iter().enumerate() {
                let snap = engine.snapshot_transport(edge);
                if let Some(snap) = snap {
                    println!(
                        "  {:12}: utilization={:.2}, items_in_transit={}",
                        labels[i], snap.utilization, snap.items_in_transit
                    );
                }
            }
            println!();
        }
    }

    // --- Summary ---

    println!("=== Final Summary ===");
    for (i, &sink) in sinks.iter().enumerate() {
        let snap = engine.snapshot_node(sink);
        if let Some(snap) = snap {
            println!(
                "  {:12}: inputs={:?}, outputs={:?}",
                labels[i], snap.input_contents, snap.output_contents
            );
        }
    }

    println!("\nState hash: {}", engine.state_hash());
}
