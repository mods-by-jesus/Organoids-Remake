mod menu;
mod rendering;
mod simulation;

use bevy::app::AppExit;
use bevy::asset::RenderAssetUsages;
use bevy::audio::{AudioSink, AudioSinkPlayback, PlaybackMode, Volume};
use bevy::camera::ScalingMode;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat};
use bevy::render::view::NoIndirectDrawing;
use bevy::ui::RelativeCursorPosition;
use bevy::window::{PresentMode, PrimaryWindow, WindowMode, WindowResolution};
use rand::Rng;
use rendering::{
    InstancedDiscPlugin, LiquidMediumMaterial, SimulationRenderEntity, clear_cell_wake_trails,
    spawn_simulation_layers, sync_instance_data,
};
use simulation::{
    CELL_AGGRESSIVENESS_DISPLAY_MAX, CELL_DIVISION_THRESHOLD_DISPLAY_MAX, CELL_LYSIS_DISPLAY_MAX,
    CELL_MUTATION_DISPLAY_MAX, CELL_PERCEPTION_DISPLAY_MAX, CELL_PERSISTENCE_DISPLAY_MAX,
    CELL_SIZE_GENE_MAX, CELL_SIZE_GENE_MIN, CELL_SPEED_DISPLAY_MAX, CELL_TURN_DISPLAY_MAX,
    CELL_VIABILITY_MAX, FrameStats, PERCEPTION_GENE_MAX, PERCEPTION_GENE_MIN, SPEED_GENE_MAX,
    SPEED_GENE_MIN, SimConfig, TURN_GENE_MAX, TURN_GENE_MIN, WorldState, cell_display_color,
    grass_energy_multiplier, lysis_combat_profile, meat_energy_multiplier,
};
use std::{collections::HashMap, fs, path::PathBuf, time::Instant};

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Menu,
    Running,
}

#[derive(Component)]
struct StatsText;

#[derive(Component)]
struct EnergyBalanceText;

#[derive(Component)]
struct FpsAverageText;

#[derive(Component)]
struct FpsAverageDeltaText;

#[derive(Resource, Default)]
struct FpsAverageStats {
    elapsed: f32,
    accumulated_frame_time: f32,
    frame_count: u32,
    current_average: f32,
    previous_average: Option<f32>,
    delta: f32,
}

impl FpsAverageStats {
    fn observe(&mut self, dt: f32, fallback_fps: f64) {
        let dt = dt.clamp(0.0, 0.25);
        if dt <= f32::EPSILON {
            if self.current_average <= 0.0 {
                self.current_average = fallback_fps as f32;
            }
            return;
        }

        self.elapsed += dt;
        self.accumulated_frame_time += dt;
        self.frame_count = self.frame_count.saturating_add(1);

        let live_average = self.frame_count as f32 / self.accumulated_frame_time.max(0.001);
        if self.current_average <= 0.0 {
            self.current_average = fallback_fps as f32;
        } else {
            self.current_average = live_average;
        }

        if self.elapsed >= 2.5 {
            let previous = self.previous_average.unwrap_or(live_average);
            self.current_average = live_average;
            self.delta = live_average - previous;
            self.previous_average = Some(live_average);
            self.elapsed = 0.0;
            self.accumulated_frame_time = 0.0;
            self.frame_count = 0;
        }
    }
}

#[derive(Resource, Default)]
struct EcoLogState {
    wall_elapsed: f32,
    sim_elapsed: f32,
    initialized: bool,
    last_cells: usize,
    last_food: usize,
    last_viability: f32,
    last_food_energy: f32,
}

#[derive(Clone, Copy)]
enum PopulationCounterKind {
    Cells,
    Food,
}

#[derive(Component)]
struct PopulationCountText(PopulationCounterKind);

#[derive(Component)]
struct PopulationDeltaText(PopulationCounterKind);

#[derive(Component)]
struct StatsBodyText;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct RunningUiEntity;

#[derive(Resource, Default)]
struct SelectedCell {
    cell_id: Option<u64>,
    last_click_cell_id: Option<u64>,
    last_click_time: f64,
}

#[derive(Resource, Default)]
struct SpeciesLedgerUiState {
    open: bool,
    journal_open: bool,
    selected_species: Option<u32>,
    last_click_species: Option<u32>,
    last_click_time: f64,
    rendered_revision: u64,
    rendered_range_start: usize,
    rendered_range_end: usize,
    scroll_target_species: Option<u32>,
}

#[derive(Resource, Default)]
struct SpeciesLedgerDragState {
    scrollbar_dragging: bool,
    scroll_initialized: bool,
    scroll_target_y: f32,
}

#[derive(Resource, Default)]
struct SpeciesMiniatureImageCache {
    handles: HashMap<u32, Handle<Image>>,
    signatures: HashMap<u32, u64>,
    journal_handles: HashMap<u32, Handle<Image>>,
    journal_signatures: HashMap<u32, u64>,
}

#[derive(Resource, Default)]
struct SpeciesCameraFocus {
    active: bool,
    target: Vec2,
    target_scale: f32,
}

#[derive(Resource, Default)]
struct SpeciesAreaHighlightState {
    species: Option<u32>,
    rendered_revision: u64,
}

#[derive(Component)]
struct SpeciesAreaHighlightEntity;

#[derive(Resource, Default)]
struct SpeciesNameBook {
    prefixes: Vec<String>,
    suffixes: Vec<String>,
    epithets: Vec<String>,
}

#[derive(Clone, Default)]
struct SpeciesSnapshot {
    species: u32,
    alive: usize,
    alive_delta: isize,
    average_position: Vec2,
    area_min: Vec2,
    area_max: Vec2,
    representative_cell_id: Option<u64>,
    average_viability: f32,
    average_speed: f32,
    average_turn: f32,
    average_aggressiveness: f32,
    average_lysis: f32,
    average_size: f32,
    average_perception: f32,
    average_persistence: f32,
    average_mutation: f32,
    segmented_ratio: f32,
    average_radii: [f32; 8],
    average_angle_offsets: [f32; 8],
    display_radii: [f32; 8],
    display_angle_offsets: [f32; 8],
    display_section_count: u8,
    display_section_radii: [[f32; 8]; 4],
    display_section_angle_offsets: [[f32; 8]; 4],
    display_section_scale: [f32; 4],
    display_section_spacing: f32,
    display_section_centers: [Vec2; 4],
    display_section_headings: [f32; 4],
    display_section_angles: [f32; 3],
    display_section_parents: [u8; 3],
    display_edge_controls: [Vec2; 3],
}

#[derive(Resource, Default)]
struct SpeciesLedgerStats {
    snapshots: Vec<SpeciesSnapshot>,
    accumulator: f32,
    sort_accumulator: f32,
    revision: u64,
}

#[derive(Component)]
struct SpeciesLedgerButton;

#[derive(Component)]
struct SpeciesLedgerPanel;

#[derive(Component)]
struct SpeciesLedgerScrollArea;

#[derive(Component)]
struct SpeciesLedgerGrid;

#[derive(Component)]
struct SpeciesLedgerScrollbarTrack;

#[derive(Component)]
struct SpeciesLedgerScrollbarThumb;

#[derive(Component)]
struct SpeciesLedgerRow {
    species: u32,
}

#[derive(Component)]
struct SpeciesLedgerNameText;

#[derive(Component)]
struct SpeciesLedgerCountText {
    species: u32,
}

#[allow(dead_code)]
#[derive(Component)]
struct SpeciesLedgerDetailsPanel;

#[allow(dead_code)]
#[derive(Component)]
struct SpeciesLedgerDetailsText;

#[derive(Component)]
struct SpeciesLedgerStatusIcon;

#[derive(Component)]
struct SpeciesLedgerDietIcon;

#[derive(Component)]
struct SpeciesLedgerRelationIcon {
    species: u32,
}

#[derive(Component)]
struct SpeciesLedgerMiniature {
    species: u32,
}

#[derive(Component)]
struct SpeciesLedgerMiniImage {
    species: u32,
}

#[derive(Component)]
struct SpeciesJournalPanel;

#[derive(Component)]
struct SpeciesJournalPortraitImage;

#[derive(Component)]
struct SpeciesJournalTitleText;

#[derive(Component)]
struct SpeciesJournalSubtitleText;

#[derive(Component)]
struct SpeciesJournalTrendText;

#[derive(Component)]
struct SpeciesJournalBodyText;

#[derive(Component)]
struct SpeciesJournalAreaRow;

#[derive(Component)]
struct SpeciesJournalAreaText;

#[derive(Component)]
struct SpeciesJournalDietIcon;

#[derive(Component, Clone, Copy)]
enum SpeciesJournalMetric {
    Population,
    Viability,
    Size,
    Speed,
    Turn,
    Perception,
    Persistence,
    Aggressiveness,
    Lysis,
    Mutation,
}

#[derive(Component)]
struct SpeciesJournalMetricFill {
    metric: SpeciesJournalMetric,
}

#[derive(Component)]
struct SpeciesJournalMetricValue {
    metric: SpeciesJournalMetric,
}

#[derive(Clone, Copy)]
enum SpeciesJournalTooltipKind {
    Portrait,
    Diet,
    Metric(SpeciesJournalMetric),
}

#[derive(Component)]
struct SpeciesJournalTooltipTarget {
    kind: SpeciesJournalTooltipKind,
}

#[derive(Resource)]
struct ChronicleUiState {
    open: bool,
    event_filters: u8,
    graph_mode: ChronicleGraphMode,
}

impl Default for ChronicleUiState {
    fn default() -> Self {
        Self {
            open: false,
            event_filters: CHRONICLE_ALL_EVENT_FILTERS,
            graph_mode: ChronicleGraphMode::Overall,
        }
    }
}

impl ChronicleUiState {
    fn event_enabled(&self, kind: ChronicleEventKind) -> bool {
        self.event_filters & chronicle_filter_bit(kind) != 0
    }

    fn toggle_event_filter(&mut self, kind: ChronicleEventKind) {
        self.event_filters ^= chronicle_filter_bit(kind);
    }
}

#[derive(Resource)]
struct SimulationChronicle {
    elapsed: f32,
    sample_accumulator: f32,
    revision: u64,
    snapshots: Vec<ChronicleSnapshot>,
    events: Vec<ChronicleEvent>,
    species_records: HashMap<u32, ChronicleSpeciesRecord>,
    dominant_species: Option<u32>,
    energy_state: ChronicleEnergyState,
    first_lysis_reported: bool,
    first_segmented_reported: bool,
    last_population_check_time: f32,
    last_population_check_cells: usize,
}

impl Default for SimulationChronicle {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            sample_accumulator: 0.0,
            revision: 0,
            snapshots: Vec::with_capacity(CHRONICLE_MAX_SNAPSHOTS),
            events: Vec::with_capacity(CHRONICLE_MAX_EVENTS),
            species_records: HashMap::new(),
            dominant_species: None,
            energy_state: ChronicleEnergyState::Balanced,
            first_lysis_reported: false,
            first_segmented_reported: false,
            last_population_check_time: 0.0,
            last_population_check_cells: 0,
        }
    }
}

#[derive(Resource, Default)]
struct ChronicleGraphCache {
    revision: u64,
    mode: ChronicleGraphMode,
    handle: Option<Handle<Image>>,
}

#[derive(Resource, Default)]
struct ChronicleEventScrollState {
    scrollbar_dragging: bool,
    initialized: bool,
    target_y: f32,
}

#[derive(Clone, Copy)]
struct ChronicleSnapshot {
    time: f32,
    cells: usize,
    food: usize,
    wild_food: usize,
    feeder_food: usize,
    meat: usize,
    avg_viability: f32,
    energy_in: f32,
    energy_out: f32,
    energy_net: f32,
    metabolism: f32,
    mitosis: f32,
    lysis: f32,
    fps: f32,
    sim_ms: f32,
    render_ms: f32,
    species: usize,
    segmented: usize,
    lysis_capable: usize,
}

struct ChronicleEvent {
    time: f32,
    kind: ChronicleEventKind,
    title: String,
    body: String,
    species: Option<u32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChronicleEventKind {
    World,
    Species,
    Extinction,
    Population,
    Energy,
    Trait,
}

#[derive(Default)]
struct ChronicleSpeciesRecord {
    alive: usize,
    peak_alive: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChronicleEnergyState {
    Deficit,
    Balanced,
    Surplus,
}

impl Default for ChronicleEnergyState {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Component)]
struct ChronicleButton;

#[derive(Component)]
struct ChroniclePanel;

#[derive(Component)]
struct ChronicleOverviewText;

#[derive(Clone, Copy)]
enum ChronicleSummaryMetric {
    Time,
    Cells,
    Species,
    Food,
    Viability,
    Energy,
    Costs,
    Traits,
    Performance,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChronicleGraphMode {
    Overall,
    Cells,
    Food,
    Viability,
    Energy,
}

impl Default for ChronicleGraphMode {
    fn default() -> Self {
        Self::Overall
    }
}

#[derive(Component)]
struct ChronicleSummaryValue {
    metric: ChronicleSummaryMetric,
}

#[derive(Component)]
struct ChronicleEventText;

#[derive(Component)]
struct ChronicleGraphImage;

#[derive(Component)]
struct ChronicleFilterButton {
    kind: ChronicleEventKind,
}

#[derive(Component)]
struct ChronicleGraphButton {
    mode: ChronicleGraphMode,
}

#[derive(Component)]
struct ChronicleEventScrollArea;

#[derive(Component)]
struct ChronicleEventScrollbarTrack;

#[derive(Component)]
struct ChronicleEventScrollbarThumb;

#[derive(Clone, Copy)]
enum ChronicleLegendLine {
    Cells,
    Food,
    Viability,
    EnergyPositive,
    EnergyNegative,
}

#[derive(Clone, Copy)]
enum ChronicleTooltipKind {
    Summary(ChronicleSummaryMetric),
    Filter(ChronicleEventKind),
    Graph,
    GraphMode(ChronicleGraphMode),
    Legend(ChronicleLegendLine),
}

#[derive(Component)]
struct ChronicleTooltipTarget {
    kind: ChronicleTooltipKind,
}

#[derive(Resource)]
struct GameUiState {
    paused: bool,
    passport_open: bool,
    pause_menu_open: bool,
    speed_panel_open: bool,
    speed_multiplier: f32,
}

impl Default for GameUiState {
    fn default() -> Self {
        Self {
            paused: false,
            passport_open: false,
            pause_menu_open: false,
            speed_panel_open: true,
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
    Perception,
    Persistence,
    Aggressiveness,
    Diet,
    Lysis,
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
struct GeneIconNode {
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
struct GeneTooltipTarget {
    kind: GeneStatId,
}

#[derive(Component, Default)]
struct GeneTooltip {
    reveal: f32,
}

#[derive(Component)]
struct GeneTooltipTitle;

#[derive(Component)]
struct GeneTooltipValue;

#[derive(Component)]
struct GeneTooltipBody;

#[derive(Component)]
struct PassportPanel;

#[derive(Component)]
struct PassportCellTitle;

#[derive(Component)]
struct PassportToggleButton;

#[derive(Component, Default)]
struct PanelReveal {
    progress: f32,
    hidden_offset: f32,
}

impl PanelReveal {
    fn horizontal(hidden_offset: f32) -> Self {
        Self {
            progress: 0.0,
            hidden_offset,
        }
    }
}

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
    last_effect: Option<usize>,
}

#[derive(Component)]
struct RunningAudioEntity;

#[derive(Component)]
struct CellEffectAudio;

const MAX_CELL_SOUNDS_PER_FRAME: usize = 3;
const MAX_ACTIVE_CELL_SOUNDS: usize = 8;

#[derive(Component)]
struct AmbientAudio;

#[derive(Clone, Copy)]
enum AudioVolumeKind {
    Effects,
    Ambient,
}

#[derive(Component)]
struct PauseAudioSlider(AudioVolumeKind);

#[derive(Component)]
struct PauseAudioFill(AudioVolumeKind);

#[derive(Component)]
struct PauseAudioValue(AudioVolumeKind);

const START_VIEW_HEIGHT: f32 = 1_470.0;
const CAMERA_MOVE_SPEED: f32 = 1_100.0;
const ZOOM_FACTOR: f32 = 1.18;
const MIN_ZOOM_SCALE: f32 = 0.08;
const MAX_ZOOM_SCALE: f32 = 12.0;
const SPECIES_LEDGER_SORT_INTERVAL: f32 = 5.0;
const SPECIES_LEDGER_PANEL_LEFT: f32 = 16.0;
const SPECIES_LEDGER_PANEL_WIDTH: f32 = 540.0;
const SPECIES_LEDGER_PANEL_BOTTOM: f32 = 74.0;
const SPECIES_LEDGER_PANEL_HEIGHT_PERCENT: f32 = 64.0;
const SPECIES_LEDGER_PANEL_REVEAL_OFFSET: f32 = 590.0;
const SPECIES_JOURNAL_PANEL_GAP: f32 = 12.0;
const SPECIES_JOURNAL_PANEL_WIDTH: f32 = 620.0;
const SPECIES_JOURNAL_PANEL_LEFT: f32 =
    SPECIES_LEDGER_PANEL_LEFT + SPECIES_LEDGER_PANEL_WIDTH + SPECIES_JOURNAL_PANEL_GAP;
const SPECIES_JOURNAL_PANEL_REVEAL_OFFSET: f32 =
    SPECIES_JOURNAL_PANEL_LEFT + SPECIES_JOURNAL_PANEL_WIDTH + 24.0;
const SPECIES_LEDGER_COLUMNS: usize = 3;
const SPECIES_LEDGER_CARD_WIDTH: f32 = 154.0;
const SPECIES_LEDGER_CARD_HEIGHT: f32 = 148.0;
const SPECIES_LEDGER_COLUMN_GAP: f32 = 8.0;
const SPECIES_LEDGER_ROW_GAP: f32 = 8.0;
const SPECIES_LEDGER_ROW_STRIDE: f32 = SPECIES_LEDGER_CARD_HEIGHT + SPECIES_LEDGER_ROW_GAP;
const SPECIES_LEDGER_VIRTUAL_BUFFER_ROWS: usize = 2;
const SPECIES_LEDGER_WHEEL_LINE_SCROLL: f32 = 18.0;
const SPECIES_LEDGER_WHEEL_PIXEL_SCROLL: f32 = 0.55;
const SPECIES_LEDGER_SCROLL_FOLLOW: f32 = 15.0;
const SPECIES_LEDGER_SCROLLBAR_FOLLOW: f32 = 11.0;
const SPECIES_LEDGER_AUTO_SCROLL_FOLLOW: f32 = 6.5;
const CHRONICLE_SAMPLE_INTERVAL: f32 = 1.0;
const CHRONICLE_MAX_SNAPSHOTS: usize = 900;
const CHRONICLE_MAX_EVENTS: usize = 240;
const CHRONICLE_BUTTON_LEFT: f32 = 72.0;
const CHRONICLE_PANEL_LEFT: f32 = 72.0;
const CHRONICLE_PANEL_BOTTOM: f32 = 74.0;
const CHRONICLE_PANEL_WIDTH: f32 = 860.0;
const CHRONICLE_PANEL_HEIGHT_PERCENT: f32 = 58.0;
const CHRONICLE_PANEL_REVEAL_OFFSET: f32 = 930.0;
const CHRONICLE_GRAPH_WIDTH: u32 = 430;
const CHRONICLE_GRAPH_HEIGHT: u32 = 220;
const CHRONICLE_EVENT_WHEEL_LINE_SCROLL: f32 = 42.0;
const CHRONICLE_EVENT_WHEEL_PIXEL_SCROLL: f32 = 0.75;
const CHRONICLE_EVENT_SCROLL_FOLLOW: f32 = 13.0;
const CHRONICLE_EVENT_SCROLLBAR_FOLLOW: f32 = 11.0;
const CHRONICLE_FILTER_WORLD: u8 = 1 << 0;
const CHRONICLE_FILTER_SPECIES: u8 = 1 << 1;
const CHRONICLE_FILTER_EXTINCTION: u8 = 1 << 2;
const CHRONICLE_FILTER_POPULATION: u8 = 1 << 3;
const CHRONICLE_FILTER_ENERGY: u8 = 1 << 4;
const CHRONICLE_FILTER_TRAIT: u8 = 1 << 5;
const CHRONICLE_ALL_EVENT_FILTERS: u8 = CHRONICLE_FILTER_WORLD
    | CHRONICLE_FILTER_SPECIES
    | CHRONICLE_FILTER_EXTINCTION
    | CHRONICLE_FILTER_POPULATION
    | CHRONICLE_FILTER_ENERGY
    | CHRONICLE_FILTER_TRAIT;
const SPECIES_EPITHET_SLOTS: u32 = 10_000;
const SPECIES_CLASS_STRIDE: u32 = 10_000_000;
const UI_FONT: &str = "fonts/FiraSansExtraCondensed-Regular.ttf";

fn relative_cursor_fraction_x(cursor: &RelativeCursorPosition) -> Option<f32> {
    cursor
        .normalized
        .map(|position| (position.x + 0.5).clamp(0.0, 1.0))
}

fn tooltip_position_near_cursor(
    window: &Window,
    cursor: Vec2,
    width: f32,
    height: f32,
    gap: f32,
) -> Vec2 {
    let x = tooltip_axis_position(cursor.x, window.width(), width, gap);
    let y = tooltip_axis_position(cursor.y, window.height(), height, gap);

    Vec2::new(x, y)
}

fn tooltip_axis_position(cursor_axis: f32, viewport_size: f32, tooltip_size: f32, gap: f32) -> f32 {
    (cursor_axis + gap).clamp(8.0, (viewport_size - tooltip_size - 8.0).max(8.0))
}

fn cursor_over_species_ledger(window: &Window, cursor: Vec2) -> bool {
    let left = SPECIES_LEDGER_PANEL_LEFT;
    let right = left + SPECIES_LEDGER_PANEL_WIDTH;
    let bottom = window.height() - SPECIES_LEDGER_PANEL_BOTTOM;
    let top = bottom - window.height() * (SPECIES_LEDGER_PANEL_HEIGHT_PERCENT / 100.0);
    cursor.x >= left && cursor.x <= right && cursor.y >= top && cursor.y <= bottom
}

fn cursor_over_species_journal(window: &Window, cursor: Vec2) -> bool {
    let left = SPECIES_JOURNAL_PANEL_LEFT;
    let right = left + SPECIES_JOURNAL_PANEL_WIDTH;
    let bottom = window.height() - SPECIES_LEDGER_PANEL_BOTTOM;
    let top = bottom - window.height() * (SPECIES_LEDGER_PANEL_HEIGHT_PERCENT / 100.0);
    cursor.x >= left && cursor.x <= right && cursor.y >= top && cursor.y <= bottom
}

fn species_ledger_scrollbar_fraction(window: &Window, cursor: Vec2) -> Option<f32> {
    let panel_left = SPECIES_LEDGER_PANEL_LEFT;
    let panel_right = panel_left + SPECIES_LEDGER_PANEL_WIDTH;
    let panel_bottom = window.height() - SPECIES_LEDGER_PANEL_BOTTOM;
    let panel_top = panel_bottom - window.height() * (SPECIES_LEDGER_PANEL_HEIGHT_PERCENT / 100.0);
    let track_top = panel_top + 52.0;
    let track_bottom = panel_bottom - 20.0;
    if cursor.x < panel_right - 34.0
        || cursor.x > panel_right - 2.0
        || cursor.y < track_top
        || cursor.y > track_bottom
    {
        return None;
    }
    Some(((cursor.y - track_top) / (track_bottom - track_top).max(1.0)).clamp(0.0, 1.0))
}

fn chronicle_event_rect(window: &Window) -> (f32, f32, f32, f32) {
    let panel_left = CHRONICLE_PANEL_LEFT;
    let panel_bottom = window.height() - CHRONICLE_PANEL_BOTTOM;
    let panel_top = panel_bottom - window.height() * (CHRONICLE_PANEL_HEIGHT_PERCENT / 100.0);
    let inner_left = panel_left + 14.0;
    let inner_width = CHRONICLE_PANEL_WIDTH - 28.0;
    let body_width = inner_width - 12.0;
    let event_left = inner_left;
    let event_right = event_left + body_width * 0.46;
    let event_top = panel_top + 14.0 + 34.0 + 10.0 + 76.0 + 10.0;
    let event_bottom = panel_bottom - 14.0;
    (event_left, event_right, event_top, event_bottom)
}

fn cursor_over_chronicle_events(window: &Window, cursor: Vec2) -> bool {
    let (left, right, top, bottom) = chronicle_event_rect(window);
    cursor.x >= left && cursor.x <= right && cursor.y >= top && cursor.y <= bottom
}

fn cursor_over_chronicle_panel(window: &Window, cursor: Vec2) -> bool {
    let left = CHRONICLE_PANEL_LEFT;
    let right = left + CHRONICLE_PANEL_WIDTH;
    let bottom = window.height() - CHRONICLE_PANEL_BOTTOM;
    let top = bottom - window.height() * (CHRONICLE_PANEL_HEIGHT_PERCENT / 100.0);
    cursor.x >= left && cursor.x <= right && cursor.y >= top && cursor.y <= bottom
}

fn chronicle_event_scrollbar_fraction(window: &Window, cursor: Vec2) -> Option<f32> {
    let (_, right, top, bottom) = chronicle_event_rect(window);
    let track_top = top + 44.0;
    let track_bottom = bottom - 10.0;
    if cursor.x < right - 28.0
        || cursor.x > right - 2.0
        || cursor.y < track_top
        || cursor.y > track_bottom
    {
        return None;
    }
    Some(((cursor.y - track_top) / (track_bottom - track_top).max(1.0)).clamp(0.0, 1.0))
}

fn species_ledger_row_count(total_species: usize) -> usize {
    total_species.div_ceil(SPECIES_LEDGER_COLUMNS).max(1)
}

fn species_ledger_content_height(total_species: usize) -> f32 {
    species_ledger_row_count(total_species) as f32 * SPECIES_LEDGER_ROW_STRIDE
        + SPECIES_LEDGER_ROW_GAP * 2.0
}

fn species_ledger_visible_index_range(
    total_species: usize,
    scroll_y: f32,
    view_height: f32,
) -> (usize, usize) {
    if total_species == 0 {
        return (0, 0);
    }
    let row_count = species_ledger_row_count(total_species);
    let first_visible_row = (scroll_y.max(0.0) / SPECIES_LEDGER_ROW_STRIDE).floor() as usize;
    let start_row = first_visible_row.saturating_sub(SPECIES_LEDGER_VIRTUAL_BUFFER_ROWS);
    let visible_rows = (view_height.max(1.0) / SPECIES_LEDGER_ROW_STRIDE).ceil() as usize
        + SPECIES_LEDGER_VIRTUAL_BUFFER_ROWS * 2
        + 1;
    let end_row = (start_row + visible_rows).min(row_count);
    let start = start_row * SPECIES_LEDGER_COLUMNS;
    let end = (end_row * SPECIES_LEDGER_COLUMNS).min(total_species);
    (start, end)
}

fn species_ledger_scroll_target_y(index: usize, view_height: f32) -> f32 {
    let row = index / SPECIES_LEDGER_COLUMNS;
    let row_top = row as f32 * SPECIES_LEDGER_ROW_STRIDE;
    let row_center = row_top + SPECIES_LEDGER_CARD_HEIGHT * 0.5;
    (row_center - view_height.max(1.0) * 0.46).max(0.0)
}

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
        .init_resource::<SpeciesLedgerUiState>()
        .init_resource::<SpeciesLedgerDragState>()
        .init_resource::<SpeciesMiniatureImageCache>()
        .init_resource::<SpeciesCameraFocus>()
        .init_resource::<SpeciesAreaHighlightState>()
        .init_resource::<SpeciesLedgerStats>()
        .init_resource::<ChronicleUiState>()
        .init_resource::<SimulationChronicle>()
        .init_resource::<ChronicleGraphCache>()
        .init_resource::<ChronicleEventScrollState>()
        .init_resource::<GameUiState>()
        .init_resource::<FrameStats>()
        .init_resource::<FpsAverageStats>()
        .init_resource::<EcoLogState>()
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
                        mode: WindowMode::Windowed,
                        decorations: true,
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
        .add_systems(
            Startup,
            (
                setup_camera,
                load_cell_audio,
                load_species_name_book,
                maximize_primary_window,
            ),
        )
        .add_systems(
            OnEnter(AppState::Running),
            (
                initialize_world_state,
                load_species_name_book,
                clear_cell_wake_trails,
                spawn_simulation_layers,
                setup_game_stats_ui,
                setup_biolab_ui_v2,
                setup_species_ledger_ui,
                setup_chronicle_ui,
                start_running_audio,
                update_window_title,
            )
                .chain(),
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
                update_cell_effect_volume,
                pause_audio_slider_system,
                sync_pause_audio_sliders,
                update_ambient_volume,
                sync_instance_data,
                update_stats_overlay,
                update_selection_ui,
                update_pause_ui,
                passport_toggle_action_system,
                passport_toggle_button_style_system,
                animate_game_buttons,
                pause_menu_button_system,
                pause_menu_button_style_system,
                speed_button_system,
                update_speed_button_styles,
            )
                .chain()
                .run_if(in_state(AppState::Running)),
        )
        .add_systems(
            Update,
            eco_log_system
                .after(update_stats_overlay)
                .run_if(in_state(AppState::Running)),
        )
        .add_systems(
            Update,
            update_simulation_chronicle
                .after(step_simulation)
                .run_if(in_state(AppState::Running)),
        )
        .add_systems(
            Update,
            (
                chronicle_button_system,
                chronicle_filter_button_system,
                chronicle_graph_button_system,
                update_chronicle_filter_button_styles,
                chronicle_event_scroll_system,
                update_chronicle_ui,
            )
                .chain()
                .after(update_simulation_chronicle)
                .run_if(in_state(AppState::Running)),
        )
        .add_systems(
            Update,
            update_speed_panel_visibility.run_if(in_state(AppState::Running)),
        )
        .add_systems(
            Update,
            (
                species_ledger_button_system,
                update_species_ledger_stats,
                species_ledger_scroll_system,
                update_species_ledger_ui,
                update_species_ledger_row_visuals,
                update_species_ledger_miniature_visuals,
                update_species_journal_ui,
                species_journal_area_row_system,
                update_species_area_highlight_system,
                species_ledger_row_system,
                apply_species_camera_focus,
            )
                .chain()
                .run_if(in_state(AppState::Running)),
        )
        .add_systems(
            Update,
            update_diet_icon_system
                .after(update_selection_ui)
                .run_if(in_state(AppState::Running)),
        )
        .add_systems(
            Update,
            update_selected_species_titles
                .after(update_selection_ui)
                .run_if(in_state(AppState::Running)),
        )
        .add_systems(
            Update,
            update_gene_tooltip
                .after(update_selection_ui)
                .run_if(in_state(AppState::Running)),
        )
        .run();
}

fn load_taxonomy_words(file_name: &str) -> Vec<String> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "assets", "taxonomy", file_name]
        .iter()
        .collect();
    fs::read_to_string(path)
        .map(|content| {
            content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn fallback_taxonomy_words(words: Vec<String>, fallback: &[&str]) -> Vec<String> {
    if words.is_empty() {
        fallback.iter().map(|word| (*word).to_string()).collect()
    } else {
        words
    }
}

fn make_species_name_book(config: &SimConfig) -> SpeciesNameBook {
    let prefixes = fallback_taxonomy_words(
        load_taxonomy_words("genus_prefixes.txt"),
        &["Vita", "Luma", "Novi", "Cala"],
    );
    let suffixes = fallback_taxonomy_words(
        load_taxonomy_words("genus_suffixes.txt"),
        &["um", "is", "on", "a"],
    );
    let epithets = fallback_taxonomy_words(
        load_taxonomy_words("species_epithets.txt"),
        &["primus", "lucens", "minor", "flexus"],
    );
    let _ = config;
    SpeciesNameBook {
        prefixes,
        suffixes,
        epithets,
    }
}

fn load_species_name_book(mut commands: Commands, config: Res<SimConfig>) {
    commands.insert_resource(make_species_name_book(&config));
}

fn species_name_for(names: &SpeciesNameBook, species: u32) -> String {
    let prefix_count = names.prefixes.len().max(1);
    let suffix_count = names.suffixes.len().max(1);
    let epithet_count = names.epithets.len().max(1);
    let genus_key = species_genus_key(species) as usize;
    let epithet_key = (species % SPECIES_EPITHET_SLOTS) as usize;
    let prefix = &names.prefixes[genus_key % prefix_count];
    let suffix = &names.suffixes[(genus_key / prefix_count) % suffix_count];
    let epithet = &names.epithets[(epithet_key + genus_key * 13) % epithet_count];
    format!("{prefix}{suffix} {epithet}")
}

fn species_snapshot_by_id<'a>(
    stats: &'a SpeciesLedgerStats,
    species: u32,
) -> Option<&'a SpeciesSnapshot> {
    stats
        .snapshots
        .iter()
        .find(|snapshot| snapshot.species == species)
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
    mut state: ResMut<CellAudioState>,
    config: Res<SimConfig>,
) {
    state.last_effect = None;
    commands.spawn((
        AudioPlayer(library.ambient.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(0.30 * config.ambient_volume),
            ..default()
        },
        RunningAudioEntity,
        AmbientAudio,
    ));
}

fn play_cell_audio_events(
    mut world: ResMut<WorldState>,
    library: Res<CellAudioLibrary>,
    config: Res<SimConfig>,
    mut state: ResMut<CellAudioState>,
    mut commands: Commands,
    camera: Query<(&Transform, &Projection), With<MainCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    active_effects: Query<(), With<CellEffectAudio>>,
) {
    let events = std::mem::take(&mut world.cell_sound_events);
    if events.is_empty() {
        return;
    }
    let Ok((camera_transform, projection)) = camera.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let volume = cell_effect_zoom_volume(projection) * config.sound_volume;
    if volume <= 0.001 {
        return;
    }
    let Projection::Orthographic(orthographic) = projection else {
        return;
    };
    let half_view = visible_world_size(orthographic, window) * 0.5;
    let camera_center = camera_transform.translation.truncate();
    let available_slots = MAX_ACTIVE_CELL_SOUNDS.saturating_sub(active_effects.iter().count());
    let spawn_budget = available_slots.min(MAX_CELL_SOUNDS_PER_FRAME);
    if spawn_budget == 0 {
        return;
    }
    let mut rng = rand::rng();
    let mut visible_events: Vec<Vec2> = events
        .into_iter()
        .filter(|position| sound_event_is_visible(*position, camera_center, half_view))
        .collect();

    for _ in 0..spawn_budget.min(visible_events.len()) {
        let event_index = rng.random_range(0..visible_events.len());
        visible_events.swap_remove(event_index);
        let mut effect_index = rng.random_range(0..library.effects.len());
        if library.effects.len() > 1 && state.last_effect == Some(effect_index) {
            effect_index =
                (effect_index + rng.random_range(1..library.effects.len())) % library.effects.len();
        }
        state.last_effect = Some(effect_index);
        commands.spawn((
            AudioPlayer(library.effects[effect_index].clone()),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(volume),
                ..default()
            },
            RunningAudioEntity,
            CellEffectAudio,
        ));
    }
}

fn sound_event_is_visible(position: Vec2, camera_center: Vec2, half_view: Vec2) -> bool {
    let offset = position - camera_center;
    offset.x.abs() <= half_view.x && offset.y.abs() <= half_view.y
}

fn cell_effect_zoom_volume(projection: &Projection) -> f32 {
    let Projection::Orthographic(projection) = projection else {
        return 0.0;
    };
    const FULL_VOLUME_SCALE: f32 = 0.18;
    const SILENT_SCALE: f32 = 3.0;
    let proximity =
        ((SILENT_SCALE - projection.scale) / (SILENT_SCALE - FULL_VOLUME_SCALE)).clamp(0.0, 1.0);
    proximity * 0.30
}

#[cfg(test)]
fn orthographic_projection_at_scale(scale: f32) -> Projection {
    let mut projection = OrthographicProjection::default_3d();
    projection.scale = scale;
    Projection::Orthographic(projection)
}

fn update_cell_effect_volume(
    camera: Query<&Projection, With<MainCamera>>,
    config: Res<SimConfig>,
    mut effects: Query<&mut AudioSink, With<CellEffectAudio>>,
) {
    let Ok(projection) = camera.single() else {
        return;
    };
    let volume = Volume::Linear(cell_effect_zoom_volume(projection) * config.sound_volume);
    for mut sink in &mut effects {
        sink.set_volume(volume);
    }
}

fn update_ambient_volume(
    config: Res<SimConfig>,
    mut ambient: Query<&mut AudioSink, With<AmbientAudio>>,
) {
    if !config.is_changed() {
        return;
    }
    for mut sink in &mut ambient {
        sink.set_volume(Volume::Linear(0.30 * config.ambient_volume));
    }
}

#[cfg(test)]
mod audio_tests {
    use super::*;

