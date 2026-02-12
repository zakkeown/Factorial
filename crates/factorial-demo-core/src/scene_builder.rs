use std::collections::HashMap;
use std::path::Path;

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::{BuildingTypeId, EdgeId, ItemTypeId, ModifierId, NodeId};
use factorial_core::item::Inventory;
use factorial_core::processor::{Modifier, ModifierKind, Processor, StackingRule};
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::{
    BatchTransport, FlowTransport, ItemTransport, Transport, VehicleTransport,
};
use factorial_data::load_game_data;
use factorial_fluid::{FluidModule, FluidNetworkId};
use factorial_power::{PowerModule, PowerNetworkId};
use factorial_tech_tree::TechTree;

use crate::error::DemoError;
use crate::scene_schema::{
    FluidRoleData, ModifierKindData, PowerRoleData, SceneData, TransportData,
};

/// Metadata about a node in the active scene, for rendering.
#[derive(Debug, Clone)]
pub struct NodeMeta {
    pub node_id: NodeId,
    pub scene_id: String,
    pub label: String,
    pub description: Option<String>,
    pub position: (f32, f32),
    pub visual_hint: Option<String>,
    pub building_name: String,
}

/// Metadata about an edge in the active scene, for rendering.
#[derive(Debug, Clone)]
pub struct EdgeMeta {
    pub edge_id: EdgeId,
    pub from_scene_id: String,
    pub to_scene_id: String,
    pub label: Option<String>,
    pub waypoints: Vec<(f32, f32)>,
    pub transport_kind: String,
}

/// A fully constructed scene ready for simulation and inspection.
pub struct ActiveScene {
    pub engine: Engine,
    pub scene_data: SceneData,
    pub node_meta: Vec<NodeMeta>,
    pub edge_meta: Vec<EdgeMeta>,
    pub node_id_map: HashMap<String, NodeId>,
    pub edge_id_map: HashMap<(String, String), EdgeId>,
    pub paused: bool,
    pub ticks_per_second: u32,
    // Module state (None if the scene doesn't use the module)
    pub power_module: Option<PowerModule>,
    pub fluid_module: Option<FluidModule>,
    pub tech_tree: Option<TechTree>,
    pub power_network_names: HashMap<String, PowerNetworkId>,
    pub fluid_network_names: HashMap<String, FluidNetworkId>,
}

/// Pre-resolved node info (names resolved to IDs before engine takes ownership of registry).
struct ResolvedNode<'a> {
    scene_node: &'a crate::scene_schema::SceneNode,
    building_type_id: BuildingTypeId,
    processor: Option<Processor>,
    input_capacity: u32,
    output_capacity: u32,
}

/// Pre-resolved edge info.
struct ResolvedEdge<'a> {
    scene_edge: &'a crate::scene_schema::SceneEdge,
    item_filter: Option<ItemTypeId>,
}

