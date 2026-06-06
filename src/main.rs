mod menu;
mod rendering;
mod simulation;

use bevy::app::AppExit;
use bevy::audio::{PlaybackMode, Volume};
use bevy::camera::ScalingMode;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};
use rendering::{
    InstancedDiscPlugin, LiquidMediumMaterial, SimulationRenderEntity, spawn_simulation_layers,
    sync_instance_data,
};
use simulation::{
    CELL_DIVISION_THRESHOLD_DISPLAY_MAX, CELL_MUTATION_DISPLAY_MAX, CELL_SPEED_DISPLAY_MAX,
    CELL_TURN_DISPLAY_MAX, CELL_VIABILITY_MAX, FrameStats, SimConfig, WorldState,
};
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

#[derive(Component)]
struct RunningUiEntity;

#[derive(Resource, Default)]
struct SelectedCell {
    cell_id: Option<u64>,
}

#[derive(Resource)]
struct GameUiState {
    paused: bool,
    passport_open: bool,
    pause_menu_open: bool,
    speed_multiplier: f32,
}

impl Default for GameUiState {
    fn default() -> Self {
        Self {
            paused: false,
            passport_open: false,
            pause_menu_open: false,
            speed_multiplier: 1.0,
        }
    }
}

#[derive(Component)]
struct SelectionPanel;

#[derive(Component)]
struct SelectionCellTitle;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum GeneStatId {
    Viability,
    Speed,
    Turn,
    Mutation,
    Size,
}

#[derive(Component)]
struct GeneBarFill {
    kind: GeneStatId,
}

#[derive(Component)]
struct GeneValueText {
    kind: GeneStatId,
}

#[derive(Component)]
struct GeneRangeText {
    kind: GeneStatId,
}

#[derive(Component)]
struct DivisionThresholdMarker;

#[derive(Component)]
struct DivisionTooltip;

#[derive(Component)]
struct DivisionTooltipText;

#[derive(Component)]
struct DivisionTooltipValueText;

#[derive(Component)]
struct PassportPanel;

#[derive(Component)]
struct PassportCellTitle;

#[derive(Component)]
struct PassportToggleButton;

#[derive(Component)]
struct PauseIndicator;

#[derive(Component)]
struct PauseMenu;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum PauseMenuAction {
    Resume,
    MainMenu,
    Exit,
}

#[derive(Component)]
struct SpeedButton {
    multiplier: f32,
}

#[derive(Component)]
struct SpeedPanel;

#[derive(Component)]
struct SpeedButtonLabel;

#[derive(Resource)]
struct CellAudioLibrary {
    effects: Vec<Handle<AudioSource>>,
    ambient: Handle<AudioSource>,
}

#[derive(Resource, Default)]
struct CellAudioState {
    last_event_serial: u64,
    next_effect: usize,
    cooldown: f32,
}

#[derive(Component)]
struct RunningAudioEntity;

const START_VIEW_HEIGHT: f32 = 1_470.0;
const CAMERA_MOVE_SPEED: f32 = 1_100.0;
const ZOOM_FACTOR: f32 = 1.18;
const MIN_ZOOM_SCALE: f32 = 0.08;
const MAX_ZOOM_SCALE: f32 = 12.0;
const UI_FONT: &str = "fonts/FiraSansExtraCondensed-Regular.ttf";

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
        .init_resource::<SelectedCell>()
        .init_resource::<GameUiState>()
        .init_resource::<FrameStats>()
        .init_resource::<CellAudioState>()
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
        .add_systems(Startup, (setup_camera, load_cell_audio))
        .add_systems(
            OnEnter(AppState::Running),
            (
                spawn_simulation_layers,
                setup_game_stats_ui,
                setup_biolab_ui_v2,
                initialize_world_state,
                start_running_audio,
                update_window_title,
            ),
        )
        .add_systems(OnExit(AppState::Running), cleanup_running_game)
        .add_systems(
            Update,
            (
                game_ui_input_system,
                camera_controls,
                select_cell_system,
                step_simulation,
                play_cell_audio_events,
                sync_instance_data,
                update_stats_overlay,
                update_selection_ui,
                update_pause_ui,
                passport_toggle_button_system,
                pause_menu_button_system,
                pause_menu_button_style_system,
                speed_button_system,
                update_speed_button_styles,
            )
                .chain()
                .run_if(in_state(AppState::Running)),
        )
        .run();
}

fn load_cell_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    let effect_paths = [
        "sounds/biotroph-death1.wav",
        "sounds/biotroph-death2.wav",
        "sounds/biotroph-eat1.wav",
        "sounds/biotroph-eat2.wav",
        "sounds/biotroph-eat3.wav",
        "sounds/biotroph-eat4.wav",
        "sounds/biotroph-fear1.wav",
        "sounds/biotroph-fear2.wav",
        "sounds/cell-spawn.wav",
        "sounds/necrotroph-death1.wav",
        "sounds/necrotroph-death2.wav",
        "sounds/necrotroph-death3.wav",
        "sounds/necrotroph-death4.wav",
        "sounds/necrotroph-death5.wav",
        "sounds/necrotroph-eat1.wav",
        "sounds/necrotroph-eat2.wav",
        "sounds/necrotroph-spotting1.wav",
        "sounds/necrotroph-spotting2.wav",
    ];
    commands.insert_resource(CellAudioLibrary {
        effects: effect_paths
            .into_iter()
            .map(|path| asset_server.load(path))
            .collect(),
        ambient: asset_server.load("sounds/underwater-ambient-loop.wav"),
    });
}

fn start_running_audio(
    mut commands: Commands,
    library: Res<CellAudioLibrary>,
    world: Res<WorldState>,
    mut state: ResMut<CellAudioState>,
) {
    state.last_event_serial = world.cell_sound_event_serial;
    state.cooldown = 0.0;
    commands.spawn((
        AudioPlayer(library.ambient.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(0.18),
            ..default()
        },
        RunningAudioEntity,
    ));
}

