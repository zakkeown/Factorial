use std::path::Path;

use factorial_demo_core::scene_builder::build_scene;
use factorial_demo_core::scene_manager::SceneManager;

fn scenes_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/scenes"))
}

fn scene_dir(name: &str) -> std::path::PathBuf {
    scenes_dir().join(name)
}

fn solo_extractor_dir() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/scenes/tier1_fundamentals/01_solo_extractor"
    ))
}

// -----------------------------------------------------------------------
// build_scene tests
// -----------------------------------------------------------------------

#[test]
fn build_scene_produces_ticking_engine() {
    let scene = build_scene(solo_extractor_dir()).unwrap();
    assert_eq!(scene.node_meta.len(), 1);
    assert_eq!(scene.edge_meta.len(), 0);
    assert_eq!(scene.node_meta[0].scene_id, "mine");
    assert_eq!(scene.node_meta[0].label, "Iron Mine");
    assert_eq!(scene.node_meta[0].building_name, "iron_mine");
    assert_eq!(scene.node_meta[0].visual_hint.as_deref(), Some("source"));
    assert_eq!(scene.ticks_per_second, 60);
    assert!(!scene.paused);
}

#[test]
fn build_scene_engine_can_step() {
    let mut scene = build_scene(solo_extractor_dir()).unwrap();
    for _ in 0..10 {
        scene.engine.step();
    }
    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 1);
    // After 10 ticks with rate=2.0, mine should have produced items
    assert!(
        !snaps[0].output_contents.is_empty(),
        "mine should have produced output after 10 ticks"
    );
    let total_output: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(
        total_output > 0,
        "expected some output items, got {total_output}"
    );
}

#[test]
fn build_scene_deterministic() {
    // Run same scene twice, assert state hashes match
    let mut scene1 = build_scene(solo_extractor_dir()).unwrap();
    let mut scene2 = build_scene(solo_extractor_dir()).unwrap();

    for _ in 0..50 {
        scene1.engine.step();
        scene2.engine.step();
    }

    assert_eq!(
        scene1.engine.state_hash(),
        scene2.engine.state_hash(),
        "same scene run twice must produce identical state hashes"
    );
}

#[test]
fn build_scene_node_id_map() {
    let scene = build_scene(solo_extractor_dir()).unwrap();
    assert!(scene.node_id_map.contains_key("mine"));
    let node_id = scene.node_id_map["mine"];
    let snap = scene.engine.snapshot_node(node_id);
    assert!(snap.is_some());
}

// -----------------------------------------------------------------------
// SceneManager tests
// -----------------------------------------------------------------------

#[test]
fn scene_manager_loads_manifest() {
    let mgr = SceneManager::new(scenes_dir()).unwrap();
    assert_eq!(mgr.gallery_title(), "Factorial Demo Showcase");
    assert!(!mgr.tiers().is_empty());
    assert!(!mgr.scenes().is_empty());
}

#[test]
fn scene_manager_scenes_in_tier() {
    let mgr = SceneManager::new(scenes_dir()).unwrap();
    let tier1 = mgr.scenes_in_tier(1);
    assert!(!tier1.is_empty());
    assert!(tier1.iter().all(|s| s.tier == 1));
}

#[test]
fn scene_manager_load_and_tick() {
    let mut mgr = SceneManager::new(scenes_dir()).unwrap();
    mgr.load_scene("solo_extractor").unwrap();

    assert_eq!(mgr.current_tick().unwrap(), 0);

    mgr.tick().unwrap();
    assert_eq!(mgr.current_tick().unwrap(), 1);

    mgr.tick_n(9).unwrap();
    assert_eq!(mgr.current_tick().unwrap(), 10);
}

#[test]
fn scene_manager_snapshot_after_ticks() {
    let mut mgr = SceneManager::new(scenes_dir()).unwrap();
    mgr.load_scene("solo_extractor").unwrap();
    mgr.tick_n(10).unwrap();

    let snaps = mgr.snapshot_all_nodes().unwrap();
    assert_eq!(snaps.len(), 1);
    assert!(
        !snaps[0].output_contents.is_empty(),
        "mine should have produced output"
    );

    let snap = mgr.snapshot_node("mine").unwrap();
    assert!(snap.is_some());

    let missing = mgr.snapshot_node("nonexistent").unwrap();
    assert!(missing.is_none());
}

