use bevy::prelude::*;

use crate::AppState;
use crate::state::{DemoState, SelectedNode};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Menu), spawn_menu)
            .add_systems(OnExit(AppState::Menu), despawn_menu)
            .add_systems(Update, handle_menu_buttons.run_if(in_state(AppState::Menu)))
            .add_systems(OnEnter(AppState::Viewing), spawn_hud)
            .add_systems(OnExit(AppState::Viewing), despawn_hud)
            .add_systems(
                Update,
                (update_inspector, handle_back_button).run_if(in_state(AppState::Viewing)),
            );
    }
}

// -----------------------------------------------------------------------
// Menu UI
// -----------------------------------------------------------------------

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct SceneButton(String);

fn spawn_menu(mut commands: Commands, demo: NonSend<DemoState>) {
    let bg = Color::srgb(0.1, 0.1, 0.12);
    let panel_bg = Color::srgb(0.15, 0.15, 0.18);
    let btn_bg = Color::srgb(0.22, 0.22, 0.28);

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(15.0),
                ..default()
            },
            BackgroundColor(bg),
            MenuRoot,
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new(demo.manager.gallery_title()),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Description
            root.spawn((
                Text::new(demo.manager.gallery_description()),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            // Tier panels
            for tier in demo.manager.tiers() {
                root.spawn((
                    Node {
                        width: Val::Percent(90.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(panel_bg),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(format!("Tier {}: {}", tier.number, tier.name)),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.8, 0.3)),
                    ));

                    panel.spawn((
                        Text::new(&tier.description),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 0.5, 0.5)),
                    ));

                    // Scene buttons for this tier
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(8.0),
                            row_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|row| {
                            for scene in demo.manager.scenes_in_tier(tier.number) {
                                row.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                                        ..default()
                                    },
                                    BackgroundColor(btn_bg),
                                    SceneButton(scene.id.clone()),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new(&scene.title),
                                        TextFont {
                                            font_size: 13.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                            }
                        });
                });
            }
        });
}

fn despawn_menu(mut commands: Commands, menu: Query<Entity, With<MenuRoot>>) {
    for entity in &menu {
        commands.entity(entity).despawn_recursive();
    }
}

fn handle_menu_buttons(
    interactions: Query<(&Interaction, &SceneButton), Changed<Interaction>>,
    mut demo: NonSendMut<DemoState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed && demo.manager.load_scene(&button.0).is_ok() {
            demo.tick_count = 0;
            next_state.set(AppState::Viewing);
        }
    }
}

// -----------------------------------------------------------------------
// HUD / Inspector UI
// -----------------------------------------------------------------------

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct BackButton;

#[derive(Component)]
struct InspectorPanel;

#[derive(Component)]
struct InspectorText;

#[derive(Component)]
struct TickCounter;

fn spawn_hud(mut commands: Commands, demo: NonSend<DemoState>) {
    let scene_title = demo
        .manager
        .active_scene_data()
        .map(|d| d.title.clone())
        .unwrap_or_default();

    let panel_bg = Color::srgba(0.1, 0.1, 0.12, 0.85);

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            PickingBehavior::IGNORE,
            HudRoot,
        ))
        .with_children(|root| {
            // Top bar
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(40.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(15.0),
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(panel_bg),
                PickingBehavior::IGNORE,
            ))
            .with_children(|bar| {
                // Back button
                bar.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.2, 0.2)),
                    BackButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("< Back"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

                bar.spawn((
                    Text::new(&scene_title),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                bar.spawn((
                    Text::new("Tick: 0"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.8, 0.6)),
                    TickCounter,
                ));
            });

            // Right inspector panel
            root.spawn((
                Node {
                    width: Val::Px(250.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    top: Val::Px(40.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(panel_bg),
                InspectorPanel,
                PickingBehavior::IGNORE,
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Click a node to inspect"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                    InspectorText,
                ));
            });
        });
}

fn despawn_hud(mut commands: Commands, hud: Query<Entity, With<HudRoot>>) {
    for entity in &hud {
        commands.entity(entity).despawn_recursive();
    }
}

