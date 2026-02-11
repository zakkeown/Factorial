//! Serde data file structs for game content definitions.
//!
//! These structs define the on-disk format for items, recipes, buildings,
//! and module configurations. They are deserialized from RON, JSON, or TOML
//! data files and then resolved into engine types by the loader.

use serde::Deserialize;

// ===========================================================================
// Core: Items
// ===========================================================================

/// An item type definition in a data file.
#[derive(Debug, Clone, Deserialize)]
pub struct ItemData {
    pub name: String,
    #[serde(default)]
    pub properties: Vec<PropertyData>,
}

/// A property on an item type.
#[derive(Debug, Clone, Deserialize)]
pub struct PropertyData {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: PropertyType,
    pub default: f64,
}

/// The storage type of a property value.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Fixed64,
    Fixed32,
    U32,
    U8,
}

// ===========================================================================
// Core: Recipes
// ===========================================================================

/// A recipe input entry, supporting both short tuple form and full form with
/// optional `consumed` flag for catalysts.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RecipeInputData {
    /// Short form: `("item_name", quantity)` — consumed by default.
    Short(String, u32),
    /// Full form with explicit fields, including catalyst support.
    Full {
        item: String,
        quantity: u32,
        #[serde(default = "default_true")]
        consumed: bool,
    },
}

fn default_true() -> bool {
    true
}

/// A recipe definition in a data file.
#[derive(Debug, Clone, Deserialize)]
pub struct RecipeData {
    pub name: String,
    pub inputs: Vec<RecipeInputData>,
    pub outputs: Vec<(String, u32)>,
    pub duration: u64,
}

// ===========================================================================
// Core: Buildings
// ===========================================================================

/// A building definition in a data file.
#[derive(Debug, Clone, Deserialize)]
pub struct BuildingData {
    pub name: String,
    pub processor: ProcessorData,
    #[serde(default = "default_footprint")]
    pub footprint: FootprintData,
    #[serde(default)]
    pub inventories: InventoryData,
}

/// The footprint (size) of a building on the grid.
#[derive(Debug, Clone, Deserialize)]
pub struct FootprintData {
    pub width: u32,
    pub height: u32,
}

fn default_footprint() -> FootprintData {
    FootprintData {
        width: 1,
        height: 1,
    }
}

/// Inventory capacities for a building.
#[derive(Debug, Clone, Deserialize)]
pub struct InventoryData {
    #[serde(default = "default_capacity")]
    pub input_capacity: u32,
    #[serde(default = "default_capacity")]
    pub output_capacity: u32,
}

fn default_capacity() -> u32 {
    100
}

impl Default for InventoryData {
    fn default() -> Self {
        Self {
            input_capacity: default_capacity(),
            output_capacity: default_capacity(),
        }
    }
}

/// The processor type for a building.
#[derive(Debug, Clone, Deserialize)]
pub enum ProcessorData {
    Source {
        item: String,
        rate: f64,
    },
    Recipe {
        recipe: String,
    },
    Demand {
        items: Vec<String>,
    },
    Passthrough,
    MultiRecipe {
        recipes: Vec<String>,
        #[serde(default)]
        default_recipe: Option<String>,
        #[serde(default)]
        switch_policy: Option<String>,
    },
}

// ===========================================================================
// Module: Power
// ===========================================================================

/// Power module configuration from a data file.
#[derive(Debug, Clone, Deserialize)]
pub struct PowerData {
    #[serde(default)]
    pub generators: Vec<PowerGeneratorData>,
    #[serde(default)]
    pub consumers: Vec<PowerConsumerData>,
    #[serde(default)]
    pub storage: Vec<PowerStorageData>,
}

/// A power generator definition.
#[derive(Debug, Clone, Deserialize)]
pub struct PowerGeneratorData {
    pub building: String,
    pub output: f64,
    #[serde(default)]
    pub priority: PriorityData,
}

/// A power consumer definition.
#[derive(Debug, Clone, Deserialize)]
pub struct PowerConsumerData {
    pub building: String,
    pub draw: f64,
    #[serde(default)]
    pub priority: PriorityData,
}