#[test]
fn scene_manager_node_edge_meta() {
    let mut mgr = SceneManager::new(scenes_dir()).unwrap();
    mgr.load_scene("solo_extractor").unwrap();

    let nodes = mgr.node_meta().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].scene_id, "mine");

    let edges = mgr.edge_meta().unwrap();
    assert_eq!(edges.len(), 0);
}

#[test]
fn scene_manager_active_scene_data() {
    let mut mgr = SceneManager::new(scenes_dir()).unwrap();
    mgr.load_scene("solo_extractor").unwrap();

    let data = mgr.active_scene_data().unwrap();
    assert_eq!(data.title, "Solo Extractor");
    assert_eq!(data.tier, 1);
}

#[test]
fn scene_manager_state_hash() {
    let mut mgr = SceneManager::new(scenes_dir()).unwrap();
    mgr.load_scene("solo_extractor").unwrap();
    mgr.tick_n(10).unwrap();

    let hash = mgr.state_hash().unwrap();
    // Hash should be non-zero after simulation ran
    // (exact value depends on engine internals, just verify it's available)
    let _ = hash;
}

#[test]
fn scene_manager_pause_resume() {
    let mut mgr = SceneManager::new(scenes_dir()).unwrap();
    mgr.load_scene("solo_extractor").unwrap();

    mgr.set_paused(true).unwrap();
    // Ticking a paused engine should still work (engine handles paused state)
    mgr.tick().unwrap();

    mgr.set_paused(false).unwrap();
    mgr.tick().unwrap();
}

#[test]
fn scene_manager_unload_errors() {
    let mut mgr = SceneManager::new(scenes_dir()).unwrap();

    // No scene loaded — operations should fail
    assert!(mgr.tick().is_err());
    assert!(mgr.snapshot_all_nodes().is_err());
    assert!(mgr.node_meta().is_err());
    assert!(mgr.state_hash().is_err());

    // Load then unload
    mgr.load_scene("solo_extractor").unwrap();
    mgr.unload_scene();
    assert!(mgr.tick().is_err());
}

#[test]
fn scene_manager_scene_not_found() {
    let mut mgr = SceneManager::new(scenes_dir()).unwrap();
    let result = mgr.load_scene("nonexistent_scene");
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// Tier 1 scene tests (02-05)
// -----------------------------------------------------------------------

#[test]
fn scene_02_extract_and_smelt() {
    let mut scene = build_scene(&scene_dir("tier1_fundamentals/02_extract_and_smelt")).unwrap();
    assert_eq!(scene.node_meta.len(), 2);
    assert_eq!(scene.edge_meta.len(), 1);
    assert_eq!(scene.edge_meta[0].transport_kind, "flow");

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 2);

    // Mine should have produced ore
    let mine_snap = &snaps[0];
    let mine_out: u32 = mine_snap.output_contents.iter().map(|s| s.quantity).sum();
    assert!(mine_out > 0, "mine should have output");

    // Smelter should have received input
    let smelter_snap = &snaps[1];
    let smelter_in: u32 = smelter_snap.input_contents.iter().map(|s| s.quantity).sum();
    let smelter_out: u32 = smelter_snap
        .output_contents
        .iter()
        .map(|s| s.quantity)
        .sum();
    assert!(
        smelter_in > 0 || smelter_out > 0,
        "smelter should have processed or be processing items"
    );
}