fn handle_back_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut demo: NonSendMut<DemoState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            demo.manager.unload_scene();
            demo.tick_count = 0;
            next_state.set(AppState::Menu);
        }
    }
}

fn update_inspector(
    demo: NonSend<DemoState>,
    selected: Res<SelectedNode>,
    mut inspector_text: Query<&mut Text, With<InspectorText>>,
    mut tick_text: Query<&mut Text, (With<TickCounter>, Without<InspectorText>)>,
) {
    // Update tick counter
    for mut text in &mut tick_text {
        **text = format!("Tick: {}", demo.tick_count);
    }

    // Update inspector content
    let Some(ref scene_id) = selected.scene_id else {
        for mut text in &mut inspector_text {
            **text = "Click a node to inspect".into();
        }
        return;
    };

    let snap = demo.manager.snapshot_node(scene_id).ok().flatten();
    let node_meta = demo.manager.node_meta().ok();

    let meta_info = node_meta
        .and_then(|meta| meta.iter().find(|m| m.scene_id == *scene_id))
        .map(|m| format!("{} ({})", m.label, m.building_name))
        .unwrap_or_else(|| scene_id.clone());

    let info = if let Some(snap) = snap {
        let in_total: u32 = snap.input_contents.iter().map(|s| s.quantity).sum();
        let out_total: u32 = snap.output_contents.iter().map(|s| s.quantity).sum();

        let mut text = format!(
            "{meta_info}\n\nState: {:?}\nProgress: {:.1}%\n\nInput buffer: {in_total}\nOutput buffer: {out_total}\n\nInput items:\n{}\n\nOutput items:\n{}",
            snap.processor_state,
            snap.progress.to_num::<f64>() * 100.0,
            format_items(&snap.input_contents, &demo),
            format_items(&snap.output_contents, &demo),
        );

        // Module state
        let module_info = format_module_info(&demo, scene_id);
        if !module_info.is_empty() {
            text.push_str("\n\n--- Modules ---");
            text.push_str(&module_info);
        }

        text
    } else {
        format!("{meta_info}\n\n(no snapshot)")
    };

    for mut text in &mut inspector_text {
        **text = info.clone();
    }
}

fn format_module_info(demo: &DemoState, scene_id: &str) -> String {
    let mut info = String::new();

    // Power satisfaction per network
    if demo.manager.has_power()
        && let Ok(data) = demo.manager.active_scene_data()
    {
        for net in &data.modules.power_networks {
            if let Some(sat) = demo.manager.power_satisfaction(&net.name) {
                info.push_str(&format!("\nPower ({}): {:.0}%", net.name, sat * 100.0));
            }
        }
    }

    // Fluid pressure per network
    if demo.manager.has_fluid()
        && let Ok(data) = demo.manager.active_scene_data()
    {
        for net in &data.modules.fluid_networks {
            if let Some(pressure) = demo.manager.fluid_pressure(&net.name) {
                info.push_str(&format!("\nFluid ({}): {:.0}%", net.name, pressure * 100.0));
            }
        }
    }

    // Logic circuit active state for this node
    if let Some(active) = demo.manager.logic_is_active(scene_id) {
        info.push_str(&format!(
            "\nCircuit: {}",
            if active { "Active" } else { "Inactive" }
        ));
    }

    // Tech tree research status
    if let Some(tt) = demo.manager.tech_tree() {
        let unlocks = tt.all_unlocks();
        info.push_str(&format!("\nTech: {} unlocked", unlocks.len()));
    }

    info
}

fn format_items(contents: &[factorial_core::item::ItemStack], demo: &DemoState) -> String {
    if contents.is_empty() {
        return "  (empty)".into();
    }
    contents
        .iter()
        .map(|s| {
            let name = demo
                .manager
                .item_name(s.item_type)
                .unwrap_or_else(|| format!("item#{}", s.item_type.0));
            format!("  {} x{}", name, s.quantity)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
