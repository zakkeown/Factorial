mod camera;
mod rendering;
mod simulation;
mod state;
mod ui;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Factorial Demo Showcase".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        .add_plugins((
            state::StatePlugin,
            camera::CameraPlugin,
            ui::UiPlugin,
            rendering::RenderingPlugin,
            simulation::SimulationPlugin,
        ))
        .run();
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Menu,
    Viewing,
}
