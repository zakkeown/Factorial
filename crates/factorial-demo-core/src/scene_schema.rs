use serde::Deserialize;

/// Top-level scene definition loaded from `scene.ron`.
#[derive(Debug, Clone, Deserialize)]
pub struct SceneData {
    pub title: String,
    pub description: String,
    pub tier: u8,
    pub order: u8,
    #[serde(default)]
    pub tags: Vec<String>,
    pub nodes: Vec<SceneNode>,
    #[serde(default)]
    pub edges: Vec<SceneEdge>,
    #[serde(default)]
    pub simulation: SimulationConfig,
    #[serde(default)]
    pub modules: ModuleWiring,
}

/// A node in the scene graph, corresponding to a building.
#[derive(Debug, Clone, Deserialize)]
pub struct SceneNode {
    /// Unique ID within this scene (used by edges to reference nodes).
    pub id: String,
    /// Must match a building name in `buildings.ron`.
    pub building: String,
    /// 2D layout position for rendering.
    pub position: (f32, f32),
    /// Display label (defaults to building name if absent).
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Visual hint for rendering: "source", "processor", "sink", etc.
    #[serde(default)]
    pub visual_hint: Option<String>,
    /// Override inventory capacities for this specific node.
    #[serde(default)]
    pub inventory_override: Option<InventoryOverride>,
    #[serde(default)]
    pub modifiers: Vec<ModifierData>,
    /// For multi-recipe processors, which recipe index to activate.
    #[serde(default)]
    pub active_recipe: Option<usize>,
}

/// An edge connecting two scene nodes via a transport.
#[derive(Debug, Clone, Deserialize)]
pub struct SceneEdge {
    /// Source scene node ID.
    pub from: String,
    /// Destination scene node ID.
    pub to: String,
    /// Transport configuration.
    pub transport: TransportData,
    #[serde(default)]
    pub label: Option<String>,
    /// Item name filter (from items.ron). If set, only this item type is routed.
    #[serde(default)]
    pub item_filter: Option<String>,
    /// Intermediate waypoints for rendering curved/routed edges.
    #[serde(default)]
    pub waypoints: Vec<(f32, f32)>,
}

/// Transport strategy configuration (mirrors factorial-core Transport variants).
#[derive(Debug, Clone, Deserialize)]
pub enum TransportData {
    Flow {
        rate: f64,
        buffer_capacity: f64,
        #[serde(default)]
        latency: u32,
    },
    Item {
        speed: f64,
        slot_count: u32,
        #[serde(default = "default_lanes")]
        lanes: u8,
    },
    Batch {
        batch_size: u32,
        cycle_time: u32,
    },
    Vehicle {
        capacity: u32,
        travel_time: u32,
    },
}

fn default_lanes() -> u8 {
    1
}

/// Simulation parameters for the scene.
#[derive(Debug, Clone, Deserialize)]
pub struct SimulationConfig {
    #[serde(default = "default_tps")]
    pub ticks_per_second: u32,
    #[serde(default)]
    pub warmup_ticks: u64,
    #[serde(default)]
    pub pause_after_warmup: bool,
    #[serde(default)]
    pub rng_seed: Option<u64>,
}

fn default_tps() -> u32 {
    60
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            ticks_per_second: default_tps(),
            warmup_ticks: 0,
            pause_after_warmup: false,
            rng_seed: None,
        }
    }
}

/// Override inventory capacities per node.
#[derive(Debug, Clone, Deserialize)]
pub struct InventoryOverride {
    pub input_capacity: Option<u32>,
    pub output_capacity: Option<u32>,
}

/// Modifier applied to a node's processor.
#[derive(Debug, Clone, Deserialize)]
pub struct ModifierData {
    pub kind: ModifierKindData,
    #[serde(default = "default_stacking")]
    pub stacking: String,
}

fn default_stacking() -> String {
    "multiplicative".to_string()
}

/// Modifier kind with its value.
#[derive(Debug, Clone, Deserialize)]
pub enum ModifierKindData {
    Speed(f64),
    Productivity(f64),
    Efficiency(f64),
}

/// Wiring for optional modules (power, fluid, logic, tech tree).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModuleWiring {
    #[serde(default)]
    pub power_networks: Vec<PowerNetworkData>,
    #[serde(default)]
    pub fluid_networks: Vec<FluidNetworkData>,
    #[serde(default)]
    pub logic_networks: Vec<LogicNetworkData>,
    #[serde(default)]
    pub tech_tree: Option<TechTreeWiring>,
}

// ---------------------------------------------------------------------------
// Power
// ---------------------------------------------------------------------------

/// A power network with typed members.
#[derive(Debug, Clone, Deserialize)]
pub struct PowerNetworkData {
    pub name: String,
    pub members: Vec<PowerMemberData>,
}

/// A member of a power network with its role.
#[derive(Debug, Clone, Deserialize)]
pub struct PowerMemberData {
    pub node: String,
    pub role: PowerRoleData,
}

