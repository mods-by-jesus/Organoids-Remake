mod menu;
mod rendering;
mod simulation;

use bevy::camera::ScalingMode;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};
use rendering::{
    InstancedDiscPlugin, LiquidMediumMaterial, spawn_simulation_layers, sync_instance_data,
};
use simulation::{FrameStats, SimConfig, WorldState};
use std::time::Instant;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Menu,
    Running,
}

#[derive(Component)]
struct StatsText;

#[derive(Component)]
struct MainCamera;

const START_VIEW_HEIGHT: f32 = 1_470.0;
const CAMERA_MOVE_SPEED: f32 = 1_100.0;
const ZOOM_FACTOR: f32 = 1.18;
const MIN_ZOOM_SCALE: f32 = 0.08;
const MAX_ZOOM_SCALE: f32 = 12.0;

fn main() {
    let mut config = SimConfig::default();
    if std::env::args().len() > 1 {
        match SimConfig::from_args() {
            Ok(c) => config = c,
            Err(message) => {
                eprintln!("{message}");
                return;
            }
        }
    }

    let present_mode = if config.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.012, 0.015, 0.018)))
        .insert_resource(config.clone())
        .init_resource::<FrameStats>()
        .add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Organoids".to_string(),
                        present_mode,
                        resolution: WindowResolution::new(1920, 1080)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                }),
            FrameTimeDiagnosticsPlugin::default(),
            MaterialPlugin::<LiquidMediumMaterial>::default(),
            InstancedDiscPlugin,
            menu::MenuPlugin,
        ))
        .init_state::<AppState>()
        .add_systems(Startup, setup_camera)
        .add_systems(
            OnEnter(AppState::Running),
            (
                spawn_simulation_layers,
                setup_ui,
                initialize_world_state,
                update_window_title,
            ),
        )
        .add_systems(
            Update,
            (
                camera_controls,
                step_simulation,
                sync_instance_data,
                update_ui,
            )
                .chain()
                .run_if(in_state(AppState::Running)),
        )
        .run();
}

fn initialize_world_state(mut commands: Commands, config: Res<SimConfig>) {
    commands.insert_resource(WorldState::new(&config));
}

fn update_window_title(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    config: Res<SimConfig>,
) {
    if let Some(mut window) = windows.iter_mut().next() {
        window.title = format!("Organoids - {} клеток / {} еды", config.cells, config.food);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("main_camera"),
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: START_VIEW_HEIGHT,
            },
            scale: 1.0,
            far: 5_000.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0.0, 0.0, 1_500.0).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
        NoIndirectDrawing,
    ));

    commands.spawn((
        Name::new("ui_camera"),
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        IsDefaultUiCamera,
    ));
}

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("загрузка"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.86, 0.91, 0.95)),
        TextShadow::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(10),
            left: px(12),
            ..default()
        },
        StatsText,
    ));
}

fn step_simulation(time: Res<Time>, mut world: ResMut<WorldState>, mut stats: ResMut<FrameStats>) {
    let started = Instant::now();
    world.update(time.delta_secs());
    stats.sim_time = started.elapsed();
}