fn play_cell_audio_events(
    time: Res<Time>,
    world: Res<WorldState>,
    library: Res<CellAudioLibrary>,
    mut state: ResMut<CellAudioState>,
    mut commands: Commands,
) {
    state.cooldown = (state.cooldown - time.delta_secs()).max(0.0);
    if world.cell_sound_event_serial == state.last_event_serial || state.cooldown > 0.0 {
        return;
    }
    state.last_event_serial = world.cell_sound_event_serial;
    let effect = library.effects[state.next_effect % library.effects.len()].clone();
    state.next_effect = state.next_effect.wrapping_add(1);
    state.cooldown = 0.14;
    commands.spawn((
        AudioPlayer(effect),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(0.24),
            ..default()
        },
        RunningAudioEntity,
    ));
}

fn initialize_world_state(
    mut commands: Commands,
    config: Res<SimConfig>,
    mut selected: ResMut<SelectedCell>,
    mut ui_state: ResMut<GameUiState>,
) {
    selected.cell_id = None;
    *ui_state = GameUiState::default();
    commands.insert_resource(WorldState::new(&config));
}

fn update_window_title(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    config: Res<SimConfig>,
) {
    if let Some(mut window) = windows.iter_mut().next() {
        window.title = format!("Organoids - {} клеток / {} еды", config.cells, config.food);
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

#[allow(dead_code)]
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
        RunningUiEntity,
    ));
}

fn setup_game_stats_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Text::new("Загрузка"),
        TextFont {
            font: asset_server.load(UI_FONT),
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
        RunningUiEntity,
    ));
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GeneCategory {
    Life,
    Movement,
    Reproduction,
}

struct GeneStatDescriptor {
    id: GeneStatId,
    label: &'static str,
    icon: &'static str,
    category: GeneCategory,
    compact: bool,
    color: Color,
}

fn gene_stat_descriptors() -> Vec<GeneStatDescriptor> {
    vec![
        GeneStatDescriptor {
            id: GeneStatId::Viability,
            label: "Жизнеспособность",
            icon: "sprites/gene-viability.png",
            category: GeneCategory::Life,
            compact: true,
            color: Color::srgb(0.35, 0.95, 0.46),
        },
        GeneStatDescriptor {
            id: GeneStatId::Speed,
            label: "Скорость",
            icon: "sprites/gene-speed.png",
            category: GeneCategory::Movement,
            compact: true,
            color: Color::srgb(0.42, 0.72, 1.0),
        },
        GeneStatDescriptor {
            id: GeneStatId::Turn,
            label: "Поворотливость",
            icon: "sprites/gene-maneuverability.png",
            category: GeneCategory::Movement,
            compact: true,
            color: Color::srgb(0.95, 0.78, 0.36),
        },
        GeneStatDescriptor {
            id: GeneStatId::Mutation,
            label: "Мутации",
            icon: "sprites/gene-mutation.png",
            category: GeneCategory::Reproduction,
            compact: true,
            color: Color::srgb(0.77, 0.56, 1.0),
        },
        GeneStatDescriptor {
            id: GeneStatId::Size,
            label: "Размер",
            icon: "sprites/gene-size.png",
            category: GeneCategory::Life,
            compact: true,
            color: Color::srgb(0.70, 0.95, 0.86),
        },
    ]
}

fn setup_biolab_ui_v2(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT);
    let stats = gene_stat_descriptors();

    spawn_compact_selection_panel(&mut commands, &asset_server, font.clone(), &stats);
    spawn_passport_panel(&mut commands, &asset_server, font.clone(), &stats);
    spawn_division_tooltip(&mut commands, font.clone());
    spawn_pause_indicator(&mut commands, font.clone());
    spawn_pause_menu(&mut commands, font.clone());
    spawn_speed_panel(&mut commands, font);
}

fn spawn_compact_selection_panel(
    commands: &mut Commands,
    asset_server: &AssetServer,
    font: Handle<Font>,
    stats: &[GeneStatDescriptor],
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(12),
                right: px(12),
                width: px(470),
                padding: UiRect::all(px(18)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Column,
                row_gap: px(11),
                ..default()
            },
            BorderColor::all(Color::srgb(0.39, 0.64, 0.70)),
            BackgroundColor(Color::srgb(0.025, 0.035, 0.043)),
            Visibility::Hidden,
            SelectionPanel,
            RunningUiEntity,
        ))
        .with_children(|panel| {
            panel
                .spawn((Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },))
                .with_children(|header| {
                    header.spawn((
                        Text::new("ОСМОТР КЛЕТКИ"),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.76, 0.94, 0.92)),
                        SelectionCellTitle,
                    ));

                    header
                        .spawn((
                            Button,
                            Node {
                                width: px(68),
                                height: px(34),
                                border: UiRect::all(px(2)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.46, 0.76, 0.84)),
                            BackgroundColor(Color::srgb(0.07, 0.12, 0.14)),
                            PassportToggleButton,
                        ))
                        .with_child((
                            Text::new("TAB"),
                            TextFont {
                                font: font.clone(),
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.78, 0.96, 0.94)),
                        ));
                });

            for descriptor in stats.iter().filter(|stat| stat.compact) {
                spawn_biolab_stat_row(
                    panel,
                    font.clone(),
                    asset_server.load(descriptor.icon),
                    descriptor.label,
                    descriptor.id,
                    descriptor.color,
                    false,
                );
            }
        });
}

fn spawn_passport_panel(
    commands: &mut Commands,
    asset_server: &AssetServer,
    font: Handle<Font>,
    stats: &[GeneStatDescriptor],
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(12),
                right: px(12),
                width: px(830),
                height: percent(96),
                padding: UiRect::all(px(20)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Column,
                row_gap: px(16),
                overflow: Overflow::clip_y(),
                ..default()
            },
            BorderColor::all(Color::srgb(0.44, 0.74, 0.82)),
            BackgroundColor(Color::srgb(0.018, 0.027, 0.034)),
            Visibility::Hidden,
            PassportPanel,
            RunningUiEntity,
        ))
        .with_children(|passport| {
            passport
                .spawn((Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },))
                .with_children(|header| {
                    header.spawn((
                        Text::new("ПАСПОРТ КЛЕТКИ"),
                        TextFont {
                            font: font.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.78, 0.97, 0.94)),
                        PassportCellTitle,
                    ));

                    header
                        .spawn((
                            Button,
                            Node {
                                width: px(92),
                                height: px(36),
                                border: UiRect::all(px(2)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.46, 0.76, 0.84)),
                            BackgroundColor(Color::srgb(0.07, 0.12, 0.14)),
                            PassportToggleButton,
                        ))
                        .with_child((
                            Text::new("Скрыть"),
                            TextFont {
                                font: font.clone(),
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.96, 0.94)),
                        ));
                });

            passport
                .spawn((Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(16),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },))
                .with_children(|columns| {
                    spawn_passport_column(
                        columns,
                        asset_server,
                        font.clone(),
                        stats,
                        &[(GeneCategory::Life, "Жизнь")],
                    );
                    spawn_passport_column(
                        columns,
                        asset_server,
                        font.clone(),
                        stats,
                        &[(GeneCategory::Movement, "Движение")],
                    );
                    spawn_passport_column(
                        columns,
                        asset_server,
                        font,
                        stats,
                        &[(GeneCategory::Reproduction, "Размножение")],
                    );
                });
        });
}

