//! Tech Tree Module for the Factorial factory game engine.
//!
//! Provides research systems with prerequisites, multiple cost models,
//! unlock tracking, and infinite (repeatable) research with cost scaling.
//!
//! # Overview
//!
//! Technologies are registered at startup via [`TechTree::register`]. Each
//! [`Technology`] has prerequisites, a [`ResearchCost`], a list of [`Unlock`]s,
//! and an optional `repeatable` flag for infinite research.
//!
//! At runtime, game code drives research by calling [`TechTree::start_research`]
//! and [`TechTree::contribute_items`] / [`TechTree::contribute_points`] /
//! [`TechTree::tick_rate`] depending on the cost model. When a technology is
//! completed, the tech tree emits a [`TechEvent::ResearchCompleted`] event and
//! records which [`Unlock`]s should be applied.
//!
//! # Cost Models
//!
//! The module supports six cost models matching real factory games:
//!
//! - **Items** (Factorio/DSP): consume specific items
//! - **Points** (ONI): accumulate science points
//! - **Delivery** (Satisfactory): one-time delivery of items
//! - **Rate** (Captain of Industry): points per tick over time
//! - **ItemRate** (Shapez): deliver items at a target rate
//! - **Custom**: game-defined completion logic via callback ID

use factorial_core::fixed::{Fixed64, Ticks};
use factorial_core::id::{BuildingTypeId, ItemTypeId, RecipeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

/// Identifies a technology in the tech tree. Cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TechId(pub u32);

/// Identifies a custom research cost function registered by game code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResearchCostFnId(pub u32);

// ---------------------------------------------------------------------------
// Research cost models
// ---------------------------------------------------------------------------

/// How a technology's research cost is paid. Each variant corresponds to
/// a different factory game's research mechanic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchCost {
    /// Factorio/DSP: consume specific items at research buildings.
    /// Each entry is (item_type, required_quantity).
    Items(Vec<(ItemTypeId, u32)>),

    /// ONI: spend accumulated science points.
    Points(u32),

    /// Satisfactory: one-time delivery of specific items.
    /// Identical structure to Items but with different completion semantics
    /// (all items must be delivered at once, not consumed over time).
    Delivery(Vec<(ItemTypeId, u32)>),

    /// Captain of Industry: accumulate points at a fixed rate per tick.
    /// `points_per_tick` is the rate, `total` is the target to reach.
    Rate {
        points_per_tick: Fixed64,
        total: Fixed64,
    },

    /// Shapez: deliver items at a target rate for a duration.
    ItemRate {
        item: ItemTypeId,
        rate: Fixed64,
        duration: Ticks,
    },

    /// Game-defined completion logic. The game registers a callback ID
    /// and handles completion checks externally.
    Custom(ResearchCostFnId),
}

// ---------------------------------------------------------------------------
// Unlocks
// ---------------------------------------------------------------------------

/// What completing a technology unlocks. Game code listens for
/// `ResearchCompleted` events and applies the unlocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unlock {
    /// Unlocks a building type for placement.
    Building(BuildingTypeId),

    /// Unlocks a recipe for use in processors.
    Recipe(RecipeId),

    /// Game-defined unlock. The string key is opaque to the engine;
    /// game code interprets it.
    Custom(String),
}

// ---------------------------------------------------------------------------
// Cost scaling (infinite research)
// ---------------------------------------------------------------------------

/// How the cost of a repeatable technology scales with each completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CostScaling {
    /// Cost increases linearly: base + increment * level.
    Linear { base: u32, increment: u32 },

    /// Cost increases exponentially: base * multiplier^level.
    /// `multiplier` is stored as a Fixed64 ratio (e.g. 1.5 means 50% increase).
    Exponential { base: u32, multiplier: Fixed64 },
}