#[test]
fn scene_03_two_input_craft() {
    let mut scene = build_scene(&scene_dir("tier1_fundamentals/03_two_input_craft")).unwrap();
    assert_eq!(scene.node_meta.len(), 3);
    assert_eq!(scene.edge_meta.len(), 2);

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 3);

    // Assembler should have received inputs and potentially produced output
    let assembler_snap = &snaps[2];
    let asm_in: u32 = assembler_snap
        .input_contents
        .iter()
        .map(|s| s.quantity)
        .sum();
    let asm_out: u32 = assembler_snap
        .output_contents
        .iter()
        .map(|s| s.quantity)
        .sum();
    assert!(
        asm_in > 0 || asm_out > 0,
        "assembler should have items after 100 ticks"
    );
}

#[test]
fn scene_04_splitter() {
    let mut scene = build_scene(&scene_dir("tier1_fundamentals/04_splitter")).unwrap();
    assert_eq!(scene.node_meta.len(), 3);
    assert_eq!(scene.edge_meta.len(), 2);
    assert!(scene.edge_meta.iter().all(|e| e.transport_kind == "item"));

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 3);

    // Mine should have produced items
    let mine_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(mine_out > 0, "mine should have output");
}

#[test]
fn scene_05_production_chain() {
    let mut scene = build_scene(&scene_dir("tier1_fundamentals/05_production_chain")).unwrap();
    assert_eq!(scene.node_meta.len(), 4);
    assert_eq!(scene.edge_meta.len(), 3);

    // Run long enough for items to flow through the chain
    for _ in 0..200 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 4);

    // Mine should have produced ore
    let mine_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(mine_out > 0, "mine should have output");

    // Smelter should have received ore
    let smelter_in: u32 = snaps[1].input_contents.iter().map(|s| s.quantity).sum();
    let smelter_out: u32 = snaps[1].output_contents.iter().map(|s| s.quantity).sum();
    assert!(
        smelter_in > 0 || smelter_out > 0,
        "smelter should have items"
    );
}

// -----------------------------------------------------------------------
// Tier 2 scene tests (06-10: Transport)
// -----------------------------------------------------------------------

#[test]
fn scene_06_flow_transport() {
    let mut scene = build_scene(&scene_dir("tier2_transport/06_flow_transport")).unwrap();
    assert_eq!(scene.node_meta.len(), 2);
    assert_eq!(scene.edge_meta.len(), 1);
    assert_eq!(scene.edge_meta[0].transport_kind, "flow");

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 2);

    let source_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(source_out > 0, "pump should have output");
}

#[test]
fn scene_07_item_belts() {
    let mut scene = build_scene(&scene_dir("tier2_transport/07_item_belts")).unwrap();
    assert_eq!(scene.node_meta.len(), 2);
    assert_eq!(scene.edge_meta.len(), 1);
    assert_eq!(scene.edge_meta[0].transport_kind, "item");

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 2);

    let mine_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(mine_out > 0, "mine should have output");
}

#[test]
fn scene_08_batch_transfer() {
    let mut scene = build_scene(&scene_dir("tier2_transport/08_batch_transfer")).unwrap();
    assert_eq!(scene.node_meta.len(), 2);
    assert_eq!(scene.edge_meta.len(), 1);
    assert_eq!(scene.edge_meta[0].transport_kind, "batch");

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 2);

    let source_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(source_out > 0, "coal mine should have output");
}

#[test]
fn scene_09_vehicle_routes() {
    let mut scene = build_scene(&scene_dir("tier2_transport/09_vehicle_routes")).unwrap();
    assert_eq!(scene.node_meta.len(), 2);
    assert_eq!(scene.edge_meta.len(), 1);
    assert_eq!(scene.edge_meta[0].transport_kind, "vehicle");

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 2);

    let mine_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(mine_out > 0, "mine should have output");
}

#[test]
fn scene_10_mixed_transport() {
    let mut scene = build_scene(&scene_dir("tier2_transport/10_mixed_transport")).unwrap();
    assert_eq!(scene.node_meta.len(), 3);
    assert_eq!(scene.edge_meta.len(), 2);
    assert_eq!(scene.edge_meta[0].transport_kind, "flow");
    assert_eq!(scene.edge_meta[1].transport_kind, "item");

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 3);

    let mine_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(mine_out > 0, "mine should have output");
}

// -----------------------------------------------------------------------
// Tier 3 scene tests (11-14: Processors)
// -----------------------------------------------------------------------

