use bevy::prelude::*;

use crate::AppState;
use crate::state::DemoState;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationTimer(Timer::from_seconds(
            1.0 / 60.0,
            TimerMode::Repeating,
        )))
        .add_systems(Update, tick_simulation.run_if(in_state(AppState::Viewing)));
    }
}

#[derive(Resource)]
pub struct SimulationTimer(pub Timer);

fn tick_simulation(
    time: Res<Time>,
    mut timer: ResMut<SimulationTimer>,
    mut demo: NonSendMut<DemoState>,
) {
    timer.0.tick(time.delta());

    for _ in 0..timer.0.times_finished_this_tick() {
        if demo.manager.tick().is_ok() {
            demo.tick_count += 1;
        }
    }
}
