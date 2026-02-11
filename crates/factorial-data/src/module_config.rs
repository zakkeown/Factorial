//! Module configuration structs with resolved IDs.
//!
//! These types hold the resolved (non-string) configuration for optional
//! engine modules: power, fluid, tech-tree, and logic. They are produced
//! by the loader after resolving name references against the registry.

use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_logic::WireColor;
use factorial_logic::condition::ComparisonOp;
use factorial_power::PowerPriority;
use factorial_tech_tree::{CostScaling, ResearchCost, Unlock};
use std::collections::HashMap;
use std::path::Path;

use crate::loader::{DataLoadError, deserialize_file, deserialize_list, resolve_name};
use crate::schema::*;

// ===========================================================================
// Resolved config types
// ===========================================================================

/// Power module configuration (resolved).
pub struct PowerConfig {
    pub generators: Vec<(BuildingTypeId, f64, PowerPriority)>,
    pub consumers: Vec<(BuildingTypeId, f64, PowerPriority)>,
    /// (id, capacity, charge_rate, discharge_rate)
    pub storage: Vec<(BuildingTypeId, f64, f64, f64)>,
}

/// Fluid module configuration (resolved).
pub struct FluidConfig {
    pub fluid_types: Vec<(String, ItemTypeId)>,
    pub producers: Vec<(BuildingTypeId, ItemTypeId, f64)>,
    pub consumers: Vec<(BuildingTypeId, ItemTypeId, f64)>,
    pub storage: Vec<(BuildingTypeId, ItemTypeId, f64, f64, f64)>,
}

/// Tech tree module configuration (resolved).
pub struct TechTreeConfig {
    pub technologies: Vec<ResolvedTech>,
}

/// A single resolved technology entry.
pub struct ResolvedTech {
    pub name: String,
    pub cost: ResearchCost,
    pub prerequisites: Vec<String>,
    pub unlocks: Vec<Unlock>,
    pub repeatable: bool,
    pub cost_scaling: Option<CostScaling>,
}

/// Logic / circuit network module configuration (resolved).
pub struct LogicConfig {
    pub circuit_controlled: Vec<(BuildingTypeId, WireColor, ItemTypeId, ComparisonOp, i64)>,
    pub constant_combinators: Vec<(BuildingTypeId, Vec<(ItemTypeId, i32)>)>,
}

// ===========================================================================
// Loading functions
// ===========================================================================