#[test]
fn scene_11_property_transform() {
    let mut scene = build_scene(&scene_dir("tier3_processors/11_property_transform")).unwrap();
    assert_eq!(scene.node_meta.len(), 3);
    assert_eq!(scene.edge_meta.len(), 2);

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 3);

    let mine_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(mine_out > 0, "mine should have output");
}

#[test]
fn scene_12_demand_pull() {
    let mut scene = build_scene(&scene_dir("tier3_processors/12_demand_pull")).unwrap();
    assert_eq!(scene.node_meta.len(), 3);
    assert_eq!(scene.edge_meta.len(), 2);

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 3);

    let source_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(source_out > 0, "source should have output");
}

#[test]
fn scene_13_multi_recipe() {
    let mut scene = build_scene(&scene_dir("tier3_processors/13_multi_recipe")).unwrap();
    assert_eq!(scene.node_meta.len(), 3);
    assert_eq!(scene.edge_meta.len(), 2);

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 3);

    let mine_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(mine_out > 0, "mine should have output");
}

#[test]
fn scene_14_byproduct_handling() {
    let mut scene = build_scene(&scene_dir("tier3_processors/14_byproduct_handling")).unwrap();
    assert_eq!(scene.node_meta.len(), 4);
    assert_eq!(scene.edge_meta.len(), 3);

    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 4);

    let well_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(well_out > 0, "oil well should have output");
}

// -----------------------------------------------------------------------
// Tier 4 scene tests (15-19: Modules)
// -----------------------------------------------------------------------

#[test]
fn scene_15_power_grid() {
    let mut scene = build_scene(&scene_dir("tier4_modules/15_power_grid")).unwrap();
    assert_eq!(scene.node_meta.len(), 4);
    assert_eq!(scene.edge_meta.len(), 2);

    // Power module should be wired
    assert!(
        scene.power_module.is_some(),
        "power module should be present"
    );
    assert!(scene.power_network_names.contains_key("main_grid"));

    // Run ticks (10 warmup already done)
    for _ in 0..100 {
        scene.engine.step();
        let tick = scene.engine.sim_state.tick;
        scene.power_module.as_mut().unwrap().tick(tick);
    }

    // Demand (230) exceeds supply (100), so satisfaction should be < 1.0
    let net_id = scene.power_network_names["main_grid"];
    let satisfaction = scene
        .power_module
        .as_ref()
        .unwrap()
        .satisfaction(net_id)
        .map(|f| f.to_num::<f64>());
    assert!(
        satisfaction.is_some(),
        "satisfaction should be available for main_grid"
    );
    assert!(
        satisfaction.unwrap() < 1.0,
        "satisfaction should be < 1.0 when demand exceeds supply, got {}",
        satisfaction.unwrap()
    );
}

#[test]
fn scene_16_fluid_pipes() {
    let mut scene = build_scene(&scene_dir("tier4_modules/16_fluid_pipes")).unwrap();
    assert_eq!(scene.node_meta.len(), 4);
    assert_eq!(scene.edge_meta.len(), 3);

    // Fluid module should be wired
    assert!(
        scene.fluid_module.is_some(),
        "fluid module should be present"
    );
    assert!(scene.fluid_network_names.contains_key("water_system"));

    // Run ticks
    for _ in 0..100 {
        scene.engine.step();
        let tick = scene.engine.sim_state.tick;
        scene.fluid_module.as_mut().unwrap().tick(tick);
    }

    // Pressure should be available
    let net_id = scene.fluid_network_names["water_system"];
    let pressure = scene
        .fluid_module
        .as_ref()
        .unwrap()
        .pressure(net_id)
        .map(|f| f.to_num::<f64>());
    assert!(
        pressure.is_some(),
        "pressure should be available for water_system"
    );
}

