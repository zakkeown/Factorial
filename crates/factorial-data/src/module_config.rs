//! Module configuration structs with resolved IDs.
//!
//! These types hold the resolved (non-string) configuration for optional
//! engine modules: power, fluid, tech-tree, and logic. They are produced
//! by the loader after resolving name references against the registry.

/// Power module configuration (resolved).
pub struct PowerConfig;

/// Fluid module configuration (resolved).
pub struct FluidConfig;

/// Tech tree module configuration (resolved).
pub struct TechTreeConfig;

/// Logic / circuit network module configuration (resolved).
pub struct LogicConfig;
