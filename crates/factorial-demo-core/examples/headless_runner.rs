//! Headless runner: loads all scenes, runs them, prints snapshots, verifies determinism.
//!
//! Run with: `cargo run --package factorial-demo-core --example headless_runner`

use std::path::Path;

use factorial_demo_core::scene_builder::build_scene;
use factorial_demo_core::scene_manager::SceneManager;

const TICKS: u64 = 100;

fn main() {
    let scenes_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/scenes"));

    let mgr = SceneManager::new(scenes_dir).expect("failed to load manifest");

    println!(
        "=== {} ===\n{}\n",
        mgr.gallery_title(),
        mgr.gallery_description()
    );
    println!("Tiers: {}", mgr.tiers().len());
    println!("Scenes: {}\n", mgr.scenes().len());

    for entry in mgr.scenes() {
        println!("--- {} (tier {}) ---", entry.title, entry.tier);
        println!("    {}", entry.summary);

        let scene_dir = scenes_dir.join(&entry.path);

        // Run 1
        let mut scene1 = build_scene(&scene_dir).unwrap_or_else(|e| {
            panic!("failed to build scene '{}': {e}", entry.id);
        });

        for _ in 0..TICKS {
            scene1.engine.step();
        }

        let hash1 = scene1.engine.state_hash();
        let snaps = scene1.engine.snapshot_all_nodes();

        println!(
            "    After {TICKS} ticks: {} nodes, state hash = {hash1:#018x}",
            snaps.len()
        );

        for (meta, snap) in scene1.node_meta.iter().zip(snaps.iter()) {
            let in_total: u32 = snap.input_contents.iter().map(|s| s.quantity).sum();
            let out_total: u32 = snap.output_contents.iter().map(|s| s.quantity).sum();
            println!(
                "      [{:>12}] state={:?}, progress={:.2}, in={in_total}, out={out_total}",
                meta.label, snap.processor_state, snap.progress
            );
        }

        // Run 2 — determinism check
        let mut scene2 = build_scene(&scene_dir).unwrap_or_else(|e| {
            panic!("failed to build scene '{}' (run 2): {e}", entry.id);
        });
        for _ in 0..TICKS {
            scene2.engine.step();
        }
        let hash2 = scene2.engine.state_hash();

        if hash1 == hash2 {
            println!("    Determinism: PASS (hashes match)");
        } else {
            println!("    Determinism: FAIL! hash1={hash1:#018x} != hash2={hash2:#018x}");
            std::process::exit(1);
        }

        println!();
    }

    println!("All {} scenes passed.", mgr.scenes().len());
}