fn spawn_passport_column(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    font: Handle<Font>,
    stats: &[GeneStatDescriptor],
    categories: &[(GeneCategory, &'static str)],
) {
    parent
        .spawn((Node {
            width: percent(33),
            flex_direction: FlexDirection::Column,
            row_gap: px(13),
            ..default()
        },))
        .with_children(|column| {
            for (category, label) in categories {
                column.spawn((
                    Text::new(*label),
                    TextFont {
                        font: font.clone(),
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.64, 0.93, 0.88)),
                ));

                for descriptor in stats.iter().filter(|stat| stat.category == *category) {
                    spawn_biolab_stat_row(
                        column,
                        font.clone(),
                        asset_server.load(descriptor.icon),
                        descriptor.label,
                        descriptor.id,
                        descriptor.color,
                        true,
                    );
                }
            }
        });
}

fn spawn_biolab_stat_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    icon: Handle<Image>,
    label: &str,
    kind: GeneStatId,
    fill_color: Color,
    show_range: bool,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(11),
                width: percent(100),
                padding: UiRect::axes(px(10), px(8)),
                border: UiRect::left(px(4)),
                ..default()
            },
            BorderColor::all(fill_color),
            BackgroundColor(Color::srgb(0.04, 0.065, 0.075)),
        ))
        .with_children(|row| {
            row.spawn((
                ImageNode::new(icon),
                Node {
                    width: px(34),
                    height: px(34),
                    ..default()
                },
            ));

            row.spawn((Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                flex_grow: 1.0,
                ..default()
            },))
                .with_children(|content| {
                    content
                        .spawn((Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            ..default()
                        },))
                        .with_children(|line| {
                            line.spawn((
                                Text::new(label),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.70, 0.76, 0.80)),
                            ));

                            line.spawn((
                                Text::new("0"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.91, 0.96, 0.97)),
                                GeneValueText { kind },
                            ));
                        });

                    content
                        .spawn((
                            Node {
                                width: percent(100),
                                height: px(13),
                                border: UiRect::all(px(2)),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.20, 0.31, 0.34)),
                            BackgroundColor(Color::srgb(0.08, 0.12, 0.14)),
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                Node {
                                    width: percent(0),
                                    height: percent(100),
                                    ..default()
                                },
                                BackgroundColor(fill_color),
                                GeneBarFill { kind },
                            ));

                            if kind == GeneStatId::Viability {
                                bar.spawn((
                                    Button,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: percent(0),
                                        top: px(-10),
                                        width: px(34),
                                        height: px(34),
                                        margin: UiRect::left(px(-17)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    DivisionThresholdMarker,
                                ))
                                .with_child((
                                    Node {
                                        width: px(8),
                                        height: px(27),
                                        border: UiRect::all(px(2)),
                                        ..default()
                                    },
                                    BorderColor::all(Color::srgb(0.95, 1.0, 0.74)),
                                    BackgroundColor(Color::srgb(0.78, 1.0, 0.56)),
                                ));
                            }
                        });

                    if show_range {
                        content.spawn((
                            Text::new("0-100"),
                            TextFont {
                                font,
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.43, 0.56, 0.60)),
                            GeneRangeText { kind },
                        ));
                    }
                });
        });
}

fn spawn_pause_indicator(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(14),
                left: percent(50),
                width: px(130),
                height: px(34),
                margin: UiRect::left(px(-65)),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(8),
                ..default()
            },
            BorderColor::all(Color::srgb(0.60, 0.86, 0.92)),
            BackgroundColor(Color::srgb(0.025, 0.037, 0.046)),
            Visibility::Hidden,
            PauseIndicator,
            RunningUiEntity,
        ))
        .with_children(|pause| {
            pause.spawn((
                Text::new("||"),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.82, 1.0, 0.96)),
            ));
            pause.spawn((
                Text::new("Пауза"),
                TextFont {
                    font,
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.82, 1.0, 0.96)),
            ));
        });
}

fn spawn_division_tooltip(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: px(370),
                padding: UiRect::all(px(13)),
                border: UiRect::all(px(3)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                ..default()
            },
            BorderColor::all(Color::srgb(0.78, 1.0, 0.62)),
            BackgroundColor(Color::srgb(0.020, 0.045, 0.035)),
            GlobalZIndex(80),
            Visibility::Hidden,
            DivisionTooltip,
            RunningUiEntity,
        ))
        .with_children(|tooltip| {
            tooltip.spawn((
                Text::new("ПОРОГ ДЕЛЕНИЯ"),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.98, 0.88)),
            ));

            tooltip.spawn((
                Text::new("0%"),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.92, 1.0, 0.46)),
                DivisionTooltipValueText,
            ));

            tooltip.spawn((
                Text::new(
                    "Когда жизнеспособность достигает этой отметки, клетка может дать потомство.",
                ),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.84, 0.94, 0.88)),
                DivisionTooltipText,
            ));
        });
}

fn spawn_pause_menu(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: percent(50),
                left: percent(50),
                width: px(360),
                margin: UiRect::new(px(-180), px(0), px(-140), px(0)),
                padding: UiRect::all(px(20)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            BorderColor::all(Color::srgb(0.50, 0.80, 0.86)),
            BackgroundColor(Color::srgb(0.020, 0.030, 0.037)),
            GlobalZIndex(120),
            Visibility::Hidden,
            PauseMenu,
            RunningUiEntity,
        ))
        .with_children(|menu| {
            menu.spawn((
                Text::new("ПАУЗА"),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.80, 0.98, 0.94)),
            ));

            spawn_pause_menu_button(menu, font.clone(), PauseMenuAction::Resume, "Продолжить");
            spawn_pause_menu_button(
                menu,
                font.clone(),
                PauseMenuAction::MainMenu,
                "Главное меню",
            );
            spawn_pause_menu_button(menu, font, PauseMenuAction::Exit, "Выход");
        });
}

