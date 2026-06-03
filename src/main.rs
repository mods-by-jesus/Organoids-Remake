mod rendering;
mod simulation;

use bevy::camera::ScalingMode;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};
use rendering::{InstancedDiscPlugin, spawn_simulation_layers, sync_instance_data};
use simulation::{ARENA_HEIGHT, ARENA_WIDTH, FrameStats, SimConfig, WorldState};
use std::time::Instant;

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
    let config = match SimConfig::from_args() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            return;
        }
    };

    let present_mode = if config.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.012, 0.015, 0.018)))
        .insert_resource(WorldState::new(&config))
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
                        title: format!("Organoids - {} cells / {} food", config.cells, config.food),
                        present_mode,
                        resolution: WindowResolution::new(1920, 1080)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                }),
            FrameTimeDiagnosticsPlugin::default(),
            InstancedDiscPlugin,
        ))
        .add_systems(Startup, (setup_camera, spawn_simulation_layers, setup_ui))
        .add_systems(
            Update,
            (
                camera_controls,
                step_simulation,
                sync_instance_data,
                update_ui,
            )
                .chain(),
        )
        .run();
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
        Text::new("starting"),
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
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
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

    let view_size = visible_world_size(projection, window);
    if mouse_buttons.pressed(MouseButton::Middle) {
        let mut drag_delta = Vec2::ZERO;
        for event in mouse_motion.read() {
            drag_delta += event.delta;
        }

        if drag_delta != Vec2::ZERO {
            transform.translation.x -= drag_delta.x * view_size.x / window.width();
            transform.translation.y += drag_delta.y * view_size.y / window.height();
        }
    } else {
        mouse_motion.clear();
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

    clamp_camera_to_arena(&mut transform, projection, window);
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

fn clamp_camera_to_arena(
    transform: &mut Transform,
    projection: &OrthographicProjection,
    window: &Window,
) {
    let view_size = visible_world_size(projection, window);
    let max_x = (ARENA_WIDTH * 0.5 - view_size.x * 0.5).max(0.0);
    let max_y = (ARENA_HEIGHT * 0.5 - view_size.y * 0.5).max(0.0);

    transform.translation.x = transform.translation.x.clamp(-max_x, max_x);
    transform.translation.y = transform.translation.y.clamp(-max_y, max_y);
}

fn update_ui(
    diagnostics: Res<DiagnosticsStore>,
    world: Res<WorldState>,
    stats: Res<FrameStats>,
    mut text: Query<&mut Text, With<StatsText>>,
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
        "FPS {fps:>6.1} | frame {frame_ms:>5.2} ms\ncells {:>5} | food {:>5}\nsim {:>5.2} ms | upload {:>5.2} ms\narena {:.0} x {:.0}",
        world.cells.len(),
        world.food.len(),
        stats.sim_time.as_secs_f64() * 1_000.0,
        stats.upload_time.as_secs_f64() * 1_000.0,
        ARENA_WIDTH,
        ARENA_HEIGHT,
    );
}