/// Role a node plays in a power network.
#[derive(Debug, Clone, Deserialize)]
pub enum PowerRoleData {
    Producer {
        capacity: f64,
    },
    Consumer {
        demand: f64,
        #[serde(default)]
        priority: Option<String>,
    },
    Storage {
        capacity: f64,
        charge: f64,
        charge_rate: f64,
    },
}

// ---------------------------------------------------------------------------
// Fluid
// ---------------------------------------------------------------------------

/// A fluid network carrying a single fluid type.
#[derive(Debug, Clone, Deserialize)]
pub struct FluidNetworkData {
    pub name: String,
    pub fluid_type: String,
    pub members: Vec<FluidMemberData>,
}

/// A member of a fluid network with its role.
#[derive(Debug, Clone, Deserialize)]
pub struct FluidMemberData {
    pub node: String,
    pub role: FluidRoleData,
}

/// Role a node plays in a fluid network.
#[derive(Debug, Clone, Deserialize)]
pub enum FluidRoleData {
    Producer {
        rate: f64,
    },
    Consumer {
        rate: f64,
    },
    Storage {
        capacity: f64,
        fill_rate: f64,
        #[serde(default)]
        initial: f64,
    },
    Pipe {
        capacity: f64,
    },
}

// ---------------------------------------------------------------------------
// Logic
// ---------------------------------------------------------------------------

/// A logic wire network with typed members.
#[derive(Debug, Clone, Deserialize)]
pub struct LogicNetworkData {
    pub name: String,
    pub wire_color: String,
    pub members: Vec<LogicMemberData>,
}

/// A member of a logic network with optional configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LogicMemberData {
    pub node: String,
    #[serde(default)]
    pub constant_signals: Vec<(String, i64)>,
    #[serde(default)]
    pub circuit_condition: Option<CircuitConditionData>,
}

/// A circuit condition for controlling node activity.
#[derive(Debug, Clone, Deserialize)]
pub struct CircuitConditionData {
    pub signal: String,
    pub op: String,
    pub value: i64,
}

// ---------------------------------------------------------------------------
// Tech tree
// ---------------------------------------------------------------------------

/// Tech tree wiring for a scene.
#[derive(Debug, Clone, Deserialize)]
pub struct TechTreeWiring {
    #[serde(default)]
    pub technologies: Vec<TechData>,
    #[serde(default)]
    pub auto_research: Vec<String>,
}

/// A technology definition within a scene.
#[derive(Debug, Clone, Deserialize)]
pub struct TechData {
    pub name: String,
    pub cost: TechCostData,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub unlocks: Vec<String>,
}

/// Cost model for a technology.
#[derive(Debug, Clone, Deserialize)]
pub enum TechCostData {
    Points(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_scene_data() {
        let input = r#"(
            title: "Solo Extractor",
            description: "A single mine producing iron ore.",
            tier: 1,
            order: 1,
            tags: ["fundamentals"],
            nodes: [
                (
                    id: "mine",
                    building: "iron_mine",
                    position: (100.0, 100.0),
                    label: Some("Iron Mine"),
                    visual_hint: Some("source"),
                ),
            ],
            edges: [],
            simulation: (
                ticks_per_second: 60,
                warmup_ticks: 0,
            ),
        )"#;

        let scene: SceneData = ron::from_str(input).unwrap();
        assert_eq!(scene.title, "Solo Extractor");
        assert_eq!(scene.tier, 1);
        assert_eq!(scene.nodes.len(), 1);
        assert_eq!(scene.nodes[0].id, "mine");
        assert_eq!(scene.nodes[0].building, "iron_mine");
        assert_eq!(scene.nodes[0].position, (100.0, 100.0));
        assert_eq!(scene.edges.len(), 0);
        assert_eq!(scene.simulation.ticks_per_second, 60);
    }