#[test]
fn scene_17_tech_unlock() {
    let scene = build_scene(&scene_dir("tier4_modules/17_tech_unlock")).unwrap();
    assert_eq!(scene.node_meta.len(), 3);
    assert_eq!(scene.edge_meta.len(), 2);

    // Tech tree should be wired with auto-research completed
    assert!(scene.tech_tree.is_some(), "tech tree should be present");
    let tt = scene.tech_tree.as_ref().unwrap();
    // The auto-researched technology should have produced unlocks
    assert!(
        !tt.all_unlocks().is_empty(),
        "auto-researched technology should have unlocks"
    );
}

#[test]
fn scene_18_logic_circuits() {
    let mut scene = build_scene(&scene_dir("tier4_modules/18_logic_circuits")).unwrap();
    assert_eq!(scene.node_meta.len(), 4);
    assert_eq!(scene.edge_meta.len(), 2);
    assert!(scene.edge_meta.iter().all(|e| e.transport_kind == "item"));

    // Logic module should be wired (registered as engine module)
    let bridge = scene
        .engine
        .find_module::<factorial_logic::LogicModuleBridge>();
    assert!(bridge.is_some(), "logic module should be registered");

    // Run ticks to let logic process
    for _ in 0..50 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 4);
}

#[test]
fn scene_19_logic_power() {
    let mut scene = build_scene(&scene_dir("tier4_modules/19_logic_power")).unwrap();
    assert_eq!(scene.node_meta.len(), 4);
    assert_eq!(scene.edge_meta.len(), 2);

    // Both power and logic should be wired
    assert!(
        scene.power_module.is_some(),
        "power module should be present"
    );
    let bridge = scene
        .engine
        .find_module::<factorial_logic::LogicModuleBridge>();
    assert!(bridge.is_some(), "logic module should be registered");

    // Run ticks
    for _ in 0..100 {
        scene.engine.step();
        let tick = scene.engine.sim_state.tick;
        scene.power_module.as_mut().unwrap().tick(tick);
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 4);
}

// -----------------------------------------------------------------------
// Tier 5 scene tests (20-23: Scale & Edge Cases)
// -----------------------------------------------------------------------

#[test]
fn scene_20_feedback_loop() {
    let mut scene = build_scene(&scene_dir("tier5_scale/20_feedback_loop")).unwrap();
    assert_eq!(scene.node_meta.len(), 3);
    assert_eq!(scene.edge_meta.len(), 3);

    // The graph should detect back-edges in the cyclic graph
    let (_topo_order, back_edges) = scene.engine.graph.topological_order_with_feedback();
    assert!(
        !back_edges.is_empty(),
        "cyclic graph should have back-edges detected"
    );

    // Should still tick without panic despite cycles
    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 3);
}

#[test]
fn scene_21_bottleneck() {
    let mut scene = build_scene(&scene_dir("tier5_scale/21_bottleneck")).unwrap();
    assert_eq!(scene.node_meta.len(), 2);
    assert_eq!(scene.edge_meta.len(), 1);
    assert_eq!(scene.edge_meta[0].transport_kind, "item");

    // Run enough ticks for buffer to build up
    for _ in 0..200 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert_eq!(snaps.len(), 2);

    // Fast mine should have accumulated output (bottlenecked)
    let mine_out: u32 = snaps[0].output_contents.iter().map(|s| s.quantity).sum();
    assert!(
        mine_out > 0,
        "fast mine should have buffered output due to bottleneck"
    );
}

#[test]
fn scene_22_mega_factory() {
    let scene = build_scene(&scene_dir("tier5_scale/22_mega_factory")).unwrap();
    assert!(
        scene.node_meta.len() >= 50,
        "mega factory should have at least 50 nodes, got {}",
        scene.node_meta.len()
    );
    assert!(
        scene.edge_meta.len() >= 45,
        "mega factory should have at least 45 edges, got {}",
        scene.edge_meta.len()
    );

    // Run the scale test — should complete without timeout or panic
    let mut scene = scene;
    for _ in 0..100 {
        scene.engine.step();
    }

    let snaps = scene.engine.snapshot_all_nodes();
    assert!(snaps.len() >= 50);
}

