use crate::{
    AppState,
    simulation::{ArenaShape, CELL_SHAPE_LABELS, SimConfig},
};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuUiState>()
            .add_systems(OnEnter(AppState::Menu), setup_menu)
            .add_systems(
                Update,
                (
                    menu_interaction_system,
                    keyboard_input_system,
                    sync_menu_values_system,
                    menu_section_visibility_system,
                    menu_button_style_system,
                    animate_menu_buttons,
                    shape_weight_slider_system,
                    sync_shape_weight_sliders,
                    menu_audio_slider_system,
                    sync_menu_audio_sliders,
                    menu_settings_scroll_system,
                )
                    .chain()
                    .run_if(in_state(AppState::Menu)),
            )
            .add_systems(OnExit(AppState::Menu), cleanup_menu);
    }
}

#[derive(Resource, Default)]
struct MenuUiState {
    advanced_open: bool,
    shapes_open: bool,
    keybinds_open: bool,
}

#[derive(Component)]
struct MenuEntity;

#[derive(Component)]
struct MenuSettingsScrollArea;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MenuButtonAction {
    Start,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct PresetButton(MenuPreset);

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuPreset {
    MicroTest,
    Simulator,
    Balanced,
    Performance,
    Stress,
}

#[derive(Component)]
struct InputField {
    value_type: MenuTextValue,
}

#[derive(Component)]
struct FocusedInput;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct MenuValueLabel {
    value_type: MenuTextValue,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MenuTextValue {
    Cells,
    Food,
    Width,
    Height,
    Obstacles,
    FoodGrowers,
    Seed,
    Vsync,
    SegmentedCells,
    Arena,
    ArenaShape,
    RandomGeometry,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MenuSection {
    Advanced,
    Shapes,
    Keybinds,
}

#[derive(Component)]
struct MenuSectionToggle {
    section: MenuSection,
}

#[derive(Component)]
struct MenuSectionBody {
    section: MenuSection,
}

#[derive(Component)]
struct VsyncToggle;

#[derive(Component)]
struct SegmentedCellsToggle;

#[derive(Component)]
struct ArenaShapeToggle;

#[derive(Component)]
struct RandomizeSeedButton;

#[derive(Component)]
struct RandomGeometryToggle;

#[derive(Component)]
struct ShapeWeightSlider(usize);

#[derive(Component)]
struct ShapeWeightFill(usize);

#[derive(Component)]
struct ShapeWeightValue(usize);

#[derive(Clone, Copy)]
enum MenuAudioKind {
    Effects,
    Ambient,
}

#[derive(Component)]
struct MenuAudioSlider(MenuAudioKind);

#[derive(Component)]
struct MenuAudioFill(MenuAudioKind);

#[derive(Component)]
struct MenuAudioValue(MenuAudioKind);

fn relative_cursor_fraction_x(cursor: &RelativeCursorPosition) -> Option<f32> {
    cursor
        .normalized
        .map(|position| (position.x + 0.5).clamp(0.0, 1.0))
}

const NORMAL_BORDER: Color = Color::srgb(0.18, 0.30, 0.34);
const FOCUSED_BORDER: Color = Color::srgb(0.46, 0.82, 0.90);
const NORMAL_BG: Color = Color::srgb(0.045, 0.070, 0.080);
const HOVERED_BG: Color = Color::srgb(0.075, 0.125, 0.140);
const PRESSED_BG: Color = Color::srgb(0.105, 0.180, 0.195);
const UI_FONT: &str = "fonts/FiraSansExtraCondensed-Regular.ttf";

fn setup_menu(
    mut commands: Commands,
    config: Res<SimConfig>,
    asset_server: Res<AssetServer>,
    mut menu_state: ResMut<MenuUiState>,
) {
    menu_state.advanced_open = false;
    menu_state.shapes_open = false;
    menu_state.keybinds_open = false;
    let font = asset_server.load(UI_FONT);

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Relative,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(34)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.012, 0.018, 0.022)),
            MenuEntity,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(1080),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(18),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.012, 0.020, 0.026)),
                BorderColor::all(Color::srgb(0.28, 0.52, 0.58)),
            ))
            .with_children(|frame| {
                frame
                    .spawn((Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::FlexEnd,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },))
                    .with_children(|header| {
                        header.spawn((
                            Text::new("ORGANOIDS"),
                            TextFont {
                                font: font.clone(),
                                font_size: 54.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.58, 0.92, 0.96)),
                        ));

                        header.spawn((
                            Text::new("лабораторный запуск симуляции"),
                            TextFont {
                                font: font.clone(),
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.56, 0.70, 0.74)),
                        ));
                    });

                frame
                    .spawn((Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        column_gap: px(18),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    },))
                    .with_children(|columns| {
                        spawn_settings_column(columns, font.clone(), &config);
                        spawn_launch_column(columns, font.clone(), &config);
                    });
            });
        });
}

