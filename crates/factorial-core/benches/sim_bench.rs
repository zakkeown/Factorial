//! Criterion benchmarks for the Factorial simulation engine.
//!
//! Three benchmark groups:
//! - `small_factory`: 200 nodes, 500 edges, FlowTransport -- target <2ms/tick
//! - `medium_factory`: 5000 nodes, 10000 edges, mixed transport -- target <5ms/tick
//! - `belt_heavy`: 1000 ItemTransport belts with 50 slots each -- measure belt throughput

use criterion::{criterion_group, criterion_main, Criterion};
use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::item::Inventory;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::*;

// ===========================================================================
// Helpers
// ===========================================================================

fn fixed(v: f64) -> Fixed64 {
    Fixed64::from_num(v)
}

fn iron() -> ItemTypeId {
    ItemTypeId(0)
}
fn gear() -> ItemTypeId {
    ItemTypeId(2)
}

fn building() -> BuildingTypeId {
    BuildingTypeId(0)
}

fn simple_inventory(capacity: u32) -> Inventory {
    Inventory::new(1, 1, capacity)
}

fn make_source(item: ItemTypeId, rate: f64) -> Processor {
    Processor::Source(SourceProcessor {
        output_type: item,
        base_rate: fixed(rate),
        depletion: Depletion::Infinite,
        accumulated: fixed(0.0),
    })
}

fn make_recipe(
    inputs: Vec<(ItemTypeId, u32)>,
    outputs: Vec<(ItemTypeId, u32)>,
    duration: u32,
) -> Processor {
    Processor::Fixed(FixedRecipe {
        inputs: inputs
            .into_iter()
            .map(|(item_type, quantity)| RecipeInput {
                item_type,
                quantity,
            })
            .collect(),
        outputs: outputs
            .into_iter()
            .map(|(item_type, quantity)| RecipeOutput {
                item_type,
                quantity,
            })
            .collect(),
        duration,
    })
}

fn make_flow_transport(rate: f64) -> Transport {
    Transport::Flow(FlowTransport {
        rate: fixed(rate),
        buffer_capacity: fixed(1000.0),
        latency: 0,
    })
}

fn make_item_transport(slot_count: u32) -> Transport {
    Transport::Item(ItemTransport {
        speed: fixed(1.0),
        slot_count,
        lanes: 1,
    })
}

fn make_batch_transport() -> Transport {
    Transport::Batch(BatchTransport {
        batch_size: 10,
        cycle_time: 5,
    })
}

/// Add a node to the engine with the given processor and inventories.
fn add_node(
    engine: &mut Engine,
    processor: Processor,
    input_capacity: u32,
    output_capacity: u32,
) -> NodeId {
    let pending = engine.graph.queue_add_node(building());
    let result = engine.graph.apply_mutations();
    let node = result.resolve_node(pending).unwrap();

    engine.set_processor(node, processor);
    engine.set_input_inventory(node, simple_inventory(input_capacity));
    engine.set_output_inventory(node, simple_inventory(output_capacity));

    node
}

/// Connect two nodes with the given transport.
fn connect(engine: &mut Engine, from: NodeId, to: NodeId, transport: Transport) -> EdgeId {
    let pending = engine.graph.queue_connect(from, to);
    let result = engine.graph.apply_mutations();
    let edge = result.resolve_edge(pending).unwrap();
    engine.set_transport(edge, transport);
    edge
}

// ===========================================================================
// Factory builders
// ===========================================================================

/// Build a small factory: 200 nodes, ~500 edges, all FlowTransport.
///
/// Structure: 40 source nodes, each feeding a chain of 4 assemblers.
/// Each chain: Source -> A1 -> A2 -> A3 -> A4 (Sink).
/// That gives 40 * 5 = 200 nodes and 40 * 4 = 160 edges.
/// To reach ~500 edges, we add cross-links between adjacent chains.
fn build_small_factory() -> Engine {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    let chain_count = 40;
    let chain_length = 5; // 1 source + 4 assemblers per chain
    let mut chains: Vec<Vec<NodeId>> = Vec::with_capacity(chain_count);

    for _ in 0..chain_count {
        let mut chain = Vec::with_capacity(chain_length);

        // Source node.
        let source = add_node(&mut engine, make_source(iron(), 3.0), 100, 100);
        chain.push(source);

        // Chain of assemblers.
        for _ in 1..chain_length {
            let assembler = add_node(
                &mut engine,
                make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
                100,
                100,
            );
            chain.push(assembler);
        }

        // Connect chain linearly.
        for i in 0..chain.len() - 1 {
            connect(
                &mut engine,
                chain[i],
                chain[i + 1],
                make_flow_transport(10.0),
            );
        }

        chains.push(chain);
    }

    // Add cross-links between adjacent chains to reach ~500 edges.
    // Connect each chain's output to the next chain's last assembler.
    for i in 0..chains.len() - 1 {
        for depth in 1..chain_length {
            connect(
                &mut engine,
                chains[i][depth],
                chains[i + 1][depth],
                make_flow_transport(5.0),
            );
        }
    }

    // Warm up the factory for a few ticks so state is populated.
    for _ in 0..5 {
        engine.step();
    }

    engine
}

