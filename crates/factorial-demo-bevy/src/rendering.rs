use bevy::prelude::*;
use factorial_core::processor::ProcessorState;

use crate::AppState;
use crate::state::{DemoState, SelectedNode};

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Viewing), spawn_scene_entities)
            .add_systems(OnExit(AppState::Viewing), despawn_scene_entities)
            .add_systems(
                Update,
                (update_node_colors, draw_edges, handle_node_click)
                    .run_if(in_state(AppState::Viewing)),
            );
    }
}

#[derive(Component)]
pub struct SceneEntity;

#[derive(Component)]
pub struct NodeEntity {
    pub scene_id: String,
    pub index: usize,
}

#[derive(Component)]
pub struct EdgeEntity {
    pub from_pos: Vec2,
    pub to_pos: Vec2,
    pub transport_kind: String,
}

const NODE_SIZE: Vec2 = Vec2::new(80.0, 50.0);

fn spawn_scene_entities(mut commands: Commands, demo: NonSend<DemoState>) {
    let node_meta = match demo.manager.node_meta() {
        Ok(m) => m,
        Err(_) => return,
    };
    let edge_meta = match demo.manager.edge_meta() {
        Ok(m) => m,
        Err(_) => return,
    };

    for (i, meta) in node_meta.iter().enumerate() {
        let base_color = hint_color(meta.visual_hint.as_deref());

        commands
            .spawn((
                Sprite::from_color(base_color, NODE_SIZE),
                Transform::from_xyz(meta.position.0, -meta.position.1, 1.0),
                SceneEntity,
                NodeEntity {
                    scene_id: meta.scene_id.clone(),
                    index: i,
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text2d::new(&meta.label),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Transform::from_xyz(0.0, -35.0, 2.0),
                    SceneEntity,
                ));
            });
    }

    for meta in edge_meta.iter() {
        let from_node = node_meta.iter().find(|n| n.scene_id == meta.from_scene_id);
        let to_node = node_meta.iter().find(|n| n.scene_id == meta.to_scene_id);

        if let (Some(from), Some(to)) = (from_node, to_node) {
            commands.spawn((
                SceneEntity,
                EdgeEntity {
                    from_pos: Vec2::new(from.position.0, -from.position.1),
                    to_pos: Vec2::new(to.position.0, -to.position.1),
                    transport_kind: meta.transport_kind.clone(),
                },
            ));
        }
    }
}

fn despawn_scene_entities(mut commands: Commands, entities: Query<Entity, With<SceneEntity>>) {
    for entity in &entities {
        commands.entity(entity).despawn_recursive();
    }
}

fn update_node_colors(demo: NonSend<DemoState>, mut nodes: Query<(&NodeEntity, &mut Sprite)>) {
    let snapshots = match demo.manager.snapshot_all_nodes() {
        Ok(s) => s,
        Err(_) => return,
    };

    // Check for brownout: any power network with satisfaction < 1.0
    let has_brownout = demo.manager.has_power()
        && demo.manager.active_scene_data().ok().is_some_and(|data| {
            data.modules.power_networks.iter().any(|net| {
                demo.manager
                    .power_satisfaction(&net.name)
                    .is_some_and(|s| s < 1.0)
            })
        });

    for (node, mut sprite) in &mut nodes {
        if let Some(snap) = snapshots.get(node.index) {
            let base = state_color(&snap.processor_state);

            // Circuit-controlled inactive nodes get dimmed appearance
            if demo.manager.logic_is_active(&node.scene_id) == Some(false) {
                sprite.color = Color::srgba(0.3, 0.3, 0.35, 0.6);
            }
            // Brownout nodes get reddish tint
            else if has_brownout {
                sprite.color = tint_brownout(base);
            } else {
                sprite.color = base;
            }
        }
    }
}

fn tint_brownout(base: Color) -> Color {
    let c = Srgba::from(base);
    Color::srgb((c.red + 0.3).min(1.0), c.green * 0.7, c.blue * 0.7)
}

fn draw_edges(edges: Query<&EdgeEntity>, mut gizmos: Gizmos) {
    for edge in &edges {
        let color = transport_color(&edge.transport_kind);
        let from = Vec3::new(edge.from_pos.x, edge.from_pos.y, 0.0);
        let to = Vec3::new(edge.to_pos.x, edge.to_pos.y, 0.0);

        gizmos.line(from, to, color);

        // Arrowhead
        let dir = (to - from).normalize();
        let perp = Vec3::new(-dir.y, dir.x, 0.0);
        let tip = to - dir * 8.0;
        gizmos.line(to, tip + perp * 5.0, color);
        gizmos.line(to, tip - perp * 5.0, color);
    }
}

fn handle_node_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<crate::camera::MainCamera>>,
    nodes: Query<(&NodeEntity, &GlobalTransform)>,
    mut selected: ResMut<SelectedNode>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, cam_transform)) = camera_q.get_single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
        return;
    };

    let half = NODE_SIZE / 2.0;
    let mut clicked = None;

    for (node, transform) in &nodes {
        let pos = transform.translation().truncate();
        if world_pos.x >= pos.x - half.x
            && world_pos.x <= pos.x + half.x
            && world_pos.y >= pos.y - half.y
            && world_pos.y <= pos.y + half.y
        {
            clicked = Some(node.scene_id.clone());
            break;
        }
    }

    selected.scene_id = clicked;
}

fn hint_color(hint: Option<&str>) -> Color {
    match hint {
        Some("source") => Color::srgb(0.2, 0.6, 0.2),
        Some("processor") => Color::srgb(0.2, 0.4, 0.7),
        Some("sink") => Color::srgb(0.6, 0.3, 0.2),
        _ => Color::srgb(0.5, 0.5, 0.5),
    }
}

fn state_color(state: &ProcessorState) -> Color {
    match state {
        ProcessorState::Idle => Color::srgb(0.4, 0.4, 0.4),
        ProcessorState::Working { .. } => Color::srgb(0.2, 0.7, 0.2),
        ProcessorState::Stalled { .. } => Color::srgb(0.8, 0.2, 0.2),
    }
}

fn transport_color(kind: &str) -> Color {
    match kind {
        "flow" => Color::srgb(0.3, 0.6, 0.9),
        "item" => Color::srgb(0.9, 0.7, 0.2),
        "batch" => Color::srgb(0.6, 0.3, 0.8),
        "vehicle" => Color::srgb(0.9, 0.5, 0.2),
        _ => Color::srgb(0.5, 0.5, 0.5),
    }
}