fn spawn_settings_column(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    config: &SimConfig,
) {
    parent
        .spawn((
            Node {
                width: px(610),
                height: vh(72),
                padding: UiRect::all(px(18)),
                border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BorderColor::all(Color::srgb(0.33, 0.58, 0.64)),
            BackgroundColor(Color::srgb(0.020, 0.032, 0.040)),
            ScrollPosition::default(),
            RelativeCursorPosition::default(),
            MenuSettingsScrollArea,
        ))
        .with_children(|column| {
            spawn_section_title(column, font.clone(), "Параметры мира");

            spawn_setting_row(
                column,
                font.clone(),
                "Клетки",
                MenuTextValue::Cells,
                config.cells,
            );
            spawn_setting_row(
                column,
                font.clone(),
                "Еда",
                MenuTextValue::Food,
                config.food,
            );
            spawn_setting_row(
                column,
                font.clone(),
                "Ширина арены",
                MenuTextValue::Width,
                config.width as usize,
            );
            spawn_setting_row(
                column,
                font.clone(),
                "Высота арены",
                MenuTextValue::Height,
                config.height as usize,
            );
            spawn_arena_shape_row(column, font.clone(), config.arena_shape);

            spawn_section_title(column, font.clone(), "Среда");
            spawn_setting_row(
                column,
                font.clone(),
                "Препятствия",
                MenuTextValue::Obstacles,
                config.obstacles,
            );
            spawn_setting_row(
                column,
                font.clone(),
                "Кормушки",
                MenuTextValue::FoodGrowers,
                config.food_growers,
            );

            spawn_shapes_toggle(column, font.clone());
            column
                .spawn((
                    Node {
                        width: percent(100),
                        display: Display::None,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        row_gap: px(4),
                        column_gap: px(8),
                        ..default()
                    },
                    MenuSectionBody {
                        section: MenuSection::Shapes,
                    },
                ))
                .with_children(|shapes| {
                    spawn_random_geometry_row(shapes, font.clone(), config.random_cell_geometry);
                    for (index, label) in CELL_SHAPE_LABELS.iter().enumerate() {
                        spawn_shape_weight_slider(
                            shapes,
                            font.clone(),
                            index,
                            label,
                            config.cell_shape_weights[index],
                        );
                    }
                });

            spawn_keybinds_toggle(column, font.clone());
            column
                .spawn((
                    Node {
                        width: percent(100),
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(8),
                        ..default()
                    },
                    MenuSectionBody {
                        section: MenuSection::Keybinds,
                    },
                ))
                .with_children(|keybinds| {
                    spawn_keybind_row(keybinds, font.clone(), "Пауза", "Space");
                    spawn_keybind_row(keybinds, font.clone(), "Паспорт клетки", "Tab");
                    spawn_keybind_row(keybinds, font.clone(), "Реестр видов", "Q");
                    spawn_keybind_row(keybinds, font.clone(), "Биожурнал выбранного вида", "E");
                    spawn_keybind_row(keybinds, font.clone(), "Панель скорости", "C");
                    spawn_keybind_row(keybinds, font.clone(), "Меню паузы", "Esc");
                    spawn_keybind_row(keybinds, font.clone(), "Камера", "W A S D + колесо");
                    spawn_keybind_row(keybinds, font.clone(), "Скорость симуляции", "1-7");
                });

            spawn_advanced_toggle(column, font.clone());
            column
                .spawn((
                    Node {
                        width: percent(100),
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    MenuSectionBody {
                        section: MenuSection::Advanced,
                    },
                ))
                .with_children(|advanced| {
                    spawn_setting_row(
                        advanced,
                        font.clone(),
                        "Seed",
                        MenuTextValue::Seed,
                        config.seed as usize,
                    );
                    spawn_seed_randomize_button(advanced, font.clone());
                    spawn_menu_audio_slider(
                        advanced,
                        font.clone(),
                        "Громкость звуков",
                        MenuAudioKind::Effects,
                        config.sound_volume,
                    );
                    spawn_menu_audio_slider(
                        advanced,
                        font.clone(),
                        "Громкость эмбиента",
                        MenuAudioKind::Ambient,
                        config.ambient_volume,
                    );
                    spawn_segmented_cells_row(advanced, font.clone(), config.segmented_cells);
                    spawn_vsync_row(advanced, font, config.vsync);
                });
        });
}

fn spawn_menu_audio_slider(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    label: &str,
    kind: MenuAudioKind,
    value: f32,
) {
    parent
        .spawn((Node {
            width: percent(100),
            height: px(30),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.78, 0.80)),
                Node {
                    width: px(175),
                    ..default()
                },
            ));
            row.spawn((
                Button,
                Node {
                    width: px(320),
                    height: px(14),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.07, 0.11, 0.12)),
                RelativeCursorPosition::default(),
                MenuAudioSlider(kind),
            ))
            .with_child((
                Node {
                    width: percent(value * 100.0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.36, 0.72, 0.68)),
                MenuAudioFill(kind),
            ));
            row.spawn((
                Text::new(format!("{:.0}%", value * 100.0)),
                TextFont {
                    font,
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.92, 0.92)),
                Node {
                    width: px(48),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                MenuAudioValue(kind),
            ));
        });
}