/// A power storage definition.
#[derive(Debug, Clone, Deserialize)]
pub struct PowerStorageData {
    pub building: String,
    pub capacity: f64,
    pub charge_rate: f64,
    pub discharge_rate: f64,
}

/// Priority level for power producers/consumers.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorityData {
    High,
    #[default]
    Medium,
    Low,
}

// ===========================================================================
// Module: Fluid
// ===========================================================================

/// Fluid module configuration from a data file.
#[derive(Debug, Clone, Deserialize)]
pub struct FluidData {
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub producers: Vec<FluidProducerData>,
    #[serde(default)]
    pub consumers: Vec<FluidConsumerData>,
    #[serde(default)]
    pub storage: Vec<FluidStorageData>,
}

/// A fluid producer definition.
#[derive(Debug, Clone, Deserialize)]
pub struct FluidProducerData {
    pub building: String,
    pub fluid: String,
    pub rate: f64,
}

/// A fluid consumer definition.
#[derive(Debug, Clone, Deserialize)]
pub struct FluidConsumerData {
    pub building: String,
    pub fluid: String,
    pub rate: f64,
}

/// A fluid storage definition.
#[derive(Debug, Clone, Deserialize)]
pub struct FluidStorageData {
    pub building: String,
    pub fluid: String,
    pub capacity: f64,
    pub fill_rate: f64,
    pub drain_rate: f64,
}

// ===========================================================================
// Module: Tech Tree
// ===========================================================================

/// A technology (research) definition in a data file.
#[derive(Debug, Clone, Deserialize)]
pub struct ResearchData {
    pub name: String,
    pub cost: ResearchCostData,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub unlocks: Vec<UnlockData>,
    #[serde(default)]
    pub repeatable: bool,
    #[serde(default)]
    pub cost_scaling: Option<CostScalingData>,
}

/// How a technology's research cost is paid.
#[derive(Debug, Clone, Deserialize)]
pub enum ResearchCostData {
    Points { amount: u32 },
    Items { items: Vec<(String, u32)> },
    Delivery { items: Vec<(String, u32)> },
    Rate { points_per_tick: f64, total: u32 },
}

/// What completing a technology unlocks.
#[derive(Debug, Clone, Deserialize)]
pub enum UnlockData {
    Building(String),
    Recipe(String),
    Custom(String),
}

/// How the cost of a repeatable technology scales with each completion.
#[derive(Debug, Clone, Deserialize)]
pub enum CostScalingData {
    Linear { base: u32, increment: u32 },
    Exponential { base: u32, multiplier: f64 },
}

// ===========================================================================
// Module: Logic
// ===========================================================================

/// Logic module configuration from a data file.
#[derive(Debug, Clone, Deserialize)]
pub struct LogicData {
    #[serde(default)]
    pub circuit_controlled: Vec<CircuitControlData>,
    #[serde(default)]
    pub constant_combinators: Vec<ConstantCombinatorData>,
}

/// A circuit control definition binding a condition to a building.
#[derive(Debug, Clone, Deserialize)]
pub struct CircuitControlData {
    pub building: String,
    pub wire: WireColorData,
    pub condition: ConditionData,
}

/// Wire color for circuit networks.
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum WireColorData {
    Red,
    Green,
}

/// A condition for circuit control.
#[derive(Debug, Clone, Deserialize)]
pub struct ConditionData {
    pub signal: String,
    pub op: String,
    pub value: i64,
}

/// A constant combinator definition.
#[derive(Debug, Clone, Deserialize)]
pub struct ConstantCombinatorData {
    pub building: String,
    pub signals: Vec<(String, i32)>,
}

// ===========================================================================
// TOML wrappers (TOML does not support top-level arrays)
// ===========================================================================

/// Wrapper for a list of items in TOML format.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlItems {
    pub items: Vec<ItemData>,
}

/// Wrapper for a list of recipes in TOML format.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlRecipes {
    pub recipes: Vec<RecipeData>,
}

/// Wrapper for a list of buildings in TOML format.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlBuildings {
    pub buildings: Vec<BuildingData>,
}

