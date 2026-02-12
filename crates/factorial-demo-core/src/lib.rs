//! Demo showcase core for the Factorial engine.
//!
//! Provides a scene-based demo system that loads curated factory vignettes
//! from data files, builds engine instances, and exposes a query API for
//! renderers (Bevy, Godot, Web/WASM) to visualize.
//!
//! # Usage
//!
//! ```rust,ignore
//! use factorial_demo_core::scene_manager::SceneManager;
//!
//! let mut mgr = SceneManager::new("scenes/")?;
//! mgr.load_scene("solo_extractor")?;
//! mgr.tick()?;
//! let nodes = mgr.snapshot_all_nodes()?;
//! ```

pub mod error;
pub mod manifest;
pub mod scene_builder;
pub mod scene_manager;
pub mod scene_schema;

pub use error::DemoError;
pub use manifest::{SceneEntry, SceneManifest, TierDef};
pub use scene_builder::{ActiveScene, EdgeMeta, NodeMeta, build_scene};
pub use scene_manager::SceneManager;
pub use scene_schema::SceneData;
