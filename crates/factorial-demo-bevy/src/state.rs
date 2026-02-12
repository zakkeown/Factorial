use std::path::Path;

use bevy::prelude::*;
use factorial_demo_core::scene_manager::SceneManager;

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_demo_state);
    }
}

/// Core demo state wrapping the SceneManager.
/// Not `Send + Sync` because Engine contains `dyn Module` trait objects,
/// so we use `NonSend` / `NonSendMut` to access this resource.
pub struct DemoState {
    pub manager: SceneManager,
    pub tick_count: u64,
}

/// Which node the user has selected for inspection.
#[derive(Resource, Default)]
pub struct SelectedNode {
    pub scene_id: Option<String>,
}

fn init_demo_state(world: &mut World) {
    let scenes_dir = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../factorial-demo-core/scenes"
    ));

    let manager = SceneManager::new(scenes_dir).expect("failed to load scene manifest");

    world.insert_non_send_resource(DemoState {
        manager,
        tick_count: 0,
    });
    world.insert_resource(SelectedNode::default());
}