impl CostScaling {
    /// Compute the scaled cost for the given completion level (0-indexed).
    /// Level 0 means the first completion, level 1 means the second, etc.
    pub fn cost_at_level(&self, level: u32) -> u32 {
        match self {
            CostScaling::Linear { base, increment } => {
                base.saturating_add(increment.saturating_mul(level))
            }
            CostScaling::Exponential { base, multiplier } => {
                let mut cost = Fixed64::from_num(*base);
                for _ in 0..level {
                    cost = cost.saturating_mul(*multiplier);
                }
                // Clamp to u32 range.
                let result: i64 = cost.to_num();
                if result < 0 {
                    0
                } else if result > u32::MAX as i64 {
                    u32::MAX
                } else {
                    result as u32
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Technology definition
// ---------------------------------------------------------------------------

/// A technology that can be researched. Registered at startup; immutable
/// after registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Technology {
    /// Unique identifier.
    pub id: TechId,

    /// Human-readable name.
    pub name: String,

    /// Technologies that must be completed before this one can start.
    pub prerequisites: Vec<TechId>,

    /// How the research cost is paid.
    pub cost: ResearchCost,

    /// What completing this technology unlocks.
    pub unlocks: Vec<Unlock>,

    /// Whether this technology can be researched multiple times (infinite research).
    pub repeatable: bool,

    /// Cost scaling for repeatable technologies. Ignored if `repeatable` is false.
    pub cost_scaling: Option<CostScaling>,
}

// ---------------------------------------------------------------------------
// Research state (runtime)
// ---------------------------------------------------------------------------

/// The current state of research for a single technology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchState {
    /// Research has not been started.
    NotStarted,

    /// Research is in progress. Progress tracking depends on the cost model.
    InProgress(ResearchProgress),

    /// Research has been completed. For repeatable techs, stores the
    /// number of times completed.
    Completed {
        /// Number of times this technology has been completed.
        /// Always >= 1. For non-repeatable techs, always 1.
        times_completed: u32,
    },
}

/// Progress tracking for in-progress research.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchProgress {
    /// For Items cost model: tracks how many of each item have been contributed.
    Items(Vec<(ItemTypeId, u32)>),

    /// For Points cost model: tracks accumulated points.
    Points(u32),

    /// For Delivery cost model: tracks delivered items (all-or-nothing).
    Delivery(Vec<(ItemTypeId, u32)>),

    /// For Rate cost model: tracks accumulated points (as Fixed64).
    Rate(Fixed64),

    /// For ItemRate cost model: tracks ticks elapsed.
    ItemRate(Ticks),

    /// For Custom cost model: game manages progress externally.
    Custom,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events emitted by the tech tree module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TechEvent {
    /// A technology has started being researched.
    ResearchStarted { tech_id: TechId, tick: Ticks },

    /// A technology has been completed.
    ResearchCompleted {
        tech_id: TechId,
        unlocks: Vec<Unlock>,
        /// For repeatable techs, the level just completed (1-indexed).
        level: u32,
        tick: Ticks,
    },
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during tech tree operations.
#[derive(Debug, thiserror::Error)]
pub enum TechTreeError {
    #[error("technology not found: {0:?}")]
    TechNotFound(TechId),

    #[error("prerequisite not met: {0:?} requires {1:?}")]
    PrerequisiteNotMet(TechId, TechId),

    #[error("technology {0:?} is already being researched")]
    AlreadyInProgress(TechId),

    #[error("technology {0:?} is already completed and not repeatable")]
    AlreadyCompleted(TechId),

    #[error("duplicate technology id: {0:?}")]
    DuplicateId(TechId),

    #[error("prerequisite {prereq:?} for technology {tech:?} does not exist")]
    InvalidPrerequisite { tech: TechId, prereq: TechId },

    #[error("wrong cost model for technology {0:?}: expected {1}")]
    WrongCostModel(TechId, &'static str),
}

// ---------------------------------------------------------------------------
// TechTree — the main module struct
// ---------------------------------------------------------------------------

/// The tech tree module. Holds technology definitions and runtime research state.
///
/// Technologies are registered at startup. Research state is tracked at runtime
/// and is fully serializable for save/load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechTree {
    /// Registered technologies, keyed by TechId.
    technologies: HashMap<TechId, Technology>,

    /// Runtime research state for each technology.
    states: HashMap<TechId, ResearchState>,

    /// Completion counts for repeatable technologies. Persisted independently
    /// of the current `ResearchState` so that starting a new round of
    /// repeatable research does not lose the count.
    completions: HashMap<TechId, u32>,

    /// Next auto-assigned TechId (used by `register`).
    next_id: u32,

    /// Events emitted since last drain. Not serialized (transient).
    #[serde(skip)]
    events: Vec<TechEvent>,
}

impl TechTree {
    /// Create a new, empty tech tree.
    pub fn new() -> Self {
        Self {
            technologies: HashMap::new(),
            states: HashMap::new(),
            completions: HashMap::new(),
            next_id: 0,
            events: Vec::new(),
        }
    }

    // -- Registration API --

    /// Register a technology. The `id` field in the Technology must be unique.
    /// Returns an error if the id is already registered or if any prerequisite
    /// references a non-existent technology.
    pub fn register(&mut self, tech: Technology) -> Result<TechId, TechTreeError> {
        let id = tech.id;

        if self.technologies.contains_key(&id) {
            return Err(TechTreeError::DuplicateId(id));
        }

        // Validate prerequisites exist.
        for prereq in &tech.prerequisites {
            if !self.technologies.contains_key(prereq) {
                return Err(TechTreeError::InvalidPrerequisite {
                    tech: id,
                    prereq: *prereq,
                });
            }
        }

        self.technologies.insert(id, tech);
        self.states.insert(id, ResearchState::NotStarted);

        // Track next_id to stay above any manually assigned IDs.
        if id.0 >= self.next_id {
            self.next_id = id.0 + 1;
        }

        Ok(id)
    }

    /// Allocate the next available TechId. Useful for auto-assigning IDs.
    pub fn next_tech_id(&mut self) -> TechId {
        let id = TechId(self.next_id);
        self.next_id += 1;
        id
    }

    // -- Query API --

    /// Get a technology definition by ID.
    pub fn get_technology(&self, id: TechId) -> Option<&Technology> {
        self.technologies.get(&id)
    }

    /// Get the current research state for a technology.
    pub fn get_state(&self, id: TechId) -> Option<&ResearchState> {
        self.states.get(&id)
    }

    /// Check whether all prerequisites for a technology are completed.
    pub fn prerequisites_met(&self, id: TechId) -> Result<bool, TechTreeError> {
        let tech = self
            .technologies
            .get(&id)
            .ok_or(TechTreeError::TechNotFound(id))?;

        for prereq in &tech.prerequisites {
            match self.states.get(prereq) {
                Some(ResearchState::Completed { .. }) => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    /// Check whether a technology has been completed (at least once).
    pub fn is_completed(&self, id: TechId) -> bool {
        self.completion_count(id) > 0
    }

    /// Check whether a technology is currently being researched.
    pub fn is_in_progress(&self, id: TechId) -> bool {
        matches!(self.states.get(&id), Some(ResearchState::InProgress(_)))
    }

    /// Get the number of times a repeatable technology has been completed.
    /// Returns 0 if never completed.
    pub fn completion_count(&self, id: TechId) -> u32 {
        self.completions.get(&id).copied().unwrap_or(0)
    }

    /// Get all unlocks that have been earned (from completed technologies).
    pub fn all_unlocks(&self) -> Vec<Unlock> {
        let mut unlocks = Vec::new();
        for (id, state) in &self.states {
            if let ResearchState::Completed { .. } = state
                && let Some(tech) = self.technologies.get(id)
            {
                unlocks.extend(tech.unlocks.iter().cloned());
            }
        }
        unlocks
    }

    /// Get the number of registered technologies.
    pub fn technology_count(&self) -> usize {
        self.technologies.len()
    }

    /// Get the effective cost for a technology, accounting for cost scaling
    /// on repeatable techs. For non-repeatable techs, returns the base cost.
    pub fn effective_cost(&self, id: TechId) -> Result<ResearchCost, TechTreeError> {
        let tech = self
            .technologies
            .get(&id)
            .ok_or(TechTreeError::TechNotFound(id))?;

        if !tech.repeatable {
            return Ok(tech.cost.clone());
        }

        let level = self.completion_count(id);
        let Some(scaling) = &tech.cost_scaling else {
            return Ok(tech.cost.clone());
        };

        // Apply scaling to the cost.
        let scaled = scale_cost(&tech.cost, scaling, level);
        Ok(scaled)
    }

    // -- Research actions --

    /// Start researching a technology. Validates prerequisites and state.
    /// Emits `ResearchStarted` on success.
    pub fn start_research(&mut self, id: TechId, tick: Ticks) -> Result<(), TechTreeError> {
        let tech = self
            .technologies
            .get(&id)
            .ok_or(TechTreeError::TechNotFound(id))?;

        // Check prerequisites.
        for prereq in &tech.prerequisites {
            match self.states.get(prereq) {
                Some(ResearchState::Completed { .. }) => {}
                _ => return Err(TechTreeError::PrerequisiteNotMet(id, *prereq)),
            }
        }

        // Check current state.
        match self.states.get(&id) {
            Some(ResearchState::InProgress(_)) => {
                return Err(TechTreeError::AlreadyInProgress(id));
            }
            Some(ResearchState::Completed { .. }) if !tech.repeatable => {
                return Err(TechTreeError::AlreadyCompleted(id));
            }
            _ => {}
        }

        // Initialize progress based on cost model.
        let progress = match &tech.cost {
            ResearchCost::Items(items) => {
                ResearchProgress::Items(items.iter().map(|(item, _)| (*item, 0)).collect())
            }
            ResearchCost::Points(_) => ResearchProgress::Points(0),
            ResearchCost::Delivery(items) => {
                ResearchProgress::Delivery(items.iter().map(|(item, _)| (*item, 0)).collect())
            }
            ResearchCost::Rate { .. } => ResearchProgress::Rate(Fixed64::ZERO),
            ResearchCost::ItemRate { .. } => ResearchProgress::ItemRate(0),
            ResearchCost::Custom(_) => ResearchProgress::Custom,
        };

        self.states.insert(id, ResearchState::InProgress(progress));
        self.events
            .push(TechEvent::ResearchStarted { tech_id: id, tick });

        Ok(())
    }

    /// Contribute items toward an Items-cost or Delivery-cost research.
    /// Returns the amount of each item actually consumed (may be less than
    /// offered if research needs fewer). Completes research if all items met.
    pub fn contribute_items(
        &mut self,
        id: TechId,
        contributions: &[(ItemTypeId, u32)],
        tick: Ticks,
    ) -> Result<Vec<(ItemTypeId, u32)>, TechTreeError> {
        let tech = self
            .technologies
            .get(&id)
            .ok_or(TechTreeError::TechNotFound(id))?
            .clone();

        let effective_cost = self.effective_cost(id)?;

        let state = self
            .states
            .get_mut(&id)
            .ok_or(TechTreeError::TechNotFound(id))?;

        let (progress_items, required) = match (state, &effective_cost) {
            (
                ResearchState::InProgress(ResearchProgress::Items(progress)),
                ResearchCost::Items(required),
            ) => (progress, required),
            (
                ResearchState::InProgress(ResearchProgress::Delivery(progress)),
                ResearchCost::Delivery(required),
            ) => (progress, required),
            _ => return Err(TechTreeError::WrongCostModel(id, "Items or Delivery")),
        };

        let mut consumed = Vec::new();

        for (item_type, amount) in contributions {
            // Find this item in the required list.
            let required_amount = required
                .iter()
                .find(|(req_item, _)| req_item == item_type)
                .map(|(_, qty)| *qty)
                .unwrap_or(0);

            // Find current progress for this item.
            let current = progress_items.iter_mut().find(|(pi, _)| pi == item_type);

            if let Some((_, current_qty)) = current {
                let remaining = required_amount.saturating_sub(*current_qty);
                let to_consume = (*amount).min(remaining);
                *current_qty += to_consume;
                consumed.push((*item_type, to_consume));
            }
        }

        // Check if research is complete.
        // Re-borrow state after mutation.
        let state = self.states.get(&id).unwrap();
        let is_complete = if let ResearchState::InProgress(progress) = state {
            match (progress, &effective_cost) {
                (ResearchProgress::Items(p), ResearchCost::Items(r))
                | (ResearchProgress::Delivery(p), ResearchCost::Delivery(r)) => p
                    .iter()
                    .zip(r.iter())
                    .all(|((_, have), (_, need))| have >= need),
                _ => false,
            }
        } else {
            false
        };

        if is_complete {
            self.complete_research(id, &tech, tick);
        }

        Ok(consumed)
    }

    /// Contribute science points toward a Points-cost research.
    /// Returns the number of points actually consumed. Completes research
    /// when the target is met.
    pub fn contribute_points(
        &mut self,
        id: TechId,
        points: u32,
        tick: Ticks,
    ) -> Result<u32, TechTreeError> {
        let tech = self
            .technologies
            .get(&id)
            .ok_or(TechTreeError::TechNotFound(id))?
            .clone();

        let effective_cost = self.effective_cost(id)?;

        let required = match &effective_cost {
            ResearchCost::Points(total) => *total,
            _ => return Err(TechTreeError::WrongCostModel(id, "Points")),
        };

        let state = self
            .states
            .get_mut(&id)
            .ok_or(TechTreeError::TechNotFound(id))?;

        let current = match state {
            ResearchState::InProgress(ResearchProgress::Points(p)) => p,
            _ => return Err(TechTreeError::WrongCostModel(id, "Points")),
        };

        let remaining = required.saturating_sub(*current);
        let to_consume = points.min(remaining);
        *current += to_consume;

        let is_complete = *current >= required;

        if is_complete {
            self.complete_research(id, &tech, tick);
        }

        Ok(to_consume)
    }

    /// Advance a Rate-cost research by one tick. The rate is determined by
    /// the technology's cost definition. Completes research when total is met.
    pub fn tick_rate(&mut self, id: TechId, tick: Ticks) -> Result<bool, TechTreeError> {
        let tech = self
            .technologies
            .get(&id)
            .ok_or(TechTreeError::TechNotFound(id))?
            .clone();

        let effective_cost = self.effective_cost(id)?;

        let (points_per_tick, total) = match &effective_cost {
            ResearchCost::Rate {
                points_per_tick,
                total,
            } => (*points_per_tick, *total),
            _ => return Err(TechTreeError::WrongCostModel(id, "Rate")),
        };

        let state = self
            .states
            .get_mut(&id)
            .ok_or(TechTreeError::TechNotFound(id))?;

        let accumulated = match state {
            ResearchState::InProgress(ResearchProgress::Rate(p)) => p,
            _ => return Err(TechTreeError::WrongCostModel(id, "Rate")),
        };

        *accumulated = accumulated.saturating_add(points_per_tick);

        let is_complete = *accumulated >= total;

        if is_complete {
            self.complete_research(id, &tech, tick);
        }

        Ok(is_complete)
    }

    /// Advance an ItemRate-cost research by one tick. The duration is
    /// determined by the technology's cost definition. Game code is responsible
    /// for ensuring the item rate requirement is met each tick before calling
    /// this. Completes research when the duration is met.
    pub fn tick_item_rate(&mut self, id: TechId, tick: Ticks) -> Result<bool, TechTreeError> {
        let tech = self
            .technologies
            .get(&id)
            .ok_or(TechTreeError::TechNotFound(id))?
            .clone();

        let effective_cost = self.effective_cost(id)?;

        let duration = match &effective_cost {
            ResearchCost::ItemRate { duration, .. } => *duration,
            _ => return Err(TechTreeError::WrongCostModel(id, "ItemRate")),
        };

        let state = self
            .states
            .get_mut(&id)
            .ok_or(TechTreeError::TechNotFound(id))?;

        let elapsed = match state {
            ResearchState::InProgress(ResearchProgress::ItemRate(t)) => t,
            _ => return Err(TechTreeError::WrongCostModel(id, "ItemRate")),
        };

        *elapsed += 1;

        let is_complete = *elapsed >= duration;

        if is_complete {
            self.complete_research(id, &tech, tick);
        }

        Ok(is_complete)
    }

    /// Mark a Custom-cost research as complete. Game code decides when the
    /// custom condition is met.
    pub fn complete_custom(&mut self, id: TechId, tick: Ticks) -> Result<(), TechTreeError> {
        let tech = self
            .technologies
            .get(&id)
            .ok_or(TechTreeError::TechNotFound(id))?
            .clone();

        match &tech.cost {
            ResearchCost::Custom(_) => {}
            _ => return Err(TechTreeError::WrongCostModel(id, "Custom")),
        }

        let state = self
            .states
            .get(&id)
            .ok_or(TechTreeError::TechNotFound(id))?;

        match state {
            ResearchState::InProgress(ResearchProgress::Custom) => {}
            _ => return Err(TechTreeError::WrongCostModel(id, "Custom")),
        }

        self.complete_research(id, &tech, tick);
        Ok(())
    }

    // -- Event API --

    /// Drain all pending events. Returns events and clears the internal list.
    pub fn drain_events(&mut self) -> Vec<TechEvent> {
        std::mem::take(&mut self.events)
    }

    /// Get a read-only view of pending events.
    pub fn pending_events(&self) -> &[TechEvent] {
        &self.events
    }

    // -- Internal helpers --

    /// Complete research for a technology. Updates state and emits event.
    fn complete_research(&mut self, id: TechId, tech: &Technology, tick: Ticks) {
        let prev = self.completion_count(id);
        let level = prev + 1;

        // Update the completion count.
        self.completions.insert(id, level);

        // Update the state.
        self.states.insert(
            id,
            ResearchState::Completed {
                times_completed: level,
            },
        );

        self.events.push(TechEvent::ResearchCompleted {
            tech_id: id,
            unlocks: tech.unlocks.clone(),
            level,
            tick,
        });
    }
}

impl Default for TechTree {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Cost scaling helper
// ---------------------------------------------------------------------------

/// Apply cost scaling to a research cost at a given completion level.
fn scale_cost(base_cost: &ResearchCost, scaling: &CostScaling, level: u32) -> ResearchCost {
    match base_cost {
        ResearchCost::Items(items) => {
            let scaled: Vec<(ItemTypeId, u32)> = items
                .iter()
                .map(|(item, base_qty)| {
                    let base_scaling = match scaling {
                        CostScaling::Linear { base, increment } => {
                            // Scale factor: (base + increment * level) / base
                            // Applied to each item's quantity.
                            let scaled_total = base.saturating_add(increment.saturating_mul(level));
                            let factor = if *base == 0 { 1 } else { scaled_total / base };
                            base_qty.saturating_mul(factor.max(1))
                        }
                        CostScaling::Exponential { multiplier, .. } => {
                            let mut cost = Fixed64::from_num(*base_qty);
                            for _ in 0..level {
                                cost = cost.saturating_mul(*multiplier);
                            }
                            let result: i64 = cost.to_num();
                            if result < 0 {
                                0
                            } else if result > u32::MAX as i64 {
                                u32::MAX
                            } else {
                                result as u32
                            }
                        }
                    };
                    (*item, base_scaling)
                })
                .collect();
            ResearchCost::Items(scaled)
        }
        ResearchCost::Points(base) => {
            let scaled = scaling.cost_at_level(level);
            // Use the scaled value directly when CostScaling already accounts
            // for the base. For Points, the scaling IS the cost.
            let _ = base; // base is embedded in the scaling
            ResearchCost::Points(scaled)
        }
        ResearchCost::Delivery(items) => {
            let scaled: Vec<(ItemTypeId, u32)> = items
                .iter()
                .map(|(item, base_qty)| {
                    let scaled_qty = match scaling {
                        CostScaling::Linear { base, increment } => {
                            let factor_total = base.saturating_add(increment.saturating_mul(level));
                            let factor = if *base == 0 { 1 } else { factor_total / base };
                            base_qty.saturating_mul(factor.max(1))
                        }
                        CostScaling::Exponential { multiplier, .. } => {
                            let mut cost = Fixed64::from_num(*base_qty);
                            for _ in 0..level {
                                cost = cost.saturating_mul(*multiplier);
                            }
                            let result: i64 = cost.to_num();
                            if result < 0 {
                                0
                            } else if result > u32::MAX as i64 {
                                u32::MAX
                            } else {
                                result as u32
                            }
                        }
                    };
                    (*item, scaled_qty)
                })
                .collect();
            ResearchCost::Delivery(scaled)
        }
        ResearchCost::Rate {
            points_per_tick,
            total,
        } => {
            // Scale the total, keep rate the same.
            let scaled_total = {
                let base_total: i64 = total.to_num();
                let scaled = scaling.cost_at_level(level);
                // Use the larger of the scaled value and the base, since
                // cost_at_level uses its own base.
                let _ = base_total;
                Fixed64::from_num(scaled)
            };
            ResearchCost::Rate {
                points_per_tick: *points_per_tick,
                total: scaled_total,
            }
        }
        ResearchCost::ItemRate {
            item,
            rate,
            duration,
        } => {
            // Scale the duration.
            let scaled_duration = match scaling {
                CostScaling::Linear { base, increment } => {
                    let scaled = base.saturating_add(increment.saturating_mul(level));
                    scaled as Ticks
                }
                CostScaling::Exponential { base, multiplier } => {
                    let mut cost = Fixed64::from_num(*base);
                    for _ in 0..level {
                        cost = cost.saturating_mul(*multiplier);
                    }
                    let result: i64 = cost.to_num();
                    if result < 0 { 0 } else { result as Ticks }
                }
            };
            let _ = duration;
            ResearchCost::ItemRate {
                item: *item,
                rate: *rate,
                duration: scaled_duration,
            }
        }
        ResearchCost::Custom(fn_id) => {
            // Custom costs are not scalable by the engine; game code handles it.
            ResearchCost::Custom(*fn_id)
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use factorial_core::fixed::Fixed64;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn red_science() -> ItemTypeId {
        ItemTypeId(100)
    }

    fn green_science() -> ItemTypeId {
        ItemTypeId(101)
    }

    fn steel_furnace() -> BuildingTypeId {
        BuildingTypeId(10)
    }

    fn steel_plate_recipe() -> RecipeId {
        RecipeId(20)
    }

    /// Register a simple linear tech tree: A -> B -> C
    fn setup_linear_tree() -> TechTree {
        let mut tree = TechTree::new();

        tree.register(Technology {
            id: TechId(0),
            name: "Iron Smelting".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(100),
            unlocks: vec![Unlock::Recipe(RecipeId(0))],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.register(Technology {
            id: TechId(1),
            name: "Steel Smelting".to_string(),
            prerequisites: vec![TechId(0)],
            cost: ResearchCost::Items(vec![(red_science(), 50), (green_science(), 50)]),
            unlocks: vec![
                Unlock::Building(steel_furnace()),
                Unlock::Recipe(steel_plate_recipe()),
            ],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.register(Technology {
            id: TechId(2),
            name: "Advanced Metallurgy".to_string(),
            prerequisites: vec![TechId(1)],
            cost: ResearchCost::Points(500),
            unlocks: vec![Unlock::Custom("advanced_alloys".to_string())],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree
    }

    // -----------------------------------------------------------------------
    // Test 1: Prerequisites block research until met
    // -----------------------------------------------------------------------
    #[test]
    fn prerequisites_block_research() {
        let mut tree = setup_linear_tree();

        // Can't start Steel Smelting (TechId(1)) without Iron Smelting (TechId(0)).
        let result = tree.start_research(TechId(1), 0);
        assert!(matches!(
            result,
            Err(TechTreeError::PrerequisiteNotMet(TechId(1), TechId(0)))
        ));

        // Start and complete Iron Smelting.
        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_points(TechId(0), 100, 1).unwrap();
        assert!(tree.is_completed(TechId(0)));

        // Now Steel Smelting should be startable.
        tree.start_research(TechId(1), 2).unwrap();
        assert!(tree.is_in_progress(TechId(1)));
    }

    // -----------------------------------------------------------------------
    // Test 2: Chain of prerequisites enforced
    // -----------------------------------------------------------------------
    #[test]
    fn chain_prerequisites_enforced() {
        let mut tree = setup_linear_tree();

        // Can't skip to Advanced Metallurgy (needs Steel Smelting, which needs Iron Smelting).
        let result = tree.start_research(TechId(2), 0);
        assert!(result.is_err());

        // Complete the chain.
        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_points(TechId(0), 100, 1).unwrap();
        tree.start_research(TechId(1), 2).unwrap();
        tree.contribute_items(TechId(1), &[(red_science(), 50), (green_science(), 50)], 3)
            .unwrap();
        assert!(tree.is_completed(TechId(1)));

        // Now Advanced Metallurgy should be startable.
        tree.start_research(TechId(2), 4).unwrap();
        assert!(tree.is_in_progress(TechId(2)));
    }

    // -----------------------------------------------------------------------
    // Test 3: Items cost model works
    // -----------------------------------------------------------------------
    #[test]
    fn items_cost_model() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Items(vec![(red_science(), 10), (green_science(), 5)]),
            unlocks: vec![Unlock::Building(steel_furnace())],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        // Partial contribution.
        let consumed = tree
            .contribute_items(TechId(0), &[(red_science(), 7)], 1)
            .unwrap();
        assert_eq!(consumed, vec![(red_science(), 7)]);
        assert!(tree.is_in_progress(TechId(0)));

        // Complete red, partial green.
        let consumed = tree
            .contribute_items(TechId(0), &[(red_science(), 5), (green_science(), 3)], 2)
            .unwrap();
        // Only 3 red needed, 3 green consumed.
        assert_eq!(consumed, vec![(red_science(), 3), (green_science(), 3)]);
        assert!(tree.is_in_progress(TechId(0)));

        // Complete green.
        let consumed = tree
            .contribute_items(TechId(0), &[(green_science(), 10)], 3)
            .unwrap();
        assert_eq!(consumed, vec![(green_science(), 2)]);
        assert!(tree.is_completed(TechId(0)));
    }

    // -----------------------------------------------------------------------
    // Test 4: Points cost model works
    // -----------------------------------------------------------------------
    #[test]
    fn points_cost_model() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(100),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        let consumed = tree.contribute_points(TechId(0), 60, 1).unwrap();
        assert_eq!(consumed, 60);
        assert!(tree.is_in_progress(TechId(0)));

        let consumed = tree.contribute_points(TechId(0), 60, 2).unwrap();
        assert_eq!(consumed, 40); // Only 40 remaining.
        assert!(tree.is_completed(TechId(0)));
    }

    // -----------------------------------------------------------------------
    // Test 5: Delivery cost model works
    // -----------------------------------------------------------------------
    #[test]
    fn delivery_cost_model() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Delivery(vec![(red_science(), 20), (green_science(), 10)]),
            unlocks: vec![Unlock::Recipe(steel_plate_recipe())],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        // Deliver all at once.
        let consumed = tree
            .contribute_items(TechId(0), &[(red_science(), 20), (green_science(), 10)], 1)
            .unwrap();
        assert_eq!(consumed, vec![(red_science(), 20), (green_science(), 10)]);
        assert!(tree.is_completed(TechId(0)));
    }

    // -----------------------------------------------------------------------
    // Test 6: Rate cost model works
    // -----------------------------------------------------------------------
    #[test]
    fn rate_cost_model() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Rate {
                points_per_tick: Fixed64::from_num(10),
                total: Fixed64::from_num(50),
            },
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        // Tick 4 times (40 points), should not complete.
        for tick in 1..=4 {
            let complete = tree.tick_rate(TechId(0), tick).unwrap();
            assert!(!complete);
        }
        assert!(tree.is_in_progress(TechId(0)));

        // Tick once more (50 points), should complete.
        let complete = tree.tick_rate(TechId(0), 5).unwrap();
        assert!(complete);
        assert!(tree.is_completed(TechId(0)));
    }

    // -----------------------------------------------------------------------
    // Test 7: ItemRate cost model works
    // -----------------------------------------------------------------------
    #[test]
    fn item_rate_cost_model() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::ItemRate {
                item: red_science(),
                rate: Fixed64::from_num(5),
                duration: 10,
            },
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        // Tick 9 times, should not complete.
        for tick in 1..=9 {
            let complete = tree.tick_item_rate(TechId(0), tick).unwrap();
            assert!(!complete);
        }

        // 10th tick completes it.
        let complete = tree.tick_item_rate(TechId(0), 10).unwrap();
        assert!(complete);
        assert!(tree.is_completed(TechId(0)));
    }

    // -----------------------------------------------------------------------
    // Test 8: Custom cost model works
    // -----------------------------------------------------------------------
    #[test]
    fn custom_cost_model() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Custom(ResearchCostFnId(42)),
            unlocks: vec![Unlock::Custom("secret_unlock".to_string())],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();
        assert!(tree.is_in_progress(TechId(0)));

        tree.complete_custom(TechId(0), 5).unwrap();
        assert!(tree.is_completed(TechId(0)));
    }

    // -----------------------------------------------------------------------
    // Test 9: Unlock events emitted on completion
    // -----------------------------------------------------------------------
    #[test]
    fn unlock_events_emitted() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(10),
            unlocks: vec![
                Unlock::Building(steel_furnace()),
                Unlock::Recipe(steel_plate_recipe()),
            ],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        // Drain the start event.
        let events = tree.drain_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            TechEvent::ResearchStarted {
                tech_id: TechId(0),
                tick: 0
            }
        ));

        // Complete research.
        tree.contribute_points(TechId(0), 10, 5).unwrap();

        let events = tree.drain_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            TechEvent::ResearchCompleted {
                tech_id,
                unlocks,
                level,
                tick,
            } => {
                assert_eq!(*tech_id, TechId(0));
                assert_eq!(unlocks.len(), 2);
                assert_eq!(unlocks[0], Unlock::Building(steel_furnace()));
                assert_eq!(unlocks[1], Unlock::Recipe(steel_plate_recipe()));
                assert_eq!(*level, 1);
                assert_eq!(*tick, 5);
            }
            _ => panic!("expected ResearchCompleted event"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 10: Infinite research scales cost correctly
    // -----------------------------------------------------------------------
    #[test]
    fn infinite_research_linear_scaling() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Mining Productivity".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(1000),
            unlocks: vec![Unlock::Custom("mining_bonus".to_string())],
            repeatable: true,
            cost_scaling: Some(CostScaling::Linear {
                base: 1000,
                increment: 500,
            }),
        })
        .unwrap();

        // Level 0: cost = 1000
        let cost = tree.effective_cost(TechId(0)).unwrap();
        assert_eq!(cost, ResearchCost::Points(1000));

        // Complete level 0.
        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_points(TechId(0), 1000, 1).unwrap();
        assert_eq!(tree.completion_count(TechId(0)), 1);

        // Level 1: cost = 1000 + 500 = 1500
        let cost = tree.effective_cost(TechId(0)).unwrap();
        assert_eq!(cost, ResearchCost::Points(1500));

        // Complete level 1.
        tree.start_research(TechId(0), 2).unwrap();
        tree.contribute_points(TechId(0), 1500, 3).unwrap();
        assert_eq!(tree.completion_count(TechId(0)), 2);

        // Level 2: cost = 1000 + 1000 = 2000
        let cost = tree.effective_cost(TechId(0)).unwrap();
        assert_eq!(cost, ResearchCost::Points(2000));
    }

    // -----------------------------------------------------------------------
    // Test 11: Infinite research exponential scaling
    // -----------------------------------------------------------------------
    #[test]
    fn infinite_research_exponential_scaling() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "White Science".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(100),
            unlocks: vec![],
            repeatable: true,
            cost_scaling: Some(CostScaling::Exponential {
                base: 100,
                multiplier: Fixed64::from_num(2),
            }),
        })
        .unwrap();

        // Level 0: 100
        let cost = tree.effective_cost(TechId(0)).unwrap();
        assert_eq!(cost, ResearchCost::Points(100));

        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_points(TechId(0), 100, 1).unwrap();

        // Level 1: 100 * 2 = 200
        let cost = tree.effective_cost(TechId(0)).unwrap();
        assert_eq!(cost, ResearchCost::Points(200));

        tree.start_research(TechId(0), 2).unwrap();
        tree.contribute_points(TechId(0), 200, 3).unwrap();

        // Level 2: 100 * 2^2 = 400
        let cost = tree.effective_cost(TechId(0)).unwrap();
        assert_eq!(cost, ResearchCost::Points(400));
    }

    // -----------------------------------------------------------------------
    // Test 12: Non-repeatable tech can't be restarted
    // -----------------------------------------------------------------------
    #[test]
    fn non_repeatable_cannot_restart() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(10),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_points(TechId(0), 10, 1).unwrap();
        assert!(tree.is_completed(TechId(0)));

        let result = tree.start_research(TechId(0), 2);
        assert!(matches!(result, Err(TechTreeError::AlreadyCompleted(_))));
    }

    // -----------------------------------------------------------------------
    // Test 13: Can't start research that's already in progress
    // -----------------------------------------------------------------------
    #[test]
    fn cannot_start_already_in_progress() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(100),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        let result = tree.start_research(TechId(0), 1);
        assert!(matches!(result, Err(TechTreeError::AlreadyInProgress(_))));
    }

    // -----------------------------------------------------------------------
    // Test 14: Duplicate tech ID registration fails
    // -----------------------------------------------------------------------
    #[test]
    fn duplicate_id_fails() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "A".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(10),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        let result = tree.register(Technology {
            id: TechId(0),
            name: "B".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(10),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        });
        assert!(matches!(result, Err(TechTreeError::DuplicateId(_))));
    }