fn spawn_pause_menu_button(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    action: PauseMenuAction,
    label: &str,
) {
    parent
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: px(48),
                border: UiRect::all(px(2)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.34, 0.58, 0.64)),
            BackgroundColor(Color::srgb(0.06, 0.10, 0.12)),
            action,
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font,
                font_size: 17.0,
                ..default()
            },
            TextColor(Color::srgb(0.84, 0.94, 0.95)),
        ));
}

fn spawn_speed_panel(commands: &mut Commands, font: Handle<Font>) {
    let speeds: &[(f32, &str)] = &[
        (0.0, "⏸"),
        (0.1, "0.1×"),
        (0.5, "0.5×"),
        (1.0, "1×"),
        (2.0, "2×"),
        (5.0, "5×"),
        (10.0, "10×"),
    ];

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(14),
                left: percent(50),
                margin: UiRect::new(px(-220), px(0), px(0), px(0)),
                width: px(440),
                padding: UiRect::axes(px(12), px(8)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            BorderColor::all(Color::srgb(0.44, 0.74, 0.82)),
            BackgroundColor(Color::srgb(0.018, 0.027, 0.034)),
            GlobalZIndex(100),
            SpeedPanel,
            RunningUiEntity,
        ))
        .with_children(|panel| {
            // Label
            panel.spawn((
                Text::new("СКОРОСТЬ"),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.80, 0.76)),
                Node {
                    margin: UiRect::right(px(6)),
                    ..default()
                },
            ));

            for (mult, label) in speeds {
                let is_active = *mult == 1.0;
                let bg = if is_active {
                    Color::srgb(0.14, 0.30, 0.34)
                } else {
                    Color::srgb(0.06, 0.10, 0.12)
                };
                let border_col = if is_active {
                    Color::srgb(0.50, 0.88, 0.92)
                } else {
                    Color::srgb(0.34, 0.58, 0.64)
                };

                panel
                    .spawn((
                        Button,
                        Node {
                            width: px(48),
                            height: px(30),
                            border: UiRect::all(px(2)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BorderColor::all(border_col),
                        BackgroundColor(bg),
                        SpeedButton { multiplier: *mult },
                    ))
                    .with_child((
                        Text::new(*label),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.82, 0.96, 0.94)),
                        SpeedButtonLabel,
                    ));
            }
        });

    info!(
        "[SpeedPanel] spawned speed control panel with {} buttons",
        speeds.len()
    );
}

fn speed_button_system(
    interactions: Query<(&Interaction, &SpeedButton), (Changed<Interaction>, With<Button>)>,
    mut ui_state: ResMut<GameUiState>,
) {
    for (interaction, speed_btn) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if speed_btn.multiplier == 0.0 {
            // Pause toggle
            ui_state.paused = !ui_state.paused;
            if !ui_state.paused {
                ui_state.pause_menu_open = false;
            }
            info!("[SpeedPanel] pause toggled -> {}", ui_state.paused);
        } else {
            ui_state.speed_multiplier = speed_btn.multiplier;
            ui_state.paused = false;
            ui_state.pause_menu_open = false;
            info!("[SpeedPanel] speed set to {}x", speed_btn.multiplier);
        }
    }
}

fn update_speed_button_styles(
    ui_state: Res<GameUiState>,
    mut buttons: Query<(
        &SpeedButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (speed_btn, interaction, mut bg, mut border) in &mut buttons {
        let is_active = if speed_btn.multiplier == 0.0 {
            ui_state.paused
        } else {
            !ui_state.paused && (speed_btn.multiplier - ui_state.speed_multiplier).abs() < 0.001
        };

        let base_bg = if is_active {
            Color::srgb(0.14, 0.30, 0.34)
        } else {
            Color::srgb(0.06, 0.10, 0.12)
        };

        bg.0 = match *interaction {
            Interaction::Pressed => Color::srgb(0.18, 0.38, 0.42),
            Interaction::Hovered => {
                if is_active {
                    Color::srgb(0.17, 0.35, 0.39)
                } else {
                    Color::srgb(0.09, 0.17, 0.19)
                }
            }
            Interaction::None => base_bg,
        };

        *border = if is_active {
            BorderColor::all(Color::srgb(0.50, 0.88, 0.92))
        } else {
            BorderColor::all(Color::srgb(0.34, 0.58, 0.64))
        };
    }
}

#[allow(dead_code)]
fn setup_biolab_selection_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(12),
                right: px(12),
                width: px(395),
                padding: UiRect::all(px(15)),
                border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Column,
                row_gap: px(11),
                ..default()
            },
            BorderColor::all(Color::srgb(0.39, 0.64, 0.70)),
            BackgroundColor(Color::srgb(0.025, 0.035, 0.043)),
            Visibility::Hidden,
            SelectionPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("\u{041E}\u{0421}\u{041C}\u{041E}\u{0422}\u{0420} \u{041A}\u{041B}\u{0415}\u{0422}\u{041A}\u{0418}"),
                TextFont {
                    font: font.clone(),
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.76, 0.94, 0.92)),
                SelectionCellTitle,
            ));

            spawn_biolab_gene_row(
                panel,
                font.clone(),
                asset_server.load("sprites/gene-viability.png"),
                "\u{0416}\u{0438}\u{0437}\u{043D}\u{0435}\u{0441}\u{043F}\u{043E}\u{0441}\u{043E}\u{0431}\u{043D}\u{043E}\u{0441}\u{0442}\u{044C}",
                GeneStatId::Viability,
                Color::srgb(0.35, 0.95, 0.46),
                true,
            );
            spawn_biolab_gene_row(
                panel,
                font.clone(),
                asset_server.load("sprites/gene-speed.png"),
                "\u{0421}\u{043A}\u{043E}\u{0440}\u{043E}\u{0441}\u{0442}\u{044C}",
                GeneStatId::Speed,
                Color::srgb(0.42, 0.72, 1.0),
                false,
            );
            spawn_biolab_gene_row(
                panel,
                font.clone(),
                asset_server.load("sprites/gene-maneuverability.png"),
                "\u{041F}\u{043E}\u{0432}\u{043E}\u{0440}\u{043E}\u{0442}\u{043B}\u{0438}\u{0432}\u{043E}\u{0441}\u{0442}\u{044C}",
                GeneStatId::Turn,
                Color::srgb(0.95, 0.78, 0.36),
                false,
            );
            spawn_biolab_gene_row(
                panel,
                font.clone(),
                asset_server.load("sprites/gene-mutation.png"),
                "\u{041C}\u{0443}\u{0442}\u{0430}\u{0446}\u{0438}\u{0438}",
                GeneStatId::Mutation,
                Color::srgb(0.77, 0.56, 1.0),
                false,
            );

            panel.spawn((
                Text::new("\u{0417}\u{0430}\u{0441}\u{0435}\u{0447}\u{043A}\u{0430}: \u{043F}\u{043E}\u{0440}\u{043E}\u{0433} \u{0434}\u{0435}\u{043B}\u{0435}\u{043D}\u{0438}\u{044F}"),
                TextFont {
                    font,
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.66, 0.79, 0.82)),
                Node {
                    padding: UiRect::all(px(8)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.05, 0.08, 0.09)),
                Visibility::Hidden,
                DivisionTooltip,
                DivisionTooltipText,
            ));
        });
}