fn spawn_shapes_toggle(parent: &mut ChildSpawnerCommands, font: Handle<Font>) {
    parent
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: px(36),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.28, 0.48, 0.54)),
            BackgroundColor(NORMAL_BG),
            MenuSectionToggle {
                section: MenuSection::Shapes,
            },
        ))
        .with_child((
            Text::new("Геометрия клеток"),
            TextFont {
                font,
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.72, 0.86, 0.88)),
        ));
}

fn spawn_keybinds_toggle(parent: &mut ChildSpawnerCommands, font: Handle<Font>) {
    parent
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: px(36),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.28, 0.48, 0.54)),
            BackgroundColor(NORMAL_BG),
            MenuSectionToggle {
                section: MenuSection::Keybinds,
            },
        ))
        .with_child((
            Text::new("Кейбинды"),
            TextFont {
                font,
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.72, 0.86, 0.88)),
        ));
}

fn spawn_keybind_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    action: &str,
    key: &str,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(30),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(10),
                padding: UiRect::axes(px(10), px(5)),
                border: UiRect::left(px(3)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.30, 0.58, 0.64)),
            BackgroundColor(Color::srgb(0.032, 0.052, 0.060)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(action),
                TextFont {
                    font: font.clone(),
                    font_size: 13.5,
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.82, 0.84)),
            ));
            row.spawn((
                Text::new(key),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.88, 1.0, 0.96)),
            ));
        });
}

fn spawn_random_geometry_row(parent: &mut ChildSpawnerCommands, font: Handle<Font>, enabled: bool) {
    parent
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: px(30),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(10)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.28, 0.50, 0.56)),
            BackgroundColor(NORMAL_BG),
            RandomGeometryToggle,
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("Полный рандом геометрии"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.88, 0.90)),
            ));
            row.spawn((
                Text::new(if enabled { "ВКЛ" } else { "ВЫКЛ" }),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.96, 0.94)),
                MenuValueLabel {
                    value_type: MenuTextValue::RandomGeometry,
                },
            ));
        });
}

fn spawn_shape_weight_slider(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    index: usize,
    label: &str,
    weight: f32,
) {
    parent
        .spawn((Node {
            width: px(280),
            height: px(24),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.78, 0.80)),
                Node {
                    width: px(82),
                    ..default()
                },
            ));
            row.spawn((
                Button,
                Node {
                    width: px(140),
                    height: px(14),
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.07, 0.11, 0.12)),
                RelativeCursorPosition::default(),
                ShapeWeightSlider(index),
            ))
            .with_child((
                Node {
                    width: percent(weight),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.36, 0.72, 0.68)),
                ShapeWeightFill(index),
            ));
            row.spawn((
                Text::new(format!("{weight:.1}%")),
                TextFont {
                    font,
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.92, 0.92)),
                Node {
                    width: px(42),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                ShapeWeightValue(index),
            ));
        });
}

