use crate::fixed::Fixed64;
use crate::id::{ItemTypeId, ModifierId, PropertyId};
use crate::rng::SimRng;

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Recipe types
// ---------------------------------------------------------------------------

/// An input requirement for a fixed recipe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeInput {
    pub item_type: ItemTypeId,
    pub quantity: u32,
    /// When `true` (default), the input is consumed normally.
    /// When `false`, the input acts as a catalyst: it must be present to start
    /// the recipe but is not consumed during crafting.
    #[serde(default = "default_true")]
    pub consumed: bool,
}

/// A chance-based extra output applied after base production.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BonusOutput {
    /// Probability in [0, 1] that the bonus triggers each cycle.
    pub chance: Fixed64,
    /// Extra quantity produced when the bonus triggers.
    pub quantity: u32,
    /// Item type for the bonus. `None` means same type as the parent output.
    #[serde(default)]
    pub bonus_item_type: Option<ItemTypeId>,
}

/// An output product of a fixed recipe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeOutput {
    pub item_type: ItemTypeId,
    pub quantity: u32,
    /// Optional bonus output triggered probabilistically each cycle.
    #[serde(default)]
    pub bonus: Option<BonusOutput>,
}

// ---------------------------------------------------------------------------
// Depletion model
// ---------------------------------------------------------------------------

/// How a source processor depletes over time.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Depletion {
    /// Never runs out.
    Infinite,
    /// Fixed amount remaining; once zero the source stops.
    Finite { remaining: Fixed64 },
    /// Production rate decays exponentially with the given half-life in ticks.
    Decaying { half_life: u64 },
}

// ---------------------------------------------------------------------------
// Property transforms
// ---------------------------------------------------------------------------

/// A transformation applied to an item property.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PropertyTransform {
    /// Set a property to an absolute value.
    Set(PropertyId, Fixed64),
    /// Add a delta to a property.
    Add(PropertyId, Fixed64),
    /// Multiply a property by a factor.
    Multiply(PropertyId, Fixed64),
}

// ---------------------------------------------------------------------------
// Processor variants
// ---------------------------------------------------------------------------

/// Produces items from nothing (mines, extractors, wells).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceProcessor {
    pub output_type: ItemTypeId,
    /// Items produced per tick at base speed (before modifiers).
    pub base_rate: Fixed64,
    pub depletion: Depletion,
    /// Fractional production accumulator. When this reaches >= 1 whole items
    /// are emitted. Allows sub-1 base_rate to work correctly.
    pub accumulated: Fixed64,
    /// Optional initial properties to stamp onto every produced item stack.
    #[serde(default)]
    pub initial_properties: Option<std::collections::BTreeMap<PropertyId, Fixed64>>,
}

/// Consumes a fixed set of inputs and produces a fixed set of outputs after a
/// fixed number of ticks (assemblers, smelters, chemical plants).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FixedRecipe {
    pub inputs: Vec<RecipeInput>,
    pub outputs: Vec<RecipeOutput>,
    /// Base ticks to complete one crafting cycle (before speed modifiers).
    pub duration: u32,
}

/// Transforms a property on items passing through (heating, cooling, refining).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PropertyProcessor {
    pub input_type: ItemTypeId,
    pub output_type: ItemTypeId,
    pub transform: PropertyTransform,
}

/// Consumes items from input at a steady rate (sinks, consumers, research labs).
/// Like Source in reverse — accumulates fractional demand, consumes from input
/// when whole items are available.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DemandProcessor {
    pub input_type: ItemTypeId,
    /// Items consumed per tick at base speed (before modifiers).
    pub base_rate: Fixed64,
    /// Fractional consumption accumulator.
    pub accumulated: Fixed64,
    /// Total whole items consumed over the processor's lifetime.
    #[serde(default)]
    pub consumed_total: u64,
    /// Optional set of accepted item types. When `Some`, the processor consumes
    /// from any matching type in the input inventory (in list order). When `None`,
    /// falls back to `input_type` only (backwards compatible).
    #[serde(default)]
    pub accepted_types: Option<Vec<ItemTypeId>>,
}

// ---------------------------------------------------------------------------
// Multi-recipe (runtime recipe switching)
// ---------------------------------------------------------------------------

/// Policy for how to handle an in-progress craft when switching recipes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RecipeSwitchPolicy {
    /// Finish the current craft, then switch (default).
    #[default]
    CompleteFirst,
    /// Cancel immediately — in-progress inputs are lost.
    CancelImmediate,
    /// Cancel immediately — return consumed inputs to the input inventory.
    RefundInputs,
}

/// A processor that holds multiple recipes and can switch between them.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultiRecipeProcessor {
    pub recipes: Vec<FixedRecipe>,
    pub active_recipe: usize,
    #[serde(default)]
    pub switch_policy: RecipeSwitchPolicy,
    /// When `Some`, a recipe switch is pending (will be applied on cycle completion).
    #[serde(default)]
    pub pending_switch: Option<usize>,
    /// Inputs consumed at the start of the current in-progress craft.
    /// Used for the `RefundInputs` policy.
    #[serde(default)]
    pub in_progress_inputs: Vec<(ItemTypeId, u32)>,
}

/// Top-level processor enum. Dispatches via enum match (no trait objects).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Processor {
    Source(SourceProcessor),
    Fixed(FixedRecipe),
    Property(PropertyProcessor),
    Demand(DemandProcessor),
    /// Passes all items from input to output unchanged.
    /// Used for junction nodes (splitters, mergers, balancers).
    Passthrough,
    /// Holds multiple recipes with runtime switching support.
    MultiRecipe(MultiRecipeProcessor),
}

// ---------------------------------------------------------------------------
// Processor state
// ---------------------------------------------------------------------------

/// Why the processor cannot make progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StallReason {
    MissingInputs,
    OutputFull,
    NoPower,
    Depleted,
}

/// Runtime state of a processor -- tracked externally in SoA storage but
/// logically belongs with the processor.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProcessorState {
    #[default]
    Idle,
    Working {
        progress: u32,
    },
    Stalled {
        reason: StallReason,
    },
}

// ---------------------------------------------------------------------------
// Modifiers
// ---------------------------------------------------------------------------

/// What a modifier does to a processor's behaviour.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModifierKind {
    /// Multiplies effective speed (reduces duration). 2.0 = twice as fast.
    Speed(Fixed64),
    /// Bonus output multiplier. 0.1 = +10% extra output.
    Productivity(Fixed64),
    /// Reduces input consumption multiplier. 0.8 = uses 80% inputs.
    Efficiency(Fixed64),
}

/// How a modifier combines with others of the same kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum StackingRule {
    /// Each modifier multiplies the previous result. (default, existing behavior)
    #[default]
    Multiplicative,
    /// Modifiers are summed, then applied as a single multiplier.
    Additive,
    /// Each additional modifier has diminishing effect (50%, 25%, 12.5%...).
    Diminishing,
    /// Only the strongest modifier of this kind applies.
    Capped,
}

/// A modifier instance applied to a processor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Modifier {
    pub id: ModifierId,
    pub kind: ModifierKind,
    #[serde(default)]
    pub stacking: StackingRule,
}

// ---------------------------------------------------------------------------
// Tick result
// ---------------------------------------------------------------------------

/// The outcome of a single tick for a processor.
#[derive(Debug, Clone, Default)]
pub struct ProcessorResult {
    /// Items consumed from the input inventory this tick.
    pub consumed: Vec<(ItemTypeId, u32)>,
    /// Items produced to the output inventory this tick.
    pub produced: Vec<(ItemTypeId, u32)>,
    /// Whether the processor changed state (Idle->Working, Working->Idle, etc.).
    pub state_changed: bool,
    /// Property transform to apply to produced items (from PropertyProcessor).
    pub property_transform: Option<PropertyTransform>,
    /// Initial properties to stamp onto produced items (from SourceProcessor).
    pub initial_properties: Option<std::collections::BTreeMap<PropertyId, Fixed64>>,
}

// ---------------------------------------------------------------------------
// Resolved modifier totals
// ---------------------------------------------------------------------------

/// Pre-computed modifier multipliers after canonical sorting and folding.
struct ResolvedModifiers {
    speed: Fixed64,
    productivity: Fixed64,
    efficiency: Fixed64,
}

impl ResolvedModifiers {
    /// Sort modifiers by `ModifierId` (canonical order) then fold each
    /// category using the modifier's stacking rule.
    fn resolve(modifiers: &[Modifier]) -> Self {
        let one = Fixed64::from_num(1);

        // Sort indices by ModifierId for determinism.
        let mut sorted: Vec<&Modifier> = modifiers.iter().collect();
        sorted.sort_by_key(|m| m.id);

        let mut speed = one;
        let mut productivity = one;
        let mut efficiency = one;

        // Group modifiers by kind, then apply stacking rules
        for m in &sorted {
            let (target, value) = match &m.kind {
                ModifierKind::Speed(v) => (&mut speed, *v),
                ModifierKind::Productivity(v) => (&mut productivity, *v),
                ModifierKind::Efficiency(v) => (&mut efficiency, *v),
            };

            match m.stacking {
                StackingRule::Multiplicative => {
                    *target *= value;
                }
                StackingRule::Additive => {
                    // Additive: accumulate the delta (value - 1.0)
                    *target += value - one;
                }
                StackingRule::Diminishing => {
                    // Each successive modifier's effect is halved
                    let delta = value - one;
                    *target *= one + delta / Fixed64::from_num(2);
                }
                StackingRule::Capped => {
                    // Only keep the larger value
                    if value > *target {
                        *target = value;
                    }
                }
            }
        }

        Self {
            speed,
            productivity,
            efficiency,
        }
    }
}