#[allow(dead_code)]
fn spawn_biolab_gene_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    icon: Handle<Image>,
    label: &str,
    kind: GeneStatId,
    fill_color: Color,
    show_division_marker: bool,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            width: percent(100),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                ImageNode::new(icon),
                Node {
                    width: px(30),
                    height: px(30),
                    ..default()
                },
            ));

            row.spawn((Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                width: px(228),
                ..default()
            },))
                .with_children(|content| {
                    content.spawn((
                        Text::new(label),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.68, 0.72, 0.78)),
                    ));

                    content
                        .spawn((
                            Node {
                                width: percent(100),
                                height: px(9),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.10, 0.14, 0.16)),
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                Node {
                                    width: percent(0),
                                    height: percent(100),
                                    ..default()
                                },
                                BackgroundColor(fill_color),
                                GeneBarFill { kind },
                            ));

                            if show_division_marker {
                                bar.spawn((
                                    Button,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: percent(0),
                                        top: px(-5),
                                        width: px(5),
                                        height: px(19),
                                        border: UiRect::all(px(1)),
                                        ..default()
                                    },
                                    BorderColor::all(Color::srgb(0.95, 1.0, 0.74)),
                                    BackgroundColor(Color::srgb(0.78, 1.0, 0.56)),
                                    DivisionThresholdMarker,
                                ));
                            }
                        });
                });

            row.spawn((
                Text::new("0"),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.94, 0.96)),
                Node {
                    width: px(68),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                GeneValueText { kind },
            ));
        });
}

#[allow(dead_code)]
fn setup_selection_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(12),
                right: px(12),
                width: px(360),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                ..default()
            },
            BorderColor::all(Color::srgb(0.48, 0.58, 0.68)),
            BackgroundColor(Color::srgb(0.045, 0.052, 0.064)),
            Visibility::Hidden,
            SelectionPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Клетка"),
                TextFont {
                    font: font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.88, 0.93, 0.96)),
                SelectionCellTitle,
            ));

            spawn_gene_row(
                panel,
                font.clone(),
                asset_server.load("sprites/gene-viability.png"),
                "Жизнеспособность",
                GeneStatId::Viability,
                Color::srgb(0.35, 0.95, 0.46),
            );
            spawn_gene_row(
                panel,
                font.clone(),
                asset_server.load("sprites/gene-speed.png"),
                "Скорость",
                GeneStatId::Speed,
                Color::srgb(0.42, 0.72, 1.0),
            );
            spawn_gene_row(
                panel,
                font,
                asset_server.load("sprites/gene-maneuverability.png"),
                "Поворотливость",
                GeneStatId::Turn,
                Color::srgb(0.95, 0.78, 0.36),
            );
        });
}

#[allow(dead_code)]
fn spawn_gene_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    icon: Handle<Image>,
    label: &str,
    kind: GeneStatId,
    fill_color: Color,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            width: percent(100),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                ImageNode::new(icon),
                Node {
                    width: px(30),
                    height: px(30),
                    ..default()
                },
            ));

            row.spawn((Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                width: px(210),
                ..default()
            },))
                .with_children(|content| {
                    content.spawn((
                        Text::new(label),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.68, 0.72, 0.78)),
                    ));

                    content
                        .spawn((
                            Node {
                                width: percent(100),
                                height: px(8),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.14, 0.16, 0.19)),
                        ))
                        .with_child((
                            Node {
                                width: percent(0),
                                height: percent(100),
                                ..default()
                            },
                            BackgroundColor(fill_color),
                            GeneBarFill { kind },
                        ));
                });

            row.spawn((
                Text::new("0"),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.94, 0.96)),
                Node {
                    width: px(68),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                GeneValueText { kind },
            ));
        });
}

fn step_simulation(
    time: Res<Time>,
    ui_state: Res<GameUiState>,
    mut world: ResMut<WorldState>,
    mut stats: ResMut<FrameStats>,
) {
    if ui_state.paused {
        stats.sim_time = std::time::Duration::ZERO;
        return;
    }

    let started = Instant::now();
    let dt = time.delta_secs() * ui_state.speed_multiplier;
    world.update(dt);
    stats.sim_time = started.elapsed();
}