fn spawn_launch_column(parent: &mut ChildSpawnerCommands, font: Handle<Font>, config: &SimConfig) {
    parent
        .spawn((
            Node {
                width: px(452),
                padding: UiRect::all(px(18)),
                border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            BorderColor::all(Color::srgb(0.33, 0.58, 0.64)),
            BackgroundColor(Color::srgb(0.018, 0.030, 0.038)),
        ))
        .with_children(|column| {
            spawn_section_title(column, font.clone(), "Пресеты");
            spawn_preset_button(
                column,
                font.clone(),
                MenuPreset::MicroTest,
                "Микро-тест",
                "1 клетка, 100 еды, 1 кормушка",
            );
            spawn_preset_button(
                column,
                font.clone(),
                MenuPreset::Simulator,
                "Симулятор · 10K",
                "Основной сценарий, полный баланс",
            );
            spawn_preset_button(
                column,
                font.clone(),
                MenuPreset::Balanced,
                "Баланс",
                "8 000 клеток, просторная среда",
            );
            spawn_preset_button(
                column,
                font.clone(),
                MenuPreset::Performance,
                "Производительность",
                "5 000 клеток, меньше объектов",
            );
            spawn_preset_button(
                column,
                font.clone(),
                MenuPreset::Stress,
                "Стресс-тест",
                "20 000 клеток, много еды",
            );

            spawn_section_title(column, font.clone(), "Сводка запуска");
            spawn_summary_row(
                column,
                font.clone(),
                "Клетки",
                MenuTextValue::Cells,
                config.cells.to_string(),
            );
            spawn_summary_row(
                column,
                font.clone(),
                "Еда",
                MenuTextValue::Food,
                config.food.to_string(),
            );
            spawn_summary_row(
                column,
                font.clone(),
                "Арена",
                MenuTextValue::Arena,
                format!("{:.0} x {:.0}", config.width, config.height),
            );
            spawn_summary_row(
                column,
                font.clone(),
                "Форма",
                MenuTextValue::ArenaShape,
                arena_shape_label(config.arena_shape),
            );
            spawn_summary_row(
                column,
                font.clone(),
                "VSync",
                MenuTextValue::Vsync,
                vsync_label(config.vsync),
            );

            column
                .spawn((
                    Button,
                    Node {
                        width: percent(100),
                        height: px(58),
                        margin: UiRect::top(px(8)),
                        border: UiRect::all(px(1)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.38, 0.88, 0.58)),
                    BackgroundColor(Color::srgb(0.10, 0.34, 0.18)),
                    MenuButtonAction::Start,
                ))
                .with_child((
                    Text::new("Запустить симуляцию"),
                    TextFont {
                        font,
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.90, 1.0, 0.92)),
                ));
        });
}

fn spawn_section_title(parent: &mut ChildSpawnerCommands, font: Handle<Font>, label: &str) {
    parent.spawn((
        Text::new(label),
        TextFont {
            font,
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.62, 0.91, 0.88)),
    ));
}

fn spawn_setting_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    label: &str,
    value_type: MenuTextValue,
    initial_value: usize,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            width: percent(100),
            column_gap: px(14),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.75, 0.80)),
                Node {
                    width: px(250),
                    ..default()
                },
            ));

            row.spawn((
                Button,
                Node {
                    width: px(220),
                    height: px(38),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(NORMAL_BORDER),
                BackgroundColor(NORMAL_BG),
                InputField { value_type },
            ))
            .with_child((
                Text::new(initial_value.to_string()),
                TextFont {
                    font,
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.95, 0.96)),
                MenuValueLabel { value_type },
            ));
        });
}

fn spawn_advanced_toggle(parent: &mut ChildSpawnerCommands, font: Handle<Font>) {
    parent
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: px(36),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.28, 0.48, 0.54)),
            BackgroundColor(NORMAL_BG),
            MenuSectionToggle {
                section: MenuSection::Advanced,
            },
        ))
        .with_child((
            Text::new("Продвинутые настройки"),
            TextFont {
                font,
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.72, 0.86, 0.88)),
        ));
}

fn spawn_seed_randomize_button(parent: &mut ChildSpawnerCommands, font: Handle<Font>) {
    parent
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: px(34),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.28, 0.50, 0.56)),
            BackgroundColor(NORMAL_BG),
            RandomizeSeedButton,
        ))
        .with_child((
            Text::new("Случайный seed"),
            TextFont {
                font,
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.72, 0.88, 0.90)),
        ));
}

fn spawn_vsync_row(parent: &mut ChildSpawnerCommands, font: Handle<Font>, enabled: bool) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            width: percent(100),
            column_gap: px(14),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new("VSync"),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.75, 0.80)),
                Node {
                    width: px(250),
                    ..default()
                },
            ));

            row.spawn((
                Button,
                Node {
                    width: px(220),
                    height: px(38),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(NORMAL_BORDER),
                BackgroundColor(NORMAL_BG),
                VsyncToggle,
            ))
            .with_child((
                Text::new(vsync_label(enabled)),
                TextFont {
                    font,
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.95, 0.96)),
                MenuValueLabel {
                    value_type: MenuTextValue::Vsync,
                },
            ));
        });
}