    #[test]
    fn cell_audio_gets_louder_when_zooming_in_and_silent_when_far() {
        let close = cell_effect_zoom_volume(&orthographic_projection_at_scale(0.2));
        let medium = cell_effect_zoom_volume(&orthographic_projection_at_scale(1.0));
        let far = cell_effect_zoom_volume(&orthographic_projection_at_scale(3.0));

        assert!(close > medium);
        assert!(medium > far);
        assert_eq!(far, 0.0);
    }

    #[test]
    fn cell_audio_events_only_play_inside_camera_bounds() {
        let center = Vec2::new(100.0, -50.0);
        let half_view = Vec2::new(500.0, 300.0);
        assert!(sound_event_is_visible(
            Vec2::new(590.0, 240.0),
            center,
            half_view
        ));
        assert!(!sound_event_is_visible(
            Vec2::new(601.0, 0.0),
            center,
            half_view
        ));
    }
}

#[cfg(test)]
mod species_ledger_tests {
    use super::*;

    #[test]
    fn tooltip_position_clamps_to_bottom_without_large_flip() {
        let y = tooltip_axis_position(590.0, 600.0, 122.0, 14.0);
        assert_eq!(y, 470.0);
    }

    #[test]
    fn species_ledger_visible_range_keeps_dom_nodes_bounded() {
        let (start, end) = species_ledger_visible_index_range(10_000, 0.0, 620.0);
        assert_eq!(start, 0);
        assert!(
            end <= 30,
            "visible cards should stay virtualized, got {end}"
        );

        let (scrolled_start, scrolled_end) =
            species_ledger_visible_index_range(10_000, SPECIES_LEDGER_ROW_STRIDE * 120.0, 620.0);
        assert!(scrolled_start > 0);
        assert!(scrolled_end - scrolled_start <= 30);
    }

    #[test]
    fn species_ledger_content_height_represents_all_species() {
        let height = species_ledger_content_height(10_000);
        let expected_rows = 10_000_usize.div_ceil(SPECIES_LEDGER_COLUMNS);
        assert!(height >= expected_rows as f32 * SPECIES_LEDGER_ROW_STRIDE);
    }

    #[test]
    fn species_ledger_scroll_target_centers_species_row() {
        let first_row = species_ledger_scroll_target_y(0, 620.0);
        assert_eq!(first_row, 0.0);

        let index = SPECIES_LEDGER_COLUMNS * 42 + 1;
        let target = species_ledger_scroll_target_y(index, 620.0);
        let row_top = 42.0 * SPECIES_LEDGER_ROW_STRIDE;
        assert!(target > row_top - 360.0);
        assert!(target < row_top);
    }
}

#[cfg(test)]
mod chronicle_tests {
    use super::*;

    fn assert_no_mojibake(text: &str) {
        for chars in [
            ['\u{0412}', '\u{00b7}'],
            ['\u{0420}', '\u{0454}'],
            ['\u{0420}', '\u{00bb}'],
            ['\u{0420}', '\u{00b5}'],
            ['\u{0420}', '\u{00b0}'],
            ['\u{0420}', '\u{0451}'],
            ['\u{0420}', '\u{0455}'],
            ['\u{0420}', '\u{0491}'],
            ['\u{0421}', '\u{0403}'],
            ['\u{0421}', '\u{040a}'],
            ['\u{0421}', '\u{040f}'],
            ['\u{0421}', '\u{040c}'],
            ['\u{0421}', '\u{2026}'],
        ] {
            let bad = chars.iter().collect::<String>();
            assert!(
                !text.contains(&bad),
                "chronicle text contains mojibake {bad:?}: {text}"
            );
        }
    }

    #[test]
    fn chronicle_labels_and_values_are_readable_utf8() {
        for kind in [
            ChronicleEventKind::World,
            ChronicleEventKind::Species,
            ChronicleEventKind::Extinction,
            ChronicleEventKind::Population,
            ChronicleEventKind::Energy,
            ChronicleEventKind::Trait,
        ] {
            assert_no_mojibake(chronicle_filter_label(kind));
        }

        let snapshot = ChronicleSnapshot {
            time: 2.0,
            cells: 10_000,
            food: 3_000,
            wild_food: 2_500,
            feeder_food: 500,
            meat: 12,
            avg_viability: 0.62,
            energy_in: 1_500.0,
            energy_out: 1_240.0,
            energy_net: 260.0,
            metabolism: 900.0,
            mitosis: 200.0,
            lysis: 30.0,
            fps: 144.0,
            sim_ms: 1.2,
            render_ms: 0.8,
            species: 42,
            segmented: 18,
            lysis_capable: 6,
        };

        for metric in [
            ChronicleSummaryMetric::Time,
            ChronicleSummaryMetric::Cells,
            ChronicleSummaryMetric::Species,
            ChronicleSummaryMetric::Food,
            ChronicleSummaryMetric::Viability,
            ChronicleSummaryMetric::Energy,
            ChronicleSummaryMetric::Costs,
            ChronicleSummaryMetric::Traits,
            ChronicleSummaryMetric::Performance,
        ] {
            assert_no_mojibake(chronicle_summary_label(metric));
            assert_no_mojibake(&chronicle_summary_value(metric, &snapshot).0);
        }

        let mut chronicle = SimulationChronicle::default();
        chronicle.snapshots.push(snapshot);
        chronicle_push_event(
            &mut chronicle,
            ChronicleEventKind::World,
            "Запуск",
            "Проверка",
            None,
        );
        let state = ChronicleUiState::default();
        for target in [
            ChronicleTooltipKind::Summary(ChronicleSummaryMetric::Energy),
            ChronicleTooltipKind::Filter(ChronicleEventKind::World),
            ChronicleTooltipKind::Graph,
            ChronicleTooltipKind::GraphMode(ChronicleGraphMode::Cells),
            ChronicleTooltipKind::Legend(ChronicleLegendLine::EnergyNegative),
        ] {
            let (heading, body, _, _, _) = chronicle_tooltip_copy(target, &chronicle, &state);
            assert_no_mojibake(&heading);
            assert_no_mojibake(&body);
        }

        for mode in [
            ChronicleGraphMode::Overall,
            ChronicleGraphMode::Cells,
            ChronicleGraphMode::Food,
            ChronicleGraphMode::Viability,
            ChronicleGraphMode::Energy,
        ] {
            let image = render_chronicle_graph(&chronicle.snapshots, mode);
            assert_eq!(image.texture_descriptor.size.width, CHRONICLE_GRAPH_WIDTH);
            assert_eq!(image.texture_descriptor.size.height, CHRONICLE_GRAPH_HEIGHT);
            assert!(image.data.as_ref().is_some_and(|data| !data.is_empty()));
        }
    }
}

fn initialize_world_state(
    mut commands: Commands,
    config: Res<SimConfig>,
    mut selected: ResMut<SelectedCell>,
    mut ui_state: ResMut<GameUiState>,
    mut species_ui: ResMut<SpeciesLedgerUiState>,
    mut species_focus: ResMut<SpeciesCameraFocus>,
    mut species_area: ResMut<SpeciesAreaHighlightState>,
    mut species_stats: ResMut<SpeciesLedgerStats>,
    mut eco_log: ResMut<EcoLogState>,
    mut chronicle_ui: ResMut<ChronicleUiState>,
    mut chronicle: ResMut<SimulationChronicle>,
    mut chronicle_graph_cache: ResMut<ChronicleGraphCache>,
    mut chronicle_event_scroll: ResMut<ChronicleEventScrollState>,
) {
    selected.cell_id = None;
    *ui_state = GameUiState::default();
    *species_ui = SpeciesLedgerUiState::default();
    *species_focus = SpeciesCameraFocus::default();
    *species_area = SpeciesAreaHighlightState::default();
    *species_stats = SpeciesLedgerStats::default();
    *eco_log = EcoLogState::default();
    *chronicle_ui = ChronicleUiState::default();
    *chronicle = SimulationChronicle::default();
    *chronicle_graph_cache = ChronicleGraphCache::default();
    *chronicle_event_scroll = ChronicleEventScrollState::default();
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

fn maximize_primary_window(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.set_maximized(true);
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

fn setup_species_ledger_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT);
    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: px(18),
                bottom: px(18),
                width: px(46),
                height: px(46),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.39, 0.64, 0.70)),
            BackgroundColor(Color::srgb(0.035, 0.055, 0.064)),
            SpeciesLedgerButton,
            RunningUiEntity,
        ))
        .with_child((
            ImageNode::new(asset_server.load("sprites/icon-species-ledger.png")),
            Node {
                width: px(28),
                height: px(28),
                ..default()
            },
        ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(SPECIES_LEDGER_PANEL_LEFT - SPECIES_LEDGER_PANEL_REVEAL_OFFSET),
                bottom: px(SPECIES_LEDGER_PANEL_BOTTOM),
                width: px(SPECIES_LEDGER_PANEL_WIDTH),
                height: percent(SPECIES_LEDGER_PANEL_HEIGHT_PERCENT),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                padding: UiRect::all(px(12)),
                border: UiRect::all(px(2)),
                overflow: Overflow::clip(),
                display: Display::None,
                ..default()
            },
            BorderColor::all(Color::srgb(0.39, 0.64, 0.70)),
            BackgroundColor(Color::srgba(0.012, 0.018, 0.022, 0.96)),
            Visibility::Hidden,
            PanelReveal::horizontal(SPECIES_LEDGER_PANEL_REVEAL_OFFSET),
            SpeciesLedgerPanel,
            RunningUiEntity,
        ))
        .with_children(|panel| {
            panel
                .spawn((Node {
                    width: percent(100),
                    height: px(24),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_child((
                    Text::new("РЕЕСТР ВИДОВ"),
                    TextFont {
                        font: font.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.76, 0.94, 0.92)),
                ));

            panel
                .spawn((Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(0),
                    position_type: PositionType::Relative,
                    ..default()
                },))
                .with_children(|viewport| {
                    viewport
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(0),
                                top: px(0),
                                width: percent(100),
                                height: percent(100),
                                padding: UiRect::new(px(8), px(18), px(8), px(8)),
                                border: UiRect::all(px(1)),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.17, 0.36, 0.41)),
                            BackgroundColor(Color::srgb(0.014, 0.025, 0.030)),
                            ScrollPosition::default(),
                            RelativeCursorPosition::default(),
                            SpeciesLedgerScrollArea,
                        ))
                        .with_child((
                            Node {
                                width: percent(100),
                                height: px(SPECIES_LEDGER_ROW_STRIDE),
                                position_type: PositionType::Relative,
                                ..default()
                            },
                            SpeciesLedgerGrid,
                        ));

                    viewport
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                right: px(3),
                                top: px(8),
                                width: px(18),
                                height: percent(96),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.15, 0.36, 0.42, 0.30)),
                            Visibility::Hidden,
                            SpeciesLedgerScrollbarTrack,
                        ))
                        .with_child((
                            Node {
                                position_type: PositionType::Absolute,
                                top: percent(0),
                                left: px(4),
                                width: px(10),
                                height: percent(18),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.46, 0.86, 0.92)),
                            SpeciesLedgerScrollbarThumb,
                        ));
                });
        });

    spawn_species_journal_panel(&mut commands, font.clone());
}

fn setup_chronicle_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT);
    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: px(CHRONICLE_BUTTON_LEFT),
                bottom: px(18),
                width: px(46),
                height: px(46),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.39, 0.64, 0.70)),
            BackgroundColor(Color::srgb(0.035, 0.055, 0.064)),
            ChronicleButton,
            RunningUiEntity,
        ))
        .with_child((
            Text::new("H"),
            TextFont {
                font: font.clone(),
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.78, 0.96, 0.94)),
        ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(CHRONICLE_PANEL_LEFT - CHRONICLE_PANEL_REVEAL_OFFSET),
                bottom: px(CHRONICLE_PANEL_BOTTOM),
                width: px(CHRONICLE_PANEL_WIDTH),
                height: percent(CHRONICLE_PANEL_HEIGHT_PERCENT),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(2)),
                overflow: Overflow::clip(),
                display: Display::None,
                ..default()
            },
            BorderColor::all(Color::srgb(0.39, 0.64, 0.70)),
            BackgroundColor(Color::srgba(0.012, 0.018, 0.022, 0.96)),
            Visibility::Hidden,
            PanelReveal::horizontal(CHRONICLE_PANEL_REVEAL_OFFSET),
            ChroniclePanel,
            RunningUiEntity,
        ))
        .with_children(|panel| {
            panel
                .spawn((Node {
                    width: percent(100),
                    height: px(34),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },))
                .with_children(|header| {
                    header.spawn((
                        Text::new("ХРОНИКА СИМУЛЯЦИИ"),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.76, 0.94, 0.92)),
                    ));
                    header.spawn((
                        Text::new("H"),
                        TextFont {
                            font: font.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.52, 0.78, 0.82)),
                    ));
                });

            panel.spawn((
                Text::new("ожидание данных"),
                TextFont {
                    font: font.clone(),
                    font_size: 1.0,
                    ..default()
                },
                TextColor(Color::srgba(0.62, 0.80, 0.82, 0.0)),
                ChronicleOverviewText,
            ));

            panel
                .spawn((Node {
                    width: percent(100),
                    min_height: px(76),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(6),
                    row_gap: px(6),
                    align_content: AlignContent::FlexStart,
                    ..default()
                },))
                .with_children(|summary| {
                    for metric in [
                        ChronicleSummaryMetric::Time,
                        ChronicleSummaryMetric::Cells,
                        ChronicleSummaryMetric::Species,
                        ChronicleSummaryMetric::Food,
                        ChronicleSummaryMetric::Viability,
                        ChronicleSummaryMetric::Energy,
                        ChronicleSummaryMetric::Costs,
                        ChronicleSummaryMetric::Traits,
                        ChronicleSummaryMetric::Performance,
                    ] {
                        summary
                            .spawn((
                                Node {
                                    width: px(chronicle_summary_width(metric)),
                                    height: px(34),
                                    padding: UiRect::new(px(7), px(7), px(3), px(3)),
                                    border: UiRect::left(px(3)),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                BorderColor::all(chronicle_summary_color(metric)),
                                BackgroundColor(Color::srgb(0.020, 0.036, 0.042)),
                                Interaction::default(),
                                ChronicleTooltipTarget {
                                    kind: ChronicleTooltipKind::Summary(metric),
                                },
                            ))
                            .with_children(|card| {
                                card.spawn((
                                    Text::new(chronicle_summary_label(metric)),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 10.5,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.50, 0.70, 0.72)),
                                ));
                                card.spawn((
                                    Text::new("-"),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.80, 0.96, 0.94)),
                                    ChronicleSummaryValue { metric },
                                ));
                            });
                    }
                });

            panel
                .spawn((Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(12),
                    ..default()
                },))
                .with_children(|body| {
                    body
                        .spawn((
                            Node {
                                width: percent(46),
                                height: percent(100),
                                padding: UiRect::all(px(10)),
                                border: UiRect::all(px(1)),
                                flex_direction: FlexDirection::Column,
                                row_gap: px(8),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.17, 0.36, 0.41)),
                            BackgroundColor(Color::srgb(0.014, 0.025, 0.030)),
                        ))
                        .with_children(|events| {
                            events.spawn((
                                Text::new("События"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 15.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.76, 0.94, 0.92)),
                            ));
                            events
                                .spawn((Node {
                                    width: percent(100),
                                    height: px(28),
                                    flex_direction: FlexDirection::Row,
                                    column_gap: px(5),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },))
                                .with_children(|filters| {
                                    for kind in [
                                        ChronicleEventKind::World,
                                        ChronicleEventKind::Species,
                                        ChronicleEventKind::Extinction,
                                        ChronicleEventKind::Population,
                                        ChronicleEventKind::Energy,
                                        ChronicleEventKind::Trait,
                                    ] {
                                        filters
                                            .spawn((
                                                Button,
                                                Node {
                                                    width: px(54),
                                                    height: px(24),
                                                    border: UiRect::all(px(1)),
                                                    align_items: AlignItems::Center,
                                                    justify_content: JustifyContent::Center,
                                                    ..default()
                                                },
                                                BorderColor::all(chronicle_kind_color(kind)),
                                                BackgroundColor(Color::srgb(0.030, 0.047, 0.055)),
                                                ChronicleFilterButton { kind },
                                                ChronicleTooltipTarget {
                                                    kind: ChronicleTooltipKind::Filter(kind),
                                                },
                                            ))
                                            .with_child((
                                                Text::new(chronicle_filter_label(kind)),
                                                TextFont {
                                                    font: font.clone(),
                                                    font_size: 11.0,
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.78, 0.92, 0.91)),
                                            ));
                                    }
                                });
                            events
                                .spawn((Node {
                                    width: percent(100),
                                    flex_grow: 1.0,
                                    min_height: px(0),
                                    position_type: PositionType::Relative,
                                    ..default()
                                },))
                                .with_children(|viewport| {
                                    viewport
                                        .spawn((
                                            Node {
                                                position_type: PositionType::Absolute,
                                                left: px(0),
                                                top: px(0),
                                                width: percent(100),
                                                height: percent(100),
                                                padding: UiRect::right(px(18)),
                                                overflow: Overflow::scroll_y(),
                                                ..default()
                                            },
                                            ScrollPosition::default(),
                                            RelativeCursorPosition::default(),
                                            ChronicleEventScrollArea,
                                        ))
                                        .with_child((
                                            Text::new(""),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 13.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.72, 0.84, 0.84)),
                                            Pickable::IGNORE,
                                            ChronicleEventText,
                                        ));
                                    viewport
                                        .spawn((
                                            Node {
                                                position_type: PositionType::Absolute,
                                                right: px(2),
                                                top: px(3),
                                                width: px(14),
                                                height: percent(96),
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgba(0.15, 0.36, 0.42, 0.30)),
                                            Visibility::Hidden,
                                            ChronicleEventScrollbarTrack,
                                        ))
                                        .with_child((
                                            Node {
                                                position_type: PositionType::Absolute,
                                                top: percent(0),
                                                left: px(3),
                                                width: px(8),
                                                height: percent(18),
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgb(0.46, 0.86, 0.92)),
                                            ChronicleEventScrollbarThumb,
                                        ));
                                });
                        });

                    body
                        .spawn((
                            Node {
                                width: percent(54),
                                height: percent(100),
                                padding: UiRect::all(px(10)),
                                border: UiRect::all(px(1)),
                                flex_direction: FlexDirection::Column,
                                row_gap: px(8),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.17, 0.36, 0.41)),
                            BackgroundColor(Color::srgb(0.014, 0.025, 0.030)),
                        ))
                        .with_children(|graphs| {
                            graphs.spawn((
                                Text::new("Графики последних срезов"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 15.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.76, 0.94, 0.92)),
                            ));
                            graphs
                                .spawn((Node {
                                    width: percent(100),
                                    height: px(28),
                                    flex_direction: FlexDirection::Row,
                                    column_gap: px(6),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },))
                                .with_children(|modes| {
                                    for mode in [
                                        ChronicleGraphMode::Overall,
                                        ChronicleGraphMode::Cells,
                                        ChronicleGraphMode::Food,
                                        ChronicleGraphMode::Viability,
                                        ChronicleGraphMode::Energy,
                                    ] {
                                        modes
                                            .spawn((
                                                Button,
                                                Node {
                                                    width: px(70),
                                                    height: px(24),
                                                    border: UiRect::all(px(1)),
                                                    align_items: AlignItems::Center,
                                                    justify_content: JustifyContent::Center,
                                                    ..default()
                                                },
                                                BorderColor::all(chronicle_graph_mode_color(mode)),
                                                BackgroundColor(Color::srgb(0.030, 0.047, 0.055)),
                                                ChronicleGraphButton { mode },
                                                ChronicleTooltipTarget {
                                                    kind: ChronicleTooltipKind::GraphMode(mode),
                                                },
                                            ))
                                            .with_child((
                                                Text::new(chronicle_graph_mode_label(mode)),
                                                TextFont {
                                                    font: font.clone(),
                                                    font_size: 11.0,
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.78, 0.92, 0.91)),
                                            ));
                                    }
                                });
                            graphs.spawn((
                                ImageNode::default(),
                                Node {
                                    width: percent(100),
                                    height: px(CHRONICLE_GRAPH_HEIGHT as f32),
                                    border: UiRect::all(px(1)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgb(0.16, 0.34, 0.39)),
                                Interaction::default(),
                                ChronicleTooltipTarget {
                                    kind: ChronicleTooltipKind::Graph,
                                },
                                ChronicleGraphImage,
                            ));
                            graphs
                                .spawn((Node {
                                    width: percent(100),
                                    min_height: px(28),
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    column_gap: px(8),
                                    row_gap: px(6),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },))
                                .with_children(|legend| {
                                    for (label, color) in [
                                        ("клетки", Color::srgb(0.34, 1.0, 0.52)),
                                        ("еда", Color::srgb(1.0, 0.86, 0.30)),
                                        ("жизнь", Color::srgb(0.90, 1.0, 0.94)),
                                        ("баланс +", Color::srgb(0.40, 1.0, 0.66)),
                                        ("баланс -", Color::srgb(1.0, 0.36, 0.31)),
                                    ] {
                                        let line = match label {
                                            "клетки" => ChronicleLegendLine::Cells,
                                            "еда" => ChronicleLegendLine::Food,
                                            "жизнь" => ChronicleLegendLine::Viability,
                                            "баланс +" => ChronicleLegendLine::EnergyPositive,
                                            _ => ChronicleLegendLine::EnergyNegative,
                                        };
                                        legend
                                            .spawn((
                                                Node {
                                                    width: px(76),
                                                    height: px(23),
                                                    padding: UiRect::horizontal(px(6)),
                                                    flex_direction: FlexDirection::Row,
                                                    column_gap: px(5),
                                                    align_items: AlignItems::Center,
                                                    border: UiRect::all(px(1)),
                                                    ..default()
                                                },
                                                BorderColor::all(color),
                                                BackgroundColor(Color::srgb(0.020, 0.036, 0.042)),
                                                Interaction::default(),
                                                ChronicleTooltipTarget {
                                                    kind: ChronicleTooltipKind::Legend(line),
                                                },
                                            ))
                                            .with_children(|item| {
                                                item.spawn((
                                                    Node {
                                                        width: px(12),
                                                        height: px(3),
                                                        ..default()
                                                    },
                                                    BackgroundColor(color),
                                                ));
                                                item.spawn((
                                                    Text::new(label),
                                                    TextFont {
                                                        font: font.clone(),
                                                        font_size: 11.0,
                                                        ..default()
                                                    },
                                                    TextColor(Color::srgb(0.72, 0.86, 0.86)),
                                                ));
                                            });
                                    }
                                });
                            graphs.spawn((
                                Text::new(
                                    "зелёный: клетки · жёлтый: еда · белый: жизнеспособность · красный/зелёный: баланс энергии",
                                ),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 1.0,
                                    ..default()
                                },
                                TextColor(Color::srgba(0.58, 0.72, 0.74, 0.0)),
                            ));
                        });
                });
        });
}