fn select_cell_system(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    camera: Query<(&Transform, &Projection), With<MainCamera>>,
    world: Res<WorldState>,
    mut selected: ResMut<SelectedCell>,
    mut ui_state: ResMut<GameUiState>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok((_, window)) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    if ui_state.pause_menu_open {
        return;
    }

    if selected.cell_id.is_some() {
        let panel_width = if ui_state.passport_open { 870.0 } else { 510.0 };
        let panel_height = if ui_state.passport_open {
            window.height()
        } else {
            650.0
        };
        if cursor.x > window.width() - panel_width && cursor.y < panel_height {
            return;
        }
    }

    let Ok((transform, projection)) = camera.single() else {
        return;
    };
    let Projection::Orthographic(projection) = projection else {
        return;
    };

    let world_position = cursor_to_world(cursor, transform.translation, projection, window);
    let view_size = visible_world_size(projection, window);
    let screen_pick_radius = (view_size.y / window.height().max(1.0) * 15.0).max(8.0);
    let mut best = None;
    let mut best_dist_sq = f32::MAX;

    for i in 0..world.cells.len() {
        let dx = world.cells.x[i] - world_position.x;
        let dy = world.cells.y[i] - world_position.y;
        let dist_sq = dx * dx + dy * dy;
        let pick_radius = world.cells.radius[i] + screen_pick_radius;

        if dist_sq <= pick_radius * pick_radius && dist_sq < best_dist_sq {
            best = Some(i);
            best_dist_sq = dist_sq;
        }
    }

    selected.cell_id = best.map(|index| world.cells.id[index]);
    if selected.cell_id.is_none() {
        ui_state.passport_open = false;
    }
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

struct StatUiValue {
    normalized: f32,
    display: String,
    range: String,
}

fn game_ui_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedCell>,
    mut ui_state: ResMut<GameUiState>,
) {
    if keys.just_pressed(KeyCode::Space) {
        ui_state.paused = !ui_state.paused;
        if !ui_state.paused {
            ui_state.pause_menu_open = false;
        }
    }

    if keys.just_pressed(KeyCode::Tab) {
        if selected.cell_id.is_some() {
            ui_state.passport_open = !ui_state.passport_open;
        } else {
            ui_state.passport_open = false;
        }
    }

    if keys.just_pressed(KeyCode::Escape) {
        if ui_state.pause_menu_open {
            ui_state.pause_menu_open = false;
            ui_state.paused = false;
        } else {
            ui_state.pause_menu_open = true;
            ui_state.paused = true;
        }
    }

    // Speed control shortcuts: 1-7 keys
    const SPEED_KEYS: [(KeyCode, f32); 7] = [
        (KeyCode::Digit1, 0.0),
        (KeyCode::Digit2, 0.1),
        (KeyCode::Digit3, 0.5),
        (KeyCode::Digit4, 1.0),
        (KeyCode::Digit5, 2.0),
        (KeyCode::Digit6, 5.0),
        (KeyCode::Digit7, 10.0),
    ];
    for (key, speed) in SPEED_KEYS {
        if keys.just_pressed(key) {
            if speed == 0.0 {
                ui_state.paused = !ui_state.paused;
                if !ui_state.paused {
                    ui_state.pause_menu_open = false;
                }
            } else {
                ui_state.speed_multiplier = speed;
                ui_state.paused = false;
                ui_state.pause_menu_open = false;
            }
        }
    }

    if selected.cell_id.is_none() {
        ui_state.passport_open = false;
    }
}

