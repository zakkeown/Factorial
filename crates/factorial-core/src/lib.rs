//! Factorial Core -- the simulation engine for factory-building games.
//!
//! This crate provides the foundational production graph, processors,
//! transport strategies, events, queries, serialization, and deterministic
//! fixed-point arithmetic that every Factorial game depends on.
//!
//! # Six-Phase Tick Pipeline
//!
//! Each call to [`engine::Engine::step`] advances the simulation by one tick
//! through the following phases:
//!
//! 1. **Pre-tick** -- Apply queued graph mutations (add/remove nodes and edges).
//! 2. **Transport** -- Move items along edges using the configured strategy.
//! 3. **Process** -- Buildings consume inputs and produce outputs via processors.
//! 4. **Component** -- Module-registered systems (power, fluid, etc.) run.
//! 5. **Post-tick** -- Deliver buffered events and collect deferred mutations.
//! 6. **Bookkeeping** -- Increment tick counter and compute the state hash.
//!
//! # Graph Mutation Pattern
//!
//! Graph changes are queued and then applied atomically, never immediate:
//!
//! ```rust,ignore
//! let pending = engine.graph.queue_add_node(BuildingTypeId(0));
//! let result = engine.graph.apply_mutations();
//! let node_id = result.resolve_node(pending).unwrap();
//! ```
//!
//! # Key Types
//!
//! - [`engine::Engine`] -- Main simulation engine and pipeline orchestrator.
//! - [`graph::ProductionGraph`] -- Directed graph of nodes (buildings) and
//!   edges (transport links) with topological ordering.
//! - [`processor::Processor`] -- Four processor types: Source, FixedRecipe,
//!   PropertyTransform, and Demand.
//! - [`transport::Transport`] -- Four transport strategies: Flow, Item,
//!   Batch, and Vehicle.
//! - [`fixed::Fixed64`] -- Q32.32 fixed-point type for deterministic math.
//! - [`registry::Registry`] -- Immutable registry of building types, recipes,
//!   and item types (frozen at startup).
//! - [`event::EventBus`] -- Subscription-based event bus with buffered delivery.
//! - [`serialize`] -- Versioned serialization and snapshot support via bitcode.

pub mod component;
#[cfg(feature = "data-loader")]
pub mod data_loader;
pub mod dirty;
pub mod engine;
pub mod event;
pub mod fixed;
pub mod graph;
pub mod id;
pub mod item;
pub mod junction;
pub mod migration;
pub mod module;
pub mod processor;
pub mod profiling;
pub mod query;
pub mod registry;
pub mod replay;
pub mod serialize;
pub mod sim;
pub mod transport;
pub mod validation;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