fn spawn_species_journal_panel(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(SPECIES_JOURNAL_PANEL_LEFT - SPECIES_JOURNAL_PANEL_REVEAL_OFFSET),
                bottom: px(SPECIES_LEDGER_PANEL_BOTTOM),
                width: px(SPECIES_JOURNAL_PANEL_WIDTH),
                height: percent(SPECIES_LEDGER_PANEL_HEIGHT_PERCENT),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(2)),
                overflow: Overflow::clip(),
                display: Display::None,
                ..default()
            },
            BorderColor::all(Color::srgb(0.39, 0.64, 0.70)),
            BackgroundColor(Color::srgba(0.010, 0.017, 0.020, 0.97)),
            Visibility::Hidden,
            Pickable::default(),
            PanelReveal::horizontal(SPECIES_JOURNAL_PANEL_REVEAL_OFFSET),
            SpeciesJournalPanel,
            RunningUiEntity,
        ))
        .with_children(|journal| {
            journal
                .spawn((Node {
                    width: percent(100),
                    height: px(32),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },))
                .with_children(|header| {
                    header.spawn((
                        Text::new("БИОЖУРНАЛ ВИДА"),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.76, 0.96, 0.93)),
                    ));
                    header.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.74, 1.0, 0.78)),
                        SpeciesJournalTrendText,
                    ));
                });

            journal
                .spawn((Node {
                    width: percent(100),
                    height: px(228),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(14),
                    min_height: px(0),
                    ..default()
                },))
                .with_children(|summary| {
                    summary
                        .spawn((
                            Node {
                                width: px(222),
                                height: px(222),
                                border: UiRect::all(px(2)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.32, 0.68, 0.74)),
                            BackgroundColor(Color::srgb(0.012, 0.027, 0.032)),
                            Interaction::default(),
                            SpeciesJournalTooltipTarget {
                                kind: SpeciesJournalTooltipKind::Portrait,
                            },
                        ))
                        .with_child((
                            ImageNode::default(),
                            Node {
                                width: px(206),
                                height: px(206),
                                ..default()
                            },
                            SpeciesJournalPortraitImage,
                        ));

                    summary
                        .spawn((Node {
                            flex_grow: 1.0,
                            min_width: px(0),
                            height: percent(100),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(8),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },))
                        .with_children(|info| {
                            info.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 19.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.90, 1.0, 0.96)),
                                SpeciesJournalTitleText,
                            ));
                            info.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.54, 0.78, 0.82)),
                                SpeciesJournalSubtitleText,
                            ));

                            info.spawn((
                                Node {
                                    width: percent(100),
                                    height: px(44),
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: px(10),
                                    padding: UiRect::new(px(8), px(8), px(0), px(0)),
                                    border: UiRect::left(px(3)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgb(0.32, 0.72, 0.68)),
                                BackgroundColor(Color::srgb(0.018, 0.038, 0.043)),
                                Interaction::default(),
                                SpeciesJournalTooltipTarget {
                                    kind: SpeciesJournalTooltipKind::Diet,
                                },
                            ))
                            .with_children(|diet_row| {
                                diet_row.spawn((
                                    ImageNode::default(),
                                    Node {
                                        width: px(28),
                                        height: px(28),
                                        ..default()
                                    },
                                    SpeciesJournalDietIcon,
                                ));
                                diet_row.spawn((
                                    Text::new(""),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 11.5,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.74, 0.91, 0.90)),
                                    SpeciesJournalBodyText,
                                ));
                            });
                            info.spawn((
                                Node {
                                    width: percent(100),
                                    height: px(40),
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::SpaceBetween,
                                    column_gap: px(10),
                                    padding: UiRect::new(px(10), px(10), px(0), px(0)),
                                    border: UiRect::left(px(3)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgb(0.32, 0.72, 0.68)),
                                BackgroundColor(Color::srgb(0.012, 0.030, 0.034)),
                                Interaction::default(),
                                Pickable::default(),
                                SpeciesJournalAreaRow,
                            ))
                            .with_children(|area_row| {
                                area_row.spawn((
                                    Text::new("Ареал"),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 12.5,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.74, 0.96, 0.92)),
                                    Pickable::IGNORE,
                                ));
                                area_row.spawn((
                                    Text::new(""),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 11.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.58, 0.82, 0.84)),
                                    SpeciesJournalAreaText,
                                    Pickable::IGNORE,
                                ));
                            });
                        });
                });

            journal.spawn((
                Node {
                    width: percent(100),
                    height: px(1),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.39, 0.44)),
            ));

            journal.spawn((
                Text::new("ДИНАМИКА ВИДА"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.91, 0.89)),
            ));

            journal
                .spawn((Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(10),
                    ..default()
                },))
                .with_children(|metrics| {
                    metrics
                        .spawn((Node {
                            flex_grow: 1.0,
                            min_width: px(0),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(9),
                            ..default()
                        },))
                        .with_children(|left| {
                            for metric in [
                                SpeciesJournalMetric::Population,
                                SpeciesJournalMetric::Viability,
                                SpeciesJournalMetric::Size,
                                SpeciesJournalMetric::Mutation,
                            ] {
                                spawn_species_journal_metric(left, font.clone(), metric);
                            }
                        });
                    metrics
                        .spawn((Node {
                            flex_grow: 1.0,
                            min_width: px(0),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(9),
                            ..default()
                        },))
                        .with_children(|right| {
                            for metric in [
                                SpeciesJournalMetric::Speed,
                                SpeciesJournalMetric::Turn,
                                SpeciesJournalMetric::Perception,
                                SpeciesJournalMetric::Persistence,
                                SpeciesJournalMetric::Aggressiveness,
                                SpeciesJournalMetric::Lysis,
                            ] {
                                spawn_species_journal_metric(right, font.clone(), metric);
                            }
                        });
                });
        });
}

fn species_journal_metric_label(metric: SpeciesJournalMetric) -> &'static str {
    match metric {
        SpeciesJournalMetric::Population => "Численность",
        SpeciesJournalMetric::Viability => "Жизнь",
        SpeciesJournalMetric::Size => "Размер",
        SpeciesJournalMetric::Speed => "Скорость",
        SpeciesJournalMetric::Turn => "Поворот",
        SpeciesJournalMetric::Perception => "Восприятие",
        SpeciesJournalMetric::Persistence => "Настойчивость",
        SpeciesJournalMetric::Aggressiveness => "Агрессия",
        SpeciesJournalMetric::Lysis => "Лизис",
        SpeciesJournalMetric::Mutation => "Мутации",
    }
}

fn species_journal_metric_color(metric: SpeciesJournalMetric) -> Color {
    match metric {
        SpeciesJournalMetric::Population => Color::srgb(0.52, 1.0, 0.62),
        SpeciesJournalMetric::Viability => gene_stat_color(GeneStatId::Viability),
        SpeciesJournalMetric::Size => gene_stat_color(GeneStatId::Size),
        SpeciesJournalMetric::Speed => gene_stat_color(GeneStatId::Speed),
        SpeciesJournalMetric::Turn => gene_stat_color(GeneStatId::Turn),
        SpeciesJournalMetric::Perception => gene_stat_color(GeneStatId::Perception),
        SpeciesJournalMetric::Persistence => gene_stat_color(GeneStatId::Persistence),
        SpeciesJournalMetric::Aggressiveness => gene_stat_color(GeneStatId::Aggressiveness),
        SpeciesJournalMetric::Lysis => gene_stat_color(GeneStatId::Lysis),
        SpeciesJournalMetric::Mutation => gene_stat_color(GeneStatId::Mutation),
    }
}

fn spawn_species_journal_metric(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    metric: SpeciesJournalMetric,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(46),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::new(px(10), px(10), px(7), px(7)),
                border: UiRect::left(px(3)),
                ..default()
            },
            BorderColor::all(species_journal_metric_color(metric)),
            BackgroundColor(Color::srgb(0.016, 0.031, 0.036)),
            Interaction::default(),
            SpeciesJournalTooltipTarget {
                kind: SpeciesJournalTooltipKind::Metric(metric),
            },
        ))
        .with_children(|row| {
            row.spawn((Node {
                width: percent(100),
                height: px(15),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },))
                .with_children(|label_row| {
                    label_row.spawn((
                        Text::new(species_journal_metric_label(metric)),
                        TextFont {
                            font: font.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.68, 0.83, 0.84)),
                    ));
                    label_row.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.88, 1.0, 0.96)),
                        SpeciesJournalMetricValue { metric },
                    ));
                });

            row.spawn((
                Node {
                    width: percent(100),
                    height: px(7),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.18, 0.38, 0.43)),
                BackgroundColor(Color::srgb(0.018, 0.034, 0.039)),
            ))
            .with_child((
                Node {
                    width: percent(0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(species_journal_metric_color(metric)),
                SpeciesJournalMetricFill { metric },
            ));
        });
}

fn setup_game_stats_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT);
    commands
        .spawn((
            Text::new("Загрузка"),
            TextFont {
                font: font.clone(),
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
        ))
        .with_children(|text| {
            let span_font = TextFont {
                font: font.clone(),
                font_size: 16.0,
                ..default()
            };
            text.spawn((TextSpan::new("\nклетки: "), span_font.clone()));
            text.spawn((
                TextSpan::new("0"),
                span_font.clone(),
                TextColor(Color::srgb(0.86, 0.91, 0.95)),
                PopulationCountText(PopulationCounterKind::Cells),
            ));
            text.spawn((
                TextSpan::new(" +0"),
                span_font.clone(),
                TextColor(Color::srgb(0.66, 0.72, 0.76)),
                PopulationDeltaText(PopulationCounterKind::Cells),
            ));
            text.spawn((TextSpan::new("\nеда: "), span_font.clone()));
            text.spawn((
                TextSpan::new("0"),
                span_font.clone(),
                TextColor(Color::srgb(0.86, 0.91, 0.95)),
                PopulationCountText(PopulationCounterKind::Food),
            ));
            text.spawn((
                TextSpan::new(" +0"),
                span_font.clone(),
                TextColor(Color::srgb(0.66, 0.72, 0.76)),
                PopulationDeltaText(PopulationCounterKind::Food),
            ));
            text.spawn((TextSpan::new(""), span_font, StatsBodyText));
            text.spawn((
                TextSpan::new("\nИТОГО       0.0 ед/с"),
                TextFont {
                    font,
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.78, 0.34)),
                EnergyBalanceText,
            ));
        });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GeneCategory {
    State,
    Morphology,
    Movement,
    Behavior,
    Heredity,
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
            category: GeneCategory::State,
            compact: true,
            color: gene_stat_color(GeneStatId::Viability),
        },
        GeneStatDescriptor {
            id: GeneStatId::Speed,
            label: "Скорость",
            icon: "sprites/gene-speed.png",
            category: GeneCategory::Movement,
            compact: true,
            color: gene_stat_color(GeneStatId::Speed),
        },
        GeneStatDescriptor {
            id: GeneStatId::Turn,
            label: "Поворотливость",
            icon: "sprites/gene-maneuverability.png",
            category: GeneCategory::Movement,
            compact: true,
            color: gene_stat_color(GeneStatId::Turn),
        },
        GeneStatDescriptor {
            id: GeneStatId::Perception,
            label: "Восприятие",
            icon: "sprites/gene-perception.png",
            category: GeneCategory::Behavior,
            compact: true,
            color: gene_stat_color(GeneStatId::Perception),
        },
        GeneStatDescriptor {
            id: GeneStatId::Persistence,
            label: "Настойчивость",
            icon: "sprites/gene-persistence.png",
            category: GeneCategory::Behavior,
            compact: true,
            color: gene_stat_color(GeneStatId::Persistence),
        },
        GeneStatDescriptor {
            id: GeneStatId::Aggressiveness,
            label: "Агрессивность",
            icon: "sprites/gene-aggressiveness.png",
            category: GeneCategory::Behavior,
            compact: true,
            color: gene_stat_color(GeneStatId::Aggressiveness),
        },
        GeneStatDescriptor {
            id: GeneStatId::Diet,
            label: "Рацион",
            icon: "sprites/gene-type-biotroph.png",
            category: GeneCategory::Behavior,
            compact: true,
            color: gene_stat_color(GeneStatId::Diet),
        },
        GeneStatDescriptor {
            id: GeneStatId::Lysis,
            label: "Лизис",
            icon: "sprites/gene-lysis.png",
            category: GeneCategory::Behavior,
            compact: true,
            color: gene_stat_color(GeneStatId::Lysis),
        },
        GeneStatDescriptor {
            id: GeneStatId::Mutation,
            label: "Мутации",
            icon: "sprites/gene-mutation.png",
            category: GeneCategory::Heredity,
            compact: true,
            color: gene_stat_color(GeneStatId::Mutation),
        },
        GeneStatDescriptor {
            id: GeneStatId::Size,
            label: "Размер",
            icon: "sprites/gene-size.png",
            category: GeneCategory::Morphology,
            compact: true,
            color: gene_stat_color(GeneStatId::Size),
        },
    ]
}

fn gene_stat_color(kind: GeneStatId) -> Color {
    match kind {
        GeneStatId::Viability => Color::srgb(0.35, 0.95, 0.46),
        GeneStatId::Speed => Color::srgb(0.42, 0.72, 1.0),
        GeneStatId::Turn => Color::srgb(0.95, 0.78, 0.36),
        GeneStatId::Perception => Color::srgb(0.38, 0.88, 0.86),
        GeneStatId::Persistence => Color::srgb(0.96, 0.62, 0.42),
        GeneStatId::Aggressiveness => Color::srgb(1.0, 0.42, 0.30),
        GeneStatId::Diet => Color::srgb(0.78, 0.96, 0.50),
        GeneStatId::Lysis => Color::srgb(0.94, 0.30, 0.48),
        GeneStatId::Mutation => Color::srgb(0.77, 0.56, 1.0),
        GeneStatId::Size => Color::srgb(0.70, 0.95, 0.86),
    }
}

fn setup_biolab_ui_v2(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT);
    let stats = gene_stat_descriptors();

    spawn_compact_selection_panel(&mut commands, &asset_server, font.clone(), &stats);
    spawn_passport_panel(&mut commands, &asset_server, font.clone(), &stats);
    spawn_division_tooltip(&mut commands, font.clone());
    spawn_gene_tooltip(&mut commands, font.clone());
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
            PanelReveal::horizontal(540.0),
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
                width: percent(92),
                max_width: px(820),
                padding: UiRect::all(px(20)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Column,
                row_gap: px(16),
                ..default()
            },
            BorderColor::all(Color::srgb(0.44, 0.74, 0.82)),
            BackgroundColor(Color::srgb(0.018, 0.027, 0.034)),
            Visibility::Hidden,
            PassportPanel,
            PanelReveal::horizontal(860.0),
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
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.78, 0.97, 0.94)),
                        PassportCellTitle,
                    ));

                    header
                        .spawn((
                            Button,
                            Node {
                                width: px(84),
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
                    column_gap: px(18),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },))
                .with_children(|columns| {
                    spawn_passport_column(
                        columns,
                        asset_server,
                        font.clone(),
                        stats,
                        &[
                            (GeneCategory::State, "Состояние"),
                            (GeneCategory::Morphology, "Морфология"),
                            (GeneCategory::Heredity, "Наследственность"),
                        ],
                    );
                    spawn_passport_column(
                        columns,
                        asset_server,
                        font,
                        stats,
                        &[
                            (GeneCategory::Movement, "Движение"),
                            (GeneCategory::Behavior, "Поведение"),
                        ],
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
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            ..default()
        },))
        .with_children(|column| {
            for (category, label) in categories {
                column.spawn((
                    Text::new(*label),
                    Node {
                        margin: UiRect::top(px(2)),
                        ..default()
                    },
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
    let row_min_height = if show_range { 76.0 } else { 68.0 };
    let row_padding_y = if show_range { 8.0 } else { 7.0 };
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(11),
                width: percent(100),
                min_height: px(row_min_height),
                padding: UiRect::axes(px(10), px(row_padding_y)),
                border: UiRect::left(px(4)),
                ..default()
            },
            BorderColor::all(fill_color),
            BackgroundColor(Color::srgb(0.04, 0.065, 0.075)),
            Interaction::default(),
            GeneTooltipTarget { kind },
        ))
        .with_children(|row| {
            row.spawn((
                ImageNode::new(icon),
                Node {
                    width: px(32),
                    height: px(32),
                    ..default()
                },
                GeneIconNode { kind },
            ));

            row.spawn((Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
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
                                    font_size: 13.5,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.70, 0.76, 0.80)),
                            ));

                            line.spawn((
                                Text::new("0"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 13.5,
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
                                height: px(12),
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
                                font_size: 11.5,
                                ..default()
                            },
                            TextColor(Color::srgb(0.58, 0.73, 0.76)),
                            TextLayout::new_with_linebreak(LineBreak::WordBoundary),
                            Node {
                                width: percent(100),
                                ..default()
                            },
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
                width: px(430),
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

fn spawn_gene_tooltip(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: px(430),
                padding: UiRect::all(px(13)),
                border: UiRect::all(px(3)),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                ..default()
            },
            BorderColor::all(Color::srgb(0.38, 0.88, 0.86)),
            BackgroundColor(Color::srgb(0.020, 0.035, 0.040)),
            GlobalZIndex(90),
            UiTransform::default(),
            Visibility::Hidden,
            GeneTooltip::default(),
            RunningUiEntity,
        ))
        .with_children(|tooltip| {
            tooltip.spawn((
                Text::new("ГЕН"),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.98, 0.88)),
                GeneTooltipTitle,
            ));
            tooltip.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: 21.0,
                    ..default()
                },
                TextColor(Color::srgb(0.38, 0.88, 0.86)),
                Node {
                    display: Display::None,
                    ..default()
                },
                GeneTooltipValue,
            ));
            tooltip.spawn((
                Text::new("Описание"),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.84, 0.94, 0.88)),
                Node {
                    width: percent(100),
                    ..default()
                },
                GeneTooltipBody,
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
                margin: UiRect::new(px(-180), px(0), px(-190), px(0)),
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
            PanelReveal::default(),
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

            spawn_pause_audio_slider(
                menu,
                font.clone(),
                "Звуки клеток",
                AudioVolumeKind::Effects,
                0.8,
            );
            spawn_pause_audio_slider(menu, font.clone(), "Эмбиент", AudioVolumeKind::Ambient, 0.6);
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

fn spawn_pause_audio_slider(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    label: &str,
    kind: AudioVolumeKind,
    value: f32,
) {
    parent
        .spawn((Node {
            width: percent(100),
            height: px(34),
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
                TextColor(Color::srgb(0.72, 0.84, 0.86)),
                Node {
                    width: px(98),
                    ..default()
                },
            ));
            row.spawn((
                Button,
                Node {
                    width: px(180),
                    height: px(14),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.07, 0.11, 0.12)),
                RelativeCursorPosition::default(),
                PauseAudioSlider(kind),
            ))
            .with_child((
                Node {
                    width: percent(value * 100.0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.38, 0.76, 0.70)),
                PauseAudioFill(kind),
            ));
            row.spawn((
                Text::new(format!("{:.0}%", value * 100.0)),
                TextFont {
                    font,
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.84, 0.94, 0.94)),
                Node {
                    width: px(42),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                PauseAudioValue(kind),
            ));
        });
}

fn audio_volume(config: &SimConfig, kind: AudioVolumeKind) -> f32 {
    match kind {
        AudioVolumeKind::Effects => config.sound_volume,
        AudioVolumeKind::Ambient => config.ambient_volume,
    }
}

fn set_audio_volume(config: &mut SimConfig, kind: AudioVolumeKind, value: f32) {
    match kind {
        AudioVolumeKind::Effects => config.sound_volume = value,
        AudioVolumeKind::Ambient => config.ambient_volume = value,
    }
}

fn pause_audio_slider_system(
    mouse: Res<ButtonInput<MouseButton>>,
    ui_state: Res<GameUiState>,
    sliders: Query<(&Interaction, &RelativeCursorPosition, &PauseAudioSlider)>,
    mut config: ResMut<SimConfig>,
) {
    if !ui_state.pause_menu_open || !mouse.pressed(MouseButton::Left) {
        return;
    }
    for (interaction, cursor, slider) in &sliders {
        if *interaction == Interaction::Pressed
            && let Some(fraction) = relative_cursor_fraction_x(cursor)
        {
            set_audio_volume(&mut config, slider.0, fraction);
        }
    }
}