fn spawn_segmented_cells_row(parent: &mut ChildSpawnerCommands, font: Handle<Font>, enabled: bool) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            width: percent(100),
            column_gap: px(14),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new("Сегментированные клетки"),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.75, 0.80)),
                Node {
                    width: px(250),
                    ..default()
                },
            ));
            row.spawn((
                Button,
                Node {
                    width: px(220),
                    height: px(38),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(NORMAL_BORDER),
                BackgroundColor(NORMAL_BG),
                SegmentedCellsToggle,
            ))
            .with_child((
                Text::new(binary_label(enabled)),
                TextFont {
                    font,
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.95, 0.96)),
                MenuValueLabel {
                    value_type: MenuTextValue::SegmentedCells,
                },
            ));
        });
}

fn spawn_arena_shape_row(parent: &mut ChildSpawnerCommands, font: Handle<Font>, shape: ArenaShape) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            width: percent(100),
            column_gap: px(14),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new("Форма карты"),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.75, 0.80)),
                Node {
                    width: px(250),
                    ..default()
                },
            ));

            row.spawn((
                Button,
                Node {
                    width: px(220),
                    height: px(38),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(NORMAL_BORDER),
                BackgroundColor(NORMAL_BG),
                ArenaShapeToggle,
            ))
            .with_child((
                Text::new(arena_shape_label(shape)),
                TextFont {
                    font,
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.95, 0.96)),
                MenuValueLabel {
                    value_type: MenuTextValue::ArenaShape,
                },
            ));
        });
}

fn spawn_preset_button(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    preset: MenuPreset,
    title: &str,
    subtitle: &str,
) {
    parent
        .spawn((
            Button,
            Node {
                width: percent(100),
                padding: UiRect::all(px(10)),
                border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                ..default()
            },
            BorderColor::all(Color::srgb(0.28, 0.50, 0.56)),
            BackgroundColor(NORMAL_BG),
            PresetButton(preset),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(title),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.96, 0.94)),
            ));
            button.spawn((
                Text::new(subtitle),
                TextFont {
                    font,
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.52, 0.64, 0.68)),
            ));
        });
}

fn spawn_summary_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    label: &str,
    value_type: MenuTextValue,
    value: String,
) {
    parent
        .spawn((Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.58, 0.68, 0.72)),
            ));
            row.spawn((
                Text::new(value),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.94, 0.95)),
                MenuValueLabel { value_type },
            ));
        });
}