fn update_stats_overlay(
    diagnostics: Res<DiagnosticsStore>,
    world: Res<WorldState>,
    stats: Res<FrameStats>,
    config: Res<SimConfig>,
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

    let Ok(mut text) = text.single_mut() else {
        return;
    };

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

fn update_selection_ui(
    world: Res<WorldState>,
    mut selected: ResMut<SelectedCell>,
    ui_state: Res<GameUiState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut compact_panel: Query<&mut Visibility, (With<SelectionPanel>, Without<PassportPanel>)>,
    mut passport_panel: Query<&mut Visibility, (With<PassportPanel>, Without<SelectionPanel>)>,
    mut compact_title: Query<
        &mut Text,
        (
            With<SelectionCellTitle>,
            Without<PassportCellTitle>,
            Without<GeneValueText>,
            Without<GeneRangeText>,
            Without<DivisionTooltipText>,
            Without<DivisionTooltipValueText>,
        ),
    >,
    mut passport_title: Query<
        &mut Text,
        (
            With<PassportCellTitle>,
            Without<SelectionCellTitle>,
            Without<GeneValueText>,
            Without<GeneRangeText>,
            Without<DivisionTooltipText>,
            Without<DivisionTooltipValueText>,
        ),
    >,
    mut bar_fills: Query<(&GeneBarFill, &mut Node), Without<DivisionThresholdMarker>>,
    mut division_markers: Query<&mut Node, (With<DivisionThresholdMarker>, Without<GeneBarFill>)>,
    marker_interactions: Query<&Interaction, With<DivisionThresholdMarker>>,
    mut tooltip: Query<
        (&mut Visibility, &mut Node),
        (
            With<DivisionTooltip>,
            Without<SelectionPanel>,
            Without<PassportPanel>,
            Without<SelectionCellTitle>,
            Without<PassportCellTitle>,
            Without<GeneValueText>,
            Without<GeneRangeText>,
            Without<GeneBarFill>,
            Without<DivisionThresholdMarker>,
            Without<DivisionTooltipText>,
            Without<DivisionTooltipValueText>,
        ),
    >,
    mut tooltip_value: Query<
        &mut Text,
        (
            With<DivisionTooltipValueText>,
            Without<DivisionTooltip>,
            Without<DivisionTooltipText>,
            Without<GeneValueText>,
            Without<GeneRangeText>,
            Without<SelectionCellTitle>,
            Without<PassportCellTitle>,
        ),
    >,
    mut gene_values: Query<
        (&GeneValueText, &mut Text),
        (
            Without<GeneRangeText>,
            Without<SelectionCellTitle>,
            Without<PassportCellTitle>,
            Without<DivisionTooltipText>,
            Without<DivisionTooltipValueText>,
        ),
    >,
    mut gene_ranges: Query<
        (&GeneRangeText, &mut Text),
        (
            Without<GeneValueText>,
            Without<SelectionCellTitle>,
            Without<PassportCellTitle>,
            Without<DivisionTooltipText>,
            Without<DivisionTooltipValueText>,
        ),
    >,
) {
    let selected_index = selected
        .cell_id
        .and_then(|cell_id| world.cell_index_by_id(cell_id));
    if selected.cell_id.is_some() && selected_index.is_none() {
        selected.cell_id = None;
    }

    let has_selection = selected_index.is_some();
    if let Ok(mut visibility) = compact_panel.single_mut() {
        *visibility = if has_selection && !ui_state.passport_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut visibility) = passport_panel.single_mut() {
        *visibility = if has_selection && ui_state.passport_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    let Some(cell_index) = selected_index else {
        if let Ok((mut tooltip_visibility, _)) = tooltip.single_mut() {
            *tooltip_visibility = Visibility::Hidden;
        }
        return;
    };

    let cell_id = world.cells.id[cell_index];
    let shape_name = world.cells.shape_name(cell_index);
    if let Ok(mut title) = compact_title.single_mut() {
        **title = format!("КЛЕТКА #{cell_id} · {shape_name}");
    }
    if let Ok(mut title) = passport_title.single_mut() {
        **title = format!("ПАСПОРТ КЛЕТКИ #{cell_id} · {shape_name}");
    }

    let division_threshold = world.cells.division_threshold[cell_index];
    for (bar, mut node) in &mut bar_fills {
        let value = stat_ui_value(&world, cell_index, bar.kind);
        node.width = Val::Percent(value.normalized.clamp(0.0, 1.0) * 100.0);
    }

    for mut marker in &mut division_markers {
        marker.left = Val::Percent(
            (division_threshold / CELL_DIVISION_THRESHOLD_DISPLAY_MAX).clamp(0.0, 1.0) * 100.0,
        );
    }

    let marker_hovered = marker_interactions.iter().any(|interaction| {
        *interaction == Interaction::Hovered || *interaction == Interaction::Pressed
    });
    if let Ok((mut tooltip_visibility, mut tooltip_node)) = tooltip.single_mut() {
        *tooltip_visibility = if marker_hovered {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if marker_hovered
            && let Ok(window) = windows.single()
            && let Some(cursor) = window.cursor_position()
        {
            let tooltip_width = 370.0;
            let tooltip_height = 122.0;
            let gap = 18.0;
            let x = if cursor.x + tooltip_width + gap > window.width() {
                cursor.x - tooltip_width - gap
            } else {
                cursor.x + gap
            }
            .clamp(8.0, (window.width() - tooltip_width - 8.0).max(8.0));
            let y = if cursor.y + tooltip_height + gap > window.height() {
                cursor.y - tooltip_height - gap
            } else {
                cursor.y + gap
            }
            .clamp(8.0, (window.height() - tooltip_height - 8.0).max(8.0));

            tooltip_node.left = px(x);
            tooltip_node.top = px(y);
        }
    }
    if let Ok(mut value_text) = tooltip_value.single_mut() {
        **value_text = format!("{division_threshold:.0}% жизнеспособности");
    }

    for (value, mut text) in &mut gene_values {
        **text = stat_ui_value(&world, cell_index, value.kind).display;
    }

    for (range, mut text) in &mut gene_ranges {
        **text = stat_ui_value(&world, cell_index, range.kind).range;
    }
}

fn stat_ui_value(world: &WorldState, cell_index: usize, id: GeneStatId) -> StatUiValue {
    let cells = &world.cells;
    let viability = cells.viability[cell_index];
    let max_viability = cells.max_viability[cell_index].max(1.0);
    let viability_ratio = (viability / max_viability).clamp(0.0, 1.0);
    let radius = cells.radius[cell_index].max(0.1);
    let membrane_size = cells.max_base_radius(cell_index).max(0.1);

    match id {
        GeneStatId::Viability => StatUiValue {
            normalized: viability_ratio,
            display: format!("{viability:.0}/{max_viability:.0}"),
            range: format!("0-{:.0}", CELL_VIABILITY_MAX),
        },
        GeneStatId::Speed => {
            let speed = cells.speed[cell_index];
            StatUiValue {
                normalized: speed / CELL_SPEED_DISPLAY_MAX,
                display: format!("{speed:.0}"),
                range: "30-130".to_string(),
            }
        }
        GeneStatId::Turn => {
            let turn = cells.turn_speed[cell_index];
            StatUiValue {
                normalized: turn / CELL_TURN_DISPLAY_MAX,
                display: format!("{turn:.1}"),
                range: "0.8-6.0".to_string(),
            }
        }
        GeneStatId::Mutation => {
            let mutation = cells.mutation_susceptibility[cell_index];
            StatUiValue {
                normalized: mutation / CELL_MUTATION_DISPLAY_MAX,
                display: format!("{mutation:.0}%"),
                range: "0-100%".to_string(),
            }
        }
        GeneStatId::Size => StatUiValue {
            normalized: (membrane_size / 10.0).clamp(0.0, 1.0),
            display: format!("{membrane_size:.1}/{radius:.1}"),
            range: "4-9".to_string(),
        },
    }
}

fn update_pause_ui(
    ui_state: Res<GameUiState>,
    mut indicator: Query<&mut Visibility, (With<PauseIndicator>, Without<PauseMenu>)>,
    mut menu: Query<&mut Visibility, (With<PauseMenu>, Without<PauseIndicator>)>,
) {
    if let Ok(mut visibility) = indicator.single_mut() {
        *visibility = if ui_state.paused {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut visibility) = menu.single_mut() {
        *visibility = if ui_state.pause_menu_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn passport_toggle_button_system(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<PassportToggleButton>),
    >,
    selected: Res<SelectedCell>,
    mut ui_state: ResMut<GameUiState>,
) {
    for (interaction, mut background) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                if selected.cell_id.is_some() {
                    ui_state.passport_open = !ui_state.passport_open;
                }
                background.0 = Color::srgb(0.10, 0.20, 0.22);
            }
            Interaction::Hovered => {
                background.0 = Color::srgb(0.10, 0.18, 0.20);
            }
            Interaction::None => {
                background.0 = Color::srgb(0.07, 0.12, 0.14);
            }
        }
    }
}

fn pause_menu_button_system(
    mut interactions: Query<(&Interaction, &PauseMenuAction), (Changed<Interaction>, With<Button>)>,
    mut ui_state: ResMut<GameUiState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            PauseMenuAction::Resume => {
                ui_state.paused = false;
                ui_state.pause_menu_open = false;
            }
            PauseMenuAction::MainMenu => {
                ui_state.paused = false;
                ui_state.pause_menu_open = false;
                ui_state.passport_open = false;
                next_state.set(AppState::Menu);
            }
            PauseMenuAction::Exit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

fn pause_menu_button_style_system(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<PauseMenuAction>),
    >,
) {
    for (interaction, mut background) in &mut interactions {
        background.0 = match *interaction {
            Interaction::Pressed => Color::srgb(0.12, 0.23, 0.25),
            Interaction::Hovered => Color::srgb(0.09, 0.17, 0.19),
            Interaction::None => Color::srgb(0.06, 0.10, 0.12),
        };
    }
}

fn cleanup_running_game(
    mut commands: Commands,
    ui_entities: Query<Entity, With<RunningUiEntity>>,
    render_entities: Query<Entity, With<SimulationRenderEntity>>,
    audio_entities: Query<Entity, With<RunningAudioEntity>>,
    mut selected: ResMut<SelectedCell>,
    mut ui_state: ResMut<GameUiState>,
) {
    for entity in &ui_entities {
        commands.entity(entity).despawn();
    }
    for entity in &render_entities {
        commands.entity(entity).despawn();
    }
    for entity in &audio_entities {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<WorldState>();
    selected.cell_id = None;
    *ui_state = GameUiState::default();
}

#[allow(dead_code)]
fn update_ui(
    diagnostics: Res<DiagnosticsStore>,
    world: Res<WorldState>,
    stats: Res<FrameStats>,
    mut text: Query<
        &mut Text,
        (
            With<StatsText>,
            Without<SelectionCellTitle>,
            Without<GeneValueText>,
            Without<DivisionTooltipText>,
        ),
    >,
    config: Res<SimConfig>,
    mut selected: ResMut<SelectedCell>,
    mut panel: Query<&mut Visibility, (With<SelectionPanel>, Without<DivisionTooltip>)>,
    mut title: Query<
        &mut Text,
        (
            With<SelectionCellTitle>,
            Without<StatsText>,
            Without<GeneValueText>,
            Without<DivisionTooltipText>,
        ),
    >,
    mut bar_fills: Query<(&GeneBarFill, &mut Node)>,
    mut division_markers: Query<&mut Node, (With<DivisionThresholdMarker>, Without<GeneBarFill>)>,
    marker_interactions: Query<&Interaction, With<DivisionThresholdMarker>>,
    mut tooltip: Query<
        (&mut Visibility, &mut Text),
        (
            With<DivisionTooltip>,
            With<DivisionTooltipText>,
            Without<SelectionPanel>,
            Without<StatsText>,
            Without<SelectionCellTitle>,
            Without<GeneValueText>,
        ),
    >,
    mut gene_values: Query<
        (&GeneValueText, &mut Text),
        (
            Without<StatsText>,
            Without<SelectionCellTitle>,
            Without<DivisionTooltipText>,
        ),
    >,
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

    let selected_index = selected
        .cell_id
        .and_then(|cell_id| world.cell_index_by_id(cell_id));
    if selected.cell_id.is_some() && selected_index.is_none() {
        selected.cell_id = None;
    }
    if let Ok(mut visibility) = panel.single_mut() {
        *visibility = if selected_index.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    let Some(cell_index) = selected_index else {
        if let Ok((mut tooltip_visibility, _)) = tooltip.single_mut() {
            *tooltip_visibility = Visibility::Hidden;
        }
        return;
    };

    if let Ok(mut title) = title.single_mut() {
        **title = format!(
            "Клетка #{cell_index} · {}",
            world.cells.shape_name(cell_index)
        );
    }

    let viability = world.cells.viability[cell_index];
    let max_viability = world.cells.max_viability[cell_index].max(1.0);
    let speed = world.cells.speed[cell_index];
    let turn = world.cells.turn_speed[cell_index];
    let mutation = world.cells.mutation_susceptibility[cell_index];
    let division_threshold = world.cells.division_threshold[cell_index];

    for (bar, mut node) in &mut bar_fills {
        let percent = match bar.kind {
            GeneStatId::Viability => viability / max_viability,
            GeneStatId::Speed => speed / CELL_SPEED_DISPLAY_MAX,
            GeneStatId::Turn => turn / CELL_TURN_DISPLAY_MAX,
            GeneStatId::Mutation => mutation / CELL_MUTATION_DISPLAY_MAX,
            _ => 0.0,
        }
        .clamp(0.0, 1.0)
            * 100.0;

        node.width = Val::Percent(percent);
    }

    if let Ok(mut marker) = division_markers.single_mut() {
        marker.left = Val::Percent(
            (division_threshold / CELL_DIVISION_THRESHOLD_DISPLAY_MAX).clamp(0.0, 1.0) * 100.0,
        );
    }

    let marker_hovered = marker_interactions.iter().any(|interaction| {
        *interaction == Interaction::Hovered || *interaction == Interaction::Pressed
    });
    if let Ok((mut tooltip_visibility, mut tooltip_text)) = tooltip.single_mut() {
        *tooltip_visibility = if marker_hovered {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        **tooltip_text = format!(
            "{}: {division_threshold:.0}%\n{}",
            "\u{041F}\u{043E}\u{0440}\u{043E}\u{0433} \u{0434}\u{0435}\u{043B}\u{0435}\u{043D}\u{0438}\u{044F}",
            "\u{041F}\u{0440}\u{043E}\u{0446}\u{0435}\u{043D}\u{0442} \u{0436}\u{0438}\u{0437}\u{043D}\u{0435}\u{0441}\u{043F}\u{043E}\u{0441}\u{043E}\u{0431}\u{043D}\u{043E}\u{0441}\u{0442}\u{0438}, \u{043D}\u{0443}\u{0436}\u{043D}\u{044B}\u{0439} \u{0434}\u{043B}\u{044F} \u{0434}\u{0435}\u{043B}\u{0435}\u{043D}\u{0438}\u{044F}."
        );
    }

    for (value, mut text) in &mut gene_values {
        **text = match value.kind {
            GeneStatId::Viability => format!("{viability:.0}/{max_viability:.0}"),
            GeneStatId::Speed => format!("{speed:.0}"),
            GeneStatId::Turn => format!("{turn:.1}"),
            GeneStatId::Mutation => format!("{mutation:.0}%"),
            _ => "0".to_string(),
        };
    }
}