fn sync_pause_audio_sliders(
    time: Res<Time>,
    config: Res<SimConfig>,
    mut fills: Query<(&PauseAudioFill, &mut Node)>,
    mut values: Query<(&PauseAudioValue, &mut Text)>,
) {
    let follow = 1.0 - (-12.0 * time.delta_secs()).exp();
    for (fill, mut node) in &mut fills {
        let target = audio_volume(&config, fill.0) * 100.0;
        let current = match node.width {
            Val::Percent(value) => value,
            _ => target,
        };
        node.width = percent(current + (target - current) * follow);
    }
    for (value, mut text) in &mut values {
        **text = format!("{:.0}%", audio_volume(&config, value.0) * 100.0);
    }
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
        (0.0, "II"),
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
            PanelReveal {
                progress: 1.0,
                hidden_offset: 82.0,
            },
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
    time: Res<Time>,
    ui_state: Res<GameUiState>,
    mut buttons: Query<(
        &SpeedButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let follow = 1.0 - (-14.0 * time.delta_secs()).exp();
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

        let target_bg = match *interaction {
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
        bg.0 = bg.0.mix(&target_bg, follow);

        *border = if is_active {
            BorderColor::all(Color::srgb(0.50, 0.88, 0.92))
        } else {
            BorderColor::all(Color::srgb(0.34, 0.58, 0.64))
        };
    }
}

fn update_speed_panel_visibility(
    time: Res<Time>,
    ui_state: Res<GameUiState>,
    mut panel: Query<(&mut Visibility, &mut Node, &mut PanelReveal), With<SpeedPanel>>,
) {
    let Ok((mut visibility, mut node, mut reveal)) = panel.single_mut() else {
        return;
    };

    if ui_state.speed_panel_open {
        *visibility = Visibility::Visible;
    }

    let target = if ui_state.speed_panel_open { 1.0 } else { 0.0 };
    let follow = 1.0 - (-12.0 * time.delta_secs()).exp();
    reveal.progress += (target - reveal.progress) * follow;
    node.bottom = px(14.0 - reveal.hidden_offset * (1.0 - reveal.progress));

    if !ui_state.speed_panel_open && reveal.progress < 0.002 {
        *visibility = Visibility::Hidden;
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
                border: UiRect::all(px(2)),
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
    time: Res<Time>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    camera: Query<(&Transform, &Projection), With<MainCamera>>,
    world: Res<WorldState>,
    mut selected: ResMut<SelectedCell>,
    mut species_ui: ResMut<SpeciesLedgerUiState>,
    chronicle_ui: Res<ChronicleUiState>,
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

    if species_ui.open
        && (cursor_over_species_ledger(window, cursor)
            || (species_ui.journal_open && cursor_over_species_journal(window, cursor)))
    {
        return;
    }
    if chronicle_ui.open && cursor_over_chronicle_panel(window, cursor) {
        return;
    }

    if selected.cell_id.is_some() {
        let panel_width = if ui_state.passport_open {
            (window.width() * 0.92).min(780.0)
        } else {
            510.0
        };
        let panel_height = if ui_state.passport_open { 520.0 } else { 650.0 };
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

    if let Some(index) = best {
        let cell_id = world.cells.id[index];
        let now = time.elapsed_secs_f64();
        let is_double =
            selected.last_click_cell_id == Some(cell_id) && now - selected.last_click_time <= 0.42;
        selected.cell_id = Some(cell_id);
        selected.last_click_cell_id = Some(cell_id);
        selected.last_click_time = now;
        if species_ui.open {
            let species = world.cells.species[index];
            species_ui.selected_species = Some(species);
            if is_double {
                species_ui.scroll_target_species = Some(species);
            }
        } else if is_double {
            let species = world.cells.species[index];
            species_ui.open = true;
            species_ui.selected_species = Some(species);
            species_ui.scroll_target_species = Some(species);
        }
    } else {
        selected.cell_id = None;
        species_ui.selected_species = None;
        species_ui.journal_open = false;
        species_ui.scroll_target_species = None;
        ui_state.passport_open = false;
    }
}

fn camera_controls(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    species_ui: Res<SpeciesLedgerUiState>,
    chronicle_ui: Res<ChronicleUiState>,
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

    let cursor_over_ui_scroll = window
        .cursor_position()
        .map(|cursor| {
            (species_ui.open
                && (cursor_over_species_ledger(window, cursor)
                    || (species_ui.journal_open && cursor_over_species_journal(window, cursor))))
                || (chronicle_ui.open && cursor_over_chronicle_panel(window, cursor))
        })
        .unwrap_or(false);

    if scroll != 0.0 && !cursor_over_ui_scroll {
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

fn trophic_type_name(aggressiveness: f32) -> &'static str {
    let aggression = (aggressiveness / CELL_AGGRESSIVENESS_DISPLAY_MAX).clamp(0.0, 1.0);
    if aggression < 0.40 {
        "Биотроф"
    } else if aggression < 0.70 {
        "Гемибиотроф"
    } else {
        "Некротроф"
    }
}

fn trophic_type_icon(aggressiveness: f32) -> &'static str {
    let aggression = (aggressiveness / CELL_AGGRESSIVENESS_DISPLAY_MAX).clamp(0.0, 1.0);
    if aggression < 0.40 {
        "sprites/gene-type-biotroph.png"
    } else if aggression < 0.70 {
        "sprites/gene-type-hemibiotroph.png"
    } else {
        "sprites/gene-type-necrotroph.png"
    }
}

fn game_ui_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedCell>,
    mut ui_state: ResMut<GameUiState>,
    mut species_ui: ResMut<SpeciesLedgerUiState>,
    mut chronicle_ui: ResMut<ChronicleUiState>,
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

    if keys.just_pressed(KeyCode::KeyQ) {
        species_ui.open = !species_ui.open;
        if !species_ui.open {
            species_ui.journal_open = false;
            species_ui.selected_species = None;
            species_ui.scroll_target_species = None;
            species_ui.last_click_species = None;
        }
    }

    if keys.just_pressed(KeyCode::KeyE) {
        if species_ui.open && species_ui.selected_species.is_some() {
            species_ui.journal_open = !species_ui.journal_open;
        } else {
            species_ui.journal_open = false;
        }
    }

    if keys.just_pressed(KeyCode::KeyC) {
        ui_state.speed_panel_open = !ui_state.speed_panel_open;
    }

    if keys.just_pressed(KeyCode::KeyH) {
        chronicle_ui.open = !chronicle_ui.open;
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
    time: Res<Time>,
    world: Res<WorldState>,
    stats: Res<FrameStats>,
    config: Res<SimConfig>,
    mut fps_average: ResMut<FpsAverageStats>,
    mut text: Query<&mut Text, With<StatsText>>,
    mut stats_spans: ParamSet<(
        Query<(&PopulationCountText, &mut TextSpan)>,
        Query<(&PopulationDeltaText, &mut TextSpan, &mut TextColor)>,
        Query<&mut TextSpan, With<StatsBodyText>>,
        Query<(&mut TextSpan, &mut TextColor), With<EnergyBalanceText>>,
        Query<&mut TextSpan, With<FpsAverageText>>,
        Query<(&mut TextSpan, &mut TextColor), With<FpsAverageDeltaText>>,
    )>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .unwrap_or(0.0);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|frame| frame.smoothed())
        .unwrap_or(0.0);

    fps_average.observe(time.delta_secs(), fps);

    let Ok(mut text) = text.single_mut() else {
        return;
    };
    **text = format!("FPS {fps:>6.1} | кадр {frame_ms:>5.2} мс");

    if let Ok(mut span) = stats_spans.p4().single_mut() {
        **span = format!("\nср. FPS {:>5.1}", fps_average.current_average);
    }
    if let Ok((mut span, mut color)) = stats_spans.p5().single_mut() {
        let delta = fps_average.delta;
        **span = format!(" {delta:+.1}");
        color.0 = if delta > 0.2 {
            Color::srgb(0.38, 1.0, 0.56)
        } else if delta < -0.2 {
            Color::srgb(1.0, 0.38, 0.34)
        } else {
            Color::srgb(0.66, 0.72, 0.76)
        };
    }

    for (counter, mut span) in &mut stats_spans.p0() {
        **span = match counter.0 {
            PopulationCounterKind::Cells => world.cells.len().to_string(),
            PopulationCounterKind::Food => world.food.active_count().to_string(),
        };
    }
    for (counter, mut span, mut color) in &mut stats_spans.p1() {
        let delta = match counter.0 {
            PopulationCounterKind::Cells => world.cell_count_delta,
            PopulationCounterKind::Food => world.food_count_delta,
        };
        **span = format!(" {delta:+}");
        color.0 = if delta > 0 {
            Color::srgb(0.38, 1.0, 0.56)
        } else if delta < 0 {
            Color::srgb(1.0, 0.38, 0.34)
        } else {
            Color::srgb(0.66, 0.72, 0.76)
        };
    }

    let flow = world.energy_flow;
    let (wild_food, feeder_food, carrion) = world.active_food_counts();
    if let Ok(mut body) = stats_spans.p2().single_mut() {
        **body = format!(
            "\nдикая {:>4} | корм. {:>4} | мясо {:>4}\nпрепят. {:>4} | кормушки {:>2}\nсим {:>5.2} мс | ренд {:>5.2} мс\nарена {:.0} x {:.0}\n\nЭНЕРГЕТИКА, ед/с\n+ дикая трава  {:>8.1}\n+ кормушки     {:>8.1}\n= общий приток {:>8.1}\n~ съедено      {:>8.1}\n~ падаль       {:>8.1}\n- метаболизм   {:>8.1}\n- порча        {:>8.1}\n- митоз        {:>8.1}\n- лизис        {:>8.1}\n= общий отток  {:>8.1}",
            wild_food,
            feeder_food,
            carrion,
            world.obstacles.len(),
            world.food_growers.len(),
            stats.sim_time.as_secs_f64() * 1_000.0,
            stats.upload_time.as_secs_f64() * 1_000.0,
            config.width,
            config.height,
            flow.wild_food_input,
            flow.feeder_input,
            flow.external_input(),
            flow.food_consumed,
            flow.carrion_transfer,
            flow.metabolism,
            flow.spoilage,
            flow.mitosis_cost,
            flow.lysis_loss,
            flow.total_outflow(),
        );
    }

    if let Ok((mut balance, mut color)) = stats_spans.p3().single_mut() {
        let net = flow.net_external_balance();
        let (state, target_color) = if net > 1.0 {
            ("ПЛЮС", Color::srgb(0.38, 1.0, 0.56))
        } else if net < -1.0 {
            ("МИНУС", Color::srgb(1.0, 0.38, 0.34))
        } else {
            ("БАЛАНС", Color::srgb(0.96, 0.80, 0.34))
        };
        **balance = format!("\nИТОГО {net:+8.1} ед/с  {state}");
        color.0 = target_color;
    }
}

fn active_food_energy(world: &WorldState) -> f32 {
    world
        .food
        .active
        .iter()
        .zip(&world.food.energy)
        .zip(&world.food.growth)
        .filter_map(|((active, energy), growth)| {
            (*active).then_some((*energy * (*growth).clamp(0.0, 1.0)).max(0.0))
        })
        .sum()
}

fn viability_summary(world: &WorldState) -> (f32, f32, usize, usize) {
    let mut total = 0.0;
    let mut capacity = 0.0;
    let mut low_25 = 0;
    let mut low_50 = 0;

    for (viability, max_viability) in world.cells.viability.iter().zip(&world.cells.max_viability) {
        let max_viability = (*max_viability).max(1.0);
        let viability = (*viability).max(0.0);
        let ratio = viability / max_viability;
        total += viability;
        capacity += max_viability;
        if ratio < 0.25 {
            low_25 += 1;
        }
        if ratio < 0.50 {
            low_50 += 1;
        }
    }

    (total, capacity, low_25, low_50)
}

fn eco_log_system(
    diagnostics: Res<DiagnosticsStore>,
    time: Res<Time>,
    config: Res<SimConfig>,
    ui_state: Res<GameUiState>,
    world: Res<WorldState>,
    stats: Res<FrameStats>,
    mut state: ResMut<EcoLogState>,
) {
    if !config.eco_log || ui_state.paused {
        return;
    }

    let (viability, viability_capacity, low_25, low_50) = viability_summary(&world);
    let food_energy = active_food_energy(&world);
    let cells = world.cells.len();
    let food = world.food.active_count();

    if !state.initialized {
        state.initialized = true;
        state.last_cells = cells;
        state.last_food = food;
        state.last_viability = viability;
        state.last_food_energy = food_energy;
        println!(
            "[eco] logging enabled: interval={:.1}s seed={} cells={} food={} arena={:.0}x{:.0}",
            config.eco_log_interval.clamp(0.5, 120.0),
            config.seed,
            config.cells,
            config.food,
            config.width,
            config.height,
        );
        return;
    }

    let frame_dt = time.delta_secs().clamp(0.0, 0.25);
    state.wall_elapsed += frame_dt;
    state.sim_elapsed += frame_dt * ui_state.speed_multiplier;
    let interval = config.eco_log_interval.clamp(0.5, 120.0);
    if state.wall_elapsed < interval {
        return;
    }

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .unwrap_or(0.0);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|frame| frame.smoothed())
        .unwrap_or(0.0);

    let delta_cells = cells as i64 - state.last_cells as i64;
    let delta_food = food as i64 - state.last_food as i64;
    let delta_viability = viability - state.last_viability;
    let delta_food_energy = food_energy - state.last_food_energy;
    let avg_viability = if viability_capacity > 0.0 {
        viability / viability_capacity * 100.0
    } else {
        0.0
    };
    let flow = world.energy_flow;
    let (wild_food, feeder_food, carrion) = world.active_food_counts();

    println!(
        "[eco {:>7.1}s] fps={:>5.1} frame={:>5.2}ms sim={:>5.2}ms render={:>5.2}ms | cells={}({:+}) food={}({:+}) wild={} feeder={} meat={} | viability={:.0}({:+.0}) avg={:.1}% low25={} low50={} | food_energy={:.0}({:+.0}) | in={:.1}/s out={:.1}/s net={:+.1}/s consumed={:.1}/s carrion={:.1}/s wild_in={:.1}/s feeder_in={:.1}/s meta={:.1}/s spoil={:.1}/s mitosis={:.1}/s lysis={:.1}/s",
        state.sim_elapsed,
        fps,
        frame_ms,
        stats.sim_time.as_secs_f64() * 1_000.0,
        stats.upload_time.as_secs_f64() * 1_000.0,
        cells,
        delta_cells,
        food,
        delta_food,
        wild_food,
        feeder_food,
        carrion,
        viability,
        delta_viability,
        avg_viability,
        low_25,
        low_50,
        food_energy,
        delta_food_energy,
        flow.external_input(),
        flow.total_outflow(),
        flow.net_external_balance(),
        flow.food_consumed,
        flow.carrion_transfer,
        flow.wild_food_input,
        flow.feeder_input,
        flow.metabolism,
        flow.spoilage,
        flow.mitosis_cost,
        flow.lysis_loss,
    );

    state.wall_elapsed = 0.0;
    state.last_cells = cells;
    state.last_food = food;
    state.last_viability = viability;
    state.last_food_energy = food_energy;
}

fn animate_panel_reveal(
    visibility: &mut Visibility,
    node: &mut Node,
    reveal: &mut PanelReveal,
    show: bool,
    dt: f32,
) {
    if show {
        *visibility = Visibility::Visible;
    }
    let target = if show { 1.0 } else { 0.0 };
    let follow = 1.0 - (-13.0 * dt).exp();
    reveal.progress += (target - reveal.progress) * follow;
    node.right = px(12.0 - reveal.hidden_offset * (1.0 - reveal.progress));
    if !show && reveal.progress < 0.002 {
        *visibility = Visibility::Hidden;
    }
}

fn update_selection_ui(
    time: Res<Time>,
    world: Res<WorldState>,
    mut selected: ResMut<SelectedCell>,
    ui_state: Res<GameUiState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut compact_panel: Query<
        (&mut Visibility, &mut Node, &mut PanelReveal),
        (
            With<SelectionPanel>,
            Without<PassportPanel>,
            Without<GeneBarFill>,
            Without<DivisionThresholdMarker>,
            Without<DivisionTooltip>,
        ),
    >,
    mut passport_panel: Query<
        (&mut Visibility, &mut Node, &mut PanelReveal),
        (
            With<PassportPanel>,
            Without<SelectionPanel>,
            Without<GeneBarFill>,
            Without<DivisionThresholdMarker>,
            Without<DivisionTooltip>,
        ),
    >,
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
    mut bar_fills: Query<
        (&GeneBarFill, &mut Node),
        (
            Without<DivisionThresholdMarker>,
            Without<SelectionPanel>,
            Without<PassportPanel>,
            Without<DivisionTooltip>,
        ),
    >,
    mut division_markers: Query<
        &mut Node,
        (
            With<DivisionThresholdMarker>,
            Without<GeneBarFill>,
            Without<SelectionPanel>,
            Without<PassportPanel>,
            Without<DivisionTooltip>,
        ),
    >,
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
    if let Ok((mut visibility, mut node, mut reveal)) = compact_panel.single_mut() {
        animate_panel_reveal(
            &mut visibility,
            &mut node,
            &mut reveal,
            has_selection && !ui_state.passport_open,
            time.delta_secs(),
        );
    }
    if let Ok((mut visibility, mut node, mut reveal)) = passport_panel.single_mut() {
        animate_panel_reveal(
            &mut visibility,
            &mut node,
            &mut reveal,
            has_selection && ui_state.passport_open,
            time.delta_secs(),
        );
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
        let target = value.normalized.clamp(0.0, 1.0) * 100.0;
        let current = match node.width {
            Val::Percent(value) => value,
            _ => 0.0,
        };
        let follow = 1.0 - (-10.0 * time.delta_secs()).exp();
        node.width = percent(current + (target - current) * follow);
    }

    for mut marker in &mut division_markers {
        let target =
            (division_threshold / CELL_DIVISION_THRESHOLD_DISPLAY_MAX).clamp(0.0, 1.0) * 100.0;
        let current = match marker.left {
            Val::Percent(value) => value,
            _ => target,
        };
        let follow = 1.0 - (-12.0 * time.delta_secs()).exp();
        marker.left = percent(current + (target - current) * follow);
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
            let position =
                tooltip_position_near_cursor(window, cursor, tooltip_width, tooltip_height, gap);
            tooltip_node.left = px(position.x);
            tooltip_node.top = px(position.y);
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

fn update_selected_species_titles(
    world: Res<WorldState>,
    selected: Res<SelectedCell>,
    names: Res<SpeciesNameBook>,
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
) {
    let Some(cell_index) = selected
        .cell_id
        .and_then(|cell_id| world.cell_index_by_id(cell_id))
    else {
        return;
    };
    let cell_id = world.cells.id[cell_index];
    let shape_name = world.cells.shape_name(cell_index);
    let species_name = species_name_for(&names, world.cells.species[cell_index]);
    if let Ok(mut title) = compact_title.single_mut() {
        **title = format!("КЛЕТКА #{cell_id} · {shape_name} · {species_name}");
    }
    if let Ok(mut title) = passport_title.single_mut() {
        **title = format!("ПАСПОРТ КЛЕТКИ #{cell_id} · {shape_name} · {species_name}");
    }
}
fn stat_ui_value(world: &WorldState, cell_index: usize, id: GeneStatId) -> StatUiValue {
    let cells = &world.cells;
    let viability = cells.viability[cell_index];
    let max_viability = cells.max_viability[cell_index].max(1.0);
    let viability_ratio = (viability / max_viability).clamp(0.0, 1.0);
    let radius = cells.radius[cell_index].max(0.1);
    let membrane_size = cells.max_base_radius(cell_index).max(0.1);
    let speed_factor = cells.morphology_speed_factor(cell_index);
    let acceleration_factor = cells.morphology_acceleration_factor(cell_index);
    let turn_factor = cells.morphology_turn_factor(cell_index);
    let viability_factor = cells.morphology_viability_factor(cell_index);
    let metabolism_factor = cells.morphology_metabolism_factor(cell_index);
    let modifier = |factor: f32| format!("{:+.0}%", (factor - 1.0) * 100.0);
    let gene_range = |min: f32, max: f32| format!("{min:.0}-{max:.0}");

    match id {
        GeneStatId::Viability => StatUiValue {
            normalized: viability_ratio,
            display: format!(
                "{viability:.0} / {:.0} -> {max_viability:.0}",
                CELL_VIABILITY_MAX
            ),
            range: format!(
                "Эффект: запас формы {} | метаболизм {}",
                modifier(viability_factor),
                modifier(metabolism_factor)
            ),
        },
        GeneStatId::Speed => {
            let speed = cells.speed[cell_index];
            StatUiValue {
                normalized: speed / CELL_SPEED_DISPLAY_MAX,
                display: format!("{speed:.0} -> {:.0}", speed * speed_factor),
                range: format!(
                    "Ген: {} | форма {} | разгон {}",
                    gene_range(SPEED_GENE_MIN, SPEED_GENE_MAX),
                    modifier(speed_factor),
                    modifier(acceleration_factor)
                ),
            }
        }
        GeneStatId::Turn => {
            let turn = cells.turn_speed[cell_index];
            StatUiValue {
                normalized: turn / CELL_TURN_DISPLAY_MAX,
                display: format!("{turn:.1} -> {:.1}", turn * turn_factor),
                range: format!(
                    "Ген: {TURN_GENE_MIN:.2}-{TURN_GENE_MAX:.1} | форма {}",
                    modifier(turn_factor)
                ),
            }
        }
        GeneStatId::Perception => {
            let perception = cells.perception[cell_index];
            StatUiValue {
                normalized: perception / CELL_PERCEPTION_DISPLAY_MAX,
                display: format!("{perception:.0}"),
                range: format!(
                    "Ген: {} | радиус обзора",
                    gene_range(PERCEPTION_GENE_MIN, PERCEPTION_GENE_MAX)
                ),
            }
        }
        GeneStatId::Persistence => {
            let persistence = cells.persistence[cell_index];
            StatUiValue {
                normalized: persistence / CELL_PERSISTENCE_DISPLAY_MAX,
                display: format!("{persistence:.0}%"),
                range: "Ген: 0-100% | удержание цели".to_string(),
            }
        }
        GeneStatId::Aggressiveness => {
            let aggressiveness = cells.aggressiveness[cell_index];
            StatUiValue {
                normalized: aggressiveness / CELL_AGGRESSIVENESS_DISPLAY_MAX,
                display: format!("{aggressiveness:.0}%"),
                range: "Ген: 0-100% | охота | сдвиг рациона к мясу".to_string(),
            }
        }
        GeneStatId::Diet => {
            let aggressiveness = cells.aggressiveness[cell_index];
            let grass_multiplier = grass_energy_multiplier(aggressiveness);
            let meat_multiplier = meat_energy_multiplier(aggressiveness);
            StatUiValue {
                normalized: aggressiveness / CELL_AGGRESSIVENESS_DISPLAY_MAX,
                display: trophic_type_name(aggressiveness).to_string(),
                range: format!(
                    "Троф: {} | трава x{grass_multiplier:.2} | мясо x{meat_multiplier:.2}",
                    trophic_type_name(aggressiveness)
                ),
            }
        }
        GeneStatId::Lysis => {
            let lysis = cells.lysis[cell_index];
            let (damage, self_cost, cooldown, reach) = lysis_combat_profile(lysis);
            StatUiValue {
                normalized: lysis / CELL_LYSIS_DISPLAY_MAX,
                display: if lysis < 8.0 {
                    "нет".to_string()
                } else {
                    format!("{lysis:.0}%")
                },
                range: format!(
                    "Урон {damage:.1} | цена {self_cost:.2} | КД {cooldown:.2}с | радиус {reach:.1}"
                ),
            }
        }
        GeneStatId::Mutation => {
            let mutation = cells.mutation_susceptibility[cell_index];
            StatUiValue {
                normalized: mutation / CELL_MUTATION_DISPLAY_MAX,
                display: format!("{mutation:.0}%"),
                range: "Ген: 0-100% | наследственная изменчивость".to_string(),
            }
        }
        GeneStatId::Size => StatUiValue {
            normalized: ((membrane_size - CELL_SIZE_GENE_MIN)
                / (CELL_SIZE_GENE_MAX - CELL_SIZE_GENE_MIN))
                .clamp(0.0, 1.0),
            display: format!("{membrane_size:.1}/{radius:.1}"),
            range: format!("Ген: {CELL_SIZE_GENE_MIN:.1}-{CELL_SIZE_GENE_MAX:.1} | тело и ядро"),
        },
    }
}
fn gene_tooltip_copy(kind: GeneStatId) -> (&'static str, &'static str) {
    match kind {
        GeneStatId::Viability => (
            "ЖИЗНЕСПОСОБНОСТЬ",
            "Общий запас энергии и здоровья. Восполняется пищей, тратится в покое; геометрия формы меняет итоговый запас.",
        ),
        GeneStatId::Size => (
            "РАЗМЕР",
            "Физический масштаб тела. Крупная клетка устойчивее в столкновениях и хранит больше запаса, но требует больше энергии.",
        ),
        GeneStatId::Speed => (
            "СКОРОСТЬ",
            "Предельная скорость движения вперед. Форма тела меняет реальный результат, а быстрые клетки платят повышенным метаболизмом.",
        ),
        GeneStatId::Turn => (
            "ПОВОРОТЛИВОСТЬ",
            "Скорость изменения направления головы. Геометрия и многосегментность могут снижать реальную маневренность.",
        ),
        GeneStatId::Perception => (
            "ВОСПРИЯТИЕ",
            "Радиус обнаружения еды и других целей. Дальние объекты остаются невидимыми, пока не попадут в этот радиус.",
        ),
        GeneStatId::Persistence => (
            "НАСТОЙЧИВОСТЬ",
            "Определяет верность выбранной цели, частоту ее смены и длительность преследования последней известной позиции.",
        ),
        GeneStatId::Aggressiveness => (
            "АГРЕССИВНОСТЬ",
            "Желание самостоятельно начинать охоту. Также сдвигает питание: высокая агрессивность улучшает усвоение мяса, но ухудшает усвоение травы.",
        ),
        GeneStatId::Diet => (
            "РАЦИОН",
            "Пищевой профиль по агрессивности: биотроф лучше ест траву, гемибиотроф держит середину, некротроф делает ставку на мясо.",
        ),
        GeneStatId::Lysis => (
            "ЛИЗИС",
            "Контактная атака по жизнеспособности жертвы. Развитие повышает урон и частоту ударов, но имеет цену.",
        ),
        GeneStatId::Mutation => (
            "МУТАЦИИ",
            "Вероятность и сила наследственных изменений при делении: гены, лучи формы и топология сегментов.",
        ),
    }
}

fn species_journal_tooltip_copy(
    kind: SpeciesJournalTooltipKind,
    snapshot: &SpeciesSnapshot,
    stats: &SpeciesLedgerStats,
    names: &SpeciesNameBook,
) -> (String, String, Color) {
    match kind {
        SpeciesJournalTooltipKind::Portrait => {
            let name = species_name_for(names, snapshot.species);
            let shape = species_shape_label_from_id(snapshot.species);
            let section_text = if snapshot.display_section_count <= 1 {
                "одна секция".to_string()
            } else {
                format!(
                    "{} секции, spacing x{:.2}",
                    snapshot.display_section_count, snapshot.display_section_spacing
                )
            };
            (
                "ПОРТРЕТ ВИДА".to_string(),
                format!(
                    "{name}\nФорма: {shape}; {section_text}. Портрет строится из формы живого представителя: лучи, угловые смещения, размеры сегментов и связи между ними."
                ),
                Color::srgb(0.42, 0.86, 0.92),
            )
        }
        SpeciesJournalTooltipKind::Diet => {
            let diet = trophic_type_name(snapshot.average_aggressiveness);
            let grass = grass_energy_multiplier(snapshot.average_aggressiveness);
            let meat = meat_energy_multiplier(snapshot.average_aggressiveness);
            (
                "РАЦИОН ВИДА".to_string(),
                format!(
                    "{diet}: трава x{grass:.2}, мясо x{meat:.2}. Рацион выводится из средней агрессивности вида, а не задается отдельным типом."
                ),
                gene_stat_color(GeneStatId::Diet),
            )
        }
        SpeciesJournalTooltipKind::Metric(metric) => {
            let max_alive = stats
                .snapshots
                .iter()
                .map(|candidate| candidate.alive)
                .max()
                .unwrap_or(snapshot.alive);
            let (_, display, color) = species_journal_metric_sample(snapshot, max_alive, metric);
            let description = match metric {
                SpeciesJournalMetric::Population => {
                    "Количество живых особей этого вида. Изменение рядом показывает разницу с прошлым обновлением реестра."
                }
                SpeciesJournalMetric::Viability => {
                    "Средний процент текущей жизнеспособности: общий запас энергии и здоровья особей вида."
                }
                SpeciesJournalMetric::Size => {
                    "Средний физический размер тела. Размер влияет на массу, метаболизм, столкновения и уязвимость к крупным хищникам."
                }
                SpeciesJournalMetric::Speed => {
                    "Средняя генетическая скорость вида до геометрических штрафов и бонусов формы."
                }
                SpeciesJournalMetric::Turn => {
                    "Средняя поворотливость головы. Длинные и многосегментные тела обычно хуже разворачиваются."
                }
                SpeciesJournalMetric::Perception => {
                    "Средний радиус видимости. Цели за пределами восприятия для клеток фактически не существуют."
                }
                SpeciesJournalMetric::Persistence => {
                    "Средняя настойчивость: насколько долго клетки держатся выбранной цели и последней известной позиции."
                }
                SpeciesJournalMetric::Aggressiveness => {
                    "Средняя агрессивность. Она повышает склонность к охоте и сдвигает усвоение энергии от травы к мясу."
                }
                SpeciesJournalMetric::Lysis => {
                    "Средний уровень контактной атаки. Лизис нужен для ближнего урона по жизнеспособности жертвы."
                }
                SpeciesJournalMetric::Mutation => {
                    "Средняя наследственная изменчивость: сила и частота изменений генов, лучей формы и сегментов при делении."
                }
            };
            (
                species_journal_metric_label(metric).to_uppercase(),
                format!("{display}\n{description}"),
                color,
            )
        }
    }
}

fn chronicle_legend_line_copy(line: ChronicleLegendLine) -> (&'static str, &'static str, Color) {
    match line {
        ChronicleLegendLine::Cells => (
            "КЛЕТКИ",
            "Зелёная линия показывает численность живых клеток в последних срезах хроники.",
            Color::srgb(0.34, 1.0, 0.52),
        ),
        ChronicleLegendLine::Food => (
            "ЕДА",
            "Жёлтая линия показывает общее количество активной еды: дикая трава, кормушки и мясо.",
            Color::srgb(1.0, 0.86, 0.30),
        ),
        ChronicleLegendLine::Viability => (
            "ЖИЗНЕСПОСОБНОСТЬ",
            "Белая линия показывает средний процент жизнеспособности популяции: общий запас энергии и здоровья.",
            Color::srgb(0.90, 1.0, 0.94),
        ),
        ChronicleLegendLine::EnergyPositive => (
            "БАЛАНС ЭНЕРГИИ: ПЛЮС",
            "Зелёный участок баланса означает, что внешний приток еды и энергии выше расходов экосистемы.",
            Color::srgb(0.40, 1.0, 0.66),
        ),
        ChronicleLegendLine::EnergyNegative => (
            "БАЛАНС ЭНЕРГИИ: МИНУС",
            "Красный участок баланса означает, что расходы экосистемы выше внешнего притока.",
            Color::srgb(1.0, 0.36, 0.31),
        ),
    }
}

fn chronicle_tooltip_copy(
    kind: ChronicleTooltipKind,
    chronicle: &SimulationChronicle,
    state: &ChronicleUiState,
) -> (String, String, Color, f32, f32) {
    match kind {
        ChronicleTooltipKind::Summary(metric) => {
            let label = chronicle_summary_label(metric).to_uppercase();
            let accent = chronicle_summary_color(metric);
            let current = chronicle.snapshots.last().map(|snapshot| {
                let (value, _) = chronicle_summary_value(metric, snapshot);
                format!("\nТекущее значение: {value}.")
            });
            let description = match metric {
                ChronicleSummaryMetric::Time => {
                    "Время симуляции с момента запуска текущей экосистемы."
                }
                ChronicleSummaryMetric::Cells => {
                    "Количество живых клеток прямо сейчас. Рост и падение популяции попадают в события хроники."
                }
                ChronicleSummaryMetric::Species => {
                    "Количество живых видов, распознанных по форме, сегментам, трофике и ключевым генам."
                }
                ChronicleSummaryMetric::Food => {
                    "Общий запас еды и расклад внутри него: дикая трава, еда кормушек и мясо."
                }
                ChronicleSummaryMetric::Viability => {
                    "Средняя жизнеспособность всех клеток: энергия и здоровье в одной шкале."
                }
                ChronicleSummaryMetric::Energy => {
                    "Внешний баланс энергии: приток минус расход. Плюс означает избыток, минус означает истощение экосистемы."
                }
                ChronicleSummaryMetric::Costs => {
                    "Крупные статьи расхода: метаболизм, деление и потери от лизиса."
                }
                ChronicleSummaryMetric::Traits => {
                    "Быстрый срез важных признаков: сколько клеток многосегментные и сколько способны к лизису."
                }
                ChronicleSummaryMetric::Performance => {
                    "Производительность последнего среза: FPS, время симуляции и время выгрузки рендера."
                }
            };
            (
                label,
                format!("{description}{}", current.unwrap_or_default()),
                accent,
                440.0,
                166.0,
            )
        }
        ChronicleTooltipKind::Filter(kind) => {
            let label = chronicle_filter_label(kind).to_uppercase();
            let accent = chronicle_kind_color(kind);
            let enabled = if state.event_enabled(kind) {
                "включён"
            } else {
                "выключен"
            };
            let count = chronicle
                .events
                .iter()
                .filter(|event| event.kind == kind)
                .count();
            let description = match kind {
                ChronicleEventKind::World => {
                    "События общего состояния мира: запуск экосистемы и глобальные изменения."
                }
                ChronicleEventKind::Species => {
                    "Появление новых видов, когда форма и ключевые гены уже не укладываются в старый вид."
                }
                ChronicleEventKind::Extinction => {
                    "Вымирания видов. Их можно выключить фильтром, не удаляя из истории."
                }
                ChronicleEventKind::Population => {
                    "Доминанты, всплески и обвалы численности популяции."
                }
                ChronicleEventKind::Energy => {
                    "Переходы энергетики между профицитом, дефицитом и балансом."
                }
                ChronicleEventKind::Trait => {
                    "Появление заметных признаков вроде лизиса или многосегментности."
                }
            };
            (
                format!("ФИЛЬТР: {label}"),
                format!(
                    "{description}\nСейчас фильтр {enabled}; событий этого типа в памяти: {count}."
                ),
                accent,
                430.0,
                176.0,
            )
        }
        ChronicleTooltipKind::Graph => {
            let samples = chronicle.snapshots.len();
            (
                "ГРАФИК СРЕЗОВ".to_string(),
                format!(
                    "График показывает последние {samples} срезов мира. Линии нормализованы каждая по своему диапазону, поэтому форма линии важнее абсолютной высоты. Срезы нужны для быстрого чтения трендов: рост, спад, дефицит и восстановление."
                ),
                Color::srgb(0.42, 0.86, 0.92),
                480.0,
                188.0,
            )
        }
        ChronicleTooltipKind::GraphMode(mode) => {
            let accent = chronicle_graph_mode_color(mode);
            let description = match mode {
                ChronicleGraphMode::Overall => {
                    "Общий режим рисует все основные линии вместе: клетки, еду, среднюю жизнеспособность и энергетический баланс."
                }
                ChronicleGraphMode::Cells => {
                    "Отдельный график численности клеток. Удобен, когда общий график слишком шумный."
                }
                ChronicleGraphMode::Food => {
                    "Отдельный график общего количества еды: трава, кормушки и мясо суммарно."
                }
                ChronicleGraphMode::Viability => {
                    "Отдельный график средней жизнеспособности популяции."
                }
                ChronicleGraphMode::Energy => {
                    "Отдельный график внешнего баланса энергии. Нулевая линия показывает границу между плюсом и минусом."
                }
            };
            (
                format!(
                    "ГРАФИК: {}",
                    chronicle_graph_mode_label(mode).to_uppercase()
                ),
                description.to_string(),
                accent,
                430.0,
                158.0,
            )
        }
        ChronicleTooltipKind::Legend(line) => {
            let (heading, description, accent) = chronicle_legend_line_copy(line);
            (
                heading.to_string(),
                description.to_string(),
                accent,
                430.0,
                154.0,
            )
        }
    }
}

fn update_gene_tooltip(
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    selected: Res<SelectedCell>,
    world: Res<WorldState>,
    chronicle_state: Res<ChronicleUiState>,
    chronicle: Res<SimulationChronicle>,
    species_state: Res<SpeciesLedgerUiState>,
    species_stats: Res<SpeciesLedgerStats>,
    species_names: Res<SpeciesNameBook>,
    targets: Query<(&Interaction, &GeneTooltipTarget)>,
    journal_targets: Query<(&Interaction, &SpeciesJournalTooltipTarget)>,
    chronicle_targets: Query<(&Interaction, &ChronicleTooltipTarget)>,
    division_markers: Query<&Interaction, With<DivisionThresholdMarker>>,
    mut tooltip: Query<(
        &mut Visibility,
        &mut Node,
        &mut UiTransform,
        &mut GeneTooltip,
        &mut BorderColor,
    )>,
    mut tooltip_text: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<GeneTooltipTitle>>,
        Query<(&mut Text, &mut TextColor), With<GeneTooltipValue>>,
        Query<&mut Text, With<GeneTooltipBody>>,
    )>,
) {
    let marker_hovered = division_markers
        .iter()
        .any(|interaction| *interaction != Interaction::None);
    let gene_hovered = (!marker_hovered)
        .then(|| {
            targets
                .iter()
                .find(|(interaction, _)| **interaction != Interaction::None)
                .map(|(_, target)| target.kind)
        })
        .flatten();
    let journal_hovered = (!marker_hovered && gene_hovered.is_none())
        .then(|| {
            journal_targets
                .iter()
                .find(|(interaction, _)| **interaction != Interaction::None)
                .map(|(_, target)| target.kind)
        })
        .flatten();
    let chronicle_hovered =
        (!marker_hovered && gene_hovered.is_none() && journal_hovered.is_none())
            .then(|| {
                chronicle_targets
                    .iter()
                    .find(|(interaction, _)| **interaction != Interaction::None)
                    .map(|(_, target)| target.kind)
            })
            .flatten();
    let selected_index = selected
        .cell_id
        .and_then(|cell_id| world.cell_index_by_id(cell_id));
    let selected_snapshot = (species_state.open && species_state.journal_open)
        .then_some(())
        .and_then(|_| {
            species_state
                .selected_species
                .and_then(|species| species_snapshot_by_id(&species_stats, species))
        });
    let gene_payload = gene_hovered.and_then(|kind| {
        selected_index.map(|_| {
            let (heading, description) = gene_tooltip_copy(kind);
            (
                heading.to_string(),
                description.to_string(),
                gene_stat_color(kind),
                430.0,
                184.0,
            )
        })
    });
    let journal_payload = journal_hovered.and_then(|kind| {
        selected_snapshot.map(|snapshot| {
            let (heading, description, accent) =
                species_journal_tooltip_copy(kind, snapshot, &species_stats, &species_names);
            (heading, description, accent, 460.0, 196.0)
        })
    });
    let chronicle_payload = (chronicle_state.open)
        .then(|| {
            chronicle_hovered.map(|kind| chronicle_tooltip_copy(kind, &chronicle, &chronicle_state))
        })
        .flatten();
    let payload = gene_payload.or(journal_payload).or(chronicle_payload);
    let show = payload.is_some();
    let Ok((mut visibility, mut node, mut transform, mut reveal, mut border_color)) =
        tooltip.single_mut()
    else {
        return;
    };
    if show {
        *visibility = Visibility::Visible;
    }
    let target_reveal = if show { 1.0 } else { 0.0 };
    let follow = 1.0 - (-18.0 * time.delta_secs()).exp();
    reveal.reveal += (target_reveal - reveal.reveal) * follow;
    transform.scale = Vec2::splat(0.96 + reveal.reveal * 0.04);
    if !show && reveal.reveal < 0.01 {
        *visibility = Visibility::Hidden;
        return;
    }

    let Some((heading, description, accent, width, height)) = payload else {
        return;
    };
    *border_color = BorderColor::all(accent);
    if let Ok((mut text, mut color)) = tooltip_text.p0().single_mut() {
        **text = heading;
        *color = TextColor(accent);
    }
    if let Ok((mut text, mut color)) = tooltip_text.p1().single_mut() {
        **text = String::new();
        *color = TextColor(accent);
    }
    if let Ok(mut text) = tooltip_text.p2().single_mut() {
        **text = description;
    }

    if let Ok(window) = windows.single()
        && let Some(cursor) = window.cursor_position()
    {
        let gap = 18.0;
        let position = tooltip_position_near_cursor(window, cursor, width, height, gap);
        node.width = px(width);
        node.left = px(position.x);
        node.top = px(position.y);
    }
}

fn species_ledger_button_system(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SpeciesLedgerButton>)>,
    mut state: ResMut<SpeciesLedgerUiState>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            state.open = !state.open;
            if !state.open {
                state.journal_open = false;
                state.selected_species = None;
                state.scroll_target_species = None;
                state.last_click_species = None;
            }
        }
    }
}

