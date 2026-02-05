use crate::fixed::Fixed64;
use crate::id::{ItemTypeId, ModifierId, PropertyId};

// ---------------------------------------------------------------------------
// Recipe types
// ---------------------------------------------------------------------------

/// An input requirement for a fixed recipe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeInput {
    pub item_type: ItemTypeId,
    pub quantity: u32,
}

/// An output product of a fixed recipe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeOutput {
    pub item_type: ItemTypeId,
    pub quantity: u32,
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

/// Top-level processor enum. Dispatches via enum match (no trait objects).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Processor {
    Source(SourceProcessor),
    Fixed(FixedRecipe),
    Property(PropertyProcessor),
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
    Working { progress: u32 },
    Stalled { reason: StallReason },
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

/// A modifier instance applied to a processor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Modifier {
    pub id: ModifierId,
    pub kind: ModifierKind,
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
    /// category multiplicatively.
    fn resolve(modifiers: &[Modifier]) -> Self {
        let one = Fixed64::from_num(1);

        // Sort indices by ModifierId for determinism.
        let mut sorted: Vec<&Modifier> = modifiers.iter().collect();
        sorted.sort_by_key(|m| m.id);

        let mut speed = one;
        let mut productivity = one;
        let mut efficiency = one;

        for m in &sorted {
            match &m.kind {
                ModifierKind::Speed(v) => speed *= *v,
                ModifierKind::Productivity(v) => productivity *= *v,
                ModifierKind::Efficiency(v) => efficiency *= *v,
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
        match self {
            Processor::Source(src) => tick_source(src, state, modifiers, output_space),
            Processor::Fixed(recipe) => {
                tick_fixed(recipe, state, modifiers, available_inputs, output_space)
            }
            Processor::Property(prop) => {
                tick_property(prop, state, available_inputs, output_space)
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
        if *state != (ProcessorState::Stalled { reason: StallReason::Depleted }) {
            *state = ProcessorState::Stalled {
                reason: StallReason::Depleted,
            };
            result.state_changed = true;
        }
        return result;
    }

    if output_space == 0 {
        if *state != (ProcessorState::Stalled { reason: StallReason::OutputFull }) {
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
                let eff_qty_fixed =
                    Fixed64::from_num(input.quantity) * mods.efficiency;
                let eff_qty = {
                    let raw: i64 = eff_qty_fixed.to_num();
                    let frac = eff_qty_fixed.frac();
                    if frac > Fixed64::from_num(0) {
                        (raw + 1).max(1) as u32
                    } else {
                        raw.max(1) as u32
                    }
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
                to_consume.push((input.item_type, eff_qty));
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
                let produced = apply_productivity(&recipe.outputs, &mods);
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
                let produced = apply_productivity(&recipe.outputs, &mods);
                result.produced = produced;
                *state = ProcessorState::Idle;
                result.state_changed = true;
            }
        }
    }

    result
}

/// Apply productivity modifier to outputs. Productivity > 1.0 means extra
/// items. We round down fractional bonus items for simplicity.
fn apply_productivity(outputs: &[RecipeOutput], mods: &ResolvedModifiers) -> Vec<(ItemTypeId, u32)> {
    outputs
        .iter()
        .map(|o| {
            let base = Fixed64::from_num(o.quantity);
            let boosted = base * mods.productivity;
            let qty = boosted.to_num::<i64>().max(1) as u32;
            (o.item_type, qty)
        })
        .collect()
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

    // Process one item per tick. The actual property transformation would be
    // applied by the caller using `prop.transform`; we just signal consume/produce.
    let qty = available.min(output_space);
    result.consumed.push((prop.input_type, qty));
    result.produced.push((prop.output_type, qty));

    let new_state = ProcessorState::Working { progress: 0 };
    if *state != new_state {
        *state = new_state;
        result.state_changed = true;
    }

    result
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
                })
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(item_type, quantity)| RecipeOutput {
                    item_type,
                    quantity,
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
        let mut proc = make_fixed_recipe(
            vec![(copper(), 1)],
            vec![(gear(), 2), (wire(), 3)],
            5,
        );
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
            },
            Modifier {
                id: ModifierId(1),
                kind: ModifierKind::Speed(fixed(2.0)),
            },
        ];
        let mods_ordered = vec![
            Modifier {
                id: ModifierId(1),
                kind: ModifierKind::Speed(fixed(2.0)),
            },
            Modifier {
                id: ModifierId(5),
                kind: ModifierKind::Speed(fixed(1.5)),
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
}
