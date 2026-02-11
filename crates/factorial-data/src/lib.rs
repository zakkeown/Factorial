//! Data-Driven Configuration for the Factorial engine.
//!
//! This crate loads game content definitions from external data files,
//! resolves cross-references by name, and produces a ready-to-use
//! [`GameData`] bundle containing a frozen [`factorial_core::registry::Registry`]
//! and optional module configurations.
//!
//! # Supported Formats
//!
//! Data files can be written in any of these formats:
//!
//! - **RON** (`.ron`) -- Rusty Object Notation, the recommended default.
//! - **JSON** (`.json`) -- Standard JSON.
//! - **TOML** (`.toml`) -- TOML with list keys for top-level arrays.
//!
//! Format is detected automatically by file extension. Only one format per
//! logical file is allowed (e.g., having both `items.ron` and `items.json`
//! in the same directory is an error).
//!
//! # Schema Structure
//!
//! The data directory must contain the following required files and may
//! include optional module files:
//!
//! | File           | Required | Contents                                    |
//! |----------------|----------|---------------------------------------------|
//! | `items.*`      | Yes      | [`schema::ItemData`] list -- item types     |
//! | `recipes.*`    | Yes      | [`schema::RecipeData`] list -- recipes       |
//! | `buildings.*`  | Yes      | [`schema::BuildingData`] list -- buildings   |
//! | `power.*`      | No       | Power generators, consumers, storage        |
//! | `fluids.*`     | No       | Fluid types, producers, consumers, storage  |
//! | `tech_tree.*`  | No       | Research nodes with costs and unlocks        |
//! | `logic.*`      | No       | Circuit controls and constant combinators   |
//!
//! # Resolution Pipeline
//!
//! [`load_game_data`] processes files in dependency order:
//!
//! 1. **Items** -- registered first, producing a name-to-[`factorial_core::id::ItemTypeId`] map.
//! 2. **Recipes** -- item names in inputs/outputs are resolved to IDs.
//! 3. **Buildings** -- recipe and item names are resolved; processors are constructed.
//! 4. **Modules** (optional) -- power, fluid, tech-tree, and logic configs
//!    resolve building/item/recipe names against the maps built above.
//!
//! Any unresolved name produces a [`DataLoadError::UnresolvedRef`].
//!
//! # Usage
//!
//! ```rust,ignore
//! use factorial_data::load_game_data;
//!
//! let game_data = load_game_data("assets/data")?;
//! let engine = Engine::new_with_registry(
//!     SimulationStrategy::Tick,
//!     game_data.registry,
//! );
//! ```

pub mod loader;
pub mod module_config;
pub mod schema;

pub use loader::{DataLoadError, GameData, load_game_data};