/// Build a medium factory: 5000 nodes, ~10000 edges, mixed transport types.
///
/// Structure: 250 chains of 20 nodes each. Each chain uses alternating
/// transport types. Cross-links add more edges.
fn build_medium_factory() -> Engine {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    let chain_count = 250;
    let chain_length = 20;
    let mut chains: Vec<Vec<NodeId>> = Vec::with_capacity(chain_count);

    for _ in 0..chain_count {
        let mut chain = Vec::with_capacity(chain_length);

        // Source node.
        let source = add_node(&mut engine, make_source(iron(), 2.0), 100, 100);
        chain.push(source);

        // Chain of assemblers.
        for _ in 1..chain_length {
            let assembler = add_node(
                &mut engine,
                make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
                100,
                100,
            );
            chain.push(assembler);
        }

        // Connect chain linearly with alternating transports.
        for i in 0..chain.len() - 1 {
            let transport = match i % 3 {
                0 => make_flow_transport(10.0),
                1 => make_item_transport(10),
                _ => make_batch_transport(),
            };
            connect(&mut engine, chain[i], chain[i + 1], transport);
        }

        chains.push(chain);
    }

    // Add cross-links between adjacent chains.
    // Each adjacent pair gets ~20 cross-links at various depths.
    for i in 0..chains.len() - 1 {
        // Connect every other depth to avoid too many.
        for depth in (1..chain_length).step_by(2) {
            connect(
                &mut engine,
                chains[i][depth],
                chains[i + 1][depth],
                make_flow_transport(5.0),
            );
        }
    }

    // Warm up.
    for _ in 0..3 {
        engine.step();
    }

    engine
}

/// Build a belt-heavy factory: 1000 ItemTransport belts with 50 slots each.
///
/// Structure: 500 source-sink pairs, each connected by 2 belts (one forward,
/// one cross-link to the next pair).
fn build_belt_heavy_factory() -> Engine {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    let pair_count = 500;
    let mut sources: Vec<NodeId> = Vec::with_capacity(pair_count);
    let mut sinks: Vec<NodeId> = Vec::with_capacity(pair_count);

    for _ in 0..pair_count {
        let source = add_node(&mut engine, make_source(iron(), 2.0), 100, 100);
        let sink = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 999)], vec![(gear(), 1)], 9999),
            200,
            100,
        );

        // Primary belt: 50 slots.
        connect(
            &mut engine,
            source,
            sink,
            make_item_transport(50),
        );

        sources.push(source);
        sinks.push(sink);
    }

    // Cross-link belts between adjacent pairs (another 499 belts).
    for i in 0..pair_count - 1 {
        connect(
            &mut engine,
            sources[i],
            sinks[i + 1],
            make_item_transport(50),
        );
    }

    // Add one more belt to reach 1000.
    if pair_count > 1 {
        connect(
            &mut engine,
            sources[pair_count - 1],
            sinks[0],
            make_item_transport(50),
        );
    }

    // Warm up.
    for _ in 0..3 {
        engine.step();
    }

    engine
}

// ===========================================================================
// Benchmarks
// ===========================================================================

fn bench_small_factory(c: &mut Criterion) {
    let mut group = c.benchmark_group("small_factory");
    group.sample_size(50);

    let mut engine = build_small_factory();

    group.bench_function("200_nodes_500_edges_flow", |b| {
        b.iter(|| {
            engine.step();
        });
    });

    group.finish();
}

fn bench_medium_factory(c: &mut Criterion) {
    let mut group = c.benchmark_group("medium_factory");
    group.sample_size(20);

    let mut engine = build_medium_factory();

    group.bench_function("5000_nodes_10000_edges_mixed", |b| {
        b.iter(|| {
            engine.step();
        });
    });

    group.finish();
}

fn bench_belt_heavy(c: &mut Criterion) {
    let mut group = c.benchmark_group("belt_heavy");
    group.sample_size(30);

    let mut engine = build_belt_heavy_factory();

    group.bench_function("1000_belts_50_slots", |b| {
        b.iter(|| {
            engine.step();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_small_factory, bench_medium_factory, bench_belt_heavy);
criterion_main!(benches);