// ---------------------------------------------------------------------------
// Processor::tick
// ---------------------------------------------------------------------------

impl Processor {
    /// Advance the processor by one tick.
    ///
    /// # Arguments
    /// * `state`            - mutable reference to the processor's runtime state
    /// * `modifiers`        - slice of modifiers (applied in canonical order)
    /// * `available_inputs` - items currently available in the input inventory
    /// * `output_space`     - total free slots in the output inventory
    ///
    /// # Returns
    /// A `ProcessorResult` describing what happened this tick.
    pub fn tick(
        &mut self,
        state: &mut ProcessorState,
        modifiers: &[Modifier],
        available_inputs: &[(ItemTypeId, u32)],
        output_space: u32,
    ) -> ProcessorResult {
        self.tick_with_rng(state, modifiers, available_inputs, output_space, None)
    }

    /// Like [`tick`](Self::tick) but with an optional per-node PRNG for
    /// bonus output rolls.
    pub fn tick_with_rng(
        &mut self,
        state: &mut ProcessorState,
        modifiers: &[Modifier],
        available_inputs: &[(ItemTypeId, u32)],
        output_space: u32,
        rng: Option<&mut SimRng>,
    ) -> ProcessorResult {
        match self {
            Processor::Source(src) => tick_source(src, state, modifiers, output_space),
            Processor::Fixed(recipe) => tick_fixed(
                recipe,
                state,
                modifiers,
                available_inputs,
                output_space,
                rng,
            ),
            Processor::Property(prop) => tick_property(prop, state, available_inputs, output_space),
            Processor::Demand(demand) => tick_demand(demand, state, modifiers, available_inputs),
            Processor::Passthrough => tick_passthrough(state, available_inputs, output_space),
            Processor::MultiRecipe(multi) => {
                tick_multi_recipe(multi, state, modifiers, available_inputs, output_space, rng)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Source processor tick
// ---------------------------------------------------------------------------

fn tick_source(
    src: &mut SourceProcessor,
    state: &mut ProcessorState,
    modifiers: &[Modifier],
    output_space: u32,
) -> ProcessorResult {
    let mut result = ProcessorResult::default();

    // Check depletion first.
    if let Depletion::Finite { remaining } = &src.depletion
        && *remaining <= Fixed64::from_num(0)
    {
        if *state
            != (ProcessorState::Stalled {
                reason: StallReason::Depleted,
            })
        {
            *state = ProcessorState::Stalled {
                reason: StallReason::Depleted,
            };
            result.state_changed = true;
        }
        return result;
    }

    if output_space == 0 {
        if *state
            != (ProcessorState::Stalled {
                reason: StallReason::OutputFull,
            })
        {
            *state = ProcessorState::Stalled {
                reason: StallReason::OutputFull,
            };
            result.state_changed = true;
        }
        return result;
    }

    let mods = ResolvedModifiers::resolve(modifiers);

    // Effective rate = base_rate * speed_modifier * productivity_modifier.
    let effective_rate = src.base_rate * mods.speed * mods.productivity;

    // Accumulate fractional items.
    src.accumulated += effective_rate;

    // Determine whole items to emit this tick.
    let mut whole: u32 = src.accumulated.to_num::<i64>().max(0) as u32;

    // Clamp by output space.
    whole = whole.min(output_space);

    // Clamp by remaining (finite depletion).
    if let Depletion::Finite { remaining } = &mut src.depletion {
        let remain_whole = remaining.to_num::<i64>().max(0) as u32;
        whole = whole.min(remain_whole);
        *remaining -= Fixed64::from_num(whole);
    }

    if whole > 0 {
        src.accumulated -= Fixed64::from_num(whole);
        result.produced.push((src.output_type, whole));
        result.initial_properties = src.initial_properties.clone();
    }

    // Update state.
    let new_state = if whole > 0 || effective_rate > Fixed64::from_num(0) {
        ProcessorState::Working { progress: 0 }
    } else {
        ProcessorState::Idle
    };

    if *state != new_state {
        *state = new_state;
        result.state_changed = true;
    }

    result
}

// ---------------------------------------------------------------------------
// Fixed recipe tick
// ---------------------------------------------------------------------------

fn tick_fixed(
    recipe: &FixedRecipe,
    state: &mut ProcessorState,
    modifiers: &[Modifier],
    available_inputs: &[(ItemTypeId, u32)],
    output_space: u32,
    rng: Option<&mut SimRng>,
) -> ProcessorResult {
    let mut result = ProcessorResult::default();
    let mods = ResolvedModifiers::resolve(modifiers);

    // Effective duration = ceil(base_duration / speed).
    // A speed of 2.0 halves the time. Minimum 1 tick.
    let base_dur = Fixed64::from_num(recipe.duration);
    let effective_dur_fixed = base_dur / mods.speed;
    let effective_dur: u32 = {
        // Ceiling of the fixed-point value, minimum 1.
        let raw: i64 = effective_dur_fixed.to_num();
        let frac = effective_dur_fixed.frac();
        let ceiled = if frac > Fixed64::from_num(0) {
            raw + 1
        } else {
            raw
        };
        (ceiled.max(1)) as u32
    };

    match state {
        ProcessorState::Idle | ProcessorState::Stalled { .. } => {
            // Try to start a new crafting cycle.
            // Check output space first -- we need room for all outputs.
            let total_output: u32 = recipe.outputs.iter().map(|o| o.quantity).sum();
            if output_space < total_output {
                let new_state = ProcessorState::Stalled {
                    reason: StallReason::OutputFull,
                };
                if *state != new_state {
                    *state = new_state;
                    result.state_changed = true;
                }
                return result;
            }

            // Check whether all inputs are satisfied (with efficiency modifier).
            let mut can_start = true;
            let mut to_consume: Vec<(ItemTypeId, u32)> = Vec::new();
            for input in &recipe.inputs {
                // Effective quantity = ceil(base_quantity * efficiency).
                // Catalysts (consumed == false) are not affected by efficiency.
                let eff_qty = if input.consumed {
                    let eff_qty_fixed = Fixed64::from_num(input.quantity) * mods.efficiency;
                    let raw: i64 = eff_qty_fixed.to_num();
                    let frac = eff_qty_fixed.frac();
                    if frac > Fixed64::from_num(0) {
                        (raw + 1).max(1) as u32
                    } else {
                        raw.max(1) as u32
                    }
                } else {
                    input.quantity
                };

                let available = available_inputs
                    .iter()
                    .find(|(id, _)| *id == input.item_type)
                    .map(|(_, q)| *q)
                    .unwrap_or(0);

                if available < eff_qty {
                    can_start = false;
                    break;
                }
                // Only consume inputs that are not catalysts.
                if input.consumed {
                    to_consume.push((input.item_type, eff_qty));
                }
            }

            if !can_start {
                let new_state = ProcessorState::Stalled {
                    reason: StallReason::MissingInputs,
                };
                if *state != new_state {
                    *state = new_state;
                    result.state_changed = true;
                }
                return result;
            }

            // Consume inputs and begin working.
            result.consumed = to_consume;

            // If effective_dur is 1 tick, produce immediately.
            if effective_dur <= 1 {
                let produced = apply_productivity(&recipe.outputs, &mods, rng);
                result.produced = produced;
                *state = ProcessorState::Idle;
                result.state_changed = true;
            } else {
                *state = ProcessorState::Working { progress: 1 };
                result.state_changed = true;
            }
        }

        ProcessorState::Working { progress } => {
            *progress += 1;
            if *progress >= effective_dur {
                // Crafting complete -- emit outputs.
                let produced = apply_productivity(&recipe.outputs, &mods, rng);
                result.produced = produced;
                *state = ProcessorState::Idle;
                result.state_changed = true;
            }
        }
    }

    result
}

/// Apply productivity modifier to outputs and roll bonus outputs.
///
/// Productivity > 1.0 means extra base items. Bonus outputs are separate:
/// they trigger probabilistically via `rng` and are NOT multiplied by
/// the productivity modifier (prevents double-dipping).
fn apply_productivity(
    outputs: &[RecipeOutput],
    mods: &ResolvedModifiers,
    mut rng: Option<&mut SimRng>,
) -> Vec<(ItemTypeId, u32)> {
    let mut produced = Vec::with_capacity(outputs.len() * 2);
    for o in outputs {
        let base = Fixed64::from_num(o.quantity);
        let boosted = base * mods.productivity;
        let qty = boosted.to_num::<i64>().max(1) as u32;
        produced.push((o.item_type, qty));

        // Roll bonus output if present and RNG available.
        if let (Some(bonus), Some(rng)) = (&o.bonus, rng.as_deref_mut())
            && rng.chance(bonus.chance)
        {
            let bonus_type = bonus.bonus_item_type.unwrap_or(o.item_type);
            produced.push((bonus_type, bonus.quantity));
        }
    }
    produced
}

// ---------------------------------------------------------------------------
// Property processor tick
// ---------------------------------------------------------------------------

fn tick_property(
    prop: &PropertyProcessor,
    state: &mut ProcessorState,
    available_inputs: &[(ItemTypeId, u32)],
    output_space: u32,
) -> ProcessorResult {
    let mut result = ProcessorResult::default();

    if output_space == 0 {
        let new_state = ProcessorState::Stalled {
            reason: StallReason::OutputFull,
        };
        if *state != new_state {
            *state = new_state;
            result.state_changed = true;
        }
        return result;
    }

    let available = available_inputs
        .iter()
        .find(|(id, _)| *id == prop.input_type)
        .map(|(_, q)| *q)
        .unwrap_or(0);

    if available == 0 {
        let new_state = ProcessorState::Stalled {
            reason: StallReason::MissingInputs,
        };
        if *state != new_state {
            *state = new_state;
            result.state_changed = true;
        }
        return result;
    }

    // Process items per tick. The actual property transformation is applied
    // by the engine using `result.property_transform`.
    let qty = available.min(output_space);
    result.consumed.push((prop.input_type, qty));
    result.produced.push((prop.output_type, qty));
    result.property_transform = Some(prop.transform.clone());

    let new_state = ProcessorState::Working { progress: 0 };
    if *state != new_state {
        *state = new_state;
        result.state_changed = true;
    }

    result
}

// ---------------------------------------------------------------------------
// Demand processor tick
// ---------------------------------------------------------------------------

fn tick_demand(
    demand: &mut DemandProcessor,
    state: &mut ProcessorState,
    modifiers: &[Modifier],
    available_inputs: &[(ItemTypeId, u32)],
) -> ProcessorResult {
    let mut result = ProcessorResult::default();
    let mods = ResolvedModifiers::resolve(modifiers);

    // Effective rate = base_rate * speed_modifier
    let effective_rate = demand.base_rate * mods.speed;

    // Accumulate fractional demand
    demand.accumulated += effective_rate;

    // Determine whole items to consume this tick
    let whole: u32 = demand.accumulated.to_num::<i64>().max(0) as u32;

    if let Some(ref types) = demand.accepted_types {
        // Multi-type mode: consume from any accepted type, in order
        let mut remaining = whole;
        for &item_type in types {
            if remaining == 0 {
                break;
            }
            let available = available_inputs
                .iter()
                .find(|(id, _)| *id == item_type)
                .map(|(_, q)| *q)
                .unwrap_or(0);
            let take = remaining.min(available);
            if take > 0 {
                result.consumed.push((item_type, take));
                remaining -= take;
            }
        }
        let actually_consumed = whole - remaining;
        if actually_consumed > 0 {
            demand.accumulated -= Fixed64::from_num(actually_consumed);
            demand.consumed_total += actually_consumed as u64;
        }
        // Stall if we wanted items but got none
        if actually_consumed == 0 && whole > 0 {
            if *state
                != (ProcessorState::Stalled {
                    reason: StallReason::MissingInputs,
                })
            {
                *state = ProcessorState::Stalled {
                    reason: StallReason::MissingInputs,
                };
                result.state_changed = true;
            }
            return result;
        }
    } else {
        // Single-type mode: consume from input_type only (existing behavior)
        let available = available_inputs
            .iter()
            .find(|(id, _)| *id == demand.input_type)
            .map(|(_, q)| *q)
            .unwrap_or(0);

        if available == 0 && whole > 0 {
            if *state
                != (ProcessorState::Stalled {
                    reason: StallReason::MissingInputs,
                })
            {
                *state = ProcessorState::Stalled {
                    reason: StallReason::MissingInputs,
                };
                result.state_changed = true;
            }
            return result;
        }

        let clamped = whole.min(available);
        if clamped > 0 {
            demand.accumulated -= Fixed64::from_num(clamped);
            demand.consumed_total += clamped as u64;
            result.consumed.push((demand.input_type, clamped));
        }
    }

    // Update state
    let new_state = if !result.consumed.is_empty() || effective_rate > Fixed64::from_num(0) {
        ProcessorState::Working { progress: 0 }
    } else {
        ProcessorState::Idle
    };

    if *state != new_state {
        *state = new_state;
        result.state_changed = true;
    }

    result
}

// ---------------------------------------------------------------------------
// Passthrough processor tick
// ---------------------------------------------------------------------------

fn tick_passthrough(
    state: &mut ProcessorState,
    available_inputs: &[(ItemTypeId, u32)],
    output_space: u32,
) -> ProcessorResult {
    if available_inputs.is_empty() {
        *state = ProcessorState::Idle;
        return ProcessorResult {
            state_changed: true,
            ..Default::default()
        };
    }

    let mut result = ProcessorResult::default();
    let mut space_remaining = output_space;

    for &(item_type, qty) in available_inputs {
        if space_remaining == 0 {
            break;
        }
        let to_move = qty.min(space_remaining);
        if to_move > 0 {
            result.consumed.push((item_type, to_move));
            result.produced.push((item_type, to_move));
            space_remaining -= to_move;
        }
    }

    if result.consumed.is_empty() {
        *state = ProcessorState::Stalled {
            reason: StallReason::OutputFull,
        };
    } else {
        *state = ProcessorState::Working { progress: 0 };
    }

    result.state_changed = true;
    result
}

// ---------------------------------------------------------------------------
// Multi-recipe processor tick
// ---------------------------------------------------------------------------

fn tick_multi_recipe(
    multi: &mut MultiRecipeProcessor,
    state: &mut ProcessorState,
    modifiers: &[Modifier],
    available_inputs: &[(ItemTypeId, u32)],
    output_space: u32,
    rng: Option<&mut SimRng>,
) -> ProcessorResult {
    // Apply pending switch when idle (before starting a new cycle).
    if matches!(state, ProcessorState::Idle | ProcessorState::Stalled { .. })
        && multi.pending_switch.is_some()
    {
        let new_idx = multi.pending_switch.take().unwrap();
        multi.active_recipe = new_idx;
    }

    let recipe = match multi.recipes.get(multi.active_recipe) {
        Some(r) => r,
        None => return ProcessorResult::default(),
    };

    // Delegate to tick_fixed for the active recipe.
    let mut result = tick_fixed(
        recipe,
        state,
        modifiers,
        available_inputs,
        output_space,
        rng,
    );

    // Track consumed inputs for RefundInputs policy.
    if !result.consumed.is_empty() && matches!(state, ProcessorState::Working { .. }) {
        multi.in_progress_inputs = result.consumed.clone();
    }

    // On cycle completion (state returned to Idle), apply pending switch.
    if matches!(state, ProcessorState::Idle) && multi.pending_switch.is_some() {
        let new_idx = multi.pending_switch.take().unwrap();
        multi.active_recipe = new_idx;
        result.state_changed = true;
    }

    result
}

/// Error type for recipe switch operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecipeSwitchError {
    #[error("node has no MultiRecipe processor")]
    NotMultiRecipe,
    #[error("recipe index {0} out of bounds (max {1})")]
    IndexOutOfBounds(usize, usize),
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helpers ---------------------------------------------------------------

    fn fixed(v: f64) -> Fixed64 {
        Fixed64::from_num(v)
    }

    fn iron() -> ItemTypeId {
        ItemTypeId(0)
    }
    fn copper() -> ItemTypeId {
        ItemTypeId(1)
    }
    fn gear() -> ItemTypeId {
        ItemTypeId(2)
    }
    fn wire() -> ItemTypeId {
        ItemTypeId(3)
    }

    fn make_fixed_recipe(
        inputs: Vec<(ItemTypeId, u32)>,
        outputs: Vec<(ItemTypeId, u32)>,
        duration: u32,
    ) -> Processor {
        Processor::Fixed(FixedRecipe {
            inputs: inputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeInput {
                    item_type,
                    quantity,
                    consumed: true,
                })
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeOutput {
                    item_type,
                    quantity,
                    bonus: None,
                })
                .collect(),
            duration,
        })
    }

    fn make_source(output: ItemTypeId, rate: f64, depletion: Depletion) -> Processor {
        Processor::Source(SourceProcessor {
            output_type: output,
            base_rate: fixed(rate),
            depletion,
            accumulated: fixed(0.0),
            initial_properties: None,
        })
    }

    // -----------------------------------------------------------------------
    // Test 1: FixedRecipe consumes inputs and produces outputs after duration
    // -----------------------------------------------------------------------
    #[test]
    fn fixed_recipe_consumes_and_produces_after_duration() {
        // 2 iron -> 1 gear, 30 ticks
        let mut proc = make_fixed_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 30);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Tick 1: should consume inputs and start working.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10)], 10);
        assert_eq!(r.consumed, vec![(iron(), 2)]);
        assert!(r.produced.is_empty());
        assert!(r.state_changed);
        assert!(matches!(state, ProcessorState::Working { progress: 1 }));

        // Ticks 2..29: working, nothing consumed or produced.
        for tick in 2..30 {
            let r = proc.tick(&mut state, &no_mods, &[(iron(), 8)], 10);
            assert!(r.consumed.is_empty());
            assert!(r.produced.is_empty(), "tick {tick} should not produce");
            assert!(matches!(state, ProcessorState::Working { .. }));
        }

        // Tick 30: should produce output and go idle.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 8)], 10);
        assert!(r.consumed.is_empty());
        assert_eq!(r.produced, vec![(gear(), 1)]);
        assert!(r.state_changed);
        assert_eq!(state, ProcessorState::Idle);
    }

    // -----------------------------------------------------------------------
    // Test 2: FixedRecipe stalls when inputs missing
    // -----------------------------------------------------------------------
    #[test]
    fn fixed_recipe_stalls_missing_inputs() {
        let mut proc = make_fixed_recipe(vec![(iron(), 5)], vec![(gear(), 1)], 10);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // No iron available.
        let r = proc.tick(&mut state, &no_mods, &[], 10);
        assert!(r.consumed.is_empty());
        assert!(r.produced.is_empty());
        assert!(r.state_changed);
        assert_eq!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::MissingInputs
            }
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: FixedRecipe stalls when output full
    // -----------------------------------------------------------------------
    #[test]
    fn fixed_recipe_stalls_output_full() {
        let mut proc = make_fixed_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Plenty of inputs but no output space.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 100)], 0);
        assert!(r.consumed.is_empty());
        assert!(r.produced.is_empty());
        assert!(r.state_changed);
        assert_eq!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::OutputFull
            }
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: FixedRecipe multi-output
    // -----------------------------------------------------------------------
    #[test]
    fn fixed_recipe_multi_output() {
        // 1 copper -> 1 gear + 1 wire, 5 ticks
        let mut proc = make_fixed_recipe(vec![(copper(), 1)], vec![(gear(), 2), (wire(), 3)], 5);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Start cycle (tick 1).
        let r = proc.tick(&mut state, &no_mods, &[(copper(), 10)], 20);
        assert_eq!(r.consumed, vec![(copper(), 1)]);
        assert!(r.produced.is_empty());

        // Ticks 2..5.
        for _ in 2..5 {
            proc.tick(&mut state, &no_mods, &[(copper(), 9)], 20);
        }

        // Tick 5: produce.
        let r = proc.tick(&mut state, &no_mods, &[(copper(), 9)], 20);
        assert_eq!(r.produced, vec![(gear(), 2), (wire(), 3)]);
        assert_eq!(state, ProcessorState::Idle);
    }

    // -----------------------------------------------------------------------
    // Test 5: SourceProcessor produces at base rate
    // -----------------------------------------------------------------------
    #[test]
    fn source_produces_at_base_rate() {
        // 2 iron per tick, infinite.
        let mut proc = make_source(iron(), 2.0, Depletion::Infinite);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        for _ in 0..5 {
            let r = proc.tick(&mut state, &no_mods, &[], 100);
            assert_eq!(r.produced, vec![(iron(), 2)]);
        }
    }

    // -----------------------------------------------------------------------
    // Test 6: SourceProcessor finite depletion
    // -----------------------------------------------------------------------
    #[test]
    fn source_finite_depletion() {
        // 1 iron per tick, 5 remaining.
        let mut proc = make_source(
            iron(),
            1.0,
            Depletion::Finite {
                remaining: fixed(5.0),
            },
        );
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // 5 ticks should produce 1 each.
        for _ in 0..5 {
            let r = proc.tick(&mut state, &no_mods, &[], 100);
            assert_eq!(r.produced, vec![(iron(), 1)]);
        }

        // 6th tick: depleted.
        let r = proc.tick(&mut state, &no_mods, &[], 100);
        assert!(r.produced.is_empty());
        assert_eq!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::Depleted
            }
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Modifier speed affects duration
    // -----------------------------------------------------------------------
    #[test]
    fn modifier_speed_halves_duration() {
        // 2 iron -> 1 gear, 30 ticks base. With 2x speed => 15 ticks.
        let mut proc = make_fixed_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 30);
        let mut state = ProcessorState::Idle;
        let mods = vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(fixed(2.0)),
            stacking: StackingRule::default(),
        }];

        // Tick 1: consume inputs, start working.
        let r = proc.tick(&mut state, &mods, &[(iron(), 10)], 10);
        assert_eq!(r.consumed, vec![(iron(), 2)]);
        assert!(matches!(state, ProcessorState::Working { progress: 1 }));

        // Ticks 2..15.
        for _ in 2..15 {
            let r = proc.tick(&mut state, &mods, &[(iron(), 8)], 10);
            assert!(r.produced.is_empty());
        }

        // Tick 15: should produce.
        let r = proc.tick(&mut state, &mods, &[(iron(), 8)], 10);
        assert_eq!(r.produced, vec![(gear(), 1)]);
        assert_eq!(state, ProcessorState::Idle);
    }

    // -----------------------------------------------------------------------
    // Test 8: Canonical modifier stacking order
    // -----------------------------------------------------------------------
    #[test]
    fn modifier_canonical_stacking_order() {
        // Two speed modifiers: id=5 (1.5x) and id=1 (2.0x).
        // Canonical order: id=1 first, then id=5.
        // Product: 2.0 * 1.5 = 3.0 regardless of insertion order.
        //
        // With base duration 30 and speed 3.0 => effective duration = 10.
        let mods_unordered = vec![
            Modifier {
                id: ModifierId(5),
                kind: ModifierKind::Speed(fixed(1.5)),
                stacking: StackingRule::default(),
            },
            Modifier {
                id: ModifierId(1),
                kind: ModifierKind::Speed(fixed(2.0)),
                stacking: StackingRule::default(),
            },
        ];
        let mods_ordered = vec![
            Modifier {
                id: ModifierId(1),
                kind: ModifierKind::Speed(fixed(2.0)),
                stacking: StackingRule::default(),
            },
            Modifier {
                id: ModifierId(5),
                kind: ModifierKind::Speed(fixed(1.5)),
                stacking: StackingRule::default(),
            },
        ];

        // Both orderings should produce identical results.
        for mods in [&mods_unordered, &mods_ordered] {
            let mut proc = make_fixed_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 30);
            let mut state = ProcessorState::Idle;

            // Tick 1: consume.
            proc.tick(&mut state, mods, &[(iron(), 10)], 10);

            // Ticks 2..10.
            for _ in 2..10 {
                let r = proc.tick(&mut state, mods, &[(iron(), 9)], 10);
                assert!(r.produced.is_empty());
            }

            // Tick 10: produce.
            let r = proc.tick(&mut state, mods, &[(iron(), 9)], 10);
            assert_eq!(r.produced, vec![(gear(), 1)]);
            assert_eq!(state, ProcessorState::Idle);
        }
    }

    // -----------------------------------------------------------------------
    // Test 9: PropertyProcessor transforms items
    // -----------------------------------------------------------------------
    #[test]
    fn property_processor_transforms() {
        let mut proc = Processor::Property(PropertyProcessor {
            input_type: iron(),
            output_type: gear(), // e.g. "heated iron"
            transform: PropertyTransform::Set(PropertyId(0), fixed(100.0)),
        });
        let mut state = ProcessorState::Idle;

        let r = proc.tick(&mut state, &[], &[(iron(), 5)], 10);
        assert_eq!(r.consumed, vec![(iron(), 5)]);
        assert_eq!(r.produced, vec![(gear(), 5)]);
    }

    // -----------------------------------------------------------------------
    // Test 10: PropertyProcessor stalls on missing input
    // -----------------------------------------------------------------------
    #[test]
    fn property_processor_stalls_missing_input() {
        let mut proc = Processor::Property(PropertyProcessor {
            input_type: iron(),
            output_type: gear(),
            transform: PropertyTransform::Add(PropertyId(0), fixed(10.0)),
        });
        let mut state = ProcessorState::Idle;

        let r = proc.tick(&mut state, &[], &[], 10);
        assert!(r.consumed.is_empty());
        assert!(r.produced.is_empty());
        assert_eq!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::MissingInputs
            }
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: SourceProcessor with fractional rate accumulates
    // -----------------------------------------------------------------------
    #[test]
    fn source_fractional_rate_accumulates() {
        // 0.5 iron per tick => should produce 1 every 2 ticks.
        let mut proc = make_source(iron(), 0.5, Depletion::Infinite);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Tick 1: accumulates 0.5, no whole item yet.
        let r = proc.tick(&mut state, &no_mods, &[], 100);
        assert!(r.produced.is_empty());

        // Tick 2: accumulates to 1.0, emit 1.
        let r = proc.tick(&mut state, &no_mods, &[], 100);
        assert_eq!(r.produced, vec![(iron(), 1)]);
    }

    // -----------------------------------------------------------------------
    // Test 12: FixedRecipe recovers from stall when inputs become available
    // -----------------------------------------------------------------------
    #[test]
    fn fixed_recipe_recovers_from_stall() {
        let mut proc = make_fixed_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 3);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Stall: no inputs.
        proc.tick(&mut state, &no_mods, &[], 10);
        assert_eq!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::MissingInputs
            }
        );

        // Provide inputs: should start working.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 5)], 10);
        assert_eq!(r.consumed, vec![(iron(), 2)]);
        assert!(matches!(state, ProcessorState::Working { .. }));
    }

    // -----------------------------------------------------------------------
    // Test 13: Efficiency modifier reduces input consumption
    // -----------------------------------------------------------------------
    #[test]
    fn efficiency_modifier_reduces_inputs() {
        // 10 iron -> 1 gear, with 0.5 efficiency => needs ceil(10 * 0.5) = 5 iron.
        let mut proc = make_fixed_recipe(vec![(iron(), 10)], vec![(gear(), 1)], 2);
        let mut state = ProcessorState::Idle;
        let mods = vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Efficiency(fixed(0.5)),
            stacking: StackingRule::default(),
        }];

        let r = proc.tick(&mut state, &mods, &[(iron(), 5)], 10);
        // Should consume 5 (ceil(10 * 0.5)).
        assert_eq!(r.consumed, vec![(iron(), 5)]);
        assert!(matches!(state, ProcessorState::Working { .. }));
    }

    // -----------------------------------------------------------------------
    // Test 14: Productivity modifier increases output
    // -----------------------------------------------------------------------
    #[test]
    fn productivity_modifier_increases_output() {
        // 1 iron -> 1 gear, with 2.0 productivity => 2 gears.
        let mut proc = make_fixed_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 2);
        let mut state = ProcessorState::Idle;
        let mods = vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Productivity(fixed(2.0)),
            stacking: StackingRule::default(),
        }];

        // Tick 1: consume.
        proc.tick(&mut state, &mods, &[(iron(), 5)], 10);
        // Tick 2: produce.
        let r = proc.tick(&mut state, &mods, &[(iron(), 4)], 10);
        assert_eq!(r.produced, vec![(gear(), 2)]);
    }

    // -----------------------------------------------------------------------
    // Test 15: Source output_space clamping
    // -----------------------------------------------------------------------
    #[test]
    fn source_clamps_to_output_space() {
        // 10 per tick but only 3 output slots.
        let mut proc = make_source(iron(), 10.0, Depletion::Infinite);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        let r = proc.tick(&mut state, &no_mods, &[], 3);
        assert_eq!(r.produced, vec![(iron(), 3)]);
    }

    // -----------------------------------------------------------------------
    // Helpers for Demand tests
    // -----------------------------------------------------------------------

    fn make_demand(input: ItemTypeId, rate: f64) -> Processor {
        Processor::Demand(DemandProcessor {
            input_type: input,
            base_rate: fixed(rate),
            accumulated: fixed(0.0),
            consumed_total: 0,
            accepted_types: None,
        })
    }

    // -----------------------------------------------------------------------
    // Test 16: DemandProcessor consumes at base rate
    // -----------------------------------------------------------------------
    #[test]
    fn demand_consumes_at_base_rate() {
        // 2 iron per tick demand.
        let mut proc = make_demand(iron(), 2.0);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10)], 0);
        assert_eq!(r.consumed, vec![(iron(), 2)]);
        assert!(r.produced.is_empty());
        assert!(matches!(state, ProcessorState::Working { .. }));
    }

    // -----------------------------------------------------------------------
    // Test 17: DemandProcessor stalls when no input available
    // -----------------------------------------------------------------------
    #[test]
    fn demand_stalls_when_no_input() {
        let mut proc = make_demand(iron(), 2.0);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        let r = proc.tick(&mut state, &no_mods, &[], 0);
        // With no inputs and whole > 0 after accumulation, should stall.
        assert!(r.consumed.is_empty());
        assert_eq!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::MissingInputs
            }
        );
    }

    // -----------------------------------------------------------------------
    // Test 18: DemandProcessor fractional rate accumulates
    // -----------------------------------------------------------------------
    #[test]
    fn demand_fractional_rate_accumulates() {
        // 0.5 iron per tick => should consume 1 every 2 ticks.
        let mut proc = make_demand(iron(), 0.5);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Tick 1: accumulates 0.5, no whole item yet.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10)], 0);
        assert!(r.consumed.is_empty());

        // Tick 2: accumulates to 1.0, consume 1.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10)], 0);
        assert_eq!(r.consumed, vec![(iron(), 1)]);
    }

    // -----------------------------------------------------------------------
    // Test 19: DemandProcessor with speed modifier
    // -----------------------------------------------------------------------
    #[test]
    fn demand_with_speed_modifier() {
        // 1 iron per tick base, 2x speed => 2 iron per tick effective.
        let mut proc = make_demand(iron(), 1.0);
        let mut state = ProcessorState::Idle;
        let mods = vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(fixed(2.0)),
            stacking: StackingRule::default(),
        }];

        let r = proc.tick(&mut state, &mods, &[(iron(), 10)], 0);
        assert_eq!(r.consumed, vec![(iron(), 2)]);
    }

    // -----------------------------------------------------------------------
    // Test 20: StackingRule::Additive
    // -----------------------------------------------------------------------
    #[test]
    fn stacking_additive() {
        // Two speed mods of 1.5 with Additive stacking:
        // speed = 1.0 + (1.5-1.0) + (1.5-1.0) = 2.0
        let mods = vec![
            Modifier {
                id: ModifierId(0),
                kind: ModifierKind::Speed(fixed(1.5)),
                stacking: StackingRule::Additive,
            },
            Modifier {
                id: ModifierId(1),
                kind: ModifierKind::Speed(fixed(1.5)),
                stacking: StackingRule::Additive,
            },
        ];
        let resolved = ResolvedModifiers::resolve(&mods);
        assert_eq!(resolved.speed, fixed(2.0));
    }

    // -----------------------------------------------------------------------
    // Test 21: StackingRule::Diminishing
    // -----------------------------------------------------------------------
    #[test]
    fn stacking_diminishing() {
        // Speed mod of 2.0 with Diminishing:
        // speed = 1.0 * (1.0 + (2.0-1.0)/2) = 1.0 * 1.5 = 1.5
        let mods = vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(fixed(2.0)),
            stacking: StackingRule::Diminishing,
        }];
        let resolved = ResolvedModifiers::resolve(&mods);
        assert_eq!(resolved.speed, fixed(1.5));
    }

    // -----------------------------------------------------------------------
    // Test 22: StackingRule::Capped
    // -----------------------------------------------------------------------
    #[test]
    fn stacking_capped() {
        // Two speed mods: 1.5 and 2.0 with Capped => only the larger (2.0) applies.
        let mods = vec![
            Modifier {
                id: ModifierId(0),
                kind: ModifierKind::Speed(fixed(1.5)),
                stacking: StackingRule::Capped,
            },
            Modifier {
                id: ModifierId(1),
                kind: ModifierKind::Speed(fixed(2.0)),
                stacking: StackingRule::Capped,
            },
        ];
        let resolved = ResolvedModifiers::resolve(&mods);
        assert_eq!(resolved.speed, fixed(2.0));
    }

    // -----------------------------------------------------------------------
    // Test 23: StackingRule::Multiplicative (default, existing behavior)
    // -----------------------------------------------------------------------
    #[test]
    fn stacking_multiplicative_default() {
        // Two speed mods of 1.5 with Multiplicative (default):
        // speed = 1.0 * 1.5 * 1.5 = 2.25
        let mods = vec![
            Modifier {
                id: ModifierId(0),
                kind: ModifierKind::Speed(fixed(1.5)),
                stacking: StackingRule::Multiplicative,
            },
            Modifier {
                id: ModifierId(1),
                kind: ModifierKind::Speed(fixed(1.5)),
                stacking: StackingRule::Multiplicative,
            },
        ];
        let resolved = ResolvedModifiers::resolve(&mods);
        assert_eq!(resolved.speed, fixed(2.25));
    }

    // -----------------------------------------------------------------------
    // Test 24: Default stacking rule is Multiplicative
    // -----------------------------------------------------------------------
    #[test]
    fn default_stacking_rule_is_multiplicative() {
        assert_eq!(StackingRule::default(), StackingRule::Multiplicative);

        // Modifier with default stacking should be Multiplicative.
        let m = Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(fixed(1.5)),
            stacking: StackingRule::default(),
        };
        assert_eq!(m.stacking, StackingRule::Multiplicative);
    }

    // -----------------------------------------------------------------------
    // Test 25: Modifier with stacking field serialization (serde default)
    // -----------------------------------------------------------------------
    #[test]
    fn modifier_stacking_serde_default() {
        // Serialize a modifier with explicit stacking.
        let m = Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(fixed(2.0)),
            stacking: StackingRule::Additive,
        };
        let bytes = bitcode::serialize(&m).expect("serialize");
        let m2: Modifier = bitcode::deserialize(&bytes).expect("deserialize");
        assert_eq!(m2.stacking, StackingRule::Additive);

        // Serialize a modifier with default stacking (Multiplicative).
        let m_default = Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(fixed(2.0)),
            stacking: StackingRule::default(),
        };
        let bytes2 = bitcode::serialize(&m_default).expect("serialize default");
        let m3: Modifier = bitcode::deserialize(&bytes2).expect("deserialize default");
        assert_eq!(m3.stacking, StackingRule::Multiplicative);
    }

    // -----------------------------------------------------------------------
    // Test 26: Passthrough processor moves items from input to output
    // -----------------------------------------------------------------------
    #[test]
    fn passthrough_moves_items_from_input_to_output() {
        let mut proc = Processor::Passthrough;
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Provide 5 iron and 3 copper as inputs with 100 output space.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 5), (copper(), 3)], 100);
        assert_eq!(r.consumed, vec![(iron(), 5), (copper(), 3)]);
        assert_eq!(r.produced, vec![(iron(), 5), (copper(), 3)]);
        assert!(r.state_changed);
        assert!(matches!(state, ProcessorState::Working { progress: 0 }));
    }

    // -----------------------------------------------------------------------
    // Test 27: Passthrough respects output_space
    // -----------------------------------------------------------------------
    #[test]
    fn passthrough_respects_output_space() {
        let mut proc = Processor::Passthrough;
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // 10 iron available but only 3 output slots.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10)], 3);
        assert_eq!(r.consumed, vec![(iron(), 3)]);
        assert_eq!(r.produced, vec![(iron(), 3)]);
    }

    // -----------------------------------------------------------------------
    // Test 28: Passthrough idles when no inputs
    // -----------------------------------------------------------------------
    #[test]
    fn passthrough_idles_when_no_inputs() {
        let mut proc = Processor::Passthrough;
        let mut state = ProcessorState::Working { progress: 0 };
        let no_mods: Vec<Modifier> = vec![];

        let r = proc.tick(&mut state, &no_mods, &[], 100);
        assert!(r.consumed.is_empty());
        assert!(r.produced.is_empty());
        assert!(r.state_changed);
        assert_eq!(state, ProcessorState::Idle);
    }

    // -----------------------------------------------------------------------
    // Test 29: Passthrough stalls when output full
    // -----------------------------------------------------------------------
    #[test]
    fn passthrough_stalls_when_output_full() {
        let mut proc = Processor::Passthrough;
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Items available but zero output space.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 5)], 0);
        assert!(r.consumed.is_empty());
        assert!(r.produced.is_empty());
        assert!(r.state_changed);
        assert_eq!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::OutputFull
            }
        );
    }

    // -----------------------------------------------------------------------
    // Test 30: Multi-type DemandProcessor accepts multiple types
    // -----------------------------------------------------------------------
    #[test]
    fn multi_demand_accepts_multiple_types() {
        let mut proc = Processor::Demand(DemandProcessor {
            input_type: iron(),
            base_rate: fixed(2.0),
            accumulated: fixed(0.0),
            consumed_total: 0,
            accepted_types: Some(vec![iron(), copper()]),
        });
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Provide 5 iron and 5 copper.
        for _ in 0..5 {
            proc.tick(&mut state, &no_mods, &[(iron(), 5), (copper(), 5)], 0);
        }

        // Should have consumed from both types (total = 2*5 = 10 items).
        // Check that total consumed is 10.
        if let Processor::Demand(d) = &proc {
            assert_eq!(d.consumed_total, 10);
        }
    }

    // ===================================================================
    // Mutation-testing targeted tests
    // ===================================================================

    // Kill: tick_source line 348 "replace * with /" in effective_rate calculation
    // The effective rate = base_rate * speed * productivity must use multiplication.
    #[test]
    fn source_speed_modifier_doubles_rate() {
        // 1 iron per tick base, 2x speed => 2 iron per tick.
        let mut proc = make_source(iron(), 1.0, Depletion::Infinite);
        let mut state = ProcessorState::Idle;
        let mods = vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(fixed(2.0)),
            stacking: StackingRule::default(),
        }];

        let r = proc.tick(&mut state, &mods, &[], 100);
        assert_eq!(r.produced, vec![(iron(), 2)]);
    }

    // Kill: tick_source line 373 boundary condition mutations
    // "replace > with ==" / ">=" / "<" in state transition logic
    #[test]
    fn source_zero_rate_goes_idle() {
        // With rate 0.0, the source should stay Idle (effective_rate == 0).
        let mut proc = make_source(iron(), 0.0, Depletion::Infinite);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        let r = proc.tick(&mut state, &no_mods, &[], 100);
        assert!(r.produced.is_empty());
        assert_eq!(state, ProcessorState::Idle);
    }

    // Kill: tick_fixed line 409 "replace > with <" and line 410 "replace + with -/+"
    // Tests that the ceiling calculation for effective_dur works correctly.
    #[test]
    fn fixed_recipe_fractional_speed_ceils_duration() {
        // base_duration=10, speed=3.0 => 10/3 = 3.333... => ceiled to 4 ticks.
        let mut proc = make_fixed_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 10);
        let mut state = ProcessorState::Idle;
        let mods = vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(fixed(3.0)),
            stacking: StackingRule::default(),
        }];

        // Tick 1: consume inputs, start working (progress=1).
        let r = proc.tick(&mut state, &mods, &[(iron(), 10)], 10);
        assert_eq!(r.consumed, vec![(iron(), 1)]);
        assert!(matches!(state, ProcessorState::Working { progress: 1 }));

        // Ticks 2, 3: still working.
        for _ in 2..4 {
            let r = proc.tick(&mut state, &mods, &[(iron(), 9)], 10);
            assert!(r.produced.is_empty());
        }

        // Tick 4: should produce (ceil(10/3) = 4).
        let r = proc.tick(&mut state, &mods, &[(iron(), 9)], 10);
        assert_eq!(r.produced, vec![(gear(), 1)]);
        assert_eq!(state, ProcessorState::Idle);
    }

    // Kill: tick_fixed line 422 "replace < with <=" in output space check
    #[test]
    fn fixed_recipe_exact_output_space_starts() {
        // Recipe outputs 1 gear. If output_space == 1, it should start (not stall).
        let mut proc = make_fixed_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 2);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10)], 1);
        assert_eq!(r.consumed, vec![(iron(), 1)]);
        assert!(matches!(state, ProcessorState::Working { .. }));
    }

    // Kill: tick_demand line 607 "replace == with !=" and line 611 "> with >="
    // Tests the demand processor's consumed_total tracking.
    #[test]
    fn demand_consumed_total_tracks_correctly() {
        let mut proc = make_demand(iron(), 3.0);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Tick 1: consume 3 iron.
        proc.tick(&mut state, &no_mods, &[(iron(), 10)], 0);
        if let Processor::Demand(d) = &proc {
            assert_eq!(d.consumed_total, 3);
        }

        // Tick 2: consume 3 more.
        proc.tick(&mut state, &no_mods, &[(iron(), 10)], 0);
        if let Processor::Demand(d) = &proc {
            assert_eq!(d.consumed_total, 6);
        }
    }

    // Kill: tick_demand line 616 "replace - with +" in accumulated subtraction
    #[test]
    fn demand_accumulated_decreases_after_consume() {
        // Rate 2.5 per tick. After tick 1: accumulated 2.5, consume 2 => accumulated 0.5.
        // After tick 2: accumulated 0.5 + 2.5 = 3.0, consume 3 => accumulated 0.0.
        let mut proc = make_demand(iron(), 2.5);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Tick 1: consume 2.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10)], 0);
        assert_eq!(r.consumed, vec![(iron(), 2)]);

        // Tick 2: consume 3 (0.5 leftover + 2.5 new = 3.0).
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10)], 0);
        assert_eq!(r.consumed, vec![(iron(), 3)]);
    }

    // Kill: tick_passthrough line 708 "replace -= with +=" in space_remaining
    #[test]
    fn passthrough_multiple_types_respects_total_space() {
        let mut proc = Processor::Passthrough;
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // 5 iron + 5 copper, but only 7 output space.
        // Should consume 5 iron (space left: 2), then 2 copper.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 5), (copper(), 5)], 7);
        assert_eq!(r.consumed, vec![(iron(), 5), (copper(), 2)]);
        assert_eq!(r.produced, vec![(iron(), 5), (copper(), 2)]);
    }

    // Kill: tick_passthrough line 705 "replace > with >=" boundary
    #[test]
    fn passthrough_zero_qty_items_not_emitted() {
        let mut proc = Processor::Passthrough;
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Provide items with 0 quantity -- should not appear in consumed/produced.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 0)], 100);
        assert!(r.consumed.is_empty());
        assert!(r.produced.is_empty());
    }

    // Kill: tick_demand line 622 "replace == with !=" and "replace && with ||"
    #[test]
    fn demand_stalls_when_items_wanted_but_none_consumed() {
        // Multi-type demand with accepted_types, but provide items not in the list.
        let mut proc = Processor::Demand(DemandProcessor {
            input_type: iron(),
            base_rate: fixed(2.0),
            accumulated: fixed(0.0),
            consumed_total: 0,
            accepted_types: Some(vec![iron()]),
        });
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Provide only copper (not in accepted_types), with rate 2.0 (whole > 0).
        let r = proc.tick(&mut state, &no_mods, &[(copper(), 10)], 0);
        assert!(r.consumed.is_empty());
        assert_eq!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::MissingInputs
            }
        );
    }

    // Kill: Capped stacking "replace > with >=" boundary
    #[test]
    fn stacking_capped_equal_values() {
        // Two speed mods both 1.5 with Capped => should still be 1.5.
        let mods = vec![
            Modifier {
                id: ModifierId(0),
                kind: ModifierKind::Speed(fixed(1.5)),
                stacking: StackingRule::Capped,
            },
            Modifier {
                id: ModifierId(1),
                kind: ModifierKind::Speed(fixed(1.5)),
                stacking: StackingRule::Capped,
            },
        ];
        let resolved = ResolvedModifiers::resolve(&mods);
        assert_eq!(resolved.speed, fixed(1.5));
    }

    // Kill: property_processor line 536/568 "replace != with =="
    #[test]
    fn property_processor_respects_output_space() {
        // Property processor with limited output space.
        let mut proc = Processor::Property(PropertyProcessor {
            input_type: iron(),
            output_type: gear(),
            transform: PropertyTransform::Set(PropertyId(0), fixed(100.0)),
        });
        let mut state = ProcessorState::Idle;

        // 10 available but only 3 output space.
        let r = proc.tick(&mut state, &[], &[(iron(), 10)], 3);
        assert_eq!(r.consumed, vec![(iron(), 3)]);
        assert_eq!(r.produced, vec![(gear(), 3)]);
    }

    // -----------------------------------------------------------------------
    // Catalyst tests
    // -----------------------------------------------------------------------

    /// Helper to build a recipe with catalyst (consumed = false) inputs.
    fn make_recipe_with_catalyst(
        consumed_inputs: Vec<(ItemTypeId, u32)>,
        catalyst_inputs: Vec<(ItemTypeId, u32)>,
        outputs: Vec<(ItemTypeId, u32)>,
        duration: u32,
    ) -> Processor {
        let mut inputs: Vec<RecipeInput> = consumed_inputs
            .into_iter()
            .map(|(item_type, quantity)| RecipeInput {
                item_type,
                quantity,
                consumed: true,
            })
            .collect();
        inputs.extend(
            catalyst_inputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeInput {
                    item_type,
                    quantity,
                    consumed: false,
                }),
        );
        Processor::Fixed(FixedRecipe {
            inputs,
            outputs: outputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeOutput {
                    item_type,
                    quantity,
                    bonus: None,
                })
                .collect(),
            duration,
        })
    }

    #[test]
    fn catalyst_present_cycle_starts() {
        // 2 iron (consumed) + 1 wire (catalyst) -> 1 gear, 3 ticks.
        let mut proc =
            make_recipe_with_catalyst(vec![(iron(), 2)], vec![(wire(), 1)], vec![(gear(), 1)], 3);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Provide both iron and wire.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10), (wire(), 5)], 100);

        // Iron should be consumed, wire should NOT.
        assert_eq!(r.consumed, vec![(iron(), 2)]);
        assert!(matches!(state, ProcessorState::Working { .. }));
    }

    #[test]
    fn catalyst_missing_stalls() {
        // 2 iron (consumed) + 1 wire (catalyst) -> 1 gear.
        let mut proc =
            make_recipe_with_catalyst(vec![(iron(), 2)], vec![(wire(), 1)], vec![(gear(), 1)], 3);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        // Provide iron but no wire catalyst.
        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10)], 100);

        assert!(r.consumed.is_empty());
        assert!(matches!(
            state,
            ProcessorState::Stalled {
                reason: StallReason::MissingInputs
            }
        ));
    }

    #[test]
    fn catalyst_not_consumed_after_cycle() {
        // 2 iron (consumed) + 1 wire (catalyst) -> 1 gear, 1 tick (immediate).
        let mut proc =
            make_recipe_with_catalyst(vec![(iron(), 2)], vec![(wire(), 1)], vec![(gear(), 1)], 1);
        let mut state = ProcessorState::Idle;
        let no_mods: Vec<Modifier> = vec![];

        let r = proc.tick(&mut state, &no_mods, &[(iron(), 10), (wire(), 5)], 100);

        // Only iron consumed, wire stays.
        assert_eq!(r.consumed, vec![(iron(), 2)]);
        assert_eq!(r.produced, vec![(gear(), 1)]);
    }

    #[test]
    fn catalyst_with_efficiency_modifier() {
        // Efficiency modifier should affect consumed inputs but not catalysts.
        // 2 iron (consumed) + 1 wire (catalyst) -> 1 gear, 1 tick.
        let mut proc =
            make_recipe_with_catalyst(vec![(iron(), 2)], vec![(wire(), 1)], vec![(gear(), 1)], 1);
        let mut state = ProcessorState::Idle;

        // Efficiency 0.5 means consumed inputs need ceil(2 * 0.5) = 1 iron.
        let mods = vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Efficiency(fixed(0.5)),
            stacking: StackingRule::Multiplicative,
        }];

        let r = proc.tick(&mut state, &mods, &[(iron(), 10), (wire(), 5)], 100);

        // Iron quantity affected by efficiency: ceil(2 * 0.5) = 1.
        assert_eq!(r.consumed, vec![(iron(), 1)]);
        // Wire catalyst not consumed at all.
        assert_eq!(r.produced, vec![(gear(), 1)]);
    }

    #[test]
    fn backwards_compat_no_consumed_field() {
        // Verify that deserializing old RecipeInput without `consumed` defaults to true.
        let json = r#"{"item_type":0,"quantity":5}"#;
        let input: RecipeInput = serde_json::from_str(json).unwrap();
        assert!(input.consumed);
        assert_eq!(input.quantity, 5);
    }

    // -----------------------------------------------------------------------
    // Bonus output tests
    // -----------------------------------------------------------------------

    /// Helper to build a recipe with a bonus output on the first output.
    fn make_recipe_with_bonus(
        inputs: Vec<(ItemTypeId, u32)>,
        base_output: (ItemTypeId, u32),
        bonus_chance: f64,
        bonus_qty: u32,
        bonus_item: Option<ItemTypeId>,
        duration: u32,
    ) -> FixedRecipe {
        FixedRecipe {
            inputs: inputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeInput {
                    item_type,
                    quantity,
                    consumed: true,
                })
                .collect(),
            outputs: vec![RecipeOutput {
                item_type: base_output.0,
                quantity: base_output.1,
                bonus: Some(BonusOutput {
                    chance: fixed(bonus_chance),
                    quantity: bonus_qty,
                    bonus_item_type: bonus_item,
                }),
            }],
            duration,
        }
    }

    #[test]
    fn bonus_output_deterministic() {
        // Two identical recipes with the same RNG seed produce the same results.
        let recipe = make_recipe_with_bonus(vec![(iron(), 1)], (gear(), 1), 0.5, 1, None, 1);
        let mods = ResolvedModifiers::resolve(&[]);

        let mut rng_a = crate::rng::SimRng::new(42);
        let mut rng_b = crate::rng::SimRng::new(42);

        for _ in 0..100 {
            let a = apply_productivity(&recipe.outputs, &mods, Some(&mut rng_a));
            let b = apply_productivity(&recipe.outputs, &mods, Some(&mut rng_b));
            assert_eq!(a, b);
        }
    }

    #[test]
    fn bonus_output_probability() {
        // Over many cycles, bonus should trigger at roughly the expected rate.
        let recipe = make_recipe_with_bonus(vec![(iron(), 1)], (gear(), 1), 0.25, 1, None, 1);
        let mods = ResolvedModifiers::resolve(&[]);
        let mut rng = crate::rng::SimRng::new(12345);

        let trials = 10_000;
        let mut bonus_count = 0u32;
        for _ in 0..trials {
            let produced = apply_productivity(&recipe.outputs, &mods, Some(&mut rng));
            // Base output is always produced. Bonus adds a second entry.
            if produced.len() > 1 {
                bonus_count += 1;
            }
        }

        // Expect ~2500 +/- 500 (generous tolerance).
        assert!(
            (1500..=3500).contains(&bonus_count),
            "expected ~2500, got {bonus_count}"
        );
    }

    #[test]
    fn bonus_output_different_item_type() {
        // Bonus produces a different item type than the base output.
        let recipe = make_recipe_with_bonus(
            vec![(iron(), 1)],
            (gear(), 1),
            1.0, // always triggers
            2,
            Some(wire()),
            1,
        );
        let mods = ResolvedModifiers::resolve(&[]);
        let mut rng = crate::rng::SimRng::new(42);

        let produced = apply_productivity(&recipe.outputs, &mods, Some(&mut rng));
        assert_eq!(produced.len(), 2);
        assert_eq!(produced[0], (gear(), 1));
        assert_eq!(produced[1], (wire(), 2));
    }

    #[test]
    fn bonus_not_multiplied_by_productivity() {
        // Productivity modifier affects base output but not bonus.
        let recipe = make_recipe_with_bonus(
            vec![(iron(), 1)],
            (gear(), 1),
            1.0, // always triggers
            3,
            None,
            1,
        );
        let mods = ResolvedModifiers::resolve(&[Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Productivity(fixed(2.0)),
            stacking: StackingRule::Multiplicative,
        }]);
        let mut rng = crate::rng::SimRng::new(42);

        let produced = apply_productivity(&recipe.outputs, &mods, Some(&mut rng));
        // Base: 1 * 2.0 productivity = 2 gears.
        assert_eq!(produced[0], (gear(), 2));
        // Bonus: 3 gears (NOT multiplied by productivity).
        assert_eq!(produced[1], (gear(), 3));
    }

    #[test]
    fn bonus_output_no_rng_skips_bonus() {
        // Without an RNG, bonus outputs are never triggered.
        let recipe = make_recipe_with_bonus(
            vec![(iron(), 1)],
            (gear(), 1),
            1.0, // would always trigger with an RNG
            5,
            None,
            1,
        );
        let mods = ResolvedModifiers::resolve(&[]);

        let produced = apply_productivity(&recipe.outputs, &mods, None);
        // Only base output, no bonus.
        assert_eq!(produced.len(), 1);
        assert_eq!(produced[0], (gear(), 1));
    }

    // -----------------------------------------------------------------------
    // MultiRecipe processor tests
    // -----------------------------------------------------------------------

    fn make_multi_recipe(
        recipes: Vec<FixedRecipe>,
        switch_policy: RecipeSwitchPolicy,
    ) -> Processor {
        Processor::MultiRecipe(MultiRecipeProcessor {
            active_recipe: 0,
            recipes,
            switch_policy,
            pending_switch: None,
            in_progress_inputs: Vec::new(),
        })
    }

    fn make_simple_fixed(
        inputs: Vec<(ItemTypeId, u32)>,
        outputs: Vec<(ItemTypeId, u32)>,
        duration: u32,
    ) -> FixedRecipe {
        FixedRecipe {
            inputs: inputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeInput {
                    item_type,
                    quantity,
                    consumed: true,
                })
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeOutput {
                    item_type,
                    quantity,
                    bonus: None,
                })
                .collect(),
            duration,
        }
    }

    #[test]
    fn multi_recipe_idle_switch() {
        // When idle, switching recipes happens immediately.
        let r0 = make_simple_fixed(vec![(iron(), 1)], vec![(gear(), 1)], 3);
        let r1 = make_simple_fixed(vec![(copper(), 1)], vec![(wire(), 1)], 3);
        let mut proc = make_multi_recipe(vec![r0, r1], RecipeSwitchPolicy::CompleteFirst);
        let mut state = ProcessorState::Idle;

        // Confirm active recipe is 0.
        if let Processor::MultiRecipe(ref m) = proc {
            assert_eq!(m.active_recipe, 0);
        }

        // Switch to recipe 1 while idle — should happen via pending_switch + tick.
        if let Processor::MultiRecipe(ref mut m) = proc {
            m.pending_switch = Some(1);
        }

        // Tick: idle + pending_switch => switch happens.
        let _ = proc.tick(&mut state, &[], &[(copper(), 10)], 100);

        if let Processor::MultiRecipe(ref m) = proc {
            assert_eq!(m.active_recipe, 1);
            assert!(m.pending_switch.is_none());
        }
    }

    #[test]
    fn multi_recipe_working_complete_first() {
        // With CompleteFirst policy, recipe switch is deferred until cycle completes.
        let r0 = make_simple_fixed(vec![(iron(), 1)], vec![(gear(), 1)], 3);
        let r1 = make_simple_fixed(vec![(copper(), 1)], vec![(wire(), 1)], 3);
        let mut proc = make_multi_recipe(vec![r0, r1], RecipeSwitchPolicy::CompleteFirst);
        let mut state = ProcessorState::Idle;

        // Tick 1: start recipe 0.
        let r = proc.tick(&mut state, &[], &[(iron(), 10)], 100);
        assert_eq!(r.consumed, vec![(iron(), 1)]);
        assert!(matches!(state, ProcessorState::Working { .. }));

        // Set pending switch mid-craft.
        if let Processor::MultiRecipe(ref mut m) = proc {
            m.pending_switch = Some(1);
        }

        // Tick 2: still working on recipe 0.
        let r = proc.tick(&mut state, &[], &[(iron(), 10)], 100);
        assert!(r.produced.is_empty());
        if let Processor::MultiRecipe(ref m) = proc {
            assert_eq!(m.active_recipe, 0); // Not switched yet.
            assert_eq!(m.pending_switch, Some(1));
        }

        // Tick 3: recipe 0 completes, then switch to recipe 1.
        let r = proc.tick(&mut state, &[], &[(iron(), 10)], 100);
        assert_eq!(r.produced, vec![(gear(), 1)]);
        if let Processor::MultiRecipe(ref m) = proc {
            assert_eq!(m.active_recipe, 1); // Switched!
            assert!(m.pending_switch.is_none());
        }
    }

    #[test]
    fn multi_recipe_crafts_active_recipe() {
        // Verify MultiRecipe crafts the active recipe correctly.
        let r0 = make_simple_fixed(vec![(iron(), 2)], vec![(gear(), 1)], 2);
        let r1 = make_simple_fixed(vec![(copper(), 1)], vec![(wire(), 3)], 2);
        let mut proc = make_multi_recipe(vec![r0, r1], RecipeSwitchPolicy::CompleteFirst);
        let mut state = ProcessorState::Idle;

        // Recipe 0: 2 iron -> 1 gear in 2 ticks.
        let r = proc.tick(&mut state, &[], &[(iron(), 10)], 100);
        assert_eq!(r.consumed, vec![(iron(), 2)]);

        let r = proc.tick(&mut state, &[], &[(iron(), 8)], 100);
        assert_eq!(r.produced, vec![(gear(), 1)]);
    }

    #[test]
    fn multi_recipe_modifiers_apply() {
        // Speed modifier should affect the active recipe in a MultiRecipe processor.
        let r0 = make_simple_fixed(vec![(iron(), 1)], vec![(gear(), 1)], 10);
        let mut proc = make_multi_recipe(vec![r0], RecipeSwitchPolicy::CompleteFirst);
        let mut state = ProcessorState::Idle;
        let mods = vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(fixed(2.0)),
            stacking: StackingRule::default(),
        }];

        // Tick 1: consume.
        let r = proc.tick(&mut state, &mods, &[(iron(), 10)], 100);
        assert_eq!(r.consumed, vec![(iron(), 1)]);

        // With 2x speed, 10-tick recipe takes 5 ticks.
        for _ in 2..5 {
            proc.tick(&mut state, &mods, &[(iron(), 9)], 100);
        }

        // Tick 5: produce.
        let r = proc.tick(&mut state, &mods, &[(iron(), 9)], 100);
        assert_eq!(r.produced, vec![(gear(), 1)]);
    }

    #[test]
    fn multi_recipe_out_of_bounds_recipe() {
        // If active_recipe is out of bounds, tick should return default (no-op).
        let r0 = make_simple_fixed(vec![(iron(), 1)], vec![(gear(), 1)], 2);
        let mut proc = make_multi_recipe(vec![r0], RecipeSwitchPolicy::CompleteFirst);
        let mut state = ProcessorState::Idle;

        // Force active_recipe out of bounds.
        if let Processor::MultiRecipe(ref mut m) = proc {
            m.active_recipe = 99;
        }

        let r = proc.tick(&mut state, &[], &[(iron(), 10)], 100);
        assert!(r.consumed.is_empty());
        assert!(r.produced.is_empty());
    }

    #[test]
    fn multi_recipe_serialization_round_trip() {
        let r0 = make_simple_fixed(vec![(iron(), 1)], vec![(gear(), 1)], 5);
        let r1 = make_simple_fixed(vec![(copper(), 2)], vec![(wire(), 1)], 3);
        let multi = MultiRecipeProcessor {
            recipes: vec![r0, r1],
            active_recipe: 1,
            switch_policy: RecipeSwitchPolicy::RefundInputs,
            pending_switch: Some(0),
            in_progress_inputs: vec![(copper(), 2)],
        };

        let bytes = bitcode::serialize(&multi).expect("serialize");
        let restored: MultiRecipeProcessor = bitcode::deserialize(&bytes).expect("deserialize");

        assert_eq!(restored.active_recipe, 1);
        assert_eq!(restored.recipes.len(), 2);
        assert_eq!(restored.switch_policy, RecipeSwitchPolicy::RefundInputs);
        assert_eq!(restored.pending_switch, Some(0));
        assert_eq!(restored.in_progress_inputs, vec![(copper(), 2)]);
    }
}