    // -----------------------------------------------------------------------
    // Test 15: Invalid prerequisite fails registration
    // -----------------------------------------------------------------------
    #[test]
    fn invalid_prerequisite_fails() {
        let mut tree = TechTree::new();
        let result = tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![TechId(99)], // doesn't exist
            cost: ResearchCost::Points(10),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        });
        assert!(matches!(
            result,
            Err(TechTreeError::InvalidPrerequisite { .. })
        ));
    }

    // -----------------------------------------------------------------------
    // Test 16: Serialization round-trip of tech tree state
    // -----------------------------------------------------------------------
    #[test]
    fn serialization_round_trip() {
        let mut tree = setup_linear_tree();

        // Complete first tech, start second.
        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_points(TechId(0), 100, 1).unwrap();
        tree.start_research(TechId(1), 2).unwrap();
        tree.contribute_items(TechId(1), &[(red_science(), 25)], 3)
            .unwrap();

        // Drain events so they don't interfere.
        tree.drain_events();

        // Serialize with serde_json (human-readable for testing).
        let json = serde_json::to_string(&tree).unwrap();
        let restored: TechTree = serde_json::from_str(&json).unwrap();

        // Verify state is preserved.
        assert!(restored.is_completed(TechId(0)));
        assert!(restored.is_in_progress(TechId(1)));
        assert_eq!(
            restored.get_state(TechId(2)),
            Some(&ResearchState::NotStarted)
        );

        // Verify technology definitions are preserved.
        assert_eq!(restored.technology_count(), 3);
        let tech = restored.get_technology(TechId(1)).unwrap();
        assert_eq!(tech.name, "Steel Smelting");
        assert_eq!(tech.prerequisites, vec![TechId(0)]);

        // Verify in-progress state is preserved.
        if let Some(ResearchState::InProgress(ResearchProgress::Items(items))) =
            restored.get_state(TechId(1))
        {
            let red = items.iter().find(|(id, _)| *id == red_science()).unwrap();
            assert_eq!(red.1, 25);
        } else {
            panic!("expected InProgress(Items) for TechId(1)");
        }
    }

    // -----------------------------------------------------------------------
    // Test 17: all_unlocks returns completed tech unlocks
    // -----------------------------------------------------------------------
    #[test]
    fn all_unlocks_returns_completed() {
        let mut tree = setup_linear_tree();

        assert!(tree.all_unlocks().is_empty());

        // Complete first tech.
        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_points(TechId(0), 100, 1).unwrap();

        let unlocks = tree.all_unlocks();
        assert_eq!(unlocks.len(), 1);
        assert_eq!(unlocks[0], Unlock::Recipe(RecipeId(0)));
    }

    // -----------------------------------------------------------------------
    // Test 18: Wrong cost model produces error
    // -----------------------------------------------------------------------
    #[test]
    fn wrong_cost_model_error() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(100),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        // Try to contribute items to a Points-cost research.
        let result = tree.contribute_items(TechId(0), &[(red_science(), 10)], 1);
        assert!(matches!(result, Err(TechTreeError::WrongCostModel(..))));

        // Try to tick rate on a Points-cost research.
        let result = tree.tick_rate(TechId(0), 1);
        assert!(matches!(result, Err(TechTreeError::WrongCostModel(..))));
    }

    // -----------------------------------------------------------------------
    // Test 19: CostScaling::Linear computes correctly
    // -----------------------------------------------------------------------
    #[test]
    fn cost_scaling_linear() {
        let scaling = CostScaling::Linear {
            base: 1000,
            increment: 500,
        };
        assert_eq!(scaling.cost_at_level(0), 1000);
        assert_eq!(scaling.cost_at_level(1), 1500);
        assert_eq!(scaling.cost_at_level(2), 2000);
        assert_eq!(scaling.cost_at_level(10), 6000);
    }

    // -----------------------------------------------------------------------
    // Test 20: CostScaling::Exponential computes correctly
    // -----------------------------------------------------------------------
    #[test]
    fn cost_scaling_exponential() {
        let scaling = CostScaling::Exponential {
            base: 100,
            multiplier: Fixed64::from_num(2),
        };
        assert_eq!(scaling.cost_at_level(0), 100);
        assert_eq!(scaling.cost_at_level(1), 200);
        assert_eq!(scaling.cost_at_level(2), 400);
        assert_eq!(scaling.cost_at_level(3), 800);
    }

    // -----------------------------------------------------------------------
    // Test 21: prerequisites_met query
    // -----------------------------------------------------------------------
    #[test]
    fn prerequisites_met_query() {
        let mut tree = setup_linear_tree();

        // Iron Smelting has no prereqs.
        assert!(tree.prerequisites_met(TechId(0)).unwrap());

        // Steel Smelting requires Iron Smelting.
        assert!(!tree.prerequisites_met(TechId(1)).unwrap());

        // Complete Iron Smelting.
        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_points(TechId(0), 100, 1).unwrap();

        assert!(tree.prerequisites_met(TechId(1)).unwrap());
        // Advanced Metallurgy still blocked (needs Steel).
        assert!(!tree.prerequisites_met(TechId(2)).unwrap());
    }

    // -----------------------------------------------------------------------
    // Test 22: Repeatable research completion count
    // -----------------------------------------------------------------------
    #[test]
    fn repeatable_completion_count() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Infinite".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(100),
            unlocks: vec![],
            repeatable: true,
            cost_scaling: Some(CostScaling::Linear {
                base: 100,
                increment: 50,
            }),
        })
        .unwrap();

        for i in 0..5 {
            tree.start_research(TechId(0), i * 2).unwrap();
            let cost = match tree.effective_cost(TechId(0)).unwrap() {
                ResearchCost::Points(p) => p,
                _ => panic!("expected Points"),
            };
            tree.contribute_points(TechId(0), cost, i * 2 + 1).unwrap();
        }

        assert_eq!(tree.completion_count(TechId(0)), 5);

        // Verify events: 5 starts + 5 completions = 10 events total.
        // (We didn't drain events, so all 10 should be there.)
        let events = tree.drain_events();
        assert_eq!(events.len(), 10);

        let completions: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, TechEvent::ResearchCompleted { .. }))
            .collect();
        assert_eq!(completions.len(), 5);

        // Verify levels.
        for (i, event) in completions.iter().enumerate() {
            if let TechEvent::ResearchCompleted { level, .. } = event {
                assert_eq!(*level, (i + 1) as u32);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Test 23: Tech not found error
    // -----------------------------------------------------------------------
    #[test]
    fn tech_not_found() {
        let mut tree = TechTree::new();
        let result = tree.start_research(TechId(99), 0);
        assert!(matches!(result, Err(TechTreeError::TechNotFound(_))));
    }

    // -----------------------------------------------------------------------
    // Test 24: next_tech_id auto-assigns
    // -----------------------------------------------------------------------
    #[test]
    fn next_tech_id_auto_assigns() {
        let mut tree = TechTree::new();

        let id1 = tree.next_tech_id();
        assert_eq!(id1, TechId(0));

        let id2 = tree.next_tech_id();
        assert_eq!(id2, TechId(1));

        // Register with id2, next should be 2.
        tree.register(Technology {
            id: id2,
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(10),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        let id3 = tree.next_tech_id();
        assert_eq!(id3, TechId(2));
    }

    // -----------------------------------------------------------------------
    // Test 25: Items contribution excess is not consumed
    // -----------------------------------------------------------------------
    #[test]
    fn items_excess_not_consumed() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Items(vec![(red_science(), 5)]),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        // Contribute 100, should only consume 5.
        let consumed = tree
            .contribute_items(TechId(0), &[(red_science(), 100)], 1)
            .unwrap();
        assert_eq!(consumed, vec![(red_science(), 5)]);
        assert!(tree.is_completed(TechId(0)));
    }

    // -----------------------------------------------------------------------
    // Test 26: Multiple prerequisites all required
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_prerequisites_all_required() {
        let mut tree = TechTree::new();

        tree.register(Technology {
            id: TechId(0),
            name: "A".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(10),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.register(Technology {
            id: TechId(1),
            name: "B".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(10),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.register(Technology {
            id: TechId(2),
            name: "C (requires A and B)".to_string(),
            prerequisites: vec![TechId(0), TechId(1)],
            cost: ResearchCost::Points(10),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        // Complete only A.
        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_points(TechId(0), 10, 1).unwrap();

        // C should not be startable (B not complete).
        assert!(!tree.prerequisites_met(TechId(2)).unwrap());
        let result = tree.start_research(TechId(2), 2);
        assert!(result.is_err());

        // Complete B.
        tree.start_research(TechId(1), 3).unwrap();
        tree.contribute_points(TechId(1), 10, 4).unwrap();

        // Now C should be startable.
        assert!(tree.prerequisites_met(TechId(2)).unwrap());
        tree.start_research(TechId(2), 5).unwrap();
        assert!(tree.is_in_progress(TechId(2)));
    }

    // -----------------------------------------------------------------------
    // Test 27: Infinite research with Items cost scaling
    // -----------------------------------------------------------------------
    #[test]
    fn infinite_research_items_scaling() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Inf Items".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Items(vec![(red_science(), 10)]),
            unlocks: vec![],
            repeatable: true,
            cost_scaling: Some(CostScaling::Exponential {
                base: 10,
                multiplier: Fixed64::from_num(2),
            }),
        })
        .unwrap();

        // Level 0: 10 items.
        tree.start_research(TechId(0), 0).unwrap();
        tree.contribute_items(TechId(0), &[(red_science(), 10)], 1)
            .unwrap();
        assert!(tree.is_completed(TechId(0)));

        // Level 1: 20 items.
        tree.start_research(TechId(0), 2).unwrap();
        let cost = tree.effective_cost(TechId(0)).unwrap();
        match &cost {
            ResearchCost::Items(items) => {
                assert_eq!(items[0].1, 20);
            }
            _ => panic!("expected Items cost"),
        }
        tree.contribute_items(TechId(0), &[(red_science(), 20)], 3)
            .unwrap();
        assert_eq!(tree.completion_count(TechId(0)), 2);

        // Level 2: 40 items.
        let cost = tree.effective_cost(TechId(0)).unwrap();
        match &cost {
            ResearchCost::Items(items) => {
                assert_eq!(items[0].1, 40);
            }
            _ => panic!("expected Items cost"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 28: Default trait implementation
    // -----------------------------------------------------------------------
    #[test]
    fn default_trait() {
        let tree = TechTree::default();
        assert_eq!(tree.technology_count(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 29: Drain events clears list
    // -----------------------------------------------------------------------
    #[test]
    fn drain_events_clears() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Test".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(10),
            unlocks: vec![],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

        tree.start_research(TechId(0), 0).unwrap();

        let events = tree.drain_events();
        assert_eq!(events.len(), 1);

        let events = tree.drain_events();
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 30: Serialization preserves repeatable state across levels
    // -----------------------------------------------------------------------
    #[test]
    fn serialization_repeatable_state() {
        let mut tree = TechTree::new();
        tree.register(Technology {
            id: TechId(0),
            name: "Infinite".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Points(100),
            unlocks: vec![],
            repeatable: true,
            cost_scaling: Some(CostScaling::Linear {
                base: 100,
                increment: 50,
            }),
        })
        .unwrap();

        // Complete 3 levels.
        for i in 0..3u32 {
            tree.start_research(TechId(0), i as u64 * 2).unwrap();
            let cost = match tree.effective_cost(TechId(0)).unwrap() {
                ResearchCost::Points(p) => p,
                _ => panic!("expected Points"),
            };
            tree.contribute_points(TechId(0), cost, i as u64 * 2 + 1)
                .unwrap();
        }

        assert_eq!(tree.completion_count(TechId(0)), 3);
        tree.drain_events();

        // Serialize and deserialize.
        let json = serde_json::to_string(&tree).unwrap();
        let restored: TechTree = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.completion_count(TechId(0)), 3);

        // Level 3 cost: 100 + 50*3 = 250
        let cost = restored.effective_cost(TechId(0)).unwrap();
        assert_eq!(cost, ResearchCost::Points(250));
    }
}