/// Load and resolve power module configuration.
pub(crate) fn load_power_config(
    path: &Path,
    building_names: &HashMap<String, BuildingTypeId>,
) -> Result<PowerConfig, DataLoadError> {
    let data: PowerData = deserialize_file(path)?;

    let generators = data
        .generators
        .iter()
        .map(|g| {
            let id = resolve_name(building_names, &g.building, path, "building")?;
            let priority = match g.priority {
                PriorityData::High => PowerPriority::High,
                PriorityData::Medium => PowerPriority::Medium,
                PriorityData::Low => PowerPriority::Low,
            };
            Ok((*id, g.output, priority))
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    let consumers = data
        .consumers
        .iter()
        .map(|c| {
            let id = resolve_name(building_names, &c.building, path, "building")?;
            let priority = match c.priority {
                PriorityData::High => PowerPriority::High,
                PriorityData::Medium => PowerPriority::Medium,
                PriorityData::Low => PowerPriority::Low,
            };
            Ok((*id, c.draw, priority))
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    let storage = data
        .storage
        .iter()
        .map(|s| {
            let id = resolve_name(building_names, &s.building, path, "building")?;
            Ok((*id, s.capacity, s.charge_rate, s.discharge_rate))
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    Ok(PowerConfig {
        generators,
        consumers,
        storage,
    })
}

/// Load and resolve fluid module configuration.
pub(crate) fn load_fluid_config(
    path: &Path,
    item_names: &HashMap<String, ItemTypeId>,
    building_names: &HashMap<String, BuildingTypeId>,
) -> Result<FluidConfig, DataLoadError> {
    let data: FluidData = deserialize_file(path)?;

    let fluid_types = data
        .types
        .iter()
        .map(|name| {
            let id = resolve_name(item_names, name, path, "item")?;
            Ok((name.clone(), *id))
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    let producers = data
        .producers
        .iter()
        .map(|p| {
            let bid = resolve_name(building_names, &p.building, path, "building")?;
            let fid = resolve_name(item_names, &p.fluid, path, "item")?;
            Ok((*bid, *fid, p.rate))
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    let consumers = data
        .consumers
        .iter()
        .map(|c| {
            let bid = resolve_name(building_names, &c.building, path, "building")?;
            let fid = resolve_name(item_names, &c.fluid, path, "item")?;
            Ok((*bid, *fid, c.rate))
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    let storage = data
        .storage
        .iter()
        .map(|s| {
            let bid = resolve_name(building_names, &s.building, path, "building")?;
            let fid = resolve_name(item_names, &s.fluid, path, "item")?;
            Ok((*bid, *fid, s.capacity, s.fill_rate, s.drain_rate))
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    Ok(FluidConfig {
        fluid_types,
        producers,
        consumers,
        storage,
    })
}

/// Load and resolve tech tree configuration.
pub(crate) fn load_tech_tree_config(
    path: &Path,
    item_names: &HashMap<String, ItemTypeId>,
    recipe_names: &HashMap<String, RecipeId>,
    building_names: &HashMap<String, BuildingTypeId>,
) -> Result<TechTreeConfig, DataLoadError> {
    let data: Vec<ResearchData> = deserialize_list(path, "research")?;

    let technologies = data
        .into_iter()
        .map(|tech| {
            let cost = resolve_research_cost(&tech.cost, item_names, path)?;
            let unlocks = tech
                .unlocks
                .iter()
                .map(|u| resolve_unlock(u, recipe_names, building_names, path))
                .collect::<Result<Vec<_>, DataLoadError>>()?;
            let cost_scaling = tech.cost_scaling.as_ref().map(resolve_cost_scaling);

            Ok(ResolvedTech {
                name: tech.name,
                cost,
                prerequisites: tech.prerequisites,
                unlocks,
                repeatable: tech.repeatable,
                cost_scaling,
            })
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    Ok(TechTreeConfig { technologies })
}

/// Resolve a `ResearchCostData` into a `ResearchCost`.
fn resolve_research_cost(
    data: &ResearchCostData,
    item_names: &HashMap<String, ItemTypeId>,
    file: &Path,
) -> Result<ResearchCost, DataLoadError> {
    match data {
        ResearchCostData::Points { amount } => Ok(ResearchCost::Points(*amount)),
        ResearchCostData::Items { items } => {
            let resolved = items
                .iter()
                .map(|(name, qty)| {
                    let id = resolve_name(item_names, name, file, "item")?;
                    Ok((*id, *qty))
                })
                .collect::<Result<Vec<_>, DataLoadError>>()?;
            Ok(ResearchCost::Items(resolved))
        }
        ResearchCostData::Delivery { items } => {
            let resolved = items
                .iter()
                .map(|(name, qty)| {
                    let id = resolve_name(item_names, name, file, "item")?;
                    Ok((*id, *qty))
                })
                .collect::<Result<Vec<_>, DataLoadError>>()?;
            Ok(ResearchCost::Delivery(resolved))
        }
        ResearchCostData::Rate {
            points_per_tick,
            total,
        } => Ok(ResearchCost::Rate {
            points_per_tick: Fixed64::from_num(*points_per_tick),
            total: Fixed64::from_num(*total),
        }),
    }
}

/// Resolve an `UnlockData` into an `Unlock`.
fn resolve_unlock(
    data: &UnlockData,
    recipe_names: &HashMap<String, RecipeId>,
    building_names: &HashMap<String, BuildingTypeId>,
    file: &Path,
) -> Result<Unlock, DataLoadError> {
    match data {
        UnlockData::Building(name) => {
            let id = resolve_name(building_names, name, file, "building")?;
            Ok(Unlock::Building(*id))
        }
        UnlockData::Recipe(name) => {
            let id = resolve_name(recipe_names, name, file, "recipe")?;
            Ok(Unlock::Recipe(*id))
        }
        UnlockData::Custom(key) => Ok(Unlock::Custom(key.clone())),
    }
}

/// Resolve a `CostScalingData` into a `CostScaling`.
fn resolve_cost_scaling(data: &CostScalingData) -> CostScaling {
    match data {
        CostScalingData::Linear { base, increment } => CostScaling::Linear {
            base: *base,
            increment: *increment,
        },
        CostScalingData::Exponential { base, multiplier } => CostScaling::Exponential {
            base: *base,
            multiplier: Fixed64::from_num(*multiplier),
        },
    }
}

/// Load and resolve logic module configuration.
pub(crate) fn load_logic_config(
    path: &Path,
    item_names: &HashMap<String, ItemTypeId>,
    building_names: &HashMap<String, BuildingTypeId>,
) -> Result<LogicConfig, DataLoadError> {
    let data: LogicData = deserialize_file(path)?;

    let circuit_controlled = data
        .circuit_controlled
        .iter()
        .map(|cc| {
            let bid = resolve_name(building_names, &cc.building, path, "building")?;
            let wire = match cc.wire {
                WireColorData::Red => WireColor::Red,
                WireColorData::Green => WireColor::Green,
            };
            let signal_id = resolve_name(item_names, &cc.condition.signal, path, "item")?;
            let op = parse_comparison_op(&cc.condition.op, path)?;
            Ok((*bid, wire, *signal_id, op, cc.condition.value))
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    let constant_combinators = data
        .constant_combinators
        .iter()
        .map(|cc| {
            let bid = resolve_name(building_names, &cc.building, path, "building")?;
            let signals = cc
                .signals
                .iter()
                .map(|(name, value)| {
                    let id = resolve_name(item_names, name, path, "item")?;
                    Ok((*id, *value))
                })
                .collect::<Result<Vec<_>, DataLoadError>>()?;
            Ok((*bid, signals))
        })
        .collect::<Result<Vec<_>, DataLoadError>>()?;

    Ok(LogicConfig {
        circuit_controlled,
        constant_combinators,
    })
}

/// Parse a comparison operator string into a `ComparisonOp`.
fn parse_comparison_op(op: &str, file: &Path) -> Result<ComparisonOp, DataLoadError> {
    match op {
        "gt" | ">" => Ok(ComparisonOp::Gt),
        "lt" | "<" => Ok(ComparisonOp::Lt),
        "eq" | "==" => Ok(ComparisonOp::Eq),
        "gte" | ">=" => Ok(ComparisonOp::Gte),
        "lte" | "<=" => Ok(ComparisonOp::Lte),
        "ne" | "!=" => Ok(ComparisonOp::Ne),
        _ => Err(DataLoadError::Parse {
            file: file.to_path_buf(),
            detail: format!("unknown comparison operator: '{op}'"),
        }),
    }
}
