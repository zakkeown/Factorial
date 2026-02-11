# Factorial

A headless factory game engine written in Rust.

## What is Factorial?

Factorial is a simulation library that models interconnected production networks -- the kind found in factory-building and automation games. It handles production graphs, transport logistics, power networks, fluid systems, tech trees, spatial placement, serialization, and cross-platform determinism out of the box. Game developers plug in their own UI, rendering, audio, and game-specific mechanics on top.

## Quick Example

A minimal factory: an iron mine feeding an assembler via a transport belt.

```rust
use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::processor::*;
use factorial_core::transport::*;
use factorial_core::sim::SimulationStrategy;

fn main() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Add two nodes and connect them.
    let pending_mine = engine.graph.queue_add_node(BuildingTypeId(0));
    let pending_asm = engine.graph.queue_add_node(BuildingTypeId(1));
    let result = engine.graph.apply_mutations();
    let mine = result.resolve_node(pending_mine).unwrap();
    let asm = result.resolve_node(pending_asm).unwrap();

    let pending_belt = engine.graph.queue_connect(mine, asm);
    let belt = engine.graph.apply_mutations()
        .resolve_edge(pending_belt).unwrap();

    // Configure processors and transport.
    engine.set_processor(mine, Processor::Source(SourceProcessor {
        output_type: ItemTypeId(0),
        base_rate: Fixed64::from_num(2),
        depletion: Depletion::Infinite,
        accumulated: Fixed64::from_num(0),
        initial_properties: None,
    }));
    engine.set_transport(belt, Transport::Flow(FlowTransport {
        rate: Fixed64::from_num(5),
        buffer_capacity: Fixed64::from_num(100),
        latency: 0,
    }));

    // Run 10 ticks.
    for _ in 0..10 {
        engine.step();
    }
    println!("State hash: {}", engine.state_hash());
}
```

See `crates/factorial-core/examples/minimal_factory.rs` for the full version.

## Documentation

Full documentation is available in the [docs/book/src/](docs/book/src/) directory.

Key sections:

- [What is Factorial?](docs/book/src/introduction/what-is-factorial.md)
- [Core Concepts](docs/book/src/core-concepts/production-graph.md) -- production graph, processors, transport, events, queries, determinism, serialization
- [Framework Modules](docs/book/src/modules/power.md) -- power, fluid, tech tree, spatial, statistics, logic networks
- [Data Loading](docs/book/src/data/data-driven.md) -- data-driven configuration via RON/JSON/TOML
- [FFI Reference](docs/book/src/ffi/conventions.md) -- API conventions, function reference, language bindings
- [WASM Bindings](docs/book/src/wasm/bindings.md) -- WebAssembly API for browser and sandboxed environments
- [Cookbook](docs/book/src/cookbook/smelting-chain.md) -- step-by-step recipes for common tasks
- [Architecture Deep Dive](docs/book/src/architecture/performance.md) -- performance model, memory layout, design decisions

## Features

- **Production graph with topological evaluation** -- directed graph of nodes (buildings) and edges (transport links), evaluated in dependency order each tick.
- **4 transport strategies** -- Flow, Item, Batch, and Vehicle, each with distinct throughput and latency characteristics.
- **6 framework modules (opt-in)** -- Power networks, fluid simulation, tech trees, spatial grid with blueprints, statistics tracking, and logic/circuit networks.
- **Data-driven configuration** -- define items, recipes, buildings, and module configs in RON, JSON, or TOML files.
- **C FFI for embedding in any language** -- stable C API with opaque handles, suitable for Unity, Godot, or any engine with C interop.
- **WASM bindings** -- integer-handle-based API for browser games and sandboxed plugin environments.
- **Cross-platform determinism via fixed-point arithmetic** -- identical simulation results on every platform, every run.
- **Serialization with versioning and migration** -- save/load game state with forward-compatible schema evolution.
- **Multiplayer-ready via state hashing** -- detect desync between clients by comparing per-tick hashes.

## Getting Started

- [Rust Quick Start](docs/book/src/getting-started/rust.md) -- add the crate, build a graph, run your first tick.
- [C/FFI Quick Start](docs/book/src/getting-started/ffi.md) -- link the shared library, call from C, C++, C#, or Swift.
- [WASM Quick Start](docs/book/src/getting-started/wasm.md) -- compile to WebAssembly, run in the browser.

## Architecture

```
┌─────────────────────────────────────────┐
│         Your Game Code                  │
│  (UI, rendering, audio, game logic)     │
├─────────────────────────────────────────┤
│      Framework Modules (opt-in)         │
│  Power · Fluid · Tech Tree · Spatial ·  │
│  Statistics · Logic                     │
├─────────────────────────────────────────┤
│           Factorial Core                │
│  Production Graph · Processors ·        │
│  Transport · Events · Queries ·         │
│  Serialization · Determinism            │
└─────────────────────────────────────────┘
```

## License

MIT
