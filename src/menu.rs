use crate::{
    AppState,
    simulation::{ArenaShape, SimConfig},
};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

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
}

#[derive(Component)]
struct MenuEntity;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MenuButtonAction {
    Start,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct PresetButton(MenuPreset);

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuPreset {
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
    Arena,
    ArenaShape,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MenuSection {
    Advanced,
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
struct ArenaShapeToggle;

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
                padding: UiRect::all(px(18)),
                border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            BorderColor::all(Color::srgb(0.33, 0.58, 0.64)),
            BackgroundColor(Color::srgb(0.020, 0.032, 0.040)),
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

            spawn_advanced_toggle(column, font.clone());
            column
                .spawn((
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    Visibility::Hidden,
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
                    spawn_vsync_row(advanced, font, config.vsync);
                });
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
                MenuPreset::Balanced,
                "Баланс",
                "10 000 клеток, 2 000 еды",
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
            Option<&ArenaShapeToggle>,
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
        arena_shape_toggle,
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
            if toggle.section == MenuSection::Advanced {
                menu_state.advanced_open = !menu_state.advanced_open;
            }
        } else if vsync_toggle.is_some() {
            config.vsync = !config.vsync;
        } else if arena_shape_toggle.is_some() {
            config.arena_shape = config.arena_shape.next();
        } else if input_field.is_some() {
            for focused_entity in &focused_query {
                commands.entity(focused_entity).remove::<FocusedInput>();
            }
            commands.entity(entity).insert(FocusedInput);
        }
    }
}

fn menu_button_style_system(
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

        background.0 = if focused {
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
        MenuTextValue::Vsync | MenuTextValue::Arena | MenuTextValue::ArenaShape => {}
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
            MenuTextValue::Arena => format!("{:.0} x {:.0}", config.width, config.height),
            MenuTextValue::ArenaShape => arena_shape_label(config.arena_shape),
        };
    }
}

fn menu_section_visibility_system(
    menu_state: Res<MenuUiState>,
    mut sections: Query<(&MenuSectionBody, &mut Visibility)>,
) {
    for (section, mut visibility) in &mut sections {
        *visibility = match section.section {
            MenuSection::Advanced if menu_state.advanced_open => Visibility::Visible,
            MenuSection::Advanced => Visibility::Hidden,
        };
    }
}

fn apply_preset(preset: MenuPreset, config: &mut SimConfig) {
    match preset {
        MenuPreset::Balanced => {
            config.cells = 10_000;
            config.food = 2_000;
            config.width = 24_000.0;
            config.height = 13_500.0;
            config.obstacles = 26;
            config.food_growers = 4;
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
    config.cells = config.cells.max(10);
    config.food = config.food.max(10);
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

fn arena_shape_label(shape: ArenaShape) -> String {
    shape.label_ru().to_string()
}

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