    #[test]
    fn deserialize_transport_data_variants() {
        let flow: TransportData =
            ron::from_str(r#"Flow(rate: 5.0, buffer_capacity: 100.0)"#).unwrap();
        assert!(
            matches!(flow, TransportData::Flow { rate, .. } if (rate - 5.0).abs() < f64::EPSILON)
        );

        let item: TransportData = ron::from_str(r#"Item(speed: 1.0, slot_count: 10)"#).unwrap();
        assert!(matches!(
            item,
            TransportData::Item {
                slot_count: 10,
                lanes: 1,
                ..
            }
        ));

        let batch: TransportData =
            ron::from_str(r#"Batch(batch_size: 20, cycle_time: 5)"#).unwrap();
        assert!(matches!(
            batch,
            TransportData::Batch {
                batch_size: 20,
                cycle_time: 5
            }
        ));

        let vehicle: TransportData =
            ron::from_str(r#"Vehicle(capacity: 50, travel_time: 10)"#).unwrap();
        assert!(matches!(
            vehicle,
            TransportData::Vehicle {
                capacity: 50,
                travel_time: 10
            }
        ));
    }

    #[test]
    fn deserialize_simulation_config_defaults() {
        let config: SimulationConfig = ron::from_str("()").unwrap();
        assert_eq!(config.ticks_per_second, 60);
        assert_eq!(config.warmup_ticks, 0);
        assert!(!config.pause_after_warmup);
        assert!(config.rng_seed.is_none());
    }

    #[test]
    fn deserialize_scene_with_edges() {
        let input = r#"(
            title: "Extract & Smelt",
            description: "Mine connected to smelter via flow transport.",
            tier: 1,
            order: 2,
            nodes: [
                (id: "mine", building: "iron_mine", position: (50.0, 100.0)),
                (id: "smelter", building: "smelter", position: (250.0, 100.0)),
            ],
            edges: [
                (
                    from: "mine",
                    to: "smelter",
                    transport: Flow(rate: 2.0, buffer_capacity: 50.0, latency: 0),
                    label: Some("ore belt"),
                    waypoints: [(150.0, 80.0)],
                ),
            ],
        )"#;

        let scene: SceneData = ron::from_str(input).unwrap();
        assert_eq!(scene.nodes.len(), 2);
        assert_eq!(scene.edges.len(), 1);
        assert_eq!(scene.edges[0].from, "mine");
        assert_eq!(scene.edges[0].to, "smelter");
        assert_eq!(scene.edges[0].label, Some("ore belt".to_string()));
        assert_eq!(scene.edges[0].waypoints.len(), 1);
    }

    #[test]
    fn deserialize_module_wiring() {
        let input = r#"(
            power_networks: [
                (name: "main_grid", members: [
                    (node: "generator", role: Producer(capacity: 100.0)),
                    (node: "smelter", role: Consumer(demand: 50.0)),
                ]),
            ],
            logic_networks: [
                (name: "control", wire_color: "red", members: [
                    (node: "combinator", constant_signals: [("iron_ore", 10)]),
                    (node: "inserter", circuit_condition: Some((signal: "iron_ore", op: "gt", value: 5))),
                ]),
            ],
        )"#;

        let wiring: ModuleWiring = ron::from_str(input).unwrap();
        assert_eq!(wiring.power_networks.len(), 1);
        assert_eq!(wiring.power_networks[0].members.len(), 2);
        assert_eq!(wiring.power_networks[0].members[0].node, "generator");
        assert!(matches!(
            wiring.power_networks[0].members[0].role,
            PowerRoleData::Producer { .. }
        ));
        assert_eq!(wiring.logic_networks.len(), 1);
        assert_eq!(wiring.logic_networks[0].wire_color, "red");
        assert_eq!(wiring.logic_networks[0].members.len(), 2);
        assert_eq!(
            wiring.logic_networks[0].members[0].constant_signals.len(),
            1
        );
        assert!(
            wiring.logic_networks[0].members[1]
                .circuit_condition
                .is_some()
        );
    }

    #[test]
    fn deserialize_fluid_network() {
        let input = r#"(
            fluid_networks: [
                (name: "water_system", fluid_type: "water", members: [
                    (node: "pump", role: Producer(rate: 10.0)),
                    (node: "tank", role: Storage(capacity: 1000.0, fill_rate: 5.0, initial: 0.0)),
                    (node: "boiler", role: Consumer(rate: 8.0)),
                ]),
            ],
        )"#;

        let wiring: ModuleWiring = ron::from_str(input).unwrap();
        assert_eq!(wiring.fluid_networks.len(), 1);
        assert_eq!(wiring.fluid_networks[0].fluid_type, "water");
        assert_eq!(wiring.fluid_networks[0].members.len(), 3);
    }

    #[test]
    fn deserialize_tech_tree_wiring() {
        let input = r#"(
            tech_tree: Some((
                technologies: [
                    (name: "advanced_smelting", cost: Points(100), prerequisites: [], unlocks: ["steel_recipe"]),
                ],
                auto_research: ["advanced_smelting"],
            )),
        )"#;

        let wiring: ModuleWiring = ron::from_str(input).unwrap();
        assert!(wiring.tech_tree.is_some());
        let tt = wiring.tech_tree.unwrap();
        assert_eq!(tt.technologies.len(), 1);
        assert_eq!(tt.technologies[0].name, "advanced_smelting");
        assert_eq!(tt.auto_research, vec!["advanced_smelting"]);
    }

    #[test]
    fn deserialize_inventory_override() {
        let input = r#"(input_capacity: Some(500), output_capacity: None)"#;
        let ovr: InventoryOverride = ron::from_str(input).unwrap();
        assert_eq!(ovr.input_capacity, Some(500));
        assert!(ovr.output_capacity.is_none());
    }

    #[test]
    fn deserialize_modifier_data() {
        let input = r#"(kind: Speed(2.0), stacking: "multiplicative")"#;
        let m: ModifierData = ron::from_str(input).unwrap();
        assert!(matches!(m.kind, ModifierKindData::Speed(v) if (v - 2.0).abs() < f64::EPSILON));
        assert_eq!(m.stacking, "multiplicative");
    }
}
