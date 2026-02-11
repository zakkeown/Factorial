# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Conventional Commits](https://www.conventionalcommits.org/).

## [Unreleased]

### Added
- `factorial-data` crate for data-driven configuration via RON/JSON/TOML files
- Core loading pipeline: items, recipes, buildings with name resolution
- Module config loading: power, fluid, tech-tree, logic
- Format detection, file discovery, and cross-reference resolution
- Full integration test: load data files, build engine, run ticks

## Data-Driven Configuration

### Added
- Scaffold `factorial-data` crate with schema, loader, and module config modules
- `GameData` struct aggregating registry, footprints, processors, inventories, and module configs
- `DataLoadError` with variants for missing files, unresolved references, duplicate names, parse errors
- RON, JSON, and TOML deserialization with automatic format detection
- Resolution pipeline: items -> recipes -> buildings -> optional modules
- Power, fluid, tech-tree, and logic module config loading
- JSON fixtures and format equivalence tests
- Error path tests for all error variants

## Performance Optimizations

### Changed
- Cache topological order in graph for faster tick evaluation
- Feedback-aware ordering for graphs with cycles
- 8 tick-pipeline optimizations reducing allocations and hashing cost
- Lazy incremental serialization: only re-encode dirty partitions

### Added
- Expanded benchmarks: large factory, chain, fanout, and hash scenarios
- Fuzz targets, property tests, adversarial and stress tests

## WASM Bindings

### Added
- `factorial-wasm` crate with integer-handle-based API for WebAssembly
- Engine lifecycle exports: create, destroy, step, advance
- Graph mutation exports: add_node, connect, apply_mutations
- Query exports: node_count, tick, state_hash, inventories
- Processor and transport configuration exports
- Serialization exports: serialize/deserialize via bitcode
- Event polling export with `FlatEvent` repr(C) struct
- Logic network exports for WASM consumers
- WASM memory allocator exports (alloc/free)
- CI workflow for WASM build check

## Logic Networks

### Added
- `factorial-logic` crate for wire-based signal networks
- Core types: `WireNetworkId`, `WireColor`, `SignalSet`, `WireNetwork`
- `ArithmeticCombinator` and `DeciderCombinator` with signal selectors
- `CircuitControl` for conditional building enable/disable
- `InventoryReader` for reading node inventories as signals
- `ConstantCombinator` for fixed signal output
- One-tick delay on combinator outputs to prevent infinite feedback
- Signal merge per network each tick
- Logic events: CircuitActivated, CircuitDeactivated, NetworkSignalsChanged
- Serde round-trip tests
- Engine module system integration and FFI bindings

## Documentation & CI

### Added
- mdBook documentation structure with SUMMARY and placeholder pages
- Introduction: What is Factorial, responsibilities matrix, glossary
- Architecture deep dive: performance model, memory layout, design decisions
- All seven core concepts pages
- All five framework module pages
- FFI conventions, function reference, and language bindings pages
- All ten cookbook recipes with compile-tested example crates
- Rust and C/FFI quick start guides, WASM quick start stub
- README overhaul as documentation landing page
- CI workflows: test/clippy/fmt, coverage gate (80%), WASM build check
- Weekly mutation testing workflow for Tier 1 crates
- Documentation build and deploy workflow
- Coverage baseline documentation
- CLAUDE.md with project conventions

### Changed
- Mutation-testing targeted tests for Tier 1 survivors

## Robustness & Testing

### Added
- Snapshot query tests for `NodeSnapshot` and `TransportSnapshot`
- Deserialization error path tests for all error variants
- Error path tests for graph, registry, and module error variants
- Coverage tests for sim, component, and id modules
- Engine edge case tests for delta mode, pause, and accessors
- Adversarial FFI tests for poisoned state, null pointers, and edge cases

## Incremental Serialization & Blueprints

### Added
- Partitioned incremental serialization
- Blueprint/ghost placement system with collision detection
- Blueprint enhancements, benchmarking, and hardening

## Engine Feature Expansion

### Added
- Per-output-type edge routing with `item_filter` on `EdgeData`
- Feedback loop support with one-tick delay
- `Passthrough` processor variant for junction nodes
- Junction runtime item routing via edge budgets
- Fair fan-out distribution for multi-output edges
- `swap_processor` for dynamic recipe selection
- Item property tracking on `ItemStack`
- Power priority system and dynamic power production
- Demand rate tracking and query API
- Multi-type acceptance for `DemandProcessor`
- `FluidBridge` for fluid-to-item conversion
- Dual-role node pattern (recipe + power) demonstration

### Fixed
- Splitter round-robin test assertion
- Code review findings across multiple modules

## Integration Testing

### Added
- Cross-crate integration test crate with Builderment factory builder
- Builderment item type constants in test_utils
- End-to-end computer and super computer production tests
- Basic robot parallel chain test
- Serialization round-trip test for full factory
- Determinism test for full factory
- Power brownout and recovery test
- Tech tree progression test
- Stats tracking and bottleneck detection test

## Framework Modules

### Added
- `factorial-stats` crate for production statistics tracking
- `factorial-tech-tree` crate with prerequisites and cost models
- `factorial-power` crate with satisfaction balancing and brownouts
- `factorial-spatial` crate with grid placement, collision, and blueprints
- `factorial-fluid` crate with pipe-based fluid simulation

## Core Engine

### Added
- Production graph with topological sort and queued mutations
- Transport strategies: Flow, Item, Batch, Vehicle
- Processor system: Source, FixedRecipe, PropertyTransform, Demand
- Simulation engine with tick and delta strategies
- Event system with typed ring buffers and reactive handlers
- Read-only query API for simulation state inspection
- Serialization and snapshots with bitcode encoding
- C FFI layer with cbindgen header generation
- Profiling, migration framework support
- Data loader (opt-in feature on core)
- Replay and validation modules
- Dirty tracking, pause/resume, module registration

### Fixed
- `BuildingResumed` event emission ordering
- Needless late init clippy warning in transport
- HashMap replaced with BTreeMap in power module for determinism
- Distinguish `FutureVersion` from `UnsupportedVersion` in serialization

## Initial Scaffold

### Added
- Workspace setup with Cargo.toml and shared dependencies
- Q32.32 fixed-point type aliases and arithmetic (`Fixed64`)
- Type-safe ID types via slotmap `newtype!` macro
- Registry with builder pattern
- Item storage with fungible inventory slots
- SoA component storage with SecondaryMap
- FFI crate scaffold with cbindgen config
