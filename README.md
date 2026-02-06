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

Full documentation is available at [Factorial Documentation](https://user.github.io/factorial/).

Key sections:

- [What is Factorial?](https://user.github.io/factorial/introduction/what-is-factorial.html)
- [Core Concepts](https://user.github.io/factorial/core-concepts/production-graph.html) -- production graph, processors, transport, events, queries, determinism, serialization
- [Framework Modules](https://user.github.io/factorial/modules/power.html) -- power, fluid, tech tree, spatial, statistics
- [FFI Reference](https://user.github.io/factorial/ffi/conventions.html) -- API conventions, function reference, language bindings
- [Cookbook](https://user.github.io/factorial/cookbook/smelting-chain.html) -- step-by-step recipes for common tasks
- [Architecture Deep Dive](https://user.github.io/factorial/architecture/performance.html) -- performance model, memory layout, design decisions

## Features

- **Production graph with topological evaluation** -- directed graph of nodes (buildings) and edges (transport links), evaluated in dependency order each tick.
- **4 transport strategies** -- Flow, Item, Batch, and Vehicle, each with distinct throughput and latency characteristics.
- **5 framework modules (opt-in)** -- Power networks, fluid simulation, tech trees, spatial grid with blueprints, and statistics tracking.
- **C FFI for embedding in any language** -- stable C API with opaque handles, suitable for Unity, Godot, or any engine with C interop.
- **Cross-platform determinism via fixed-point arithmetic** -- identical simulation results on every platform, every run.
- **Serialization with versioning and migration** -- save/load game state with forward-compatible schema evolution.
- **Multiplayer-ready via state hashing** -- detect desync between clients by comparing per-tick hashes.

## Getting Started

- [Rust Quick Start](https://user.github.io/factorial/getting-started/rust.html) -- add the crate, build a graph, run your first tick.
- [C/FFI Quick Start](https://user.github.io/factorial/getting-started/ffi.html) -- link the shared library, call from C, C++, C#, or Swift.
- [WASM Quick Start](https://user.github.io/factorial/getting-started/wasm.html) -- compile to WebAssembly, run in the browser.

## Architecture

```
┌─────────────────────────────────────────┐
│         Your Game Code                  │
│  (UI, rendering, audio, game logic)     │
├─────────────────────────────────────────┤
│      Framework Modules (opt-in)         │
│  Power · Fluid · Tech Tree · Spatial ·  │
│  Statistics                             │
├─────────────────────────────────────────┤
│           Factorial Core                │
│  Production Graph · Processors ·        │
│  Transport · Events · Queries ·         │
│  Serialization · Determinism            │
└─────────────────────────────────────────┘
```

## License

MIT