/// Build an active scene from a scene directory.
///
/// The directory must contain `scene.ron` plus the standard factorial-data
/// files (`items.ron`, `recipes.ron`, `buildings.ron`, and optionally
/// module config files).
pub fn build_scene(scene_dir: &Path) -> Result<ActiveScene, DemoError> {
    // 1. Load scene.ron
    let scene_path = scene_dir.join("scene.ron");
    let scene_content = std::fs::read_to_string(&scene_path)?;
    let scene_data: SceneData = ron::from_str(&scene_content).map_err(|e| DemoError::Parse {
        file: scene_path,
        detail: e.to_string(),
    })?;

    // 2. Load game data (items, recipes, buildings, optional modules)
    let game_data = load_game_data(scene_dir).map_err(|e| DemoError::DataLoad {
        dir: scene_dir.to_path_buf(),
        source: e,
    })?;

    // 3. Pre-resolve all name references while we still have access to the registry
    let mut resolved_nodes = Vec::new();
    for scene_node in &scene_data.nodes {
        let building_type_id = game_data
            .registry
            .building_id(&scene_node.building)
            .ok_or_else(|| DemoError::BuildingNotFound {
                name: scene_node.building.clone(),
            })?;

        let processor = game_data
            .building_processors
            .get(&building_type_id)
            .cloned();

        let (default_in_cap, default_out_cap) = game_data
            .building_inventories
            .get(&building_type_id)
            .copied()
            .unwrap_or((100, 100));

        let input_capacity = scene_node
            .inventory_override
            .as_ref()
            .and_then(|o| o.input_capacity)
            .unwrap_or(default_in_cap);
        let output_capacity = scene_node
            .inventory_override
            .as_ref()
            .and_then(|o| o.output_capacity)
            .unwrap_or(default_out_cap);

        resolved_nodes.push(ResolvedNode {
            scene_node,
            building_type_id,
            processor,
            input_capacity,
            output_capacity,
        });
    }

    let mut resolved_edges = Vec::new();
    for scene_edge in &scene_data.edges {
        let item_filter =
            if let Some(ref item_name) = scene_edge.item_filter {
                Some(game_data.registry.item_id(item_name).ok_or_else(|| {
                    DemoError::ItemNotFound {
                        name: item_name.clone(),
                    }
                })?)
            } else {
                None
            };
        resolved_edges.push(ResolvedEdge {
            scene_edge,
            item_filter,
        });
    }

    // 3b. Pre-resolve module wiring names (fluid types, logic signals)
    let resolved_fluid_types: Vec<ItemTypeId> = scene_data
        .modules
        .fluid_networks
        .iter()
        .map(|net| {
            game_data
                .registry
                .item_id(&net.fluid_type)
                .ok_or_else(|| DemoError::ItemNotFound {
                    name: net.fluid_type.clone(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let resolved_logic_signals: Vec<Vec<Vec<ItemTypeId>>> = scene_data
        .modules
        .logic_networks
        .iter()
        .map(|net| {
            net.members
                .iter()
                .map(|member| {
                    member
                        .constant_signals
                        .iter()
                        .map(|(name, _)| {
                            game_data
                                .registry
                                .item_id(name)
                                .ok_or_else(|| DemoError::ItemNotFound { name: name.clone() })
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;

    let resolved_circuit_signals: Vec<Vec<Option<ItemTypeId>>> = scene_data
        .modules
        .logic_networks
        .iter()
        .map(|net| {
            net.members
                .iter()
                .map(|member| {
                    if let Some(ref cond) = member.circuit_condition {
                        Ok(Some(game_data.registry.item_id(&cond.signal).ok_or_else(
                            || DemoError::ItemNotFound {
                                name: cond.signal.clone(),
                            },
                        )?))
                    } else {
                        Ok(None)
                    }
                })
                .collect::<Result<Vec<_>, DemoError>>()
        })
        .collect::<Result<Vec<_>, _>>()?;

    // 4. Create engine with registry (registry ownership moves to engine)
    let mut engine = Engine::new_with_registry(SimulationStrategy::Tick, game_data.registry);

    // 5. Add nodes
    let mut node_id_map: HashMap<String, NodeId> = HashMap::new();
    let mut node_meta: Vec<NodeMeta> = Vec::new();

    let pending_nodes: Vec<_> = resolved_nodes
        .iter()
        .map(|r| engine.graph.queue_add_node(r.building_type_id))
        .collect();

    let result = engine.graph.apply_mutations();

    for (resolved, pending) in resolved_nodes.iter().zip(pending_nodes.iter()) {
        let node_id = result
            .resolve_node(*pending)
            .ok_or_else(|| DemoError::GraphMutation {
                detail: format!("failed to resolve node '{}'", resolved.scene_node.id),
            })?;
        node_id_map.insert(resolved.scene_node.id.clone(), node_id);

        // Set processor
        if let Some(ref processor) = resolved.processor {
            engine.set_processor(node_id, processor.clone());
        }

        // Set inventories
        engine.set_input_inventory(node_id, Inventory::new(1, 1, resolved.input_capacity));
        engine.set_output_inventory(node_id, Inventory::new(1, 1, resolved.output_capacity));

        // Apply modifiers
        if !resolved.scene_node.modifiers.is_empty() {
            let mods: Vec<Modifier> = resolved
                .scene_node
                .modifiers
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let kind = match &m.kind {
                        ModifierKindData::Speed(v) => ModifierKind::Speed(Fixed64::from_num(*v)),
                        ModifierKindData::Productivity(v) => {
                            ModifierKind::Productivity(Fixed64::from_num(*v))
                        }
                        ModifierKindData::Efficiency(v) => {
                            ModifierKind::Efficiency(Fixed64::from_num(*v))
                        }
                    };
                    let stacking = match m.stacking.as_str() {
                        "additive" => StackingRule::Additive,
                        "diminishing" => StackingRule::Diminishing,
                        "capped" => StackingRule::Capped,
                        _ => StackingRule::Multiplicative,
                    };
                    Modifier {
                        id: ModifierId(i as u32),
                        kind,
                        stacking,
                    }
                })
                .collect();
            engine.set_modifiers(node_id, mods);
        }

        // Set active recipe for multi-recipe processors
        if let Some(recipe_index) = resolved.scene_node.active_recipe {
            let _ = engine.set_active_recipe(node_id, recipe_index);
        }

        let label = resolved
            .scene_node
            .label
            .clone()
            .unwrap_or_else(|| resolved.scene_node.building.clone());

        node_meta.push(NodeMeta {
            node_id,
            scene_id: resolved.scene_node.id.clone(),
            label,
            description: resolved.scene_node.description.clone(),
            position: resolved.scene_node.position,
            visual_hint: resolved.scene_node.visual_hint.clone(),
            building_name: resolved.scene_node.building.clone(),
        });
    }

    // 6. Add edges
    let mut edge_id_map: HashMap<(String, String), EdgeId> = HashMap::new();
    let mut edge_meta: Vec<EdgeMeta> = Vec::new();

    for resolved in &resolved_edges {
        let from_id = node_id_map
            .get(&resolved.scene_edge.from)
            .copied()
            .ok_or_else(|| DemoError::NodeNotFound {
                id: resolved.scene_edge.from.clone(),
            })?;
        let to_id = node_id_map
            .get(&resolved.scene_edge.to)
            .copied()
            .ok_or_else(|| DemoError::NodeNotFound {
                id: resolved.scene_edge.to.clone(),
            })?;

        let pending = if resolved.item_filter.is_some() {
            engine
                .graph
                .queue_connect_filtered(from_id, to_id, resolved.item_filter)
        } else {
            engine.graph.queue_connect(from_id, to_id)
        };

        let result = engine.graph.apply_mutations();
        let edge_id = result
            .resolve_edge(pending)
            .ok_or_else(|| DemoError::GraphMutation {
                detail: format!(
                    "failed to resolve edge '{}' -> '{}'",
                    resolved.scene_edge.from, resolved.scene_edge.to
                ),
            })?;

        let (transport, kind_name) = convert_transport(&resolved.scene_edge.transport);
        engine.set_transport(edge_id, transport);

        edge_id_map.insert(
            (
                resolved.scene_edge.from.clone(),
                resolved.scene_edge.to.clone(),
            ),
            edge_id,
        );

        edge_meta.push(EdgeMeta {
            edge_id,
            from_scene_id: resolved.scene_edge.from.clone(),
            to_scene_id: resolved.scene_edge.to.clone(),
            label: resolved.scene_edge.label.clone(),
            waypoints: resolved.scene_edge.waypoints.clone(),
            transport_kind: kind_name,
        });
    }

    // 7. Wire optional modules
    let mut power_module: Option<PowerModule> = None;
    let mut fluid_module: Option<FluidModule> = None;
    let mut tech_tree: Option<TechTree> = None;
    let mut power_network_names: HashMap<String, PowerNetworkId> = HashMap::new();
    let mut fluid_network_names: HashMap<String, FluidNetworkId> = HashMap::new();

    // 7a. Power networks
    if !scene_data.modules.power_networks.is_empty() {
        let mut pm = PowerModule::new();
        for net_data in &scene_data.modules.power_networks {
            let net_id = pm.create_network();
            power_network_names.insert(net_data.name.clone(), net_id);
            for member in &net_data.members {
                let node_id = node_id_map.get(&member.node).copied().ok_or_else(|| {
                    DemoError::ModuleWiring {
                        detail: format!(
                            "power network '{}': node '{}' not found",
                            net_data.name, member.node
                        ),
                    }
                })?;
                match &member.role {
                    PowerRoleData::Producer { capacity } => {
                        pm.add_producer(
                            net_id,
                            node_id,
                            factorial_power::PowerProducer {
                                capacity: Fixed64::from_num(*capacity),
                            },
                        );
                    }
                    PowerRoleData::Consumer { demand, priority } => {
                        let prio = match priority.as_deref() {
                            Some("high") => factorial_power::PowerPriority::High,
                            Some("low") => factorial_power::PowerPriority::Low,
                            _ => factorial_power::PowerPriority::Medium,
                        };
                        pm.add_consumer_with_priority(
                            net_id,
                            node_id,
                            factorial_power::PowerConsumer {
                                demand: Fixed64::from_num(*demand),
                            },
                            prio,
                        );
                    }
                    PowerRoleData::Storage {
                        capacity,
                        charge,
                        charge_rate,
                    } => {
                        pm.add_storage(
                            net_id,
                            node_id,
                            factorial_power::PowerStorage {
                                capacity: Fixed64::from_num(*capacity),
                                charge: Fixed64::from_num(*charge),
                                charge_rate: Fixed64::from_num(*charge_rate),
                            },
                        );
                    }
                }
            }
        }
        power_module = Some(pm);
    }

    // 7b. Fluid networks
    if !scene_data.modules.fluid_networks.is_empty() {
        let mut fm = FluidModule::new();
        for (net_data, &fluid_type_id) in scene_data
            .modules
            .fluid_networks
            .iter()
            .zip(resolved_fluid_types.iter())
        {
            let net_id = fm.create_network(fluid_type_id);
            fluid_network_names.insert(net_data.name.clone(), net_id);
            for member in &net_data.members {
                let node_id = node_id_map.get(&member.node).copied().ok_or_else(|| {
                    DemoError::ModuleWiring {
                        detail: format!(
                            "fluid network '{}': node '{}' not found",
                            net_data.name, member.node
                        ),
                    }
                })?;
                match &member.role {
                    FluidRoleData::Producer { rate } => {
                        fm.add_producer(
                            net_id,
                            node_id,
                            factorial_fluid::FluidProducer {
                                rate: Fixed64::from_num(*rate),
                            },
                        );
                    }
                    FluidRoleData::Consumer { rate } => {
                        fm.add_consumer(
                            net_id,
                            node_id,
                            factorial_fluid::FluidConsumer {
                                rate: Fixed64::from_num(*rate),
                            },
                        );
                    }
                    FluidRoleData::Storage {
                        capacity,
                        fill_rate,
                        initial,
                    } => {
                        fm.add_storage(
                            net_id,
                            node_id,
                            factorial_fluid::FluidStorage {
                                capacity: Fixed64::from_num(*capacity),
                                current: Fixed64::from_num(*initial),
                                fill_rate: Fixed64::from_num(*fill_rate),
                            },
                        );
                    }
                    FluidRoleData::Pipe { capacity } => {
                        fm.add_pipe(
                            net_id,
                            node_id,
                            factorial_fluid::FluidPipe {
                                capacity: Fixed64::from_num(*capacity),
                            },
                        );
                    }
                }
            }
        }
        fluid_module = Some(fm);
    }

    // 7c. Logic networks (registered as engine module via LogicModuleBridge)
    if !scene_data.modules.logic_networks.is_empty() {
        use factorial_logic::LogicModuleBridge;
        use factorial_logic::WireColor;
        use factorial_logic::combinator::SignalSelector;
        use factorial_logic::condition::{ComparisonOp, Condition};

        let mut bridge = LogicModuleBridge::new();
        for (net_idx, net_data) in scene_data.modules.logic_networks.iter().enumerate() {
            let color = match net_data.wire_color.as_str() {
                "green" => WireColor::Green,
                _ => WireColor::Red,
            };
            let net_id = bridge.logic_mut().create_network(color);
            for (mem_idx, member) in net_data.members.iter().enumerate() {
                let node_id = node_id_map.get(&member.node).copied().ok_or_else(|| {
                    DemoError::ModuleWiring {
                        detail: format!(
                            "logic network '{}': node '{}' not found",
                            net_data.name, member.node
                        ),
                    }
                })?;
                bridge.logic_mut().add_to_network(net_id, node_id);

                // Set constant signals
                if !member.constant_signals.is_empty() {
                    let signal_ids = &resolved_logic_signals[net_idx][mem_idx];
                    let signals: factorial_logic::SignalSet = member
                        .constant_signals
                        .iter()
                        .zip(signal_ids.iter())
                        .map(|((_, val), &item_id)| (item_id, Fixed64::from_num(*val)))
                        .collect();
                    bridge.logic_mut().set_constant(node_id, signals, true);
                }

                // Set circuit condition
                if let Some(ref cond_data) = member.circuit_condition
                    && let Some(signal_id) = resolved_circuit_signals[net_idx][mem_idx]
                {
                    let op = match cond_data.op.as_str() {
                        "gt" => ComparisonOp::Gt,
                        "lt" => ComparisonOp::Lt,
                        "eq" => ComparisonOp::Eq,
                        "gte" => ComparisonOp::Gte,
                        "lte" => ComparisonOp::Lte,
                        "ne" => ComparisonOp::Ne,
                        other => {
                            return Err(DemoError::ModuleWiring {
                                detail: format!("unknown comparison op '{other}'"),
                            });
                        }
                    };
                    let condition = Condition {
                        left: SignalSelector::Signal(signal_id),
                        op,
                        right: SignalSelector::Constant(Fixed64::from_num(cond_data.value)),
                    };
                    bridge
                        .logic_mut()
                        .set_circuit_control(node_id, condition, color);
                }
            }
        }
        engine.register_module(Box::new(bridge));
    }

    // 7d. Tech tree
    if let Some(ref tt_wiring) = scene_data.modules.tech_tree {
        let mut tt = TechTree::new();
        let mut tech_name_to_id: HashMap<String, factorial_tech_tree::TechId> = HashMap::new();

        // First pass: register all technologies
        for tech_data in &tt_wiring.technologies {
            let tech_id = tt.next_tech_id();
            tech_name_to_id.insert(tech_data.name.clone(), tech_id);
        }

        // Second pass: register with prerequisites resolved
        for tech_data in &tt_wiring.technologies {
            let tech_id = *tech_name_to_id.get(&tech_data.name).unwrap();
            let prerequisites: Vec<factorial_tech_tree::TechId> = tech_data
                .prerequisites
                .iter()
                .filter_map(|name| tech_name_to_id.get(name).copied())
                .collect();
            let cost = match &tech_data.cost {
                crate::scene_schema::TechCostData::Points(pts) => {
                    factorial_tech_tree::ResearchCost::Points(*pts)
                }
            };
            let unlocks: Vec<factorial_tech_tree::Unlock> = tech_data
                .unlocks
                .iter()
                .map(|s| factorial_tech_tree::Unlock::Custom(s.clone()))
                .collect();
            let technology = factorial_tech_tree::Technology {
                id: tech_id,
                name: tech_data.name.clone(),
                prerequisites,
                cost,
                unlocks,
                repeatable: false,
                cost_scaling: None,
            };
            tt.register(technology)
                .map_err(|e| DemoError::ModuleWiring {
                    detail: format!("tech tree: {e}"),
                })?;
        }

        // Auto-research: start research and contribute enough points to complete
        for name in &tt_wiring.auto_research {
            if let Some(&tech_id) = tech_name_to_id.get(name) {
                let _ = tt.start_research(tech_id, 0);
                // Get the cost and contribute enough to complete
                if let Ok(factorial_tech_tree::ResearchCost::Points(pts)) =
                    tt.effective_cost(tech_id)
                {
                    let _ = tt.contribute_points(tech_id, pts, 0);
                }
            }
        }
        tech_tree = Some(tt);
    }

    // 8. Run warmup ticks
    for _ in 0..scene_data.simulation.warmup_ticks {
        engine.step();
        if let Some(ref mut pm) = power_module {
            pm.tick(engine.sim_state.tick);
        }
        if let Some(ref mut fm) = fluid_module {
            fm.tick(engine.sim_state.tick);
        }
    }

    let paused = scene_data.simulation.pause_after_warmup;
    if paused {
        engine.pause();
    }

    Ok(ActiveScene {
        engine,
        ticks_per_second: scene_data.simulation.ticks_per_second,
        scene_data,
        node_meta,
        edge_meta,
        node_id_map,
        edge_id_map,
        paused,
        power_module,
        fluid_module,
        tech_tree,
        power_network_names,
        fluid_network_names,
    })
}

/// Convert scene TransportData to engine Transport.
fn convert_transport(data: &TransportData) -> (Transport, String) {
    match data {
        TransportData::Flow {
            rate,
            buffer_capacity,
            latency,
        } => (
            Transport::Flow(FlowTransport {
                rate: Fixed64::from_num(*rate),
                buffer_capacity: Fixed64::from_num(*buffer_capacity),
                latency: *latency,
            }),
            "flow".to_string(),
        ),
        TransportData::Item {
            speed,
            slot_count,
            lanes,
        } => (
            Transport::Item(ItemTransport {
                speed: Fixed64::from_num(*speed),
                slot_count: *slot_count,
                lanes: *lanes,
            }),
            "item".to_string(),
        ),
        TransportData::Batch {
            batch_size,
            cycle_time,
        } => (
            Transport::Batch(BatchTransport {
                batch_size: *batch_size,
                cycle_time: *cycle_time,
            }),
            "batch".to_string(),
        ),
        TransportData::Vehicle {
            capacity,
            travel_time,
        } => (
            Transport::Vehicle(VehicleTransport {
                capacity: *capacity,
                travel_time: *travel_time,
            }),
            "vehicle".to_string(),
        ),
    }
}
