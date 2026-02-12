use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, (camera_pan, camera_zoom));
    }
}

#[derive(Component)]
pub struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn camera_pan(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<CursorMoved>,
    mut camera_q: Query<(&mut Transform, &OrthographicProjection), With<MainCamera>>,
    mut last_pos: Local<Option<Vec2>>,
) {
    if mouse.pressed(MouseButton::Middle) || mouse.pressed(MouseButton::Right) {
        if let Some(current) = motion.read().last().map(|e| e.position) {
            if let Some(prev) = *last_pos {
                let delta = current - prev;
                if let Ok((mut transform, proj)) = camera_q.get_single_mut() {
                    transform.translation.x -= delta.x * proj.scale;
                    transform.translation.y += delta.y * proj.scale;
                }
            }
            *last_pos = Some(current);
        }
    } else {
        *last_pos = None;
        motion.clear();
    }
}

fn camera_zoom(
    mut scroll: EventReader<MouseWheel>,
    mut camera_q: Query<&mut OrthographicProjection, With<MainCamera>>,
) {
    for event in scroll.read() {
        let scroll_amount = match event.unit {
            MouseScrollUnit::Line => event.y * 0.1,
            MouseScrollUnit::Pixel => event.y * 0.001,
        };

        if let Ok(mut proj) = camera_q.get_single_mut() {
            proj.scale = (proj.scale - scroll_amount).clamp(0.2, 5.0);
        }
    }
}