fn chronicle_button_system(
    interactions: Query<&Interaction, (Changed<Interaction>, With<ChronicleButton>)>,
    mut state: ResMut<ChronicleUiState>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            state.open = !state.open;
        }
    }
}

fn chronicle_filter_button_system(
    interactions: Query<(&Interaction, &ChronicleFilterButton), Changed<Interaction>>,
    mut state: ResMut<ChronicleUiState>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            state.toggle_event_filter(button.kind);
        }
    }
}

fn chronicle_graph_button_system(
    interactions: Query<(&Interaction, &ChronicleGraphButton), Changed<Interaction>>,
    mut state: ResMut<ChronicleUiState>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            state.graph_mode = button.mode;
        }
    }
}

fn update_chronicle_filter_button_styles(
    state: Res<ChronicleUiState>,
    mut filter_buttons: Query<(
        &ChronicleFilterButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut graph_buttons: Query<
        (
            &ChronicleGraphButton,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Without<ChronicleFilterButton>,
    >,
) {
    if !state.open {
        return;
    }

    for (button, interaction, mut background, mut border) in &mut filter_buttons {
        let enabled = state.event_enabled(button.kind);
        let accent = chronicle_kind_color(button.kind);
        *border = BorderColor::all(if enabled {
            accent
        } else {
            Color::srgb(0.15, 0.24, 0.27)
        });
        background.0 = if enabled {
            match *interaction {
                Interaction::Pressed => Color::srgb(0.13, 0.23, 0.25),
                Interaction::Hovered => Color::srgb(0.08, 0.15, 0.17),
                Interaction::None => Color::srgb(0.030, 0.060, 0.068),
            }
        } else {
            match *interaction {
                Interaction::Pressed | Interaction::Hovered => Color::srgb(0.045, 0.052, 0.056),
                Interaction::None => Color::srgb(0.020, 0.026, 0.030),
            }
        };
    }

    for (button, interaction, mut background, mut border) in &mut graph_buttons {
        let selected = state.graph_mode == button.mode;
        let accent = chronicle_graph_mode_color(button.mode);
        *border = BorderColor::all(if selected {
            accent
        } else {
            Color::srgb(0.16, 0.33, 0.37)
        });
        background.0 = if selected {
            match *interaction {
                Interaction::Pressed => Color::srgb(0.13, 0.23, 0.25),
                Interaction::Hovered => Color::srgb(0.10, 0.18, 0.20),
                Interaction::None => Color::srgb(0.050, 0.092, 0.102),
            }
        } else {
            match *interaction {
                Interaction::Pressed => Color::srgb(0.08, 0.13, 0.15),
                Interaction::Hovered => Color::srgb(0.055, 0.086, 0.096),
                Interaction::None => Color::srgb(0.025, 0.040, 0.047),
            }
        };
    }
}

fn chronicle_event_scroll_system(
    time: Res<Time>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Res<ChronicleUiState>,
    mut scroll_state: ResMut<ChronicleEventScrollState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scroll_area: Query<(&ComputedNode, &mut ScrollPosition), With<ChronicleEventScrollArea>>,
    mut scrollbar_tracks: Query<&mut Visibility, With<ChronicleEventScrollbarTrack>>,
    mut scrollbar_thumbs: Query<&mut Node, With<ChronicleEventScrollbarThumb>>,
) {
    if !state.open {
        scroll_state.scrollbar_dragging = false;
        scroll_state.initialized = false;
        scroll_state.target_y = 0.0;
        for mut visibility in &mut scrollbar_tracks {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let window = windows.single().ok();
    let cursor = window.and_then(Window::cursor_position);
    let cursor_over_events = window
        .zip(cursor)
        .map(|(window, cursor)| cursor_over_chronicle_events(window, cursor))
        .unwrap_or(false);
    if mouse.just_pressed(MouseButton::Left) {
        scroll_state.scrollbar_dragging = window
            .zip(cursor)
            .and_then(|(window, cursor)| chronicle_event_scrollbar_fraction(window, cursor))
            .is_some();
    }
    if !mouse.pressed(MouseButton::Left) {
        scroll_state.scrollbar_dragging = false;
    }

    let mut delta = 0.0;
    for event in mouse_wheel.read() {
        if !cursor_over_events {
            continue;
        }
        let scale = match event.unit {
            MouseScrollUnit::Line => CHRONICLE_EVENT_WHEEL_LINE_SCROLL,
            MouseScrollUnit::Pixel => CHRONICLE_EVENT_WHEEL_PIXEL_SCROLL,
        };
        delta -= event.y * scale;
    }

    for (computed, mut scroll_position) in &mut scroll_area {
        let content_height = (computed.content_size().y * computed.inverse_scale_factor()).max(1.0);
        let view_height = (computed.size().y * computed.inverse_scale_factor()).max(1.0);
        let max_offset = (content_height - view_height).max(0.0);
        if !scroll_state.initialized {
            scroll_state.target_y = scroll_position.y.clamp(0.0, max_offset);
            scroll_state.initialized = true;
        }
        scroll_state.target_y = scroll_state.target_y.clamp(0.0, max_offset);

        let dragged_fraction = if scroll_state.scrollbar_dragging {
            window
                .zip(cursor)
                .and_then(|(window, cursor)| chronicle_event_scrollbar_fraction(window, cursor))
        } else {
            None
        };
        let mut follow_rate = CHRONICLE_EVENT_SCROLL_FOLLOW;
        if let Some(fraction) = dragged_fraction {
            scroll_state.target_y = (fraction * max_offset).clamp(0.0, max_offset);
            follow_rate = CHRONICLE_EVENT_SCROLLBAR_FOLLOW;
        } else if delta != 0.0 {
            scroll_state.target_y = (scroll_state.target_y + delta).clamp(0.0, max_offset);
        }

        let follow = 1.0 - (-follow_rate * time.delta_secs()).exp();
        scroll_position.y += (scroll_state.target_y - scroll_position.y) * follow;
        if (scroll_position.y - scroll_state.target_y).abs() < 0.35 {
            scroll_position.y = scroll_state.target_y;
        }
        scroll_position.y = scroll_position.y.clamp(0.0, max_offset);

        let visible_ratio = (view_height / content_height).clamp(0.0, 1.0);
        let thumb_height = (visible_ratio * 100.0).clamp(8.0, 100.0);
        let thumb_top = if max_offset > 1.0 {
            (scroll_position.y / max_offset).clamp(0.0, 1.0) * (100.0 - thumb_height)
        } else {
            0.0
        };
        for mut visibility in &mut scrollbar_tracks {
            *visibility = if max_offset > 1.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        for mut thumb in &mut scrollbar_thumbs {
            thumb.top = percent(thumb_top);
            thumb.height = percent(thumb_height);
        }
    }
}

fn chronicle_push_event(
    chronicle: &mut SimulationChronicle,
    kind: ChronicleEventKind,
    title: impl Into<String>,
    body: impl Into<String>,
    species: Option<u32>,
) {
    if chronicle.events.len() >= CHRONICLE_MAX_EVENTS {
        chronicle.events.remove(0);
    }
    chronicle.events.push(ChronicleEvent {
        time: chronicle.elapsed,
        kind,
        title: title.into(),
        body: body.into(),
        species,
    });
    chronicle.revision = chronicle.revision.wrapping_add(1);
}

fn chronicle_filter_bit(kind: ChronicleEventKind) -> u8 {
    match kind {
        ChronicleEventKind::World => CHRONICLE_FILTER_WORLD,
        ChronicleEventKind::Species => CHRONICLE_FILTER_SPECIES,
        ChronicleEventKind::Extinction => CHRONICLE_FILTER_EXTINCTION,
        ChronicleEventKind::Population => CHRONICLE_FILTER_POPULATION,
        ChronicleEventKind::Energy => CHRONICLE_FILTER_ENERGY,
        ChronicleEventKind::Trait => CHRONICLE_FILTER_TRAIT,
    }
}

fn chronicle_filter_label(kind: ChronicleEventKind) -> &'static str {
    match kind {
        ChronicleEventKind::World => "мир",
        ChronicleEventKind::Species => "виды",
        ChronicleEventKind::Extinction => "смерть",
        ChronicleEventKind::Population => "поп.",
        ChronicleEventKind::Energy => "энерг.",
        ChronicleEventKind::Trait => "гены",
    }
}

fn chronicle_kind_color(kind: ChronicleEventKind) -> Color {
    match kind {
        ChronicleEventKind::World => Color::srgb(0.72, 0.90, 0.92),
        ChronicleEventKind::Species => Color::srgb(0.44, 1.0, 0.62),
        ChronicleEventKind::Extinction => Color::srgb(1.0, 0.34, 0.32),
        ChronicleEventKind::Population => Color::srgb(0.48, 0.78, 1.0),
        ChronicleEventKind::Energy => Color::srgb(1.0, 0.84, 0.32),
        ChronicleEventKind::Trait => Color::srgb(0.82, 0.56, 1.0),
    }
}

fn chronicle_graph_mode_label(mode: ChronicleGraphMode) -> &'static str {
    match mode {
        ChronicleGraphMode::Overall => "общий",
        ChronicleGraphMode::Cells => "клетки",
        ChronicleGraphMode::Food => "еда",
        ChronicleGraphMode::Viability => "жизнь",
        ChronicleGraphMode::Energy => "энергия",
    }
}

fn chronicle_graph_mode_color(mode: ChronicleGraphMode) -> Color {
    match mode {
        ChronicleGraphMode::Overall => Color::srgb(0.42, 0.86, 0.92),
        ChronicleGraphMode::Cells => Color::srgb(0.34, 1.0, 0.52),
        ChronicleGraphMode::Food => Color::srgb(1.0, 0.86, 0.30),
        ChronicleGraphMode::Viability => Color::srgb(0.90, 1.0, 0.94),
        ChronicleGraphMode::Energy => Color::srgb(0.40, 1.0, 0.66),
    }
}

fn chronicle_summary_label(metric: ChronicleSummaryMetric) -> &'static str {
    match metric {
        ChronicleSummaryMetric::Time => "время",
        ChronicleSummaryMetric::Cells => "клетки",
        ChronicleSummaryMetric::Species => "виды",
        ChronicleSummaryMetric::Food => "еда",
        ChronicleSummaryMetric::Viability => "жизнь",
        ChronicleSummaryMetric::Energy => "энергия",
        ChronicleSummaryMetric::Costs => "расходы",
        ChronicleSummaryMetric::Traits => "признаки",
        ChronicleSummaryMetric::Performance => "кадр",
    }
}

fn chronicle_summary_width(metric: ChronicleSummaryMetric) -> f32 {
    match metric {
        ChronicleSummaryMetric::Time => 66.0,
        ChronicleSummaryMetric::Cells => 76.0,
        ChronicleSummaryMetric::Species => 68.0,
        ChronicleSummaryMetric::Food => 132.0,
        ChronicleSummaryMetric::Viability => 72.0,
        ChronicleSummaryMetric::Energy => 108.0,
        ChronicleSummaryMetric::Costs => 132.0,
        ChronicleSummaryMetric::Traits => 110.0,
        ChronicleSummaryMetric::Performance => 116.0,
    }
}

fn chronicle_summary_color(metric: ChronicleSummaryMetric) -> Color {
    match metric {
        ChronicleSummaryMetric::Time => Color::srgb(0.55, 0.86, 0.92),
        ChronicleSummaryMetric::Cells => Color::srgb(0.34, 1.0, 0.52),
        ChronicleSummaryMetric::Species => Color::srgb(0.52, 0.86, 1.0),
        ChronicleSummaryMetric::Food => Color::srgb(1.0, 0.86, 0.30),
        ChronicleSummaryMetric::Viability => Color::srgb(0.90, 1.0, 0.94),
        ChronicleSummaryMetric::Energy => Color::srgb(0.74, 1.0, 0.70),
        ChronicleSummaryMetric::Costs => Color::srgb(1.0, 0.62, 0.40),
        ChronicleSummaryMetric::Traits => Color::srgb(0.82, 0.56, 1.0),
        ChronicleSummaryMetric::Performance => Color::srgb(0.62, 0.82, 1.0),
    }
}

fn chronicle_summary_value(
    metric: ChronicleSummaryMetric,
    snapshot: &ChronicleSnapshot,
) -> (String, Color) {
    match metric {
        ChronicleSummaryMetric::Time => (
            chronicle_time_label(snapshot.time),
            Color::srgb(0.78, 0.96, 0.94),
        ),
        ChronicleSummaryMetric::Cells => (snapshot.cells.to_string(), Color::srgb(0.50, 1.0, 0.62)),
        ChronicleSummaryMetric::Species => {
            (snapshot.species.to_string(), Color::srgb(0.58, 0.88, 1.0))
        }
        ChronicleSummaryMetric::Food => (
            format!(
                "{} · {}/{}/{}",
                snapshot.food, snapshot.wild_food, snapshot.feeder_food, snapshot.meat
            ),
            Color::srgb(1.0, 0.88, 0.42),
        ),
        ChronicleSummaryMetric::Viability => (
            format!("{:.0}%", snapshot.avg_viability * 100.0),
            Color::srgb(0.90, 1.0, 0.94),
        ),
        ChronicleSummaryMetric::Energy => {
            let color = if snapshot.energy_net > 1.0 {
                Color::srgb(0.42, 1.0, 0.58)
            } else if snapshot.energy_net < -1.0 {
                Color::srgb(1.0, 0.38, 0.34)
            } else {
                Color::srgb(0.96, 0.80, 0.34)
            };
            (
                format!(
                    "{:+.0} ({:.0}/{:.0})",
                    snapshot.energy_net, snapshot.energy_in, snapshot.energy_out
                ),
                color,
            )
        }
        ChronicleSummaryMetric::Costs => (
            format!(
                "м{:.0} · д{:.0} · л{:.0}",
                snapshot.metabolism, snapshot.mitosis, snapshot.lysis
            ),
            Color::srgb(1.0, 0.68, 0.42),
        ),
        ChronicleSummaryMetric::Traits => (
            format!(
                "сег {} · хищ {}",
                snapshot.segmented, snapshot.lysis_capable
            ),
            Color::srgb(0.84, 0.62, 1.0),
        ),
        ChronicleSummaryMetric::Performance => (
            format!(
                "{:.0} FPS · {:.1}/{:.1}",
                snapshot.fps, snapshot.sim_ms, snapshot.render_ms
            ),
            Color::srgb(0.66, 0.86, 1.0),
        ),
    }
}

fn chronicle_event_marker(kind: ChronicleEventKind) -> &'static str {
    match kind {
        ChronicleEventKind::World => "•",
        ChronicleEventKind::Species => "+",
        ChronicleEventKind::Extinction => "×",
        ChronicleEventKind::Population => "~",
        ChronicleEventKind::Energy => "=",
        ChronicleEventKind::Trait => "*",
    }
}

fn chronicle_time_label(seconds: f32) -> String {
    let total = seconds.max(0.0).round() as u32;
    let minutes = total / 60;
    let seconds = total % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn update_simulation_chronicle(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    ui_state: Res<GameUiState>,
    world: Res<WorldState>,
    names: Res<SpeciesNameBook>,
    stats: Res<FrameStats>,
    mut chronicle: ResMut<SimulationChronicle>,
) {
    if ui_state.paused {
        return;
    }

    let dt = time.delta_secs().clamp(0.0, 0.25) * ui_state.speed_multiplier;
    chronicle.elapsed += dt;
    chronicle.sample_accumulator += dt;
    if chronicle.sample_accumulator < CHRONICLE_SAMPLE_INTERVAL && !chronicle.snapshots.is_empty() {
        return;
    }
    chronicle.sample_accumulator = 0.0;

    let first_sample = chronicle.snapshots.is_empty();
    let flow = world.energy_flow;
    let (wild_food, feeder_food, meat) = world.active_food_counts();
    let (viability, viability_capacity, _, _) = viability_summary(&world);
    let avg_viability = if viability_capacity > 0.0 {
        viability / viability_capacity
    } else {
        0.0
    };
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .unwrap_or(0.0) as f32;

    let mut species_counts = HashMap::<u32, (usize, usize, f32)>::new();
    let mut segmented = 0usize;
    let mut lysis_capable = 0usize;
    for index in 0..world.cells.len() {
        let species = world.cells.species[index];
        let entry = species_counts.entry(species).or_insert((0, index, 0.0));
        entry.0 += 1;
        entry.2 += world.cells.aggressiveness[index];
        if world.cells.section_count[index] >= 2 {
            segmented += 1;
        }
        if world.cells.lysis[index] >= 8.0 {
            lysis_capable += 1;
        }
    }

    if chronicle.snapshots.len() >= CHRONICLE_MAX_SNAPSHOTS {
        chronicle.snapshots.remove(0);
    }
    let snapshot_time = chronicle.elapsed;
    chronicle.snapshots.push(ChronicleSnapshot {
        time: snapshot_time,
        cells: world.cells.len(),
        food: world.food.active_count(),
        wild_food,
        feeder_food,
        meat,
        avg_viability,
        energy_in: flow.external_input(),
        energy_out: flow.total_outflow(),
        energy_net: flow.net_external_balance(),
        metabolism: flow.metabolism,
        mitosis: flow.mitosis_cost,
        lysis: flow.lysis_loss,
        fps,
        sim_ms: stats.sim_time.as_secs_f32() * 1_000.0,
        render_ms: stats.upload_time.as_secs_f32() * 1_000.0,
        species: species_counts.len(),
        segmented,
        lysis_capable,
    });
    chronicle.revision = chronicle.revision.wrapping_add(1);

    if first_sample {
        chronicle.last_population_check_time = chronicle.elapsed;
        chronicle.last_population_check_cells = world.cells.len();
        for (species, (alive, _, _)) in &species_counts {
            chronicle.species_records.insert(
                *species,
                ChronicleSpeciesRecord {
                    alive: *alive,
                    peak_alive: *alive,
                },
            );
        }
        chronicle_push_event(
            &mut chronicle,
            ChronicleEventKind::World,
            "Запуск экосистемы",
            format!(
                "{} клеток, {} еды, {} стартовых видов.",
                world.cells.len(),
                world.food.active_count(),
                species_counts.len()
            ),
            None,
        );
    } else {
        let mut pending_events = Vec::<(ChronicleEventKind, String, String, Option<u32>)>::new();

        for (species, (alive, first_index, aggression_sum)) in &species_counts {
            if !chronicle.species_records.contains_key(species) {
                let name = species_name_for(&names, *species);
                let shape = world.cells.shape_name(*first_index);
                let avg_aggression = aggression_sum / (*alive as f32).max(1.0);
                pending_events.push((
                    ChronicleEventKind::Species,
                    "Новый вид".to_string(),
                    format!(
                        "{name}: {shape}, {}, {} живых.",
                        trophic_type_name(avg_aggression),
                        alive
                    ),
                    Some(*species),
                ));
                chronicle.species_records.insert(
                    *species,
                    ChronicleSpeciesRecord {
                        alive: *alive,
                        peak_alive: *alive,
                    },
                );
                continue;
            }

            if let Some(record) = chronicle.species_records.get_mut(species) {
                if record.peak_alive >= 8
                    && *alive >= record.peak_alive.saturating_add(10)
                    && *alive as f32 >= record.peak_alive as f32 * 1.6
                {
                    let name = species_name_for(&names, *species);
                    pending_events.push((
                        ChronicleEventKind::Population,
                        "Пик вида".to_string(),
                        format!("{name} вырос с {} до {} живых.", record.peak_alive, alive),
                        Some(*species),
                    ));
                }
                record.alive = *alive;
                record.peak_alive = record.peak_alive.max(*alive);
            }
        }

        let extinct_species = chronicle
            .species_records
            .iter()
            .filter_map(|(species, record)| {
                (record.alive > 0 && !species_counts.contains_key(species)).then_some(*species)
            })
            .collect::<Vec<_>>();
        for species in extinct_species {
            if let Some(record) = chronicle.species_records.get_mut(&species) {
                record.alive = 0;
            }
            let name = species_name_for(&names, species);
            pending_events.push((
                ChronicleEventKind::Extinction,
                "Вид вымер".to_string(),
                format!("{name}: последний живой представитель исчез."),
                Some(species),
            ));
        }

        if let Some((dominant_species, (alive, _, _))) = species_counts
            .iter()
            .max_by_key(|(_, (alive, _, _))| *alive)
        {
            let share = *alive as f32 / world.cells.len().max(1) as f32;
            if share >= 0.12 && chronicle.dominant_species != Some(*dominant_species) {
                chronicle.dominant_species = Some(*dominant_species);
                let name = species_name_for(&names, *dominant_species);
                pending_events.push((
                    ChronicleEventKind::Population,
                    "Новый доминант".to_string(),
                    format!("{name}: {:.0}% популяции мира.", share * 100.0),
                    Some(*dominant_species),
                ));
            }
        }

        let energy_state = if flow.net_external_balance() < -250.0 {
            ChronicleEnergyState::Deficit
        } else if flow.net_external_balance() > 250.0 {
            ChronicleEnergyState::Surplus
        } else {
            ChronicleEnergyState::Balanced
        };
        if energy_state != chronicle.energy_state {
            chronicle.energy_state = energy_state;
            let (title, body) = match energy_state {
                ChronicleEnergyState::Deficit => (
                    "Энергетический дефицит",
                    format!(
                        "Отток сильнее притока: {:+.0} ед/с.",
                        flow.net_external_balance()
                    ),
                ),
                ChronicleEnergyState::Surplus => (
                    "Энергетический профицит",
                    format!(
                        "Приток выше расходов: {:+.0} ед/с.",
                        flow.net_external_balance()
                    ),
                ),
                ChronicleEnergyState::Balanced => (
                    "Энергия стабилизировалась",
                    format!(
                        "Баланс около нуля: {:+.0} ед/с.",
                        flow.net_external_balance()
                    ),
                ),
            };
            pending_events.push((ChronicleEventKind::Energy, title.to_string(), body, None));
        }

        if chronicle.elapsed - chronicle.last_population_check_time >= 15.0 {
            let previous = chronicle.last_population_check_cells.max(1) as f32;
            let current = world.cells.len() as f32;
            let ratio = current / previous;
            if ratio >= 1.18 {
                pending_events.push((
                    ChronicleEventKind::Population,
                    "Всплеск популяции".to_string(),
                    format!(
                        "Клеток стало на {:.0}% больше за последние 15 секунд.",
                        (ratio - 1.0) * 100.0
                    ),
                    None,
                ));
            } else if ratio <= 0.82 {
                pending_events.push((
                    ChronicleEventKind::Population,
                    "Обвал популяции".to_string(),
                    format!(
                        "Клеток стало на {:.0}% меньше за последние 15 секунд.",
                        (1.0 - ratio) * 100.0
                    ),
                    None,
                ));
            }
            chronicle.last_population_check_time = chronicle.elapsed;
            chronicle.last_population_check_cells = world.cells.len();
        }

        if lysis_capable > 0 && !chronicle.first_lysis_reported {
            chronicle.first_lysis_reported = true;
            pending_events.push((
                ChronicleEventKind::Trait,
                "Появился лизис".to_string(),
                format!("{lysis_capable} клеток способны к ближней атаке."),
                None,
            ));
        }
        if segmented > 0 && !chronicle.first_segmented_reported {
            chronicle.first_segmented_reported = true;
            pending_events.push((
                ChronicleEventKind::Trait,
                "Появилась многосегментность".to_string(),
                format!("{segmented} клеток имеют несколько секций тела."),
                None,
            ));
        }

        for (kind, title, body, species) in pending_events {
            chronicle_push_event(&mut chronicle, kind, title, body, species);
        }
    }
}

fn section_profile_for_species_snapshot(
    world: &WorldState,
    cell_index: usize,
    section: u8,
) -> ([f32; 8], [f32; 8]) {
    match section {
        0 => (
            world.cells.base_radii[cell_index],
            world.cells.angle_offsets[cell_index],
        ),
        1 => (
            world.cells.tail_base_radii[cell_index],
            world.cells.tail_angle_offsets[cell_index],
        ),
        _ => {
            let extra = world.cells.extra_sections[cell_index][section as usize - 2];
            (extra.base_radii, extra.angle_offsets)
        }
    }
}

fn max_section_radius(radii: &[f32; 8]) -> f32 {
    radii.iter().copied().fold(0.1, f32::max)
}

fn update_species_ledger_stats(
    time: Res<Time>,
    world: Res<WorldState>,
    state: Res<SpeciesLedgerUiState>,
    mut stats: ResMut<SpeciesLedgerStats>,
) {
    let dt = time.delta_secs();
    if !state.open {
        stats.accumulator = 0.35;
        return;
    }
    stats.accumulator += dt;
    stats.sort_accumulator += dt;
    if stats.accumulator < 0.35 && !stats.snapshots.is_empty() {
        return;
    }
    stats.accumulator = 0.0;

    let previous_order = stats
        .snapshots
        .iter()
        .map(|snapshot| snapshot.species)
        .collect::<Vec<_>>();
    let previous_alive = stats
        .snapshots
        .iter()
        .map(|snapshot| (snapshot.species, snapshot.alive))
        .collect::<std::collections::HashMap<_, _>>();
    let mut by_species = std::collections::HashMap::<u32, SpeciesSnapshot>::new();

    for index in 0..world.cells.len() {
        let species = world.cells.species[index];
        let snapshot = by_species
            .entry(species)
            .or_insert_with(|| SpeciesSnapshot {
                species,
                area_min: Vec2::splat(f32::MAX),
                area_max: Vec2::splat(f32::MIN),
                ..default()
            });
        snapshot.alive += 1;
        let cell_position = Vec2::new(world.cells.x[index], world.cells.y[index]);
        snapshot.average_position += cell_position;
        snapshot.area_min = snapshot.area_min.min(cell_position);
        snapshot.area_max = snapshot.area_max.max(cell_position);
        snapshot.average_viability +=
            world.cells.viability[index] / world.cells.max_viability[index].max(1.0);
        snapshot.average_speed += world.cells.speed[index];
        snapshot.average_turn += world.cells.turn_speed[index];
        snapshot.average_aggressiveness += world.cells.aggressiveness[index];
        snapshot.average_lysis += world.cells.lysis[index];
        snapshot.average_perception += world.cells.perception[index];
        snapshot.average_persistence += world.cells.persistence[index];
        snapshot.average_mutation += world.cells.mutation_susceptibility[index];
        let base_radius = world.cells.max_base_radius(index).max(0.1);
        snapshot.average_size += base_radius;
        for ray in 0..8 {
            snapshot.average_radii[ray] += world.cells.base_radii[index][ray] / base_radius;
            snapshot.average_angle_offsets[ray] += world.cells.angle_offsets[index][ray];
        }
        snapshot.segmented_ratio += if world.cells.section_count[index] >= 2 {
            1.0
        } else {
            0.0
        };
        if snapshot.representative_cell_id.is_none() {
            snapshot.representative_cell_id = Some(world.cells.id[index]);
            snapshot.display_section_count = world.cells.section_count[index];
            snapshot.display_section_angles = world.cells.section_angles[index];
            snapshot.display_section_parents = world.cells.section_parents[index];
            snapshot.display_section_spacing =
                (world.cells.section_spacing[index] / base_radius).clamp(0.75, 4.60);
            let head_center = world.cells.section_center(index, 0);
            let (sin, cos) = (-world.cells.heading[index]).sin_cos();
            for section in 0..snapshot.display_section_count.clamp(1, 4) {
                let slot = section as usize;
                let center = world.cells.section_center(index, section);
                let relative = center - head_center;
                snapshot.display_section_centers[slot] = Vec2::new(
                    relative.x * cos - relative.y * sin,
                    relative.x * sin + relative.y * cos,
                ) / base_radius;
            }
            for section in 1..snapshot.display_section_count.clamp(1, 4) {
                let slot = section as usize;
                let parent = snapshot.display_section_parents[slot - 1].min(section - 1);
                let delta = snapshot.display_section_centers[parent as usize]
                    - snapshot.display_section_centers[slot];
                snapshot.display_section_headings[slot] = (-delta.y).atan2(delta.x);
            }
            for edge in 0..snapshot.display_section_count.saturating_sub(1).min(3) as usize {
                let child = edge as u8 + 1;
                let parent = snapshot.display_section_parents[edge].min(child - 1);
                let parent_center = world.cells.section_center(index, parent);
                let child_center = world.cells.section_center(index, child);
                let side = (child_center - parent_center)
                    .try_normalize()
                    .map(|direction| Vec2::new(-direction.y, direction.x))
                    .unwrap_or(Vec2::Y);
                let control = (parent_center + child_center) * 0.5
                    + side * world.cells.edge_curve_offsets[index][edge];
                let relative = control - head_center;
                snapshot.display_edge_controls[edge] = Vec2::new(
                    relative.x * cos - relative.y * sin,
                    relative.x * sin + relative.y * cos,
                ) / base_radius;
            }
            for ray in 0..8 {
                snapshot.display_radii[ray] =
                    (world.cells.base_radii[index][ray] / base_radius).clamp(0.25, 1.35);
                snapshot.display_angle_offsets[ray] = world.cells.angle_offsets[index][ray];
            }
            let head_radius = base_radius.max(0.1);
            for section in 0..snapshot.display_section_count.clamp(1, 4) {
                let (radii, offsets) =
                    section_profile_for_species_snapshot(world.as_ref(), index, section);
                let section_radius = max_section_radius(&radii).max(0.1);
                let slot = section as usize;
                snapshot.display_section_scale[slot] =
                    (section_radius / head_radius).clamp(0.50, 1.70);
                for ray in 0..8 {
                    snapshot.display_section_radii[slot][ray] =
                        (radii[ray] / section_radius).clamp(0.24, 1.72);
                    snapshot.display_section_angle_offsets[slot][ray] = offsets[ray];
                }
            }
        }
    }

    let mut snapshots = by_species.into_values().collect::<Vec<_>>();
    for snapshot in &mut snapshots {
        let previous = previous_alive
            .get(&snapshot.species)
            .copied()
            .unwrap_or(snapshot.alive);
        snapshot.alive_delta = snapshot.alive as isize - previous as isize;
        let inv = (snapshot.alive as f32).recip();
        snapshot.average_position *= inv;
        snapshot.average_viability *= inv;
        snapshot.average_speed *= inv;
        snapshot.average_turn *= inv;
        snapshot.average_aggressiveness *= inv;
        snapshot.average_lysis *= inv;
        snapshot.average_size *= inv;
        snapshot.average_perception *= inv;
        snapshot.average_persistence *= inv;
        snapshot.average_mutation *= inv;
        snapshot.segmented_ratio *= inv;
        for ray in 0..8 {
            snapshot.average_radii[ray] = (snapshot.average_radii[ray] * inv).clamp(0.35, 1.55);
            snapshot.average_angle_offsets[ray] =
                (snapshot.average_angle_offsets[ray] * inv).clamp(-0.35, 0.35);
        }
    }

    let should_sort =
        stats.snapshots.is_empty() || stats.sort_accumulator >= SPECIES_LEDGER_SORT_INTERVAL;
    if should_sort {
        snapshots.sort_by(|a, b| {
            b.alive
                .cmp(&a.alive)
                .then_with(|| a.species.cmp(&b.species))
        });
        stats.sort_accumulator = 0.0;
    } else {
        let order = previous_order
            .iter()
            .enumerate()
            .map(|(order, species)| (*species, order))
            .collect::<std::collections::HashMap<_, _>>();
        snapshots.sort_by(|a, b| {
            let a_order = order.get(&a.species).copied().unwrap_or(usize::MAX);
            let b_order = order.get(&b.species).copied().unwrap_or(usize::MAX);
            a_order
                .cmp(&b_order)
                .then_with(|| b.alive.cmp(&a.alive))
                .then_with(|| a.species.cmp(&b.species))
        });
    }

    let new_order = snapshots
        .iter()
        .map(|snapshot| snapshot.species)
        .collect::<Vec<_>>();
    if new_order != previous_order {
        stats.revision = stats.revision.wrapping_add(1);
    }
    stats.snapshots = snapshots;
}

fn trophic_icon_tint(aggressiveness: f32) -> Color {
    let ratio = (aggressiveness / CELL_AGGRESSIVENESS_DISPLAY_MAX).clamp(0.0, 1.0);
    if ratio < 0.40 {
        Color::srgb(0.42, 1.0, 0.54)
    } else if ratio < 0.70 {
        Color::srgb(1.0, 0.88, 0.30)
    } else {
        Color::srgb(1.0, 0.30, 0.24)
    }
}

fn trophic_spectrum_color(aggressiveness: f32, alpha: f32) -> Color {
    let ratio = (aggressiveness / CELL_AGGRESSIVENESS_DISPLAY_MAX).clamp(0.0, 1.0);
    let (r, g, b) = if ratio <= 0.5 {
        let t = ratio / 0.5;
        (0.34 + t * 0.66, 1.0 - t * 0.10, 0.42 - t * 0.16)
    } else {
        let t = (ratio - 0.5) / 0.5;
        (1.0, 0.90 - t * 0.55, 0.26 - t * 0.08)
    };
    Color::srgba(r, g, b.max(0.12), alpha)
}

fn species_area_center(snapshot: &SpeciesSnapshot) -> Vec2 {
    if snapshot.area_min.x.is_finite()
        && snapshot.area_max.x.is_finite()
        && snapshot.area_min.x <= snapshot.area_max.x
    {
        (snapshot.area_min + snapshot.area_max) * 0.5
    } else {
        snapshot.average_position
    }
}

fn species_area_half_size(snapshot: &SpeciesSnapshot) -> Vec2 {
    let raw = if snapshot.area_min.x.is_finite()
        && snapshot.area_max.x.is_finite()
        && snapshot.area_min.x <= snapshot.area_max.x
    {
        (snapshot.area_max - snapshot.area_min).abs() * 0.5
    } else {
        Vec2::ZERO
    };
    let padding = (snapshot.average_size * 18.0).clamp(90.0, 360.0);
    let minimum = (snapshot.average_size * 32.0).clamp(190.0, 620.0);
    (raw + Vec2::splat(padding)).max(Vec2::splat(minimum))
}

fn species_area_focus_scale(snapshot: &SpeciesSnapshot) -> f32 {
    let half = species_area_half_size(snapshot);
    (half.x.max(half.y) * 2.35 / START_VIEW_HEIGHT).clamp(0.42, 7.0)
}

fn species_area_polygon(snapshot: &SpeciesSnapshot, expand: f32) -> Vec<Vec2> {
    let center = species_area_center(snapshot);
    let half = species_area_half_size(snapshot) * expand;
    let sides = 14;
    let phase = snapshot.species as f32 * 0.071;
    (0..sides)
        .map(|index| {
            let angle = index as f32 / sides as f32 * std::f32::consts::TAU + 0.13;
            let ripple = 1.0
                + (phase + index as f32 * 1.37).sin() * 0.055
                + (phase * 0.7 + index as f32 * 2.11).cos() * 0.035;
            center + Vec2::new(angle.cos() * half.x, angle.sin() * half.y) * ripple
        })
        .collect()
}

fn species_area_polygon_mesh(points: &[Vec2], center: Vec2) -> Mesh {
    let mut positions = Vec::with_capacity(points.len() + 1);
    let mut normals = Vec::with_capacity(points.len() + 1);
    let mut uvs = Vec::with_capacity(points.len() + 1);
    let mut indices = Vec::with_capacity(points.len() * 3);

    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 0.0, 1.0]);
    uvs.push([0.5, 0.5]);
    for point in points {
        let local = *point - center;
        positions.push([local.x, local.y, 0.0]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([0.5 + local.x.signum() * 0.5, 0.5 + local.y.signum() * 0.5]);
    }
    for index in 0..points.len() {
        let next = if index + 1 == points.len() {
            1
        } else {
            index + 2
        };
        indices.extend_from_slice(&[0, (index + 1) as u32, next as u32]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn spawn_species_area_highlight(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    snapshot: &SpeciesSnapshot,
) {
    let center = species_area_center(snapshot);
    let color_alpha_layers = [
        (1.18, 0.025, 1.82),
        (1.10, 0.040, 1.84),
        (1.04, 0.070, 1.86),
        (1.00, 0.115, 1.88),
    ];
    for (expand, alpha, z) in color_alpha_layers {
        let points = species_area_polygon(snapshot, expand);
        let mesh = meshes.add(species_area_polygon_mesh(&points, center));
        let material = materials.add(StandardMaterial {
            base_color: trophic_spectrum_color(snapshot.average_aggressiveness, alpha),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            Name::new("species_area_highlight"),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(center.x, center.y, z),
            SimulationRenderEntity,
            SpeciesAreaHighlightEntity,
        ));
    }
}

#[allow(dead_code)]
fn species_morph_class(species: u32) -> u32 {
    species / SPECIES_CLASS_STRIDE
}

fn species_genus_key(species: u32) -> u32 {
    species / SPECIES_EPITHET_SLOTS
}

fn species_genus_name_for(names: &SpeciesNameBook, species: u32) -> String {
    let prefix_count = names.prefixes.len().max(1);
    let suffix_count = names.suffixes.len().max(1);
    let genus_key = species_genus_key(species) as usize;
    let prefix = &names.prefixes[genus_key % prefix_count];
    let suffix = &names.suffixes[(genus_key / prefix_count) % suffix_count];
    format!("{prefix}{suffix}")
}

fn species_shape_label_from_id(species: u32) -> &'static str {
    match species_morph_class(species) {
        0 => "Кокк",
        1 => "Бацилла",
        2 => "Филамент",
        3 => "Спирилла",
        4 => "Вибрион",
        5 => "Диплококк",
        6 => "Веретено",
        7 => "Кубоид",
        8 => "Триквитрум",
        9 => "Ставроморф",
        10 => "Ланцетовидная",
        11 => "Плакоид",
        _ => "Лобатум",
    }
}

fn species_snapshots_related(a: &SpeciesSnapshot, b: &SpeciesSnapshot) -> bool {
    a.species != b.species && species_genus_key(a.species) == species_genus_key(b.species)
}

fn spawn_species_ledger_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    asset_server: &AssetServer,
    snapshot: &SpeciesSnapshot,
    name: String,
    index: usize,
) {
    let species = snapshot.species;
    let column = index % SPECIES_LEDGER_COLUMNS;
    let row = index / SPECIES_LEDGER_COLUMNS;
    parent
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: px(column as f32 * (SPECIES_LEDGER_CARD_WIDTH + SPECIES_LEDGER_COLUMN_GAP)),
                top: px(row as f32 * SPECIES_LEDGER_ROW_STRIDE),
                width: px(SPECIES_LEDGER_CARD_WIDTH),
                height: px(SPECIES_LEDGER_CARD_HEIGHT),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::axes(px(8), px(8)),
                border: UiRect::all(px(2)),
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(Color::srgb(0.18, 0.38, 0.43)),
            BackgroundColor(Color::srgb(0.035, 0.055, 0.064)),
            Interaction::default(),
            SpeciesLedgerRow { species },
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    position_type: PositionType::Relative,
                    width: px(88),
                    height: px(88),
                    border: UiRect::all(px(2)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.32, 0.58, 0.62)),
                BackgroundColor(Color::srgb(0.025, 0.045, 0.050)),
                SpeciesLedgerMiniature { species },
            ))
            .with_children(|mini| {
                mini.spawn((
                    ImageNode::default(),
                    Node {
                        width: px(78),
                        height: px(78),
                        ..default()
                    },
                    SpeciesLedgerMiniImage { species },
                ));
            });

            row.spawn((
                ImageNode {
                    image: asset_server.load("sprites/icon-species-alive.png"),
                    color: Color::srgb(0.38, 1.0, 0.52),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    right: px(8),
                    top: px(8),
                    width: px(17),
                    height: px(17),
                    ..default()
                },
                SpeciesLedgerStatusIcon,
            ));

            row.spawn((
                ImageNode {
                    image: asset_server.load(trophic_type_icon(snapshot.average_aggressiveness)),
                    color: trophic_icon_tint(snapshot.average_aggressiveness),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(8),
                    bottom: px(8),
                    width: px(24),
                    height: px(24),
                    ..default()
                },
                SpeciesLedgerDietIcon,
            ));

            row.spawn((
                ImageNode {
                    image: asset_server.load("sprites/icon-species-relation.png"),
                    color: Color::srgb(0.35, 0.68, 1.0),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(8),
                    top: px(8),
                    width: px(17),
                    height: px(17),
                    ..default()
                },
                Visibility::Hidden,
                SpeciesLedgerRelationIcon { species },
            ));

            row.spawn((
                Text::new(snapshot.alive.to_string()),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.72, 1.0, 0.78)),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(8),
                    bottom: px(8),
                    ..default()
                },
                SpeciesLedgerCountText { species },
            ));

            row.spawn((
                Text::new(name),
                TextFont {
                    font,
                    font_size: 10.5,
                    ..default()
                },
                TextColor(Color::srgb(0.91, 0.96, 0.97)),
                TextLayout::new_with_justify(Justify::Center),
                Node {
                    width: px(126),
                    height: px(28),
                    margin: UiRect::top(px(5)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                SpeciesLedgerNameText,
            ));
        });
}

