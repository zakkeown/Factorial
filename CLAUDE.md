# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Factorial is a headless factory game engine written in Rust. It simulates interconnected production networks (factory-building games like Factorio/Satisfactory). The engine is designed to be embedded into any game engine via Rust API, C FFI, or WASM. It provides no UI/rendering — game developers plug those in.

## Build & Development Commands

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test --package factorial-core

# Run a single test by name
cargo test --package factorial-core -- test_name

# Lint (CI runs with -D warnings)
cargo clippy --workspace --all-targets -- -D warnings

# Format check / auto-format
cargo fmt --all -- --check
cargo fmt --all

# Benchmarks
cargo bench --package factorial-core

# Run an example
cargo run --package factorial-core --example minimal_factory

# Coverage (requires cargo-llvm-cov)
cargo llvm-cov --package factorial-core --package factorial-ffi --fail-under-lines 80 --text
```

CI enforces: all tests pass, clippy with `-D warnings` (plus `RUSTFLAGS=-Dwarnings`), `cargo fmt` check, and 80% line coverage on factorial-core and factorial-ffi.

## Architecture

**Three-layer design:**
- **Factorial Core** (`factorial-core`) — Production graph, processors, transport, events, queries, serialization, determinism
- **Framework Modules** (opt-in, independent of each other) — `factorial-power`, `factorial-fluid`, `factorial-tech-tree`, `factorial-spatial`, `factorial-stats`, `factorial-logic`
- **Data Loading** — `factorial-data` (data-driven configuration via RON/JSON/TOML files)
- **Integration** — `factorial-ffi` (C FFI via cbindgen), `factorial-wasm` (WebAssembly bindings), `factorial-examples`, `factorial-integration-tests`

### Workspace Layout

All crates live under `crates/`. The workspace is defined in the root `Cargo.toml` with shared dependencies (serde, bitcode, fixed, slotmap, thiserror).

### Core Engine (`factorial-core`)

The engine runs a **six-phase tick pipeline** (see `engine.rs`):
1. **Pre-tick** — Apply queued graph mutations
2. **Transport** — Move items along edges
3. **Process** — Buildings consume inputs, produce outputs
4. **Component** — Module-registered systems run
5. **Post-tick** — Deliver buffered events, collect mutations
6. **Bookkeeping** — Update tick counter, compute state hash

Key source files in `factorial-core/src/`:
- `engine.rs` — Main simulation engine, pipeline orchestrator
- `graph.rs` — Production graph (nodes=buildings, edges=transport links), topological ordering, mutation queuing
- `processor.rs` — Four processor types: Source, FixedRecipe, PropertyTransform, Demand
- `transport.rs` — Four transport strategies: Flow, Item, Batch, Vehicle
- `event.rs` — Subscription-based event bus with buffered delivery
- `serialize.rs` — Serialization with versioning, snapshots (uses bitcode, not JSON)
- `fixed.rs` — Q32.32 fixed-point arithmetic wrapper (`Fixed64 = i64`)
- `id.rs` — Type-safe IDs via slotmap `newtype!` macro (`NodeId`, `EdgeId`, `ItemTypeId`, etc.)
- `registry.rs` — Immutable registry of building types, recipes, item types (frozen at startup)
- `test_utils.rs` — Test helpers (behind `#[cfg(any(test, feature = "test-utils"))]`)

### Logic Networks (`factorial-logic`)

Wire-based signal networks enabling Factorio-style combinators and circuit control. Key source files in `factorial-logic/src/`:
- `lib.rs` — `LogicModule` (main API), `WireNetwork`, `SignalSet`, tick pipeline, signal merge
- `combinator.rs` — `ArithmeticCombinator`, `DeciderCombinator`, signal selectors
- `condition.rs` — `Condition`, `ComparisonOp`, `CircuitControl`, `InventoryReader`
- `bridge.rs` — Engine module integration bridge

### Data Loading (`factorial-data`)

Data-driven game configuration via external files (RON, JSON, or TOML). Key source files in `factorial-data/src/`:
- `schema.rs` — Data structs: `ItemData`, `RecipeData`, `BuildingData`, plus module-specific schemas (power, fluid, tech tree, logic)
- `loader.rs` — `load_game_data(dir)` resolution pipeline: file discovery, format detection, name-to-ID resolution
- `module_config.rs` — Resolved config types: `PowerConfig`, `FluidConfig`, `TechTreeConfig`, `LogicConfig`

### WASM Bindings (`factorial-wasm`)

Integer-handle-based API for embedding Factorial in WebAssembly or any C-compatible host. Key source files in `factorial-wasm/src/`:
- `lib.rs` — Handle table (max 16 engines), result codes, `FlatEvent` repr(C), memory allocator
- `engine.rs` — create, destroy, step, advance exports
- `graph.rs` — add_node, connect, remove node/edge exports
- `processor.rs` / `transport.rs` — Configuration exports
- `query.rs` — State inspection exports
- `serialize.rs` — Bitcode serialize/deserialize exports
- `logic.rs` — Logic network exports for WASM consumers

### Graph Mutation Pattern

Graph changes are **queued then applied**, not immediate:
```rust
let pending = engine.graph.queue_add_node(BuildingTypeId(0));
let result = engine.graph.apply_mutations();
let node_id = result.resolve_node(pending).unwrap();
```

## Critical Design Constraints

**Fixed-point arithmetic everywhere in simulation.** All simulation math uses Q32.32 fixed-point (`Fixed64`), never floats. This is non-negotiable — it guarantees cross-platform determinism for multiplayer lockstep. Use `Fixed64::from_num()` and `.to_f64()` for conversion.

**Struct-of-Arrays (SoA) storage.** Node/edge state uses SlotMap + SecondaryMap for cache-friendly iteration. Follow this pattern when adding new per-node or per-edge state.

**No panics in simulation code.** Use `Result<T, E>` with explicit thiserror types. Validation happens upstream.

**Registry is immutable after startup.** Build the registry once, freeze it. Tech tree unlocks gate access but don't mutate the registry.

**Snapshot versioning.** Serialized state includes version numbers. Always include forward/backward migration logic when changing serialization format.

## Testing Conventions

- Unit tests go in `#[cfg(test)] mod tests` at the bottom of each source file
- Integration tests in `crates/factorial-core/tests/`
- Examples in `crates/factorial-core/examples/` double as documentation
- Use `test_utils.rs` helpers for common setup
- Coverage gate: 80% minimum on core and FFI crates
- Mutation testing runs weekly on factorial-core and factorial-ffi

## Rust Edition

Uses **Rust 2024 edition**.