fn camera_controls(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
    mut last_cursor: Local<Option<Vec2>>,
) {
    let Ok((window_entity, window)) = windows.single() else {
        return;
    };
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(projection) = projection.as_mut() else {
        return;
    };

    let mut keyboard_direction = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        keyboard_direction.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        keyboard_direction.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        keyboard_direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        keyboard_direction.x += 1.0;
    }

    if keyboard_direction != Vec2::ZERO {
        let speed = CAMERA_MOVE_SPEED * projection.scale * time.delta_secs();
        let movement = keyboard_direction.normalize() * speed;
        transform.translation.x += movement.x;
        transform.translation.y += movement.y;
    }

    let current_cursor = window.cursor_position();
    if mouse_buttons.pressed(MouseButton::Middle) {
        if let Some(current) = current_cursor {
            if let Some(last) = *last_cursor {
                let delta = current - last;
                if delta != Vec2::ZERO {
                    let view_size = visible_world_size(projection, window);
                    transform.translation.x -= delta.x * view_size.x / window.width();
                    transform.translation.y += delta.y * view_size.y / window.height();
                }
            }
            *last_cursor = Some(current);
        } else {
            *last_cursor = None;
        }
    } else {
        *last_cursor = None;
    }

    let mut scroll = 0.0;
    for event in mouse_wheel.read() {
        if event.window != window_entity {
            continue;
        }

        scroll += match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR,
        };
    }

    if scroll != 0.0 {
        let cursor_world_before = window
            .cursor_position()
            .map(|cursor| cursor_to_world(cursor, transform.translation, projection, window));

        let zoom_multiplier = ZOOM_FACTOR.powf(-scroll);
        projection.scale =
            (projection.scale * zoom_multiplier).clamp(MIN_ZOOM_SCALE, MAX_ZOOM_SCALE);

        if let Some(cursor_position) = window.cursor_position()
            && let Some(cursor_world_before) = cursor_world_before
        {
            let cursor_world_after =
                cursor_to_world(cursor_position, transform.translation, projection, window);
            let correction = cursor_world_before - cursor_world_after;
            transform.translation.x += correction.x;
            transform.translation.y += correction.y;
        }
    }
}

fn cursor_to_world(
    cursor: Vec2,
    camera_translation: Vec3,
    projection: &OrthographicProjection,
    window: &Window,
) -> Vec2 {
    let view_size = visible_world_size(projection, window);
    let normalized = Vec2::new(
        cursor.x / window.width() - 0.5,
        0.5 - cursor.y / window.height(),
    );

    Vec2::new(
        camera_translation.x + normalized.x * view_size.x,
        camera_translation.y + normalized.y * view_size.y,
    )
}

fn visible_world_size(projection: &OrthographicProjection, window: &Window) -> Vec2 {
    let width = window.width().max(1.0);
    let height = window.height().max(1.0);

    let size = match projection.scaling_mode {
        ScalingMode::WindowSize => Vec2::new(width, height),
        ScalingMode::AutoMin {
            min_width,
            min_height,
        } => {
            if width * min_height > min_width * height {
                Vec2::new(width * min_height / height, min_height)
            } else {
                Vec2::new(min_width, height * min_width / width)
            }
        }
        ScalingMode::AutoMax {
            max_width,
            max_height,
        } => {
            if width * max_height < max_width * height {
                Vec2::new(width * max_height / height, max_height)
            } else {
                Vec2::new(max_width, height * max_width / width)
            }
        }
        ScalingMode::FixedVertical { viewport_height } => {
            Vec2::new(width * viewport_height / height, viewport_height)
        }
        ScalingMode::FixedHorizontal { viewport_width } => {
            Vec2::new(viewport_width, height * viewport_width / width)
        }
        ScalingMode::Fixed { width, height } => Vec2::new(width, height),
    };

    size * projection.scale
}

fn update_ui(
    diagnostics: Res<DiagnosticsStore>,
    world: Res<WorldState>,
    stats: Res<FrameStats>,
    mut text: Query<&mut Text, With<StatsText>>,
    config: Res<SimConfig>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .unwrap_or(0.0);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|frame| frame.smoothed())
        .unwrap_or(0.0);

    let mut text = text.single_mut().expect("stats text exists");
    **text = format!(
        "FPS {fps:>6.1} | кадр {frame_ms:>5.2} мс\nклетки {:>5} | еда {:>5}\nпрепят. {:>4} | корм. {:>4}\nсим {:>5.2} мс | ренд {:>5.2} мс\nарена {:.0} x {:.0}",
        world.cells.len(),
        world.food.active_count(),
        world.obstacles.len(),
        world.food_growers.len(),
        stats.sim_time.as_secs_f64() * 1_000.0,
        stats.upload_time.as_secs_f64() * 1_000.0,
        config.width,
        config.height,
    );
}