fn update_species_ledger_ui(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    names: Res<SpeciesNameBook>,
    stats: Res<SpeciesLedgerStats>,
    mut state: ResMut<SpeciesLedgerUiState>,
    mut ui_queries: ParamSet<(
        Query<(&mut Visibility, &mut Node, &mut PanelReveal), With<SpeciesLedgerPanel>>,
        Query<(Entity, &mut Node), With<SpeciesLedgerGrid>>,
        Query<(&ComputedNode, &ScrollPosition), With<SpeciesLedgerScrollArea>>,
        Query<Entity, With<SpeciesLedgerRow>>,
    )>,
) {
    if let Ok((mut visibility, mut node, mut reveal)) = ui_queries.p0().single_mut() {
        if state.open {
            *visibility = Visibility::Visible;
            node.display = Display::Flex;
        }
        let target = if state.open { 1.0 } else { 0.0 };
        let follow = 1.0 - (-13.0 * time.delta_secs()).exp();
        reveal.progress += (target - reveal.progress) * follow;
        node.left = px(SPECIES_LEDGER_PANEL_LEFT - reveal.hidden_offset * (1.0 - reveal.progress));
        if !state.open && reveal.progress < 0.002 {
            *visibility = Visibility::Hidden;
            node.display = Display::None;
        }
    }

    if !state.open {
        return;
    }

    let (scroll_y, view_height) = ui_queries
        .p2()
        .single()
        .map(|(computed, scroll_position)| {
            (
                scroll_position.y.max(0.0),
                (computed.size().y * computed.inverse_scale_factor()).max(1.0),
            )
        })
        .unwrap_or((0.0, 520.0));
    let total_height = species_ledger_content_height(stats.snapshots.len());
    let (range_start, range_end) =
        species_ledger_visible_index_range(stats.snapshots.len(), scroll_y, view_height);
    let should_render = state.rendered_revision != stats.revision
        || state.rendered_range_start != range_start
        || state.rendered_range_end != range_end;

    let grid_entity = {
        let mut grid = ui_queries.p1();
        let Ok((grid_entity, mut grid_node)) = grid.single_mut() else {
            return;
        };
        grid_node.height = px(total_height);
        if !should_render {
            return;
        }
        grid_entity
    };

    let row_entities = ui_queries.p3().iter().collect::<Vec<_>>();
    for entity in row_entities {
        commands.entity(entity).despawn();
    }

    let font = asset_server.load(UI_FONT);
    commands.entity(grid_entity).with_children(|grid| {
        for (index, snapshot) in stats
            .snapshots
            .iter()
            .enumerate()
            .skip(range_start)
            .take(range_end.saturating_sub(range_start))
        {
            let name = species_name_for(&names, snapshot.species);
            spawn_species_ledger_row(grid, font.clone(), &asset_server, snapshot, name, index);
        }
    });
    state.rendered_revision = stats.revision;
    state.rendered_range_start = range_start;
    state.rendered_range_end = range_end;
}

fn update_chronicle_ui(
    time: Res<Time>,
    state: Res<ChronicleUiState>,
    chronicle: Res<SimulationChronicle>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<ChronicleGraphCache>,
    mut panels: Query<(&mut Visibility, &mut Node, &mut PanelReveal), With<ChroniclePanel>>,
    mut overview_text: Query<
        &mut Text,
        (
            With<ChronicleOverviewText>,
            Without<ChronicleEventText>,
            Without<ChronicleSummaryValue>,
        ),
    >,
    mut summary_values: Query<
        (&ChronicleSummaryValue, &mut Text, &mut TextColor),
        (Without<ChronicleOverviewText>, Without<ChronicleEventText>),
    >,
    mut event_text: Query<
        &mut Text,
        (
            With<ChronicleEventText>,
            Without<ChronicleOverviewText>,
            Without<ChronicleSummaryValue>,
        ),
    >,
    mut graph_images: Query<&mut ImageNode, With<ChronicleGraphImage>>,
) {
    let reveal_follow = 1.0 - (-13.0 * time.delta_secs()).exp();
    for (mut visibility, mut node, mut reveal) in &mut panels {
        if state.open {
            *visibility = Visibility::Visible;
            node.display = Display::Flex;
        }
        let target = if state.open { 1.0 } else { 0.0 };
        reveal.progress += (target - reveal.progress) * reveal_follow;
        node.left = px(CHRONICLE_PANEL_LEFT - reveal.hidden_offset * (1.0 - reveal.progress));
        if !state.open && reveal.progress < 0.002 {
            *visibility = Visibility::Hidden;
            node.display = Display::None;
        }
    }

    if !state.open {
        return;
    }

    if let Ok(mut text) = overview_text.single_mut() {
        if let Some(snapshot) = chronicle.snapshots.last() {
            **text = format!(
                "{} · клеток {} · видов {} · еда {} (трава {} / корм. {} / мясо {}) · жизнь {:.0}% · энергия {:+.0} ед/с ({:.0}/{:.0}) · метаб. {:.0} · митоз {:.0} · лизис {:.0} · сегм. {} · хищн. {} · FPS {:.0} · {:.1}/{:.1} мс",
                chronicle_time_label(snapshot.time),
                snapshot.cells,
                snapshot.species,
                snapshot.food,
                snapshot.wild_food,
                snapshot.feeder_food,
                snapshot.meat,
                snapshot.avg_viability * 100.0,
                snapshot.energy_net,
                snapshot.energy_in,
                snapshot.energy_out,
                snapshot.metabolism,
                snapshot.mitosis,
                snapshot.lysis,
                snapshot.segmented,
                snapshot.lysis_capable,
                snapshot.fps,
                snapshot.sim_ms,
                snapshot.render_ms,
            );
        } else {
            **text = "ожидание первого среза мира".to_string();
        }
    }

    if let Some(snapshot) = chronicle.snapshots.last() {
        for (summary, mut text, mut color) in &mut summary_values {
            let (value, target_color) = chronicle_summary_value(summary.metric, snapshot);
            **text = value;
            *color = TextColor(target_color);
        }
    } else {
        for (_, mut text, mut color) in &mut summary_values {
            **text = "-".to_string();
            *color = TextColor(Color::srgb(0.58, 0.72, 0.74));
        }
    }

    if let Ok(mut text) = event_text.single_mut() {
        if chronicle.events.is_empty() {
            **text = "события появятся после первых изменений экосистемы".to_string();
        } else {
            let events = chronicle
                .events
                .iter()
                .rev()
                .filter(|event| state.event_enabled(event.kind))
                .map(|event| {
                    let species = event
                        .species
                        .map(|id| format!(" · вид #{id}"))
                        .unwrap_or_default();
                    format!(
                        "{} {} · {}{}\n{}\n",
                        chronicle_event_marker(event.kind),
                        chronicle_time_label(event.time),
                        event.title,
                        species,
                        event.body
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            **text = if events.is_empty() {
                "нет событий с включенными фильтрами".to_string()
            } else {
                events
            };
        }
    }

    if cache.revision != chronicle.revision || cache.mode != state.graph_mode {
        let image = render_chronicle_graph(&chronicle.snapshots, state.graph_mode);
        let handle = if let Some(handle) = cache.handle.clone() {
            if let Some(existing) = images.get_mut(&handle) {
                *existing = image;
                handle
            } else {
                images.add(image)
            }
        } else {
            images.add(image)
        };
        cache.handle = Some(handle);
        cache.revision = chronicle.revision;
        cache.mode = state.graph_mode;
    }

    if let Some(handle) = cache.handle.clone() {
        for mut image in &mut graph_images {
            image.image = handle.clone();
        }
    }
}

fn render_chronicle_graph(snapshots: &[ChronicleSnapshot], mode: ChronicleGraphMode) -> Image {
    let width = CHRONICLE_GRAPH_WIDTH;
    let height = CHRONICLE_GRAPH_HEIGHT;
    let mut data = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let shade = if (y / 22) % 2 == 0 { 12 } else { 9 };
            graph_write_pixel(
                &mut data,
                width,
                height,
                x as i32,
                y as i32,
                [shade, 24, 28, 230],
            );
        }
    }
    for step in 0..=4 {
        let y = 16 + step * ((height as i32 - 32) / 4);
        graph_draw_line(
            &mut data,
            width,
            height,
            IVec2::new(14, y),
            IVec2::new(width as i32 - 14, y),
            [34, 76, 84, 120],
            1,
        );
    }

    if snapshots.len() >= 2 {
        let cells = snapshots
            .iter()
            .map(|snapshot| snapshot.cells as f32)
            .collect::<Vec<_>>();
        let food = snapshots
            .iter()
            .map(|snapshot| snapshot.food as f32)
            .collect::<Vec<_>>();
        let viability = snapshots
            .iter()
            .map(|snapshot| snapshot.avg_viability * 100.0)
            .collect::<Vec<_>>();
        let net = snapshots
            .iter()
            .map(|snapshot| snapshot.energy_net)
            .collect::<Vec<_>>();

        match mode {
            ChronicleGraphMode::Overall => {
                graph_draw_normalized_series(
                    &mut data,
                    width,
                    height,
                    &cells,
                    [86, 255, 132, 245],
                    2,
                    false,
                );
                graph_draw_normalized_series(
                    &mut data,
                    width,
                    height,
                    &food,
                    [255, 220, 80, 235],
                    2,
                    false,
                );
                graph_draw_normalized_series(
                    &mut data,
                    width,
                    height,
                    &viability,
                    [225, 255, 238, 245],
                    2,
                    false,
                );
                graph_draw_normalized_series(
                    &mut data,
                    width,
                    height,
                    &net,
                    [88, 240, 170, 245],
                    2,
                    true,
                );
            }
            ChronicleGraphMode::Cells => graph_draw_normalized_series(
                &mut data,
                width,
                height,
                &cells,
                [86, 255, 132, 250],
                3,
                false,
            ),
            ChronicleGraphMode::Food => graph_draw_normalized_series(
                &mut data,
                width,
                height,
                &food,
                [255, 220, 80, 250],
                3,
                false,
            ),
            ChronicleGraphMode::Viability => graph_draw_normalized_series(
                &mut data,
                width,
                height,
                &viability,
                [225, 255, 238, 250],
                3,
                false,
            ),
            ChronicleGraphMode::Energy => graph_draw_normalized_series(
                &mut data,
                width,
                height,
                &net,
                [88, 240, 170, 250],
                3,
                true,
            ),
        }
    }

    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(data);
    image
}

fn graph_draw_normalized_series(
    data: &mut [u8],
    width: u32,
    height: u32,
    values: &[f32],
    color: [u8; 4],
    thickness: i32,
    centered: bool,
) {
    if values.len() < 2 {
        return;
    }
    let left = 16.0;
    let right = width as f32 - 16.0;
    let top = 14.0;
    let bottom = height as f32 - 16.0;
    let (min, max) = if centered {
        let max_abs = values
            .iter()
            .map(|value| value.abs())
            .fold(1.0_f32, f32::max);
        (-max_abs, max_abs)
    } else {
        let min = values.iter().copied().fold(f32::MAX, f32::min);
        let max = values.iter().copied().fold(f32::MIN, f32::max);
        if (max - min).abs() < 0.001 {
            (0.0, max.max(1.0))
        } else {
            (min, max)
        }
    };
    if centered {
        let zero_y = graph_series_y(0.0, min, max, top, bottom);
        graph_draw_line(
            data,
            width,
            height,
            IVec2::new(left as i32, zero_y.round() as i32),
            IVec2::new(right as i32, zero_y.round() as i32),
            [66, 124, 130, 120],
            1,
        );
    }

    let mut previous = None;
    let last = values.len().saturating_sub(1).max(1) as f32;
    for (index, value) in values.iter().enumerate() {
        let x = left + (right - left) * index as f32 / last;
        let y = graph_series_y(*value, min, max, top, bottom);
        let point = IVec2::new(x.round() as i32, y.round() as i32);
        if let Some(previous) = previous {
            let mut line_color = color;
            if centered && *value < 0.0 {
                line_color = [255, 92, 78, color[3]];
            }
            graph_draw_line(data, width, height, previous, point, line_color, thickness);
        }
        previous = Some(point);
    }
}

fn graph_series_y(value: f32, min: f32, max: f32, top: f32, bottom: f32) -> f32 {
    let t = ((value - min) / (max - min).max(0.001)).clamp(0.0, 1.0);
    bottom - (bottom - top) * t
}

fn graph_draw_line(
    data: &mut [u8],
    width: u32,
    height: u32,
    from: IVec2,
    to: IVec2,
    color: [u8; 4],
    thickness: i32,
) {
    let delta = to - from;
    let steps = delta.x.abs().max(delta.y.abs()).max(1);
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = (from.x as f32 + delta.x as f32 * t).round() as i32;
        let y = (from.y as f32 + delta.y as f32 * t).round() as i32;
        for oy in -thickness..=thickness {
            for ox in -thickness..=thickness {
                if ox * ox + oy * oy <= thickness * thickness {
                    graph_write_pixel(data, width, height, x + ox, y + oy, color);
                }
            }
        }
    }
}

