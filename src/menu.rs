use crate::{AppState, simulation::SimConfig};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Menu), setup_menu)
            .add_systems(
                Update,
                menu_interaction_system.run_if(in_state(AppState::Menu)),
            )
            .add_systems(
                Update,
                keyboard_input_system.run_if(in_state(AppState::Menu)),
            )
            .add_systems(
                Update,
                menu_ui_style_system.run_if(in_state(AppState::Menu)),
            )
            .add_systems(OnExit(AppState::Menu), cleanup_menu);
    }
}

#[derive(Component)]
struct MenuEntity;

#[derive(Component)]
enum MenuButtonAction {
    Start,
}

#[derive(Component)]
struct InputField {
    value_type: MenuTextValue,
}

#[derive(Component)]
struct FocusedInput;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MenuTextValue {
    Cells,
    Food,
    Width,
    Height,
    Obstacles,
    FoodGrowers,
}

const NORMAL_BORDER: Color = Color::srgb(0.2, 0.2, 0.25);
const FOCUSED_BORDER: Color = Color::srgb(0.4, 0.8, 1.0);
const NORMAL_BG: Color = Color::srgb(0.1, 0.1, 0.12);
const HOVERED_BG: Color = Color::srgb(0.15, 0.15, 0.18);

fn setup_menu(mut commands: Commands, config: Res<SimConfig>, asset_server: Res<AssetServer>) {
    let font = asset_server.load("arial.ttf");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(25.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.04, 0.06)),
            MenuEntity,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("ORGANOIDS"),
                TextFont {
                    font: font.clone(),
                    font_size: 72.0,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.8, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
            ));

            // Form Rows
            spawn_setting_row(
                parent,
                font.clone(),
                "Количество клеток",
                MenuTextValue::Cells,
                config.cells.to_string(),
            );
            spawn_setting_row(
                parent,
                font.clone(),
                "Количество еды",
                MenuTextValue::Food,
                config.food.to_string(),
            );
            spawn_setting_row(
                parent,
                font.clone(),
                "Ширина арены",
                MenuTextValue::Width,
                config.width.to_string(),
            );
            spawn_setting_row(
                parent,
                font.clone(),
                "Высота арены",
                MenuTextValue::Height,
                config.height.to_string(),
            );
            spawn_setting_row(
                parent,
                font.clone(),
                "Obstacles",
                MenuTextValue::Obstacles,
                config.obstacles.to_string(),
            );
            spawn_setting_row(
                parent,
                font.clone(),
                "Food growers",
                MenuTextValue::FoodGrowers,
                config.food_growers.to_string(),
            );

            // Start Button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(320.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(30.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.2, 0.6, 0.2)),
                    BackgroundColor(Color::srgb(0.1, 0.4, 0.1)),
                    MenuButtonAction::Start,
                ))
                .with_child((
                    Text::new("ЗАПУСТИТЬ СИМУЛЯЦИЮ"),
                    TextFont {
                        font: font.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
        });
}

fn spawn_setting_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    label: &str,
    value_type: MenuTextValue,
    initial_value: String,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            width: Val::Px(450.0),
            ..default()
        },))
        .with_children(|row| {
            // Label
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.75)),
                Node {
                    width: Val::Px(220.0),
                    ..default()
                },
            ));

            // Input Box (Button)
            row.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(45.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(NORMAL_BORDER),
                BackgroundColor(NORMAL_BG),
                InputField { value_type },
            ))
            .with_child((
                Text::new(initial_value),
                TextFont {
                    font: font.clone(),
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
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
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    focused_query: Query<Entity, With<FocusedInput>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut config: ResMut<SimConfig>,
) {
    for (entity, interaction, input_field, start_btn, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if start_btn.is_some() {
                    // Enforce reasonable defaults/minimums on launch
                    config.cells = config.cells.max(10);
                    config.food = config.food.max(10);
                    config.width = config.width.max(1000.0);
                    config.height = config.height.max(1000.0);
                    config.obstacles = config.obstacles.min(500);
                    config.food_growers = config.food_growers.min(100);
                    next_state.set(AppState::Running);
                } else if input_field.is_some() {
                    // Remove focus from others
                    for focused_entity in &focused_query {
                        commands.entity(focused_entity).remove::<FocusedInput>();
                    }
                    // Add focus to this one
                    commands.entity(entity).insert(FocusedInput);
                }
            }
            Interaction::Hovered => {
                if start_btn.is_some() {
                    bg_color.0 = Color::srgb(0.15, 0.5, 0.15);
                } else if input_field.is_some() {
                    bg_color.0 = HOVERED_BG;
                }
            }
            Interaction::None => {
                if start_btn.is_some() {
                    bg_color.0 = Color::srgb(0.1, 0.4, 0.1);
                } else if input_field.is_some() {
                    bg_color.0 = NORMAL_BG;
                }
            }
        }
    }
}

fn menu_ui_style_system(
    focused_query: Query<Entity, With<FocusedInput>>,
    mut input_fields: Query<(Entity, &mut BorderColor), With<InputField>>,
) {
    for (entity, mut border) in &mut input_fields {
        if focused_query.get(entity).is_ok() {
            *border = BorderColor::all(FOCUSED_BORDER);
        } else {
            *border = BorderColor::all(NORMAL_BORDER);
        }
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

    // Find the text child
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
                    if current_str.len() < 8 {
                        current_str.push_str(&character);
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

    if changed {
        **text = current_str.clone();

        let val: usize = current_str.parse().unwrap_or(0);
        match input_field.value_type {
            MenuTextValue::Cells => {
                config.cells = val;
            }
            MenuTextValue::Food => {
                config.food = val;
            }
            MenuTextValue::Width => {
                config.width = val as f32;
            }
            MenuTextValue::Height => {
                config.height = val as f32;
            }
            MenuTextValue::Obstacles => {
                config.obstacles = val;
            }
            MenuTextValue::FoodGrowers => {
                config.food_growers = val;
            }
        }
    }
}

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