fn menu_interaction_system(
    mut commands: Commands,
    mut interaction_query: Query<
        (
            Entity,
            &Interaction,
            Option<&InputField>,
            Option<&MenuButtonAction>,
            Option<&PresetButton>,
            Option<&MenuSectionToggle>,
            Option<&VsyncToggle>,
            Option<&SegmentedCellsToggle>,
            Option<&ArenaShapeToggle>,
            Option<&RandomizeSeedButton>,
            Option<&RandomGeometryToggle>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    focused_query: Query<Entity, With<FocusedInput>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut config: ResMut<SimConfig>,
    mut menu_state: ResMut<MenuUiState>,
) {
    for (
        entity,
        interaction,
        input_field,
        start_btn,
        preset,
        section_toggle,
        vsync_toggle,
        segmented_cells_toggle,
        arena_shape_toggle,
        randomize_seed,
        random_geometry,
    ) in &mut interaction_query
    {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if start_btn.is_some() {
            clamp_launch_config(&mut config);
            next_state.set(AppState::Running);
        } else if let Some(preset) = preset {
            apply_preset(preset.0, &mut config);
        } else if let Some(toggle) = section_toggle {
            match toggle.section {
                MenuSection::Advanced => {
                    menu_state.advanced_open = !menu_state.advanced_open;
                    if menu_state.advanced_open {
                        menu_state.shapes_open = false;
                        menu_state.keybinds_open = false;
                    }
                }
                MenuSection::Shapes => {
                    menu_state.shapes_open = !menu_state.shapes_open;
                    if menu_state.shapes_open {
                        menu_state.advanced_open = false;
                        menu_state.keybinds_open = false;
                    }
                }
                MenuSection::Keybinds => {
                    menu_state.keybinds_open = !menu_state.keybinds_open;
                    if menu_state.keybinds_open {
                        menu_state.advanced_open = false;
                        menu_state.shapes_open = false;
                    }
                }
            }
        } else if vsync_toggle.is_some() {
            config.vsync = !config.vsync;
        } else if segmented_cells_toggle.is_some() {
            config.segmented_cells = !config.segmented_cells;
        } else if arena_shape_toggle.is_some() {
            config.arena_shape = config.arena_shape.next();
        } else if randomize_seed.is_some() {
            config.seed = rand::random::<u64>() % 1_000_000_000_000;
            for focused_entity in &focused_query {
                commands.entity(focused_entity).remove::<FocusedInput>();
            }
        } else if random_geometry.is_some() {
            config.random_cell_geometry = !config.random_cell_geometry;
        } else if input_field.is_some() {
            for focused_entity in &focused_query {
                commands.entity(focused_entity).remove::<FocusedInput>();
            }
            commands.entity(entity).insert(FocusedInput);
        }
    }
}

fn menu_button_style_system(
    time: Res<Time>,
    focused_query: Query<Entity, With<FocusedInput>>,
    mut buttons: Query<
        (
            Entity,
            &Interaction,
            Option<&InputField>,
            Option<&MenuButtonAction>,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        With<Button>,
    >,
) {
    let follow = 1.0 - (-14.0 * time.delta_secs()).exp();
    for (entity, interaction, input_field, start_button, mut border, mut background) in &mut buttons
    {
        let focused = focused_query.get(entity).is_ok();
        *border = if focused {
            BorderColor::all(FOCUSED_BORDER)
        } else if start_button.is_some() {
            BorderColor::all(Color::srgb(0.38, 0.88, 0.58))
        } else if input_field.is_some() {
            BorderColor::all(NORMAL_BORDER)
        } else {
            BorderColor::all(Color::srgb(0.28, 0.50, 0.56))
        };

        let target_background = if focused {
            Color::srgb(0.075, 0.135, 0.150)
        } else if start_button.is_some() {
            match *interaction {
                Interaction::Pressed => Color::srgb(0.15, 0.45, 0.24),
                Interaction::Hovered => Color::srgb(0.12, 0.40, 0.21),
                Interaction::None => Color::srgb(0.10, 0.34, 0.18),
            }
        } else {
            match *interaction {
                Interaction::Pressed => PRESSED_BG,
                Interaction::Hovered => HOVERED_BG,
                Interaction::None => NORMAL_BG,
            }
        };
        background.0 = background.0.mix(&target_background, follow);
    }
}

fn animate_menu_buttons(
    time: Res<Time>,
    mut buttons: Query<(&Interaction, &mut UiTransform), With<Button>>,
) {
    let follow = 1.0 - (-18.0 * time.delta_secs()).exp();
    for (interaction, mut transform) in &mut buttons {
        let (target_scale, target_y) = match *interaction {
            Interaction::Pressed => (0.965, 1.0),
            Interaction::Hovered => (1.018, -1.0),
            Interaction::None => (1.0, 0.0),
        };
        transform.scale = transform.scale.lerp(Vec2::splat(target_scale), follow);
        let current_y = match transform.translation.y {
            Val::Px(value) => value,
            _ => 0.0,
        };
        transform.translation = Val2::px(0.0, current_y + (target_y - current_y) * follow);
    }
}

fn keyboard_input_system(
    mut keyboard_input_events: MessageReader<KeyboardInput>,
    focused_query: Query<(&InputField, &Children), With<FocusedInput>>,
    mut text_query: Query<&mut Text>,
    mut config: ResMut<SimConfig>,
) {
    let Some((input_field, children)) = focused_query.iter().next() else {
        return;
    };

    let mut text_entity = None;
    for child in children.iter() {
        if text_query.get(child).is_ok() {
            text_entity = Some(child);
            break;
        }
    }

    let Some(text_entity) = text_entity else {
        return;
    };

    let mut text = text_query.get_mut(text_entity).unwrap();
    let mut current_str = text.to_string();
    let mut changed = false;

    for event in keyboard_input_events.read() {
        if event.state == ButtonState::Released {
            continue;
        }

        match &event.logical_key {
            Key::Character(character) => {
                if character.chars().all(|c| c.is_ascii_digit()) {
                    let max_len = if input_field.value_type == MenuTextValue::Seed {
                        12
                    } else {
                        8
                    };
                    if current_str.len() < max_len {
                        current_str.push_str(character);
                        changed = true;
                    }
                }
            }
            Key::Backspace => {
                current_str.pop();
                changed = true;
            }
            _ => {}
        }
    }

    if !changed {
        return;
    }

    **text = current_str.clone();
    let val: usize = current_str.parse().unwrap_or(0);
    match input_field.value_type {
        MenuTextValue::Cells => config.cells = val,
        MenuTextValue::Food => config.food = val,
        MenuTextValue::Width => config.width = val as f32,
        MenuTextValue::Height => config.height = val as f32,
        MenuTextValue::Obstacles => config.obstacles = val,
        MenuTextValue::FoodGrowers => config.food_growers = val,
        MenuTextValue::Seed => config.seed = val as u64,
        MenuTextValue::Vsync
        | MenuTextValue::Arena
        | MenuTextValue::ArenaShape
        | MenuTextValue::RandomGeometry
        | MenuTextValue::SegmentedCells => {}
    }
}

fn sync_menu_values_system(
    config: Res<SimConfig>,
    focused_query: Query<&InputField, With<FocusedInput>>,
    mut labels: Query<(&MenuValueLabel, &mut Text)>,
) {
    let focused_type = focused_query.iter().next().map(|field| field.value_type);

    for (label, mut text) in &mut labels {
        if focused_type == Some(label.value_type) {
            continue;
        }

        **text = match label.value_type {
            MenuTextValue::Cells => config.cells.to_string(),
            MenuTextValue::Food => config.food.to_string(),
            MenuTextValue::Width => format!("{:.0}", config.width),
            MenuTextValue::Height => format!("{:.0}", config.height),
            MenuTextValue::Obstacles => config.obstacles.to_string(),
            MenuTextValue::FoodGrowers => config.food_growers.to_string(),
            MenuTextValue::Seed => config.seed.to_string(),
            MenuTextValue::Vsync => vsync_label(config.vsync),
            MenuTextValue::SegmentedCells => binary_label(config.segmented_cells),
            MenuTextValue::Arena => format!("{:.0} x {:.0}", config.width, config.height),
            MenuTextValue::ArenaShape => arena_shape_label(config.arena_shape),
            MenuTextValue::RandomGeometry => {
                if config.random_cell_geometry {
                    "ВКЛ".to_string()
                } else {
                    "ВЫКЛ".to_string()
                }
            }
        };
    }
}

fn menu_section_visibility_system(
    menu_state: Res<MenuUiState>,
    mut sections: Query<(&MenuSectionBody, &mut Node)>,
) {
    for (section, mut node) in &mut sections {
        node.display = match section.section {
            MenuSection::Advanced if menu_state.advanced_open => Display::Flex,
            MenuSection::Shapes if menu_state.shapes_open => Display::Flex,
            MenuSection::Keybinds if menu_state.keybinds_open => Display::Flex,
            MenuSection::Advanced | MenuSection::Shapes | MenuSection::Keybinds => Display::None,
        };
    }
}

fn shape_weight_slider_system(
    mouse: Res<ButtonInput<MouseButton>>,
    sliders: Query<(&Interaction, &RelativeCursorPosition, &ShapeWeightSlider)>,
    mut config: ResMut<SimConfig>,
) {
    if config.random_cell_geometry || !mouse.pressed(MouseButton::Left) {
        return;
    }
    for (interaction, cursor, slider) in &sliders {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(fraction) = relative_cursor_fraction_x(cursor) {
            config.set_cell_shape_weight(slider.0, fraction * 100.0);
        }
    }
}

fn sync_shape_weight_sliders(
    time: Res<Time>,
    config: Res<SimConfig>,
    mut fills: Query<(&ShapeWeightFill, &mut Node, &mut BackgroundColor)>,
    mut values: Query<(&ShapeWeightValue, &mut Text, &mut TextColor)>,
) {
    let follow = 1.0 - (-12.0 * time.delta_secs()).exp();
    for (fill, mut node, mut color) in &mut fills {
        let target = config.cell_shape_weights[fill.0];
        let current = match node.width {
            Val::Percent(value) => value,
            _ => target,
        };
        node.width = percent(current + (target - current) * follow);
        color.0 = if config.random_cell_geometry {
            Color::srgb(0.18, 0.25, 0.25)
        } else {
            Color::srgb(0.36, 0.72, 0.68)
        };
    }
    for (value, mut text, mut color) in &mut values {
        **text = format!("{:.1}%", config.cell_shape_weights[value.0]);
        color.0 = if config.random_cell_geometry {
            Color::srgb(0.42, 0.50, 0.50)
        } else {
            Color::srgb(0.82, 0.92, 0.92)
        };
    }
}

fn menu_settings_scroll_system(
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut scroll_area: Query<
        (&RelativeCursorPosition, &ComputedNode, &mut ScrollPosition),
        With<MenuSettingsScrollArea>,
    >,
) {
    let mut delta = 0.0;
    for event in mouse_wheel.read() {
        let scale = match event.unit {
            MouseScrollUnit::Line => 28.0,
            MouseScrollUnit::Pixel => 1.0,
        };
        delta -= event.y * scale;
    }

    if delta == 0.0 {
        return;
    }

    for (cursor, computed, mut scroll_position) in &mut scroll_area {
        if cursor.normalized.is_none() {
            continue;
        }

        let max_offset = ((computed.content_size().y - computed.size().y)
            * computed.inverse_scale_factor())
        .max(0.0);
        scroll_position.y = (scroll_position.y + delta).clamp(0.0, max_offset);
    }
}

fn menu_audio_volume(config: &SimConfig, kind: MenuAudioKind) -> f32 {
    match kind {
        MenuAudioKind::Effects => config.sound_volume,
        MenuAudioKind::Ambient => config.ambient_volume,
    }
}

fn menu_audio_slider_system(
    mouse: Res<ButtonInput<MouseButton>>,
    sliders: Query<(&Interaction, &RelativeCursorPosition, &MenuAudioSlider)>,
    mut config: ResMut<SimConfig>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    for (interaction, cursor, slider) in &sliders {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(value) = relative_cursor_fraction_x(cursor) else {
            continue;
        };
        match slider.0 {
            MenuAudioKind::Effects => config.sound_volume = value,
            MenuAudioKind::Ambient => config.ambient_volume = value,
        }
    }
}

fn sync_menu_audio_sliders(
    time: Res<Time>,
    config: Res<SimConfig>,
    mut fills: Query<(&MenuAudioFill, &mut Node)>,
    mut values: Query<(&MenuAudioValue, &mut Text)>,
) {
    let follow = 1.0 - (-12.0 * time.delta_secs()).exp();
    for (fill, mut node) in &mut fills {
        let target = menu_audio_volume(&config, fill.0) * 100.0;
        let current = match node.width {
            Val::Percent(value) => value,
            _ => target,
        };
        node.width = percent(current + (target - current) * follow);
    }
    for (value, mut text) in &mut values {
        **text = format!("{:.0}%", menu_audio_volume(&config, value.0) * 100.0);
    }
}

fn apply_preset(preset: MenuPreset, config: &mut SimConfig) {
    match preset {
        MenuPreset::MicroTest => {
            config.cells = 1;
            config.food = 100;
            config.width = 2_200.0;
            config.height = 1_400.0;
            config.arena_shape = ArenaShape::Rectangle;
            config.obstacles = 0;
            config.food_growers = 1;
            config.collision_stiffness = 500.0;
            config.collision_damping = 15.0;
            config.random_cell_geometry = false;
            config.segmented_cells = true;
        }
        MenuPreset::Simulator => {
            config.cells = 10_000;
            config.food = 3_000;
            config.width = 18_000.0;
            config.height = 10_000.0;
            config.arena_shape = ArenaShape::Rectangle;
            config.obstacles = 30;
            config.food_growers = 6;
            config.collision_stiffness = 500.0;
            config.collision_damping = 15.0;
            config.random_cell_geometry = false;
            config.segmented_cells = true;
        }
        MenuPreset::Balanced => {
            config.cells = 8_000;
            config.food = 2_400;
            config.width = 20_000.0;
            config.height = 11_250.0;
            config.obstacles = 24;
            config.food_growers = 5;
        }
        MenuPreset::Performance => {
            config.cells = 5_000;
            config.food = 1_400;
            config.width = 18_000.0;
            config.height = 10_000.0;
            config.obstacles = 18;
            config.food_growers = 3;
        }
        MenuPreset::Stress => {
            config.cells = 20_000;
            config.food = 3_500;
            config.width = 28_000.0;
            config.height = 16_000.0;
            config.obstacles = 36;
            config.food_growers = 6;
        }
    }
}

fn clamp_launch_config(config: &mut SimConfig) {
    config.cells = config.cells.max(1);
    config.food = config.food.max(1);
    config.width = config.width.max(1_000.0);
    config.height = config.height.max(1_000.0);
    config.obstacles = config.obstacles.min(500);
    config.food_growers = config.food_growers.clamp(1, 100);
}

fn vsync_label(enabled: bool) -> String {
    if enabled {
        "Вкл".to_string()
    } else {
        "Выкл".to_string()
    }
}

fn binary_label(enabled: bool) -> String {
    if enabled {
        "Вкл".to_string()
    } else {
        "Выкл".to_string()
    }
}

fn arena_shape_label(shape: ArenaShape) -> String {
    shape.label_ru().to_string()
}

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