fn graph_write_pixel(data: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let index = ((y as u32 * width + x as u32) * 4) as usize;
    let alpha = color[3] as f32 / 255.0;
    let inv = 1.0 - alpha;
    data[index] = (data[index] as f32 * inv + color[0] as f32 * alpha) as u8;
    data[index + 1] = (data[index + 1] as f32 * inv + color[1] as f32 * alpha) as u8;
    data[index + 2] = (data[index + 2] as f32 * inv + color[2] as f32 * alpha) as u8;
    data[index + 3] = 255;
}

fn update_species_ledger_row_visuals(
    stats: Res<SpeciesLedgerStats>,
    state: Res<SpeciesLedgerUiState>,
    mut rows: Query<
        (&SpeciesLedgerRow, &mut BackgroundColor, &mut BorderColor),
        Without<SpeciesLedgerMiniature>,
    >,
    mut count_texts: Query<(&SpeciesLedgerCountText, &mut Text), Without<SpeciesLedgerNameText>>,
    mut relation_icons: Query<(&SpeciesLedgerRelationIcon, &mut Visibility)>,
) {
    if !state.open {
        return;
    }

    let selected_snapshot = state
        .selected_species
        .and_then(|species| species_snapshot_by_id(&stats, species));

    for (row, mut background, mut border) in &mut rows {
        let selected = state.selected_species == Some(row.species);
        let related = selected_snapshot
            .and_then(|selected| {
                species_snapshot_by_id(&stats, row.species)
                    .map(|row| species_snapshots_related(selected, row))
            })
            .unwrap_or(false);
        background.0 = if selected {
            Color::srgb(0.115, 0.215, 0.225)
        } else if related {
            Color::srgb(0.040, 0.105, 0.165)
        } else {
            Color::srgb(0.030, 0.047, 0.055)
        };
        *border = BorderColor::all(if selected {
            Color::srgb(0.76, 1.0, 0.98)
        } else if related {
            Color::srgb(0.24, 0.66, 1.0)
        } else {
            Color::srgb(0.18, 0.38, 0.43)
        });
    }

    for (marker, mut text) in &mut count_texts {
        let alive = species_snapshot_by_id(&stats, marker.species)
            .map(|snapshot| snapshot.alive)
            .unwrap_or(0);
        **text = alive.to_string();
    }

    for (marker, mut visibility) in &mut relation_icons {
        let related = selected_snapshot
            .and_then(|selected| {
                species_snapshot_by_id(&stats, marker.species)
                    .map(|row| species_snapshots_related(selected, row))
            })
            .unwrap_or(false);
        *visibility = if related {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_species_ledger_miniature_visuals(
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<SpeciesMiniatureImageCache>,
    stats: Res<SpeciesLedgerStats>,
    state: Res<SpeciesLedgerUiState>,
    mut miniatures: Query<(&SpeciesLedgerMiniature, &mut BackgroundColor)>,
    mut mini_images: Query<(&SpeciesLedgerMiniImage, &mut ImageNode)>,
) {
    if !state.open {
        return;
    }

    for (marker, mut background) in &mut miniatures {
        background.0 = if species_snapshot_by_id(&stats, marker.species).is_some() {
            Color::srgb(0.025, 0.045, 0.050)
        } else {
            Color::srgb(0.020, 0.025, 0.030)
        };
    }

    let mut visible_species = Vec::new();
    for (marker, mut image_node) in &mut mini_images {
        visible_species.push(marker.species);
        let Some(snapshot) = species_snapshot_by_id(&stats, marker.species) else {
            image_node.image = Handle::<Image>::default();
            continue;
        };
        let signature = species_miniature_signature(snapshot);
        let refresh = cache
            .signatures
            .get(&marker.species)
            .copied()
            .is_none_or(|cached| cached != signature)
            || !cache.handles.contains_key(&marker.species);
        if refresh {
            let handle = images.add(render_species_miniature(snapshot));
            cache.handles.insert(marker.species, handle);
            cache.signatures.insert(marker.species, signature);
        }
        if let Some(handle) = cache.handles.get(&marker.species) {
            image_node.image = handle.clone();
            image_node.color = Color::WHITE;
        }
    }

    if cache.handles.len() > visible_species.len() + 64 {
        let visible = visible_species
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        cache.handles.retain(|species, _| visible.contains(species));
        cache
            .signatures
            .retain(|species, _| visible.contains(species));
    }
}

fn species_miniature_signature(snapshot: &SpeciesSnapshot) -> u64 {
    let mut signature = snapshot.species as u64 ^ ((snapshot.display_section_count as u64) << 41);
    signature = signature
        .wrapping_mul(1_099_511_628_211)
        .wrapping_add((snapshot.display_section_spacing.clamp(0.0, 5.0) * 128.0).round() as u64);
    let color = cell_display_color(
        snapshot.species,
        snapshot.average_viability,
        snapshot.average_aggressiveness,
        snapshot.average_lysis,
    );
    for value in color.into_iter().take(3) {
        let q = (value.clamp(0.0, 1.0) * 255.0).round() as u64;
        signature = signature.wrapping_mul(1_099_511_628_211).wrapping_add(q);
    }
    for section in 0..snapshot.display_section_count.clamp(1, 4) as usize {
        let scale =
            (snapshot.display_section_scale[section].clamp(0.45, 1.85) * 96.0).round() as u64;
        signature = signature
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(scale);
        if section > 0 {
            signature = signature
                .wrapping_mul(1_099_511_628_211)
                .wrapping_add(snapshot.display_section_parents[section - 1] as u64)
                .wrapping_mul(1_099_511_628_211)
                .wrapping_add(
                    ((snapshot.display_section_angles[section - 1]
                        .rem_euclid(std::f32::consts::TAU)
                        / std::f32::consts::TAU)
                        * 64.0)
                        .round() as u64,
                );
            let control = snapshot.display_edge_controls[section - 1];
            let control_x = ((control.x.clamp(-6.0, 6.0) + 6.0) * 64.0).round() as u64;
            let control_y = ((control.y.clamp(-6.0, 6.0) + 6.0) * 64.0).round() as u64;
            signature = signature
                .wrapping_mul(1_099_511_628_211)
                .wrapping_add(control_x)
                .wrapping_mul(1_099_511_628_211)
                .wrapping_add(control_y);
        }
        let center = snapshot.display_section_centers[section];
        let cx = ((center.x.clamp(-6.0, 6.0) + 6.0) * 64.0).round() as u64;
        let cy = ((center.y.clamp(-6.0, 6.0) + 6.0) * 64.0).round() as u64;
        let heading = ((snapshot.display_section_headings[section]
            .rem_euclid(std::f32::consts::TAU)
            / std::f32::consts::TAU)
            * 128.0)
            .round() as u64;
        signature = signature
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(cx)
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(cy)
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(heading);
        for ray in 0..8 {
            let radius = (snapshot.display_section_radii[section][ray].clamp(0.24, 1.72) * 128.0)
                .round() as u64;
            let angle = ((snapshot.display_section_angle_offsets[section][ray].clamp(-0.45, 0.45)
                + 0.45)
                * 256.0)
                .round() as u64;
            signature = signature
                .wrapping_mul(1_099_511_628_211)
                .wrapping_add(radius)
                .wrapping_mul(1_099_511_628_211)
                .wrapping_add(angle);
        }
    }
    signature
}

fn render_species_miniature(snapshot: &SpeciesSnapshot) -> Image {
    render_species_portrait(snapshot, 192, 10.0)
}

fn render_species_journal_portrait(snapshot: &SpeciesSnapshot) -> Image {
    render_species_portrait(snapshot, 384, 24.0)
}

fn render_species_portrait(snapshot: &SpeciesSnapshot, size: u32, padding: f32) -> Image {
    let mut data = vec![0_u8; (size * size * 4) as usize];
    let base = cell_display_color(
        snapshot.species,
        snapshot.average_viability,
        snapshot.average_aggressiveness,
        snapshot.average_lysis,
    );
    let body = mini_rgba(base[0] * 0.96, base[1] * 0.96, base[2] * 0.96, 0.82);
    let gel_highlight = mini_rgba(
        (base[0] * 1.23).min(1.0),
        (base[1] * 1.23).min(1.0),
        (base[2] * 1.23).min(1.0),
        0.76,
    );
    let membrane = mini_rgba(
        (base[0] * 1.33).min(1.0),
        (base[1] * 1.33).min(1.0),
        (base[2] * 1.33).min(1.0),
        0.98,
    );
    let core_outer = mini_rgba(0.77, 0.96, 0.78, 0.76);
    let core_inner = mini_rgba(0.95, 1.0, 0.90, 0.95);
    let ray_color = mini_rgba(0.90, 1.0, 0.90, 0.18);
    let count = snapshot.display_section_count.clamp(1, 4) as usize;
    let mut local_centers = snapshot.display_section_centers;
    let mut local_controls = snapshot.display_edge_controls;
    let mut local_scales = [1.0; 4];
    for (section, scale) in local_scales.iter_mut().enumerate().take(count) {
        let value = snapshot.display_section_scale[section];
        *scale = if value > 0.01 { value } else { 1.0 }.clamp(0.48, 1.78);
    }
    let has_stored_pose = count <= 1
        || local_centers
            .iter()
            .take(count)
            .skip(1)
            .any(|center| center.length_squared() > 0.0001);
    if !has_stored_pose {
        let section_spacing = if snapshot.display_section_spacing > 0.01 {
            snapshot.display_section_spacing
        } else {
            (local_scales[0] * 2.05).max(1.15)
        }
        .clamp(0.75, 4.60);
        for section in 1..count {
            let parent =
                snapshot.display_section_parents[section - 1].min((section - 1) as u8) as usize;
            let angle = snapshot.display_section_angles[section - 1];
            let direction = Vec2::new(angle.cos(), -angle.sin());
            local_centers[section] = local_centers[parent] + direction * section_spacing;
        }
    }
    for section in 1..count {
        let parent =
            snapshot.display_section_parents[section - 1].min((section - 1) as u8) as usize;
        if local_controls[section - 1].length_squared() <= 0.0001 {
            local_controls[section - 1] = mini_bridge_fallback_control(
                snapshot.species,
                section,
                local_centers[parent],
                local_centers[section],
            );
        }
    }

    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for section in 0..count {
        let vertices = species_miniature_vertices(snapshot, section, local_centers[section], 1.0);
        let outline = smooth_mini_contour(&vertices, local_centers[section], 5);
        let (section_min, section_max) = mini_bounds_unclamped(&outline);
        min = min.min(section_min);
        max = max.max(section_max);
    }
    for section in 1..count {
        let parent =
            snapshot.display_section_parents[section - 1].min((section - 1) as u8) as usize;
        let delta = local_centers[section] - local_centers[parent];
        if delta.length_squared() > 0.0001 {
            let arch = local_controls[section - 1];
            min = min.min(arch - Vec2::splat(delta.length() * 0.45 + 0.8));
            max = max.max(arch + Vec2::splat(delta.length() * 0.45 + 0.8));
        }
    }
    let span = (max - min).max(Vec2::splat(0.1));
    let draw_scale = ((size as f32 - padding * 2.0) / span.x.max(span.y))
        .clamp(size as f32 * 0.11, size as f32 * 0.36);
    let offset = Vec2::splat(size as f32 * 0.5) - (min + span * 0.5) * draw_scale;

    let mut centers = [Vec2::ZERO; 4];
    for section in 0..count {
        centers[section] = local_centers[section] * draw_scale + offset;
    }
    let mut controls = [Vec2::ZERO; 3];
    for edge in 0..count.saturating_sub(1) {
        controls[edge] = local_controls[edge] * draw_scale + offset;
    }

    let mut section_vertices = Vec::with_capacity(count);
    let mut section_outlines = Vec::with_capacity(count);
    for section in 0..count {
        let vertices = species_miniature_vertices(snapshot, section, centers[section], draw_scale);
        let outline =
            smooth_mini_contour(&vertices, centers[section], if size >= 256 { 8 } else { 6 });
        section_vertices.push(vertices);
        section_outlines.push(outline);
    }

    let mut body_polygons = Vec::with_capacity(count * 2);
    for outline in &section_outlines {
        body_polygons.push(outline.clone());
    }
    for section in 1..count {
        let parent =
            snapshot.display_section_parents[section - 1].min((section - 1) as u8) as usize;
        if let Some(ribbon) = mini_segment_bridge_ribbon(
            size,
            snapshot.species,
            section,
            centers[parent],
            centers[section],
            controls[section - 1],
            &section_vertices[parent],
            &section_vertices[section],
        ) {
            body_polygons.push(ribbon);
        }
    }

    let union_center = centers.iter().take(count).copied().sum::<Vec2>() / count as f32;
    draw_mini_union_body(
        &mut data,
        size,
        &body_polygons,
        union_center,
        body,
        gel_highlight,
        membrane,
    );

    for section in (0..count).rev() {
        let vertices = &section_vertices[section];
        draw_mini_internal_rays(&mut data, size, centers[section], vertices, ray_color);
        draw_mini_core(
            &mut data,
            size,
            centers[section],
            draw_scale * local_scales[section] * 0.25,
            core_outer,
            core_inner,
        );
    }

    let mut image = Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(data);
    image
}

fn species_miniature_vertices(
    snapshot: &SpeciesSnapshot,
    section: usize,
    center: Vec2,
    scale: f32,
) -> [Vec2; 8] {
    let section_scale = {
        let value = snapshot.display_section_scale[section];
        if value > 0.01 { value } else { 1.0 }.clamp(0.48, 1.78)
    };
    std::array::from_fn(|ray| {
        let angle = ray as f32 / 8.0 * std::f32::consts::TAU
            + snapshot.display_section_headings[section]
            + snapshot.display_section_angle_offsets[section][ray];
        let radius = snapshot.display_section_radii[section][ray].clamp(0.24, 1.72);
        center + Vec2::new(angle.cos(), -angle.sin()) * scale * radius * section_scale
    })
}

fn smooth_mini_contour(vertices: &[Vec2; 8], center: Vec2, detail: usize) -> Vec<Vec2> {
    let detail = detail.max(2);
    let mut contour = Vec::with_capacity(vertices.len() * detail);
    for index in 0..vertices.len() {
        let p0 = vertices[(index + vertices.len() - 1) % vertices.len()];
        let p1 = vertices[index];
        let p2 = vertices[(index + 1) % vertices.len()];
        let p3 = vertices[(index + 2) % vertices.len()];
        for step in 0..detail {
            let t = step as f32 / detail as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let point = (p1 * 2.0
                + (p2 - p0) * t
                + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2
                + (-p0 + p1 * 3.0 - p2 * 3.0 + p3) * t3)
                * 0.5;
            let normal = (point - center).try_normalize().unwrap_or(Vec2::Y);
            let wave = ((index as f32 * 1.73 + step as f32 * 0.91).sin()) * 0.35;
            contour.push(point + normal * wave);
        }
    }
    contour
}

fn mini_bridge_fallback_control(species: u32, section: usize, parent: Vec2, child: Vec2) -> Vec2 {
    let delta = child - parent;
    let Some(direction) = delta.try_normalize() else {
        return parent.lerp(child, 0.5);
    };
    let side = Vec2::new(-direction.y, direction.x);
    let bend_seed = (species as f32 * 0.017 + section as f32 * 1.791).sin();
    parent.lerp(child, 0.5) + side * delta.length() * 0.16 * bend_seed
}

fn mini_shape_support_radius(vertices: &[Vec2; 8], center: Vec2, direction: Vec2) -> f32 {
    let Some(direction) = direction.try_normalize() else {
        return 1.0;
    };
    vertices
        .iter()
        .map(|vertex| (*vertex - center).dot(direction))
        .fold(0.0, f32::max)
        .max(1.0)
}

fn quadratic_mini_point(start: Vec2, control: Vec2, end: Vec2, t: f32) -> Vec2 {
    start.lerp(control, t).lerp(control.lerp(end, t), t)
}

fn quadratic_mini_tangent(start: Vec2, control: Vec2, end: Vec2, t: f32) -> Vec2 {
    ((control - start) * (1.0 - t) + (end - control) * t)
        .try_normalize()
        .unwrap_or_else(|| (end - start).try_normalize().unwrap_or(Vec2::X))
}

fn mini_segment_bridge_ribbon(
    size: u32,
    species: u32,
    section: usize,
    parent_center: Vec2,
    child_center: Vec2,
    control: Vec2,
    parent_vertices: &[Vec2; 8],
    child_vertices: &[Vec2; 8],
) -> Option<Vec<Vec2>> {
    let center_delta = child_center - parent_center;
    let distance = center_delta.length();
    if distance < 1.0 {
        return None;
    }

    let samples = if size >= 256 { 28 } else { 18 };
    let mut left = Vec::with_capacity(samples + 1);
    let mut right = Vec::with_capacity(samples + 1);
    let organic_seed = species as f32 * 0.031 + section as f32 * 2.137;

    for step in 0..=samples {
        let t = step as f32 / samples as f32;
        let point = quadratic_mini_point(parent_center, control, child_center, t);
        let tangent = quadratic_mini_tangent(parent_center, control, child_center, t);
        let normal = Vec2::new(-tangent.y, tangent.x);
        let parent_left = mini_shape_support_radius(parent_vertices, parent_center, normal);
        let child_left = mini_shape_support_radius(child_vertices, child_center, normal);
        let parent_right = mini_shape_support_radius(parent_vertices, parent_center, -normal);
        let child_right = mini_shape_support_radius(child_vertices, child_center, -normal);
        let end_blend = (t * 2.0 - 1.0).powi(2);
        let waist = 0.76 + end_blend * 0.24;
        let ripple_a = (t * std::f32::consts::TAU * 2.0 + organic_seed).sin() * 0.045;
        let ripple_b = (t * std::f32::consts::TAU * 2.7 + organic_seed * 1.37).sin() * 0.035;
        let left_radius =
            (parent_left + (child_left - parent_left) * t) * waist * (1.0 + ripple_a + ripple_b);
        let right_radius = (parent_right + (child_right - parent_right) * t)
            * waist
            * (1.0 - ripple_a + ripple_b * 0.6);
        left.push(point + normal * left_radius.max(1.0));
        right.push(point - normal * right_radius.max(1.0));
    }

    let mut ribbon = left;
    ribbon.extend(right.into_iter().rev());
    Some(ribbon)
}

fn draw_mini_union_body(
    data: &mut [u8],
    size: u32,
    polygons: &[Vec<Vec2>],
    center: Vec2,
    body: [u8; 4],
    highlight: [u8; 4],
    membrane: [u8; 4],
) {
    if polygons.is_empty() {
        return;
    }
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for polygon in polygons {
        let (poly_min, poly_max) = mini_bounds(polygon, size, 8.0);
        min = min.min(poly_min);
        max = max.max(poly_max);
    }
    let min_x = min.x.floor().clamp(0.0, (size - 1) as f32) as u32;
    let min_y = min.y.floor().clamp(0.0, (size - 1) as f32) as u32;
    let max_x = max.x.ceil().clamp(0.0, (size - 1) as f32) as u32;
    let max_y = max.y.ceil().clamp(0.0, (size - 1) as f32) as u32;
    let mut mask = vec![false; (size * size) as usize];
    let max_radius = polygons
        .iter()
        .flat_map(|polygon| polygon.iter())
        .map(|vertex| vertex.distance(center))
        .fold(1.0, f32::max);
    let light_dir = Vec2::new(-0.58, -0.82).normalize();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let inside = polygons
                .iter()
                .any(|polygon| point_in_polygon(point, polygon));
            mask[(y * size + x) as usize] = inside;
        }
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let inside = mask[(y * size + x) as usize];
            let edge = mini_union_mask_edge_distance(&mask, size, x, y, inside);
            if !inside && edge > 2.5 {
                continue;
            }

            let from_center = point - center;
            let radial = (from_center.length() / max_radius).clamp(0.0, 1.0);
            let light = from_center
                .try_normalize()
                .map(|normal| normal.dot(light_dir).max(0.0))
                .unwrap_or(0.0);
            let gel_t = (0.16 + light * 0.22 + (1.0 - radial) * 0.14).clamp(0.0, 0.48);
            let membrane_t = if inside {
                (1.0 - edge / 7.0).clamp(0.0, 1.0).powf(0.65)
            } else {
                1.0
            };
            let outer_fade = if inside {
                1.0
            } else {
                (1.0 - edge / 2.5).clamp(0.0, 1.0)
            };
            let gel = mini_lerp_rgba(body, highlight, gel_t);
            let mut color = mini_lerp_rgba(gel, membrane, membrane_t);
            let alpha = if inside {
                0.70 + membrane_t * 0.25
            } else {
                0.22 * outer_fade
            };
            color[3] = ((color[3] as f32) * alpha).round().clamp(0.0, 255.0) as u8;
            write_mini_pixel(data, size, x, y, color);
        }
    }
}

fn mini_union_mask_edge_distance(mask: &[bool], size: u32, x: u32, y: u32, inside: bool) -> f32 {
    let max_radius = if inside { 8 } else { 3 };
    let size_i = size as i32;
    let mut best = f32::MAX;
    for dy in -max_radius..=max_radius {
        for dx in -max_radius..=max_radius {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            let different = if nx < 0 || ny < 0 || nx >= size_i || ny >= size_i {
                inside
            } else {
                mask[(ny as u32 * size + nx as u32) as usize] != inside
            };
            if different {
                best = best.min(((dx * dx + dy * dy) as f32).sqrt());
            }
        }
    }
    if best.is_finite() {
        (best - 0.5).max(0.0)
    } else {
        (max_radius + 1) as f32
    }
}

fn draw_mini_internal_rays(
    data: &mut [u8],
    size: u32,
    center: Vec2,
    vertices: &[Vec2; 8],
    color: [u8; 4],
) {
    for vertex in vertices {
        let start = center.lerp(*vertex, 0.30);
        let end = center.lerp(*vertex, 0.82);
        draw_mini_capsule(data, size, start, end, 0.55, color);
    }
}

fn draw_mini_capsule(data: &mut [u8], size: u32, a: Vec2, b: Vec2, radius: f32, color: [u8; 4]) {
    let min = a.min(b) - Vec2::splat(radius + 2.0);
    let max = a.max(b) + Vec2::splat(radius + 2.0);
    let min_x = min.x.floor().clamp(0.0, (size - 1) as f32) as u32;
    let min_y = min.y.floor().clamp(0.0, (size - 1) as f32) as u32;
    let max_x = max.x.ceil().clamp(0.0, (size - 1) as f32) as u32;
    let max_y = max.y.ceil().clamp(0.0, (size - 1) as f32) as u32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let distance = distance_to_segment(point, a, b);
            if distance <= radius {
                let edge = (1.0 - distance / radius.max(0.1)).clamp(0.0, 1.0);
                let mut faded = color;
                faded[3] = ((color[3] as f32) * (0.55 + edge * 0.35)).round() as u8;
                write_mini_pixel(data, size, x, y, faded);
            }
        }
    }
}