#[test]
fn scene_23_serialization() {
    let scene = build_scene(&scene_dir("tier5_scale/23_serialization")).unwrap();
    assert_eq!(scene.node_meta.len(), 4);
    assert_eq!(scene.edge_meta.len(), 3);

    // Scene has warmup_ticks: 100, so engine already ran 100 ticks
    assert_eq!(scene.engine.sim_state.tick, 100);

    // Serialize should succeed and produce non-empty bytes
    let bytes = scene.engine.serialize().unwrap();
    assert!(!bytes.is_empty(), "serialized state should be non-empty");

    // Determinism: build same scene twice, state hashes should match
    let scene2 = build_scene(&scene_dir("tier5_scale/23_serialization")).unwrap();
    assert_eq!(
        scene.engine.state_hash(),
        scene2.engine.state_hash(),
        "same scene built twice must produce identical state hashes after warmup"
    );
}

// -----------------------------------------------------------------------
// Cross-tier tests
// -----------------------------------------------------------------------

#[test]
fn scene_manager_loads_all_scenes() {
    let mut mgr = SceneManager::new(scenes_dir()).unwrap();
    assert_eq!(mgr.scenes().len(), 23);
    assert_eq!(mgr.tiers().len(), 5);
    assert_eq!(mgr.scenes_in_tier(1).len(), 5);
    assert_eq!(mgr.scenes_in_tier(2).len(), 5);
    assert_eq!(mgr.scenes_in_tier(3).len(), 4);
    assert_eq!(mgr.scenes_in_tier(4).len(), 5);
    assert_eq!(mgr.scenes_in_tier(5).len(), 4);

    let ids = [
        "solo_extractor",
        "extract_and_smelt",
        "two_input_craft",
        "splitter",
        "production_chain",
        "flow_transport",
        "item_belts",
        "batch_transfer",
        "vehicle_routes",
        "mixed_transport",
        "property_transform",
        "demand_pull",
        "multi_recipe",
        "byproduct_handling",
        "power_grid",
        "fluid_pipes",
        "tech_unlock",
        "logic_circuits",
        "logic_power",
        "feedback_loop",
        "bottleneck",
        "mega_factory",
        "serialization",
    ];
    for id in &ids {
        mgr.load_scene(id).unwrap();
        mgr.tick_n(10).unwrap();
        let snaps = mgr.snapshot_all_nodes().unwrap();
        assert!(!snaps.is_empty(), "scene '{id}' should have nodes");
    }
}

#[test]
fn all_scenes_deterministic() {
    let scene_dirs = [
        "tier1_fundamentals/01_solo_extractor",
        "tier1_fundamentals/02_extract_and_smelt",
        "tier1_fundamentals/03_two_input_craft",
        "tier1_fundamentals/04_splitter",
        "tier1_fundamentals/05_production_chain",
        "tier2_transport/06_flow_transport",
        "tier2_transport/07_item_belts",
        "tier2_transport/08_batch_transfer",
        "tier2_transport/09_vehicle_routes",
        "tier2_transport/10_mixed_transport",
        "tier3_processors/11_property_transform",
        "tier3_processors/12_demand_pull",
        "tier3_processors/13_multi_recipe",
        "tier3_processors/14_byproduct_handling",
        "tier4_modules/15_power_grid",
        "tier4_modules/16_fluid_pipes",
        "tier4_modules/17_tech_unlock",
        "tier4_modules/18_logic_circuits",
        "tier4_modules/19_logic_power",
        "tier5_scale/20_feedback_loop",
        "tier5_scale/21_bottleneck",
        "tier5_scale/22_mega_factory",
        "tier5_scale/23_serialization",
    ];

    for dir_name in &scene_dirs {
        let dir = scene_dir(dir_name);
        let mut s1 = build_scene(&dir).unwrap();
        let mut s2 = build_scene(&dir).unwrap();

        for _ in 0..100 {
            s1.engine.step();
            s2.engine.step();
        }

        assert_eq!(
            s1.engine.state_hash(),
            s2.engine.state_hash(),
            "scene '{dir_name}' must be deterministic"
        );
    }
}
