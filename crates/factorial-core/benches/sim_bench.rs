//! Criterion benchmarks for the Factorial simulation engine.
//!
//! Three benchmark groups:
//! - `small_factory`: 200 nodes, 500 edges, FlowTransport -- target <2ms/tick
//! - `medium_factory`: 5000 nodes, 10000 edges, mixed transport -- target <5ms/tick
//! - `belt_heavy`: 1000 ItemTransport belts with 50 slots each -- measure belt throughput

use criterion::{criterion_group, criterion_main, Criterion};
use factorial_core::engine::Engine;
use factorial_core::id::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;

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
    for pair in chains.windows(2) {
        for (&src, &dst) in pair[0][1..].iter().zip(pair[1][1..].iter()) {
            connect(
                &mut engine,
                src,
                dst,
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
                _ => make_batch_transport(10, 5),
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

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    group.sample_size(30);

    let engine = build_medium_factory();

    // Full (legacy) serialize.
    group.bench_function("serialize_full_5000_nodes", |b| {
        b.iter(|| {
            engine.serialize().unwrap();
        });
    });

    // Partitioned serialize.
    group.bench_function("serialize_partitioned_5000_nodes", |b| {
        b.iter(|| {
            engine.serialize_partitioned().unwrap();
        });
    });

    // Incremental with no baseline (all partitions freshly serialized).
    group.bench_function("serialize_incremental_no_baseline", |b| {
        b.iter_batched(
            || {
                let mut e = build_medium_factory();
                // Step to dirty all partitions, then serialize.
                e.step();
                e
            },
            |mut e| {
                e.serialize_incremental(None).unwrap();
            },
            criterion::BatchSize::LargeInput,
        );
    });

    // Incremental with baseline (only partitions changed by 1 step are dirty).
    group.bench_function("serialize_incremental_with_baseline", |b| {
        b.iter_batched(
            || {
                let mut e = build_medium_factory();
                e.step();
                let baseline = e.serialize_incremental(None).unwrap();
                e.step();
                (e, baseline)
            },
            |(mut e, baseline)| {
                e.serialize_incremental(Some(&baseline)).unwrap();
            },
            criterion::BatchSize::LargeInput,
        );
    });

    // Legacy deserialize.
    let legacy_data = engine.serialize().unwrap();
    group.bench_function("deserialize_legacy_5000_nodes", |b| {
        b.iter(|| {
            Engine::deserialize(&legacy_data).unwrap();
        });
    });

    // Partitioned deserialize.
    let partitioned_data = engine.serialize_partitioned().unwrap();
    group.bench_function("deserialize_partitioned_5000_nodes", |b| {
        b.iter(|| {
            Engine::deserialize_partitioned(&partitioned_data).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_small_factory,
    bench_medium_factory,
    bench_belt_heavy,
    bench_serialization
);
criterion_main!(benches);