fn draw_mini_core(
    data: &mut [u8],
    size: u32,
    center: Vec2,
    radius: f32,
    outer: [u8; 4],
    inner: [u8; 4],
) {
    let min_x = (center.x - radius - 2.0)
        .floor()
        .clamp(0.0, (size - 1) as f32) as u32;
    let min_y = (center.y - radius - 2.0)
        .floor()
        .clamp(0.0, (size - 1) as f32) as u32;
    let max_x = (center.x + radius + 2.0)
        .ceil()
        .clamp(0.0, (size - 1) as f32) as u32;
    let max_y = (center.y + radius + 2.0)
        .ceil()
        .clamp(0.0, (size - 1) as f32) as u32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let distance = point.distance(center);
            if distance > radius + 1.6 {
                continue;
            }
            let inside = (1.0 - distance / radius.max(0.1)).clamp(0.0, 1.0);
            let edge = if distance <= radius {
                1.0
            } else {
                (1.0 - (distance - radius) / 1.6).clamp(0.0, 1.0)
            };
            let mut color = mini_lerp_rgba(outer, inner, inside.powf(0.55));
            color[3] = ((color[3] as f32) * edge).round().clamp(0.0, 255.0) as u8;
            write_mini_pixel(data, size, x, y, color);
        }
    }

    let highlight = mini_rgba(1.0, 1.0, 0.96, 0.42);
    draw_mini_disc(
        data,
        size,
        center + Vec2::new(-radius * 0.25, -radius * 0.30),
        radius * 0.28,
        highlight,
    );
}

fn draw_mini_disc(data: &mut [u8], size: u32, center: Vec2, radius: f32, color: [u8; 4]) {
    let min_x = (center.x - radius - 1.0)
        .floor()
        .clamp(0.0, (size - 1) as f32) as u32;
    let min_y = (center.y - radius - 1.0)
        .floor()
        .clamp(0.0, (size - 1) as f32) as u32;
    let max_x = (center.x + radius + 1.0)
        .ceil()
        .clamp(0.0, (size - 1) as f32) as u32;
    let max_y = (center.y + radius + 1.0)
        .ceil()
        .clamp(0.0, (size - 1) as f32) as u32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            if point.distance(center) <= radius {
                write_mini_pixel(data, size, x, y, color);
            }
        }
    }
}

fn mini_bounds(vertices: &[Vec2], size: u32, padding: f32) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for vertex in vertices {
        min = min.min(*vertex);
        max = max.max(*vertex);
    }
    (
        (min - Vec2::splat(padding)).clamp(Vec2::ZERO, Vec2::splat((size - 1) as f32)),
        (max + Vec2::splat(padding)).clamp(Vec2::ZERO, Vec2::splat((size - 1) as f32)),
    )
}

fn mini_bounds_unclamped(vertices: &[Vec2]) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for vertex in vertices {
        min = min.min(*vertex);
        max = max.max(*vertex);
    }
    (min, max)
}

fn point_in_polygon(point: Vec2, vertices: &[Vec2]) -> bool {
    let mut inside = false;
    let mut previous = vertices.len() - 1;
    for current in 0..vertices.len() {
        let a = vertices[current];
        let b = vertices[previous];
        if (a.y > point.y) != (b.y > point.y) {
            let dy = b.y - a.y;
            if dy.abs() <= 0.0001 {
                previous = current;
                continue;
            }
            let x_intersect = (b.x - a.x) * (point.y - a.y) / dy + a.x;
            if point.x < x_intersect {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn distance_to_segment(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((point - a).dot(ab) / ab.length_squared().max(0.0001)).clamp(0.0, 1.0);
    point.distance(a + ab * t)
}

fn mini_lerp_rgba(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    std::array::from_fn(|channel| {
        (a[channel] as f32 + (b[channel] as f32 - a[channel] as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

fn mini_rgba(r: f32, g: f32, b: f32, a: f32) -> [u8; 4] {
    [
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
        (a.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

fn write_mini_pixel(data: &mut [u8], size: u32, x: u32, y: u32, color: [u8; 4]) {
    let index = ((y * size + x) * 4) as usize;
    let src_a = color[3] as f32 / 255.0;
    let dst_a = data[index + 3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= f32::EPSILON {
        return;
    }
    for channel in 0..3 {
        let src = color[channel] as f32 / 255.0;
        let dst = data[index + channel] as f32 / 255.0;
        data[index + channel] =
            (((src * src_a + dst * dst_a * (1.0 - src_a)) / out_a) * 255.0).round() as u8;
    }
    data[index + 3] = (out_a * 255.0).round() as u8;
}

fn species_journal_metric_sample(
    snapshot: &SpeciesSnapshot,
    max_alive: usize,
    metric: SpeciesJournalMetric,
) -> (f32, String, Color) {
    match metric {
        SpeciesJournalMetric::Population => {
            let normalized = snapshot.alive as f32 / max_alive.max(1) as f32;
            let color = if snapshot.alive_delta > 0 {
                Color::srgb(0.50, 1.0, 0.58)
            } else if snapshot.alive_delta < 0 {
                Color::srgb(1.0, 0.36, 0.32)
            } else {
                Color::srgb(0.78, 0.93, 0.92)
            };
            (
                normalized.clamp(0.0, 1.0),
                format!("{} {:+}", snapshot.alive, snapshot.alive_delta),
                color,
            )
        }
        SpeciesJournalMetric::Viability => (
            snapshot.average_viability.clamp(0.0, 1.0),
            format!("{:.0}%", snapshot.average_viability * 100.0),
            Color::srgb(0.78, 1.0, 0.82),
        ),
        SpeciesJournalMetric::Size => (
            ((snapshot.average_size - CELL_SIZE_GENE_MIN)
                / (CELL_SIZE_GENE_MAX - CELL_SIZE_GENE_MIN))
                .clamp(0.0, 1.0),
            format!("{:.1}", snapshot.average_size),
            gene_stat_color(GeneStatId::Size),
        ),
        SpeciesJournalMetric::Speed => (
            (snapshot.average_speed / CELL_SPEED_DISPLAY_MAX).clamp(0.0, 1.0),
            format!("{:.0}", snapshot.average_speed),
            Color::srgb(0.78, 0.91, 1.0),
        ),
        SpeciesJournalMetric::Turn => (
            (snapshot.average_turn / CELL_TURN_DISPLAY_MAX).clamp(0.0, 1.0),
            format!("{:.1}", snapshot.average_turn),
            gene_stat_color(GeneStatId::Turn),
        ),
        SpeciesJournalMetric::Perception => (
            (snapshot.average_perception / CELL_PERCEPTION_DISPLAY_MAX).clamp(0.0, 1.0),
            format!("{:.0}", snapshot.average_perception),
            gene_stat_color(GeneStatId::Perception),
        ),
        SpeciesJournalMetric::Persistence => (
            (snapshot.average_persistence / CELL_PERSISTENCE_DISPLAY_MAX).clamp(0.0, 1.0),
            format!("{:.0}%", snapshot.average_persistence),
            gene_stat_color(GeneStatId::Persistence),
        ),
        SpeciesJournalMetric::Aggressiveness => (
            (snapshot.average_aggressiveness / CELL_AGGRESSIVENESS_DISPLAY_MAX).clamp(0.0, 1.0),
            format!("{:.0}%", snapshot.average_aggressiveness),
            gene_stat_color(GeneStatId::Aggressiveness),
        ),
        SpeciesJournalMetric::Lysis => (
            (snapshot.average_lysis / CELL_LYSIS_DISPLAY_MAX).clamp(0.0, 1.0),
            if snapshot.average_lysis < 8.0 {
                "нет".to_string()
            } else {
                format!("{:.0}%", snapshot.average_lysis)
            },
            if snapshot.average_lysis < 8.0 {
                Color::srgb(0.58, 0.70, 0.72)
            } else {
                Color::srgb(1.0, 0.54, 0.68)
            },
        ),
        SpeciesJournalMetric::Mutation => (
            (snapshot.average_mutation / CELL_MUTATION_DISPLAY_MAX).clamp(0.0, 1.0),
            format!("{:.0}%", snapshot.average_mutation),
            gene_stat_color(GeneStatId::Mutation),
        ),
    }
}

fn update_species_journal_ui(
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    names: Res<SpeciesNameBook>,
    stats: Res<SpeciesLedgerStats>,
    state: Res<SpeciesLedgerUiState>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<SpeciesMiniatureImageCache>,
    mut panels: Query<(&mut Visibility, &mut Node, &mut PanelReveal), With<SpeciesJournalPanel>>,
    mut image_queries: ParamSet<(
        Query<&mut ImageNode, With<SpeciesJournalPortraitImage>>,
        Query<&mut ImageNode, With<SpeciesJournalDietIcon>>,
    )>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<SpeciesJournalTitleText>>,
        Query<&mut Text, With<SpeciesJournalSubtitleText>>,
        Query<(&mut Text, &mut TextColor), With<SpeciesJournalTrendText>>,
        Query<&mut Text, With<SpeciesJournalBodyText>>,
        Query<&mut Text, With<SpeciesJournalAreaText>>,
        Query<(&SpeciesJournalMetricValue, &mut Text, &mut TextColor)>,
    )>,
    mut fills: Query<(&SpeciesJournalMetricFill, &mut Node), Without<SpeciesJournalPanel>>,
) {
    let selected_snapshot = (state.open && state.journal_open)
        .then_some(())
        .and_then(|_| {
            state
                .selected_species
                .and_then(|species| species_snapshot_by_id(&stats, species))
        });
    let show = selected_snapshot.is_some();
    let reveal_follow = 1.0 - (-13.0 * time.delta_secs()).exp();
    for (mut visibility, mut node, mut reveal) in &mut panels {
        if show {
            *visibility = Visibility::Visible;
            node.display = Display::Flex;
        }
        let target = if show { 1.0 } else { 0.0 };
        reveal.progress += (target - reveal.progress) * reveal_follow;
        node.left = px(SPECIES_JOURNAL_PANEL_LEFT - reveal.hidden_offset * (1.0 - reveal.progress));
        if !show && reveal.progress < 0.002 {
            *visibility = Visibility::Hidden;
            node.display = Display::None;
        }
    }

    let Some(snapshot) = selected_snapshot else {
        return;
    };

    let name = species_name_for(&names, snapshot.species);
    let genus = species_genus_name_for(&names, snapshot.species);
    let shape = species_shape_label_from_id(snapshot.species);
    let genus_key = species_genus_key(snapshot.species);
    let genus_alive = stats
        .snapshots
        .iter()
        .filter(|candidate| species_genus_key(candidate.species) == genus_key)
        .map(|candidate| candidate.alive)
        .sum::<usize>();
    let related_count = stats
        .snapshots
        .iter()
        .filter(|candidate| species_snapshots_related(snapshot, candidate))
        .count();
    let max_alive = stats
        .snapshots
        .iter()
        .map(|candidate| candidate.alive)
        .max()
        .unwrap_or(snapshot.alive);
    let structure = if snapshot.display_section_count >= 2 {
        format!(
            "{} секц. · сегментных {:.0}%",
            snapshot.display_section_count,
            snapshot.segmented_ratio * 100.0
        )
    } else {
        "односекционная".to_string()
    };
    let grass_multiplier = grass_energy_multiplier(snapshot.average_aggressiveness);
    let meat_multiplier = meat_energy_multiplier(snapshot.average_aggressiveness);
    let diet = trophic_type_name(snapshot.average_aggressiveness);

    if let Ok(mut title) = text_queries.p0().single_mut() {
        **title = name;
    }
    if let Ok(mut subtitle) = text_queries.p1().single_mut() {
        **subtitle =
            format!("{shape} · род {genus} · в роде {genus_alive} · родственных {related_count}");
    }
    if let Ok((mut trend, mut color)) = text_queries.p2().single_mut() {
        **trend = if snapshot.alive_delta == 0 {
            format!("{} живых", snapshot.alive)
        } else {
            format!("{} {:+}", snapshot.alive, snapshot.alive_delta)
        };
        *color = TextColor(if snapshot.alive_delta > 0 {
            Color::srgb(0.48, 1.0, 0.56)
        } else if snapshot.alive_delta < 0 {
            Color::srgb(1.0, 0.36, 0.32)
        } else {
            Color::srgb(0.76, 0.94, 0.92)
        });
    }
    if let Ok(mut body) = text_queries.p3().single_mut() {
        **body = format!(
            "{diet} · трава x{grass_multiplier:.2} · мясо x{meat_multiplier:.2}\n{structure}"
        );
    }
    if let Ok(mut area) = text_queries.p4().single_mut() {
        let half = species_area_half_size(snapshot);
        **area = format!(
            "центр {:.0}:{:.0} · {:.0} x {:.0}",
            snapshot.average_position.x,
            snapshot.average_position.y,
            half.x * 2.0,
            half.y * 2.0
        );
    }

    if let Ok(mut diet_icon) = image_queries.p1().single_mut() {
        diet_icon.image = asset_server.load(trophic_type_icon(snapshot.average_aggressiveness));
        diet_icon.color = trophic_icon_tint(snapshot.average_aggressiveness);
    }

    if let Ok(mut portrait) = image_queries.p0().single_mut() {
        let signature = species_miniature_signature(snapshot);
        let refresh = cache
            .journal_signatures
            .get(&snapshot.species)
            .copied()
            .is_none_or(|cached| cached != signature)
            || !cache.journal_handles.contains_key(&snapshot.species);
        if refresh {
            let handle = images.add(render_species_journal_portrait(snapshot));
            cache.journal_handles.insert(snapshot.species, handle);
            cache.journal_signatures.insert(snapshot.species, signature);
        }
        if let Some(handle) = cache.journal_handles.get(&snapshot.species) {
            portrait.image = handle.clone();
            portrait.color = Color::WHITE;
        }
    }
    if cache.journal_handles.len() > 32 {
        let selected = snapshot.species;
        cache
            .journal_handles
            .retain(|species, _| *species == selected);
        cache
            .journal_signatures
            .retain(|species, _| *species == selected);
    }

    let follow = 1.0 - (-12.0 * time.delta_secs()).exp();
    for (fill, mut node) in &mut fills {
        let (normalized, _, _) = species_journal_metric_sample(snapshot, max_alive, fill.metric);
        let target = normalized.clamp(0.0, 1.0) * 100.0;
        let current = match node.width {
            Val::Percent(value) => value,
            _ => 0.0,
        };
        node.width = percent(current + (target - current) * follow);
    }
    for (value, mut text, mut color) in &mut text_queries.p5() {
        let (_, display, value_color) =
            species_journal_metric_sample(snapshot, max_alive, value.metric);
        **text = display;
        *color = TextColor(value_color);
    }
}

fn species_journal_area_row_system(
    stats: Res<SpeciesLedgerStats>,
    state: Res<SpeciesLedgerUiState>,
    mut focus: ResMut<SpeciesCameraFocus>,
    mut highlight_state: ResMut<SpeciesAreaHighlightState>,
    mut rows: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<SpeciesJournalAreaRow>),
    >,
) {
    let snapshot = state
        .selected_species
        .and_then(|species| species_snapshot_by_id(&stats, species));
    let accent = snapshot
        .map(|snapshot| trophic_spectrum_color(snapshot.average_aggressiveness, 1.0))
        .unwrap_or(Color::srgb(0.32, 0.72, 0.68));

    for (interaction, mut background, mut border) in &mut rows {
        match *interaction {
            Interaction::Pressed => {
                background.0 = Color::srgb(0.05, 0.12, 0.12);
                *border = BorderColor::all(accent);
                if let Some(snapshot) = snapshot {
                    highlight_state.species = Some(snapshot.species);
                    highlight_state.rendered_revision = 0;
                    focus.active = true;
                    focus.target = species_area_center(snapshot);
                    focus.target_scale = species_area_focus_scale(snapshot);
                }
            }
            Interaction::Hovered => {
                background.0 = Color::srgb(0.026, 0.064, 0.066);
                *border = BorderColor::all(accent);
            }
            Interaction::None => {
                background.0 = Color::srgb(0.012, 0.030, 0.034);
                *border = BorderColor::all(Color::srgb(0.32, 0.72, 0.68));
            }
        }
    }
}

fn update_species_area_highlight_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    stats: Res<SpeciesLedgerStats>,
    state: Res<SpeciesLedgerUiState>,
    mut highlight_state: ResMut<SpeciesAreaHighlightState>,
    highlights: Query<Entity, With<SpeciesAreaHighlightEntity>>,
) {
    let desired_species = if state.open && state.journal_open {
        highlight_state
            .species
            .filter(|species| state.selected_species == Some(*species))
    } else {
        None
    };
    let Some(species) = desired_species else {
        if highlight_state.species.is_some() || !highlights.is_empty() {
            for entity in &highlights {
                commands.entity(entity).despawn();
            }
            *highlight_state = SpeciesAreaHighlightState::default();
        }
        return;
    };
    let Some(snapshot) = species_snapshot_by_id(&stats, species) else {
        for entity in &highlights {
            commands.entity(entity).despawn();
        }
        *highlight_state = SpeciesAreaHighlightState::default();
        return;
    };
    if highlight_state.rendered_revision == stats.revision && !highlights.is_empty() {
        return;
    }
    for entity in &highlights {
        commands.entity(entity).despawn();
    }
    spawn_species_area_highlight(&mut commands, &mut meshes, &mut materials, snapshot);
    highlight_state.rendered_revision = stats.revision;
}

#[allow(dead_code)]
fn update_species_ledger_details(
    names: Res<SpeciesNameBook>,
    stats: Res<SpeciesLedgerStats>,
    state: Res<SpeciesLedgerUiState>,
    mut details_panel: Query<&mut Visibility, With<SpeciesLedgerDetailsPanel>>,
    mut details_text: Query<
        &mut Text,
        (
            With<SpeciesLedgerDetailsText>,
            Without<SpeciesLedgerNameText>,
            Without<SpeciesLedgerCountText>,
        ),
    >,
) {
    if !state.open {
        for mut visibility in &mut details_panel {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let selected_snapshot = state
        .selected_species
        .and_then(|species| species_snapshot_by_id(&stats, species));
    let show = selected_snapshot.is_some();
    for mut visibility in &mut details_panel {
        *visibility = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let Ok(mut text) = details_text.single_mut() else {
        return;
    };
    let Some(snapshot) = selected_snapshot else {
        **text = String::new();
        return;
    };
    let name = species_name_for(&names, snapshot.species);
    let body = if snapshot.segmented_ratio >= 0.5 {
        "сегментные"
    } else {
        "односекционные"
    };
    **text = format!(
        "{name}\nживых особей: {}\nкластер формы: #{}\nстроение: {body}",
        snapshot.alive,
        species_morph_class(snapshot.species) + 1,
    );
}

fn species_ledger_scroll_system(
    time: Res<Time>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<SpeciesLedgerUiState>,
    mut drag: ResMut<SpeciesLedgerDragState>,
    stats: Res<SpeciesLedgerStats>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scroll_area: Query<(&ComputedNode, &mut ScrollPosition), With<SpeciesLedgerScrollArea>>,
    mut scrollbar_tracks: Query<&mut Visibility, With<SpeciesLedgerScrollbarTrack>>,
    mut scrollbar_thumbs: Query<&mut Node, With<SpeciesLedgerScrollbarThumb>>,
) {
    if !state.open {
        drag.scrollbar_dragging = false;
        drag.scroll_initialized = false;
        drag.scroll_target_y = 0.0;
        for mut visibility in &mut scrollbar_tracks {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let window = windows.single().ok();
    let cursor = window.and_then(Window::cursor_position);
    let cursor_over_ledger = window
        .zip(cursor)
        .map(|(window, cursor)| cursor_over_species_ledger(window, cursor))
        .unwrap_or(false);
    if mouse.just_pressed(MouseButton::Left) {
        drag.scrollbar_dragging = window
            .zip(cursor)
            .and_then(|(window, cursor)| species_ledger_scrollbar_fraction(window, cursor))
            .is_some();
    }
    if !mouse.pressed(MouseButton::Left) {
        drag.scrollbar_dragging = false;
    }

    let mut delta = 0.0;
    for event in mouse_wheel.read() {
        if !cursor_over_ledger {
            continue;
        }
        let scale = match event.unit {
            MouseScrollUnit::Line => SPECIES_LEDGER_WHEEL_LINE_SCROLL,
            MouseScrollUnit::Pixel => SPECIES_LEDGER_WHEEL_PIXEL_SCROLL,
        };
        delta -= event.y * scale;
    }

    let target_index = state.scroll_target_species.and_then(|species| {
        stats
            .snapshots
            .iter()
            .position(|snapshot| snapshot.species == species)
    });

    for (computed, mut scroll_position) in &mut scroll_area {
        let estimated_content_height = species_ledger_content_height(stats.snapshots.len());
        let computed_content_height = computed.content_size().y * computed.inverse_scale_factor();
        let view_height = (computed.size().y * computed.inverse_scale_factor()).max(1.0);
        let content_height = computed_content_height.max(estimated_content_height);
        let max_offset = (content_height - view_height).max(0.0);
        let dragged_fraction = if drag.scrollbar_dragging {
            window
                .zip(cursor)
                .and_then(|(window, cursor)| species_ledger_scrollbar_fraction(window, cursor))
        } else {
            None
        };

        if !drag.scroll_initialized {
            drag.scroll_target_y = scroll_position.y.clamp(0.0, max_offset);
            drag.scroll_initialized = true;
        }
        drag.scroll_target_y = drag.scroll_target_y.clamp(0.0, max_offset);

        let mut follow_rate = SPECIES_LEDGER_SCROLL_FOLLOW;
        if let Some(fraction) = dragged_fraction {
            drag.scroll_target_y = (fraction * max_offset).clamp(0.0, max_offset);
            follow_rate = SPECIES_LEDGER_SCROLLBAR_FOLLOW;
            state.scroll_target_species = None;
        } else if delta != 0.0 {
            drag.scroll_target_y = (drag.scroll_target_y + delta).clamp(0.0, max_offset);
            state.scroll_target_species = None;
        } else if let Some(index) = target_index {
            let target = species_ledger_scroll_target_y(index, view_height).clamp(0.0, max_offset);
            drag.scroll_target_y = target;
            follow_rate = SPECIES_LEDGER_AUTO_SCROLL_FOLLOW;
            if (scroll_position.y - target).abs() < 2.0 {
                state.scroll_target_species = None;
            }
        }

        let follow = 1.0 - (-follow_rate * time.delta_secs()).exp();
        scroll_position.y += (drag.scroll_target_y - scroll_position.y) * follow;
        if (scroll_position.y - drag.scroll_target_y).abs() < 0.35 {
            scroll_position.y = drag.scroll_target_y;
        }
        scroll_position.y = scroll_position.y.clamp(0.0, max_offset);

        let visible_ratio = (view_height / content_height).clamp(0.0, 1.0);
        let thumb_height = (visible_ratio * 100.0).clamp(8.0, 100.0);
        let thumb_top = if max_offset > 1.0 {
            (scroll_position.y / max_offset).clamp(0.0, 1.0) * (100.0 - thumb_height)
        } else {
            0.0
        };
        for mut visibility in &mut scrollbar_tracks {
            *visibility = if max_offset > 1.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        for mut thumb in &mut scrollbar_thumbs {
            thumb.top = percent(thumb_top);
            thumb.height = percent(thumb_height);
        }
    }
}
fn species_ledger_row_system(
    time: Res<Time>,
    interactions: Query<(&Interaction, &SpeciesLedgerRow), Changed<Interaction>>,
    world: Res<WorldState>,
    mut state: ResMut<SpeciesLedgerUiState>,
    mut focus: ResMut<SpeciesCameraFocus>,
    mut selected: ResMut<SelectedCell>,
) {
    let now = time.elapsed_secs_f64();
    for (interaction, row) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let is_double =
            state.last_click_species == Some(row.species) && now - state.last_click_time <= 0.42;
        let was_selected = state.selected_species == Some(row.species);
        state.last_click_species = Some(row.species);
        state.last_click_time = now;

        if was_selected && !is_double {
            state.selected_species = None;
            state.journal_open = false;
            state.scroll_target_species = None;
            continue;
        }

        state.selected_species = Some(row.species);
        if is_double {
            let alive = (0..world.cells.len())
                .filter(|&index| world.cells.species[index] == row.species)
                .count();
            if alive > 0 {
                let mut rng = rand::rng();
                let target_ordinal = rng.random_range(0..alive);
                if let Some(index) = (0..world.cells.len())
                    .filter(|&index| world.cells.species[index] == row.species)
                    .nth(target_ordinal)
                {
                    selected.cell_id = Some(world.cells.id[index]);
                    focus.active = true;
                    focus.target = Vec2::new(world.cells.x[index], world.cells.y[index]);
                    focus.target_scale = 0.42;
                }
            }
        }
    }
}

fn apply_species_camera_focus(
    time: Res<Time>,
    mut focus: ResMut<SpeciesCameraFocus>,
    mut camera: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    if !focus.active {
        return;
    }
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };
    let follow = 1.0 - (-5.5 * time.delta_secs()).exp();
    transform.translation.x += (focus.target.x - transform.translation.x) * follow;
    transform.translation.y += (focus.target.y - transform.translation.y) * follow;

    if let Projection::Orthographic(projection) = &mut *projection {
        projection.scale += (focus.target_scale - projection.scale) * follow;
        let close_position =
            (transform.translation.truncate() - focus.target).length_squared() < 16.0;
        let close_scale = (projection.scale - focus.target_scale).abs() < 0.01;
        if close_position && close_scale {
            focus.active = false;
        }
    }
}

fn update_diet_icon_system(
    asset_server: Res<AssetServer>,
    world: Res<WorldState>,
    selected: Res<SelectedCell>,
    mut gene_icons: Query<(&GeneIconNode, &mut ImageNode)>,
) {
    let Some(cell_index) = selected
        .cell_id
        .and_then(|cell_id| world.cell_index_by_id(cell_id))
    else {
        return;
    };

    let diet_icon = asset_server.load(trophic_type_icon(world.cells.aggressiveness[cell_index]));
    for (icon, mut image) in &mut gene_icons {
        if icon.kind == GeneStatId::Diet {
            image.image = diet_icon.clone();
        }
    }
}

fn update_pause_ui(
    time: Res<Time>,
    ui_state: Res<GameUiState>,
    mut indicator: Query<&mut Visibility, (With<PauseIndicator>, Without<PauseMenu>)>,
    mut menu: Query<
        (&mut Visibility, &mut Node, &mut PanelReveal),
        (With<PauseMenu>, Without<PauseIndicator>),
    >,
) {
    if let Ok(mut visibility) = indicator.single_mut() {
        *visibility = if ui_state.paused {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok((mut visibility, mut node, mut reveal)) = menu.single_mut() {
        if ui_state.pause_menu_open {
            *visibility = Visibility::Visible;
        }
        let target = if ui_state.pause_menu_open { 1.0 } else { 0.0 };
        let follow = 1.0 - (-13.0 * time.delta_secs()).exp();
        reveal.progress += (target - reveal.progress) * follow;
        node.margin.top = px(-160.0 - reveal.progress * 30.0);
        if !ui_state.pause_menu_open && reveal.progress < 0.01 {
            *visibility = Visibility::Hidden;
        }
    }
}

fn passport_toggle_action_system(
    interactions: Query<&Interaction, (Changed<Interaction>, With<PassportToggleButton>)>,
    selected: Res<SelectedCell>,
    mut ui_state: ResMut<GameUiState>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed && selected.cell_id.is_some() {
            ui_state.passport_open = !ui_state.passport_open;
        }
    }
}

fn passport_toggle_button_style_system(
    time: Res<Time>,
    mut interactions: Query<(&Interaction, &mut BackgroundColor), With<PassportToggleButton>>,
) {
    let follow = 1.0 - (-14.0 * time.delta_secs()).exp();
    for (interaction, mut background) in &mut interactions {
        let target = match *interaction {
            Interaction::Pressed => Color::srgb(0.10, 0.20, 0.22),
            Interaction::Hovered => Color::srgb(0.10, 0.18, 0.20),
            Interaction::None => Color::srgb(0.07, 0.12, 0.14),
        };
        background.0 = background.0.mix(&target, follow);
    }
}

fn animate_game_buttons(
    time: Res<Time>,
    mut buttons: Query<(&Interaction, &mut UiTransform), With<Button>>,
) {
    let follow = 1.0 - (-18.0 * time.delta_secs()).exp();
    for (interaction, mut transform) in &mut buttons {
        let (target_scale, target_y) = match *interaction {
            Interaction::Pressed => (0.965, 1.0),
            Interaction::Hovered => (1.025, -1.0),
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
    time: Res<Time>,
    mut interactions: Query<(&Interaction, &mut BackgroundColor), With<PauseMenuAction>>,
) {
    let follow = 1.0 - (-14.0 * time.delta_secs()).exp();
    for (interaction, mut background) in &mut interactions {
        let target = match *interaction {
            Interaction::Pressed => Color::srgb(0.12, 0.23, 0.25),
            Interaction::Hovered => Color::srgb(0.09, 0.17, 0.19),
            Interaction::None => Color::srgb(0.06, 0.10, 0.12),
        };
        background.0 = background.0.mix(&target, follow);
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