/// Wrapper for a list of research technologies in TOML format.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlResearch {
    pub research: Vec<ResearchData>,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Core schema: RON deserialization
    // -----------------------------------------------------------------------

    #[test]
    fn item_data_from_ron() {
        let ron = r#"
            (
                name: "iron_ore",
                properties: [
                    (name: "purity", type: fixed64, default: 1.0),
                ],
            )
        "#;
        let item: ItemData = ron::from_str(ron).unwrap();
        assert_eq!(item.name, "iron_ore");
        assert_eq!(item.properties.len(), 1);
        assert_eq!(item.properties[0].name, "purity");
        assert!((item.properties[0].default - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn item_data_no_properties_from_ron() {
        let ron = r#"(name: "copper_ore")"#;
        let item: ItemData = ron::from_str(ron).unwrap();
        assert_eq!(item.name, "copper_ore");
        assert!(item.properties.is_empty());
    }

    #[test]
    fn recipe_data_from_ron() {
        let ron = r#"
            (
                name: "smelt_iron",
                inputs: [("iron_ore", 1)],
                outputs: [("iron_plate", 1)],
                duration: 60,
            )
        "#;
        let recipe: RecipeData = ron::from_str(ron).unwrap();
        assert_eq!(recipe.name, "smelt_iron");
        assert_eq!(recipe.inputs.len(), 1);
        match &recipe.inputs[0] {
            RecipeInputData::Short(name, qty) => {
                assert_eq!(name, "iron_ore");
                assert_eq!(*qty, 1);
            }
            other => panic!("expected Short variant, got {other:?}"),
        }
        assert_eq!(recipe.outputs[0].0, "iron_plate");
        assert_eq!(recipe.duration, 60);
    }

    #[test]
    fn building_data_from_ron() {
        let ron = r#"
            (
                name: "smelter",
                processor: Recipe(recipe: "smelt_iron"),
                footprint: (width: 2, height: 2),
                inventories: (input_capacity: 50, output_capacity: 50),
            )
        "#;
        let building: BuildingData = ron::from_str(ron).unwrap();
        assert_eq!(building.name, "smelter");
        assert!(matches!(building.processor, ProcessorData::Recipe { .. }));
        assert_eq!(building.footprint.width, 2);
        assert_eq!(building.footprint.height, 2);
        assert_eq!(building.inventories.input_capacity, 50);
    }

    #[test]
    fn building_data_defaults_from_ron() {
        let ron = r#"
            (
                name: "junction",
                processor: Passthrough,
            )
        "#;
        let building: BuildingData = ron::from_str(ron).unwrap();
        assert_eq!(building.name, "junction");
        assert!(matches!(building.processor, ProcessorData::Passthrough));
        assert_eq!(building.footprint.width, 1);
        assert_eq!(building.footprint.height, 1);
        assert_eq!(building.inventories.input_capacity, 100);
        assert_eq!(building.inventories.output_capacity, 100);
    }

    #[test]
    fn building_source_processor_from_ron() {
        let ron = r#"
            (
                name: "iron_mine",
                processor: Source(item: "iron_ore", rate: 0.5),
            )
        "#;
        let building: BuildingData = ron::from_str(ron).unwrap();
        assert!(matches!(
            building.processor,
            ProcessorData::Source { ref item, rate } if item == "iron_ore" && (rate - 0.5).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn building_demand_processor_from_ron() {
        let ron = r#"
            (
                name: "sink",
                processor: Demand(items: ["iron_plate", "copper_plate"]),
            )
        "#;
        let building: BuildingData = ron::from_str(ron).unwrap();
        match &building.processor {
            ProcessorData::Demand { items } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], "iron_plate");
                assert_eq!(items[1], "copper_plate");
            }
            other => panic!("expected Demand, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Core schema: JSON deserialization
    // -----------------------------------------------------------------------

    #[test]
    fn item_data_from_json() {
        let json = r#"{
            "name": "iron_ore",
            "properties": [
                {"name": "purity", "type": "fixed64", "default": 1.0}
            ]
        }"#;
        let item: ItemData = serde_json::from_str(json).unwrap();
        assert_eq!(item.name, "iron_ore");
        assert_eq!(item.properties.len(), 1);
    }

    #[test]
    fn recipe_data_from_json() {
        let json = r#"{
            "name": "smelt_iron",
            "inputs": [["iron_ore", 1]],
            "outputs": [["iron_plate", 1]],
            "duration": 60
        }"#;
        let recipe: RecipeData = serde_json::from_str(json).unwrap();
        assert_eq!(recipe.name, "smelt_iron");
        match &recipe.inputs[0] {
            RecipeInputData::Short(name, qty) => {
                assert_eq!(name, "iron_ore");
                assert_eq!(*qty, 1);
            }
            other => panic!("expected Short variant, got {other:?}"),
        }
    }

    #[test]
    fn building_data_from_json() {
        let json = r#"{
            "name": "smelter",
            "processor": {"Recipe": {"recipe": "smelt_iron"}},
            "footprint": {"width": 3, "height": 3},
            "inventories": {"input_capacity": 200, "output_capacity": 200}
        }"#;
        let building: BuildingData = serde_json::from_str(json).unwrap();
        assert_eq!(building.name, "smelter");
        assert_eq!(building.footprint.width, 3);
    }

    // -----------------------------------------------------------------------
    // Core schema: TOML deserialization (requires wrapper structs)
    // -----------------------------------------------------------------------

    #[test]
    fn items_from_toml() {
        let toml_str = r#"
            [[items]]
            name = "iron_ore"

            [[items]]
            name = "copper_ore"
        "#;
        let wrapper: TomlItems = toml::from_str(toml_str).unwrap();
        assert_eq!(wrapper.items.len(), 2);
        assert_eq!(wrapper.items[0].name, "iron_ore");
        assert_eq!(wrapper.items[1].name, "copper_ore");
    }

    #[test]
    fn recipes_from_toml() {
        let toml_str = r#"
            [[recipes]]
            name = "smelt_iron"
            inputs = [["iron_ore", 1]]
            outputs = [["iron_plate", 1]]
            duration = 60
        "#;
        let wrapper: TomlRecipes = toml::from_str(toml_str).unwrap();
        assert_eq!(wrapper.recipes.len(), 1);
        assert_eq!(wrapper.recipes[0].name, "smelt_iron");
    }

    // -----------------------------------------------------------------------
    // Module schema: Power RON deserialization
    // -----------------------------------------------------------------------

    #[test]
    fn power_data_from_ron() {
        let ron = r#"
            (
                generators: [
                    (building: "coal_plant", output: 100.0, priority: high),
                ],
                consumers: [
                    (building: "assembler", draw: 50.0),
                ],
                storage: [
                    (building: "battery", capacity: 1000.0, charge_rate: 50.0, discharge_rate: 50.0),
                ],
            )
        "#;
        let power: PowerData = ron::from_str(ron).unwrap();
        assert_eq!(power.generators.len(), 1);
        assert_eq!(power.generators[0].building, "coal_plant");
        assert!((power.generators[0].output - 100.0).abs() < f64::EPSILON);
        assert!(matches!(power.generators[0].priority, PriorityData::High));
        assert_eq!(power.consumers.len(), 1);
        assert_eq!(power.consumers[0].building, "assembler");
        assert!(matches!(power.consumers[0].priority, PriorityData::Medium));
        assert_eq!(power.storage.len(), 1);
        assert_eq!(power.storage[0].building, "battery");
    }

    // -----------------------------------------------------------------------
    // Module schema: Fluid RON deserialization
    // -----------------------------------------------------------------------

    #[test]
    fn fluid_data_from_ron() {
        let ron = r#"
            (
                types: ["water", "steam"],
                producers: [
                    (building: "pump", fluid: "water", rate: 100.0),
                ],
                consumers: [
                    (building: "boiler", fluid: "water", rate: 50.0),
                ],
                storage: [
                    (building: "tank", fluid: "water", capacity: 10000.0, fill_rate: 200.0, drain_rate: 200.0),
                ],
            )
        "#;
        let fluid: FluidData = ron::from_str(ron).unwrap();
        assert_eq!(fluid.types, vec!["water", "steam"]);
        assert_eq!(fluid.producers.len(), 1);
        assert_eq!(fluid.producers[0].building, "pump");
        assert_eq!(fluid.producers[0].fluid, "water");
        assert_eq!(fluid.consumers.len(), 1);
        assert_eq!(fluid.storage.len(), 1);
        assert_eq!(fluid.storage[0].fluid, "water");
    }

    // -----------------------------------------------------------------------
    // Module schema: Tech Tree RON deserialization
    // -----------------------------------------------------------------------

    #[test]
    fn research_data_from_ron() {
        let ron = r#"
            (
                name: "Automation",
                cost: Points(amount: 100),
                prerequisites: ["Iron Smelting"],
                unlocks: [Building("assembler"), Recipe("gear_recipe")],
                repeatable: false,
            )
        "#;
        let research: ResearchData = ron::from_str(ron).unwrap();
        assert_eq!(research.name, "Automation");
        assert!(matches!(
            research.cost,
            ResearchCostData::Points { amount: 100 }
        ));
        assert_eq!(research.prerequisites, vec!["Iron Smelting"]);
        assert_eq!(research.unlocks.len(), 2);
        assert!(matches!(
            &research.unlocks[0],
            UnlockData::Building(b) if b == "assembler"
        ));
        assert!(!research.repeatable);
        assert!(research.cost_scaling.is_none());
    }

    #[test]
    fn research_items_cost_from_ron() {
        let ron = r#"
            (
                name: "Steel",
                cost: Items(items: [("red_science", 50), ("green_science", 50)]),
                unlocks: [Custom("steel_alloys")],
            )
        "#;
        let research: ResearchData = ron::from_str(ron).unwrap();
        match &research.cost {
            ResearchCostData::Items { items } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].0, "red_science");
                assert_eq!(items[0].1, 50);
            }
            other => panic!("expected Items, got: {other:?}"),
        }
    }

    #[test]
    fn research_repeatable_with_scaling_from_ron() {
        let ron = r#"
            (
                name: "Mining Productivity",
                cost: Points(amount: 1000),
                unlocks: [Custom("mining_bonus")],
                repeatable: true,
                cost_scaling: Some(Linear(base: 1000, increment: 500)),
            )
        "#;
        let research: ResearchData = ron::from_str(ron).unwrap();
        assert!(research.repeatable);
        match &research.cost_scaling {
            Some(CostScalingData::Linear { base, increment }) => {
                assert_eq!(*base, 1000);
                assert_eq!(*increment, 500);
            }
            other => panic!("expected Linear scaling, got: {other:?}"),
        }
    }

    #[test]
    fn research_delivery_cost_from_ron() {
        let ron = r#"
            (
                name: "Logistics",
                cost: Delivery(items: [("iron_plate", 100)]),
                unlocks: [],
            )
        "#;
        let research: ResearchData = ron::from_str(ron).unwrap();
        match &research.cost {
            ResearchCostData::Delivery { items } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].0, "iron_plate");
                assert_eq!(items[0].1, 100);
            }
            other => panic!("expected Delivery, got: {other:?}"),
        }
    }

    #[test]
    fn research_rate_cost_from_ron() {
        let ron = r#"
            (
                name: "Continuous Research",
                cost: Rate(points_per_tick: 10.0, total: 500),
                unlocks: [],
            )
        "#;
        let research: ResearchData = ron::from_str(ron).unwrap();
        match &research.cost {
            ResearchCostData::Rate {
                points_per_tick,
                total,
            } => {
                assert!((points_per_tick - 10.0).abs() < f64::EPSILON);
                assert_eq!(*total, 500);
            }
            other => panic!("expected Rate, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Module schema: Logic RON deserialization
    // -----------------------------------------------------------------------

    #[test]
    fn logic_data_from_ron() {
        let ron = r#"
            (
                circuit_controlled: [
                    (
                        building: "inserter",
                        wire: Red,
                        condition: (signal: "iron_plate", op: ">", value: 50),
                    ),
                ],
                constant_combinators: [
                    (
                        building: "combinator_1",
                        signals: [("iron_plate", 100), ("copper_plate", 50)],
                    ),
                ],
            )
        "#;
        let logic: LogicData = ron::from_str(ron).unwrap();
        assert_eq!(logic.circuit_controlled.len(), 1);
        assert_eq!(logic.circuit_controlled[0].building, "inserter");
        assert!(matches!(
            logic.circuit_controlled[0].wire,
            WireColorData::Red
        ));
        assert_eq!(logic.circuit_controlled[0].condition.signal, "iron_plate");
        assert_eq!(logic.circuit_controlled[0].condition.op, ">");
        assert_eq!(logic.circuit_controlled[0].condition.value, 50);
        assert_eq!(logic.constant_combinators.len(), 1);
        assert_eq!(logic.constant_combinators[0].building, "combinator_1");
        assert_eq!(logic.constant_combinators[0].signals.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Module schema: Power default priority
    // -----------------------------------------------------------------------

    #[test]
    fn power_consumer_default_priority_from_ron() {
        let ron = r#"(building: "assembler", draw: 50.0)"#;
        let consumer: PowerConsumerData = ron::from_str(ron).unwrap();
        assert!(matches!(consumer.priority, PriorityData::Medium));
    }

    // -----------------------------------------------------------------------
    // Module schema: Fluid empty defaults
    // -----------------------------------------------------------------------

    #[test]
    fn fluid_data_empty_defaults_from_ron() {
        let ron = r#"(types: [])"#;
        let fluid: FluidData = ron::from_str(ron).unwrap();
        assert!(fluid.types.is_empty());
        assert!(fluid.producers.is_empty());
        assert!(fluid.consumers.is_empty());
        assert!(fluid.storage.is_empty());
    }

    // -----------------------------------------------------------------------
    // Exponential cost scaling
    // -----------------------------------------------------------------------

    #[test]
    fn exponential_cost_scaling_from_ron() {
        let ron = r#"
            (
                name: "White Science",
                cost: Points(amount: 100),
                unlocks: [],
                repeatable: true,
                cost_scaling: Some(Exponential(base: 100, multiplier: 2.0)),
            )
        "#;
        let research: ResearchData = ron::from_str(ron).unwrap();
        match &research.cost_scaling {
            Some(CostScalingData::Exponential { base, multiplier }) => {
                assert_eq!(*base, 100);
                assert!((*multiplier - 2.0).abs() < f64::EPSILON);
            }
            other => panic!("expected Exponential scaling, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // TOML wrapper for buildings and research
    // -----------------------------------------------------------------------

    #[test]
    fn buildings_from_toml() {
        let toml_str = r#"
            [[buildings]]
            name = "junction"

            [buildings.processor]
            Passthrough = {}

            [buildings.footprint]
            width = 1
            height = 1
        "#;
        let wrapper: TomlBuildings = toml::from_str(toml_str).unwrap();
        assert_eq!(wrapper.buildings.len(), 1);
        assert_eq!(wrapper.buildings[0].name, "junction");
    }

    #[test]
    fn research_from_toml() {
        let toml_str = r#"
            [[research]]
            name = "Automation"
            repeatable = false

            [research.cost]
            Points = { amount = 100 }
        "#;
        let wrapper: TomlResearch = toml::from_str(toml_str).unwrap();
        assert_eq!(wrapper.research.len(), 1);
        assert_eq!(wrapper.research[0].name, "Automation");
    }

    // -----------------------------------------------------------------------
    // Property type variants
    // -----------------------------------------------------------------------

    #[test]
    fn property_type_variants_from_ron() {
        for (ron_val, _expected_name) in [
            ("fixed64", "Fixed64"),
            ("fixed32", "Fixed32"),
            ("u32", "U32"),
            ("u8", "U8"),
        ] {
            let ron = format!(r#"(name: "test", type: {}, default: 0.0)"#, ron_val);
            let prop: PropertyData = ron::from_str(&ron).unwrap();
            assert_eq!(prop.name, "test");
        }
    }
}
