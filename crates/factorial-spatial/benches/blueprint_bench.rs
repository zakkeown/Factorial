//! Criterion benchmarks for the blueprint system.

use criterion::{Criterion, criterion_group, criterion_main};
use factorial_core::engine::Engine;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use factorial_spatial::{
    Blueprint, BlueprintEntry, BlueprintNodeRef, BuildingFootprint, GridPosition, Rotation,
    SpatialIndex,
};

fn make_bench_entry(x: i32, y: i32) -> BlueprintEntry {
    BlueprintEntry {
        building_type: building(),
        position: GridPosition::new(x, y),
        footprint: BuildingFootprint::single(),
        rotation: Rotation::None,
        processor: make_source(iron(), 1.0),
        input_capacity: 100,
        output_capacity: 100,
    }
}

fn bench_blueprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("blueprint");
    group.sample_size(50);

    // Benchmark: add 100 entries in a 10x10 grid.
    group.bench_function("add_100_entries", |b| {
        b.iter(|| {
            let spatial = SpatialIndex::new();
            let mut bp = Blueprint::new();
            for y in 0..10 {
                for x in 0..10 {
                    bp.add(make_bench_entry(x * 2, y * 2), &spatial).unwrap();
                }
            }
        });
    });

    // Benchmark: commit 50 nodes with 49 connections.
    group.bench_function("commit_50_nodes_49_connections", |b| {
        b.iter(|| {
            let mut engine = Engine::new(SimulationStrategy::Tick);
            let mut spatial = SpatialIndex::new();
            let mut bp = Blueprint::new();

            let mut ids = Vec::with_capacity(50);
            for i in 0..50 {
                let id = bp.add(make_bench_entry(i * 2, 0), &spatial).unwrap();
                ids.push(id);
            }
            for i in 0..49 {
                bp.connect(
                    BlueprintNodeRef::Planned(ids[i]),
                    BlueprintNodeRef::Planned(ids[i + 1]),
                    make_flow_transport(5.0),
                    None,
                );
            }
            bp.commit(&mut engine, &mut spatial).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_blueprint);
criterion_main!(benches);
