use bevy::prelude::*;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use std::time::Duration;

pub const FOOD_RADIUS: f32 = 3.4;
pub const FEEDER_FOOD_SURFACE_GAP: f32 = -1.2;
pub const LIQUID_FLOW_SCALE: f32 = 340.0;
pub const LIQUID_FLOW_SPEED: f32 = 0.08;
pub const LIQUID_CAUSTIC_STRENGTH: f32 = 0.16;
pub const LIQUID_VIGNETTE_STRENGTH: f32 = 0.45;
pub const GRASS_FOOD_COLOR: [f32; 4] = [0.25, 1.0, 0.34, 0.94];
pub const MEAT_FOOD_COLOR: [f32; 4] = [1.0, 0.23, 0.18, 0.95];
const GRID_CELL_SIZE: f32 = 240.0;
const CELL_GRID_SIZE: f32 = 96.0;
const OBSTACLE_GRID_SIZE: f32 = 480.0;
const CELL_ACCELERATION_GAIN: f32 = 1.75;
const CELL_LINEAR_DRAG: f32 = 0.11;
const CELL_LATERAL_GRIP: f32 = 0.58;
const CELL_TURN_RATE_MULTIPLIER: f32 = 0.40;
const WANDER_GAIN: f32 = 0.45;
pub const CELL_VIABILITY_MAX: f32 = 100.0;
pub const CELL_SPEED_DISPLAY_MAX: f32 = 100.0;
pub const CELL_TURN_DISPLAY_MAX: f32 = 4.0;
pub const CELL_MUTATION_DISPLAY_MAX: f32 = 100.0;
pub const CELL_PERCEPTION_DISPLAY_MAX: f32 = 820.0;
pub const CELL_DIVISION_THRESHOLD_DISPLAY_MAX: f32 = 100.0;
pub const ARENA_RECTANGLE_CODE: f32 = 0.0;
pub const ARENA_CIRCLE_CODE: f32 = 1.0;
pub const SOFT_BODY_POINTS: usize = 8;
pub const SOFT_BODY_BASE_ANGLES: [f32; SOFT_BODY_POINTS] = [
    0.0,
    std::f32::consts::FRAC_PI_4,
    std::f32::consts::FRAC_PI_2,
    std::f32::consts::FRAC_PI_4 * 3.0,
    std::f32::consts::PI,
    std::f32::consts::FRAC_PI_4 * 5.0,
    std::f32::consts::PI * 1.5,
    std::f32::consts::FRAC_PI_4 * 7.0,
];
const SOFT_BODY_SECTOR_ANGLE: f32 = std::f32::consts::FRAC_PI_4;
const VIABILITY_DECAY_BASE: f32 = 0.12;
const VIABILITY_DECAY_SPEED: f32 = 0.05;
const SOFT_BODY_ELASTICITY_SPEED: f32 = 8.0;
const SOFT_BODY_VISUAL_FOLLOW_SPEED: f32 = 12.0;
const SOFT_BODY_COMPRESSION_RESPONSE: f32 = 0.58;
const SOFT_BODY_BIOMASS_DRAIN_RATE: f32 = 0.00045;
const CORE_RADIUS_FACTOR: f32 = 0.30;
const HARD_CORE_STIFFNESS_MULTIPLIER: f32 = 10.0;
const SOFT_BODY_BASE_MIN_FACTOR: f32 = CORE_RADIUS_FACTOR;
const SOFT_BODY_MAX_ANGLE_OFFSET: f32 = 0.261_799_4;
const SOFT_BODY_MUTATION_ANGLE_DELTA: f32 = 0.065;
const SHAPE_MUTATION_LENGTH_SCALE: f32 = 0.16;
const SOFT_BODY_SHAPE_DRAG: f32 = 0.12;
const SOFT_BODY_TURN_BONUS: f32 = 0.18;
const SOFT_BODY_COMPRESSION_IMPULSE: f32 = 0.45;
const SOFT_BODY_SOLID_PUSH_FACTOR: f32 = 0.62;
const SOFT_BODY_SOLID_PUSH_MAX: f32 = 5.5;
const MUTATION_FACTOR_DELTA_SCALE: f32 = 100.0 / (0.3 - 0.005);
const WORLD_GRASS_ENERGY: f32 = 10.0;
const FEEDER_FOOD_ENERGY: f32 = 11.0;
const FOOD_GROWER_BATCH_SIZE: usize = 12;
const GRASS_SPOILAGE_RATE: f32 = 0.0025;
const MEAT_SPOILAGE_RATE: f32 = 0.018;
const CELL_STRUCTURE_ENERGY_PER_BIOMASS: f32 = 0.16;
const DEATH_VIABILITY_RECOVERY: f32 = 0.62;
const DEATH_STRUCTURE_RECOVERY: f32 = 0.58;
const MEAT_CHUNK_ENERGY_MAX: f32 = 8.0;
const WILD_GRASS_REGROW_MIN: f32 = 3.0;
const WILD_GRASS_REGROW_SPREAD: f32 = 2.5;
const MIN_VIABILITY_MOVE_FACTOR: f32 = 0.35;
const TURN_IN_PLACE_ANGLE: f32 = 1.35;
const TURN_IN_PLACE_THROTTLE: f32 = 0.08;
const STUCK_ALIGNMENT: f32 = -0.45;
const STUCK_SPEED_FACTOR: f32 = 0.08;
const STUCK_REVERSE_DELAY: f32 = 0.75;
const EMERGENCY_REVERSE_DURATION: f32 = 0.38;
const EMERGENCY_REVERSE_THROTTLE: f32 = 0.18;
const INITIAL_LYSIS_CHANCE: f64 = 0.075;
const LYSIS_ACTIVE_THRESHOLD: f32 = 8.0;
const SPECIES_EPITHET_SLOTS: u32 = 10_000;
const SPECIES_GENUS_SLOTS: u32 = 1_000;
const SPECIES_CLASS_STRIDE: u32 = SPECIES_EPITHET_SLOTS * SPECIES_GENUS_SLOTS;
const LYSIS_COOLDOWN_MIN: f32 = 0.34;
const LYSIS_COOLDOWN_MAX: f32 = 1.05;
const LYSIS_REACH_MIN: f32 = 0.65;
const LYSIS_REACH_MAX: f32 = 5.0;
const LYSIS_HUNT_PAUSE_AFTER_KILL: f32 = 2.4;
const LYSIS_TARGET_RECHECK_MIN: f32 = 0.16;
const LYSIS_TARGET_RECHECK_MAX: f32 = 0.42;
const LYSIS_ATTACK_DEFORM_DURATION: f32 = 0.38;
const LYSIS_HIT_DEFORM_DURATION: f32 = 0.46;
const LYSIS_PARTICLES_PER_HIT: usize = 8;
const NO_CELL_TARGET: u64 = u64::MAX;
const TAIL_LONGITUDINAL_STIFFNESS: f32 = 12.0;
const TAIL_LATERAL_STIFFNESS: f32 = 5.2;
const TAIL_LONGITUDINAL_DAMPING: f32 = 4.8;
const TAIL_LATERAL_DAMPING: f32 = 1.45;
const MAX_COMPOUND_COLLISION_SEGMENTS: usize = 6;
const TAIL_MIN_SPACING_FACTOR: f32 = 1.9;
const TAIL_MAX_SPACING_FACTOR: f32 = 4.4;
const SEGMENT_SIZE_MIN_FACTOR: f32 = 0.58;
const SEGMENT_SIZE_MAX_FACTOR: f32 = 1.18;
const SEGMENT_INHERIT_SHAPE_CHANCE: f64 = 0.36;
const INITIAL_SEGMENTED_CHANCE: f64 = 0.12;
const FOOD_CURRENT_SPEED: f32 = 42.0;
const CELL_CURRENT_SPEED: f32 = 24.0;
const OBSTACLE_CURRENT_SPEED: f32 = 22.0;
const GROWER_CURRENT_SPEED: f32 = 10.0;

const FOOD_SOLID_SPAWN_MARGIN: f32 = 18.0;
const FLOOR_FOOD_RATIO: f32 = 0.25;
const EMPTY_WORLD_FEEDER_FOOD_PER_GROWER: usize = 120;
const CELL_AVOIDANCE_MARGIN: f32 = 92.0;
const CELL_AVOIDANCE_STRENGTH: f32 = 1.15;
const CELL_OBSTACLE_RESTITUTION: f32 = 0.35;
const JELLY_DECAY: f32 = 1.4;
const JELLY_HIT_GAIN: f32 = 0.42;
const HUNGER_EPSILON: f32 = 0.25;
const DIVISION_CHILD_OFFSET: f32 = 2.15;
const MITOSIS_DURATION: f32 = 2.15;
const MITOSIS_RECOVERY_DURATION: f32 = 0.75;
const MAX_VISUAL_PARTICLES: usize = 4_096;
const FOOD_PARTICLES_PER_BITE: usize = 7;
const MEAT_CHUNKS_MAX: usize = 6;
const MUTATION_CHANCE_MIN: f32 = 0.05;
const MUTATION_CHANCE_MAX: f32 = 0.48;
const MUTATION_STRENGTH_MIN: f32 = 0.01;
const MUTATION_STRENGTH_MAX: f32 = 0.13;
const MUTATION_POWER_MIN: f32 = 0.25;
const MUTATION_POWER_MAX: f32 = 0.85;
pub const CELL_SIZE_GENE_MIN: f32 = 4.0;
pub const CELL_SIZE_GENE_MAX: f32 = 11.0;
pub const SPEED_GENE_MIN: f32 = 24.0;
pub const SPEED_GENE_MAX: f32 = 96.0;
pub const TURN_GENE_MIN: f32 = 0.45;
pub const TURN_GENE_MAX: f32 = 3.6;
pub const PERCEPTION_GENE_MIN: f32 = 100.0;
pub const PERCEPTION_GENE_MAX: f32 = CELL_PERCEPTION_DISPLAY_MAX;
pub const CELL_PERSISTENCE_DISPLAY_MAX: f32 = 100.0;
pub const CELL_AGGRESSIVENESS_DISPLAY_MAX: f32 = 100.0;
pub const CELL_LYSIS_DISPLAY_MAX: f32 = 100.0;
const PERSISTENCE_GENE_MIN: f32 = 0.0;
const PERSISTENCE_GENE_MAX: f32 = CELL_PERSISTENCE_DISPLAY_MAX;
const MUTATION_GENE_MIN: f32 = 0.0;
const MUTATION_GENE_MAX: f32 = 100.0;
const DIVISION_THRESHOLD_MIN: f32 = 68.0;
const DIVISION_THRESHOLD_MAX: f32 = 94.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaShape {
    Rectangle,
    Circle,
}

impl ArenaShape {
    pub fn from_arg(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "rectangle" | "rect" => Some(Self::Rectangle),
            "circle" => Some(Self::Circle),
            _ => None,
        }
    }

    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Rectangle => "Прямоугольник",
            Self::Circle => "Круг",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Rectangle => Self::Circle,
            Self::Circle => Self::Rectangle,
        }
    }

    pub fn shader_code(self) -> f32 {
        match self {
            Self::Rectangle => ARENA_RECTANGLE_CODE,
            Self::Circle => ARENA_CIRCLE_CODE,
        }
    }
}

#[derive(Clone, Resource)]
pub struct SimConfig {
    pub cells: usize,
    pub food: usize,
    pub width: f32,
    pub height: f32,
    pub arena_shape: ArenaShape,
    pub obstacles: usize,
    pub food_growers: usize,
    pub collision_stiffness: f32,
    pub collision_damping: f32,
    pub seed: u64,
    pub vsync: bool,
    pub random_cell_geometry: bool,
    pub segmented_cells: bool,
    pub cell_shape_weights: [f32; CELL_SHAPE_COUNT],
    pub sound_volume: f32,
    pub ambient_volume: f32,
}

pub const CELL_SHAPE_COUNT: usize = 13;
pub const CELL_SHAPE_LABELS: [&str; CELL_SHAPE_COUNT] = [
    "Кокк",
    "Бацилла",
    "Филамент",
    "Спирилла",
    "Вибрион",
    "Диплококк",
    "Веретено",
    "Кубоид",
    "Триквитрум",
    "Ставроморф",
    "Ланцет",
    "Плакоид",
    "Амеба",
];
const DEFAULT_CELL_SHAPE_WEIGHTS: [f32; CELL_SHAPE_COUNT] = [
    18.0, 15.0, 7.0, 6.0, 8.0, 6.0, 7.0, 7.0, 8.0, 6.0, 5.0, 6.0, 7.0,
];

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            cells: 10_000,
            food: 3_000,
            width: 18_000.0,
            height: 10_000.0,
            arena_shape: ArenaShape::Rectangle,
            obstacles: 30,
            food_growers: 6,
            collision_stiffness: 500.0,
            collision_damping: 15.0,
            seed: 0xC011_CE11,
            vsync: false,
            random_cell_geometry: false,
            segmented_cells: true,
            cell_shape_weights: DEFAULT_CELL_SHAPE_WEIGHTS,
            sound_volume: 0.8,
            ambient_volume: 0.6,
        }
    }
}

impl SimConfig {
    pub fn set_cell_shape_weight(&mut self, changed_index: usize, value: f32) {
        if changed_index >= CELL_SHAPE_COUNT {
            return;
        }
        let value = value.clamp(0.0, 100.0);
        let remainder = 100.0 - value;
        let other_total = self
            .cell_shape_weights
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != changed_index)
            .map(|(_, weight)| *weight)
            .sum::<f32>();

        self.cell_shape_weights[changed_index] = value;
        for (index, weight) in self.cell_shape_weights.iter_mut().enumerate() {
            if index == changed_index {
                continue;
            }
            *weight = if other_total > 0.0001 {
                *weight / other_total * remainder
            } else {
                remainder / (CELL_SHAPE_COUNT - 1) as f32
            };
        }
    }

    pub fn from_args() -> Result<Self, String> {
        let mut config = SimConfig::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--cells" => {
                    config.cells = parse_next(&mut args, "--cells")?;
                }
                "--food" => {
                    config.food = parse_next(&mut args, "--food")?;
                }
                "--width" => {
                    config.width = parse_next(&mut args, "--width")?;
                }
                "--height" => {
                    config.height = parse_next(&mut args, "--height")?;
                }
                "--shape" => {
                    let raw = args
                        .next()
                        .ok_or_else(|| format!("Missing value after `--shape`.\n\n{}", usage()))?;
                    config.arena_shape = ArenaShape::from_arg(&raw)
                        .ok_or_else(|| format!("Invalid shape `{raw}`.\n\n{}", usage()))?;
                }
                "--obstacles" => {
                    config.obstacles = parse_next(&mut args, "--obstacles")?;
                }
                "--food-growers" => {
                    config.food_growers = parse_next(&mut args, "--food-growers")?;
                }
                "--collision-stiffness" => {
                    config.collision_stiffness = parse_next(&mut args, "--collision-stiffness")?;
                }
                "--collision-damping" => {
                    config.collision_damping = parse_next(&mut args, "--collision-damping")?;
                }
                "--seed" => {
                    config.seed = parse_next(&mut args, "--seed")?;
                }
                "--vsync" => {
                    config.vsync = true;
                }
                "--no-segmented-cells" => {
                    config.segmented_cells = false;
                }
                "--help" | "-h" => {
                    return Err(usage());
                }
                other => {
                    return Err(format!("Unknown argument `{other}`.\n\n{}", usage()));
                }
            }
        }

        Ok(config)
    }
}

fn parse_next<T: std::str::FromStr>(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<T, String> {
    let raw = args
        .next()
        .ok_or_else(|| format!("Missing value after `{flag}`.\n\n{}", usage()))?;
    raw.parse()
        .map_err(|_| format!("Invalid value `{raw}` for `{flag}`.\n\n{}", usage()))
}

pub fn usage() -> String {
    "Usage: organoids [--cells 10000] [--food 3000] [--width 18000] [--height 10000] [--shape rectangle|circle] [--obstacles 30] [--food-growers 6] [--collision-stiffness 500] [--collision-damping 15] [--seed 123] [--vsync] [--no-segmented-cells]".to_string()
}

fn floor_food_count(total_food: usize) -> usize {
    if total_food == 0 {
        0
    } else {
        ((total_food as f32 * FLOOR_FOOD_RATIO).round() as usize).clamp(1, total_food)
    }
}

fn feeder_food_capacity(total_food: usize, floor_food: usize, food_growers: usize) -> usize {
    if total_food == 0 {
        food_growers.saturating_mul(EMPTY_WORLD_FEEDER_FOOD_PER_GROWER)
    } else {
        total_food.saturating_sub(floor_food)
    }
}

fn signed_count_delta(current: usize, previous: usize) -> i32 {
    let delta = current as i128 - previous as i128;
    delta.clamp(i32::MIN as i128, i32::MAX as i128) as i32
}

pub fn lysis_combat_profile(lysis: f32) -> (f32, f32, f32, f32) {
    let power = (lysis / CELL_LYSIS_DISPLAY_MAX).clamp(0.0, 1.0);
    let damage = 5.0 + power * 6.0;
    let self_cost = 0.20 + power * 0.22;
    let cooldown = LYSIS_COOLDOWN_MAX + (LYSIS_COOLDOWN_MIN - LYSIS_COOLDOWN_MAX) * power;
    let reach = LYSIS_REACH_MAX + (LYSIS_REACH_MIN - LYSIS_REACH_MAX) * power;
    (damage, self_cost, cooldown, reach)
}

pub fn trophic_aggression_ratio(aggressiveness: f32) -> f32 {
    (aggressiveness / CELL_AGGRESSIVENESS_DISPLAY_MAX).clamp(0.0, 1.0)
}

pub fn grass_energy_multiplier(aggressiveness: f32) -> f32 {
    let aggression = trophic_aggression_ratio(aggressiveness);
    1.25 - aggression * 0.95
}

pub fn meat_energy_multiplier(aggressiveness: f32) -> f32 {
    let aggression = trophic_aggression_ratio(aggressiveness);
    0.30 + aggression * 3.70
}

fn strict_gene_bin(value: f32, min: f32, max: f32, bins: u32) -> u32 {
    if bins == 0 {
        return 0;
    }
    let normalized = ((value - min) / (max - min).max(0.001)).clamp(0.0, 1.0);
    (normalized * bins as f32).round() as u32
}

fn lysis_size_damage_multiplier(attacker_biomass: f32, victim_biomass: f32) -> f32 {
    (attacker_biomass.max(0.1) / victim_biomass.max(0.1))
        .powf(0.65)
        .clamp(0.35, 2.50)
}

fn digested_food_energy(kind: FoodKind, raw_energy: f32, aggressiveness: f32) -> f32 {
    match kind {
        FoodKind::Grass => raw_energy * grass_energy_multiplier(aggressiveness),
        FoodKind::Meat => raw_energy * meat_energy_multiplier(aggressiveness),
    }
}

fn arena_circle_radius(width: f32, height: f32) -> f32 {
    width.min(height) * 0.5
}

fn random_point_in_arena(
    width: f32,
    height: f32,
    shape: ArenaShape,
    margin: f32,
    rng: &mut SmallRng,
) -> Vec2 {
    match shape {
        ArenaShape::Rectangle => {
            let half_w = (width * 0.5 - margin).max(1.0);
            let half_h = (height * 0.5 - margin).max(1.0);
            Vec2::new(
                rng.random_range(-half_w..half_w),
                rng.random_range(-half_h..half_h),
            )
        }
        ArenaShape::Circle => {
            let radius = (arena_circle_radius(width, height) - margin).max(1.0);
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let distance = radius * rng.random_range(0.0_f32..1.0).sqrt();
            let (s, c) = angle.sin_cos();
            Vec2::new(c * distance, s * distance)
        }
    }
}

fn point_inside_arena(
    point: Vec2,
    width: f32,
    height: f32,
    shape: ArenaShape,
    margin: f32,
) -> bool {
    match shape {
        ArenaShape::Rectangle => {
            let half_w = width * 0.5 - margin;
            let half_h = height * 0.5 - margin;
            point.x >= -half_w && point.x <= half_w && point.y >= -half_h && point.y <= half_h
        }
        ArenaShape::Circle => {
            let radius = (arena_circle_radius(width, height) - margin).max(0.0);
            point.length_squared() <= radius * radius
        }
    }
}

fn clamp_point_to_arena(
    point: Vec2,
    width: f32,
    height: f32,
    shape: ArenaShape,
    margin: f32,
) -> Vec2 {
    match shape {
        ArenaShape::Rectangle => {
            let half_w = (width * 0.5 - margin).max(0.0);
            let half_h = (height * 0.5 - margin).max(0.0);
            Vec2::new(
                point.x.clamp(-half_w, half_w),
                point.y.clamp(-half_h, half_h),
            )
        }
        ArenaShape::Circle => {
            let radius = (arena_circle_radius(width, height) - margin).max(0.0);
            let dist_sq = point.length_squared();
            if dist_sq <= radius * radius {
                point
            } else if dist_sq > 0.0001 {
                point * (radius / dist_sq.sqrt())
            } else {
                Vec2::ZERO
            }
        }
    }
}

fn bounce_point_in_arena(
    x: &mut f32,
    y: &mut f32,
    vx: &mut f32,
    vy: &mut f32,
    width: f32,
    height: f32,
    shape: ArenaShape,
    margin: f32,
) -> bool {
    match shape {
        ArenaShape::Rectangle => {
            let before = (*x, *y);
            clamp_bounce_axis(x, vx, width * 0.5, margin);
            clamp_bounce_axis(y, vy, height * 0.5, margin);
            before != (*x, *y)
        }
        ArenaShape::Circle => {
            let radius = (arena_circle_radius(width, height) - margin).max(0.0);
            let point = Vec2::new(*x, *y);
            let dist_sq = point.length_squared();
            if dist_sq <= radius * radius {
                return false;
            }

            let normal = if dist_sq > 0.0001 {
                point * dist_sq.sqrt().recip()
            } else {
                Vec2::X
            };
            *x = normal.x * radius;
            *y = normal.y * radius;

            let outward = *vx * normal.x + *vy * normal.y;
            if outward > 0.0 {
                *vx -= normal.x * outward * 1.35;
                *vy -= normal.y * outward * 1.35;
            }
            true
        }
    }
}

#[derive(Resource)]
pub struct WorldState {
    pub cells: CellStore,
    pub food: FoodStore,
    pub obstacles: ObstacleStore,
    pub food_growers: FoodGrowerStore,
    pub visual_particles: VisualParticleStore,
    pub width: f32,
    pub height: f32,
    pub arena_shape: ArenaShape,
    grid: SpatialGrid,
    cell_grid: CellGrid,
    cell_grid_dirty: bool,
    obstacle_grid: CellGrid,
    max_obstacle_radius: f32,
    collision_pairs: Vec<(usize, usize)>,
    collision_bounds: Vec<f32>,
    rng: SmallRng,
    elapsed: f32,
    max_feeder_food: usize,
    max_carrion: usize,
    collision_stiffness: f32,
    collision_damping: f32,
    pub cell_sound_events: Vec<Vec2>,
    pub energy_flow: EnergyFlowStats,
    pub cell_count_delta: i32,
    pub food_count_delta: i32,
    energy_accumulator: EnergyFlowAccumulator,
    count_baseline: (usize, usize),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EnergyFlowStats {
    pub wild_food_input: f32,
    pub feeder_input: f32,
    pub carrion_transfer: f32,
    pub food_consumed: f32,
    pub metabolism: f32,
    pub spoilage: f32,
    pub mitosis_cost: f32,
    pub lysis_loss: f32,
}

impl EnergyFlowStats {
    pub fn external_input(self) -> f32 {
        self.wild_food_input + self.feeder_input
    }

    pub fn net_external_balance(self) -> f32 {
        self.external_input() - self.total_outflow()
    }

    pub fn total_outflow(self) -> f32 {
        self.metabolism + self.spoilage + self.mitosis_cost + self.lysis_loss
    }
}

#[derive(Default)]
struct EnergyFlowAccumulator {
    values: EnergyFlowStats,
    elapsed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodKind {
    Grass,
    Meat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FoodSource {
    Wild,
    Feeder,
    Carrion,
}

pub struct VisualParticleStore {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub radius: Vec<f32>,
    pub life: Vec<f32>,
    pub lifetime: Vec<f32>,
    pub phase: Vec<f32>,
    pub color: Vec<[f32; 4]>,
    pub style: Vec<f32>,
}

impl VisualParticleStore {
    fn new() -> Self {
        Self {
            x: Vec::with_capacity(MAX_VISUAL_PARTICLES),
            y: Vec::with_capacity(MAX_VISUAL_PARTICLES),
            vx: Vec::with_capacity(MAX_VISUAL_PARTICLES),
            vy: Vec::with_capacity(MAX_VISUAL_PARTICLES),
            radius: Vec::with_capacity(MAX_VISUAL_PARTICLES),
            life: Vec::with_capacity(MAX_VISUAL_PARTICLES),
            lifetime: Vec::with_capacity(MAX_VISUAL_PARTICLES),
            phase: Vec::with_capacity(MAX_VISUAL_PARTICLES),
            color: Vec::with_capacity(MAX_VISUAL_PARTICLES),
            style: Vec::with_capacity(MAX_VISUAL_PARTICLES),
        }
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }

    fn swap_remove(&mut self, index: usize) {
        self.x.swap_remove(index);
        self.y.swap_remove(index);
        self.vx.swap_remove(index);
        self.vy.swap_remove(index);
        self.radius.swap_remove(index);
        self.life.swap_remove(index);
        self.lifetime.swap_remove(index);
        self.phase.swap_remove(index);
        self.color.swap_remove(index);
        self.style.swap_remove(index);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellTargetKind {
    Food,
    Cell,
}

#[derive(Clone, Copy, Debug)]
pub struct CellTarget {
    pub kind: CellTargetKind,
    pub index: usize,
    pub position: Vec2,
    pub distance_squared: f32,
    pub remembered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodShape {
    Blob,
    Circle,
    Square,
    Triangle,
    Diamond,
    Star,
    Pebble,
}

impl FoodShape {
    pub fn shader_shape(self) -> f32 {
        match self {
            FoodShape::Blob => 0.0,
            FoodShape::Circle => 1.0,
            FoodShape::Square => 2.0,
            FoodShape::Triangle => 3.0,
            FoodShape::Diamond => 4.0,
            FoodShape::Star => 5.0,
            FoodShape::Pebble => 6.0,
        }
    }

    fn random(rng: &mut SmallRng) -> Self {
        match rng.random_range(0..7) {
            0 => FoodShape::Blob,
            1 => FoodShape::Circle,
            2 => FoodShape::Square,
            3 => FoodShape::Triangle,
            4 => FoodShape::Diamond,
            5 => FoodShape::Star,
            _ => FoodShape::Pebble,
        }
    }

    fn random_feeder_food(rng: &mut SmallRng) -> Self {
        match rng.random_range(0..5) {
            0 => FoodShape::Blob,
            1 => FoodShape::Circle,
            2 => FoodShape::Diamond,
            3 => FoodShape::Star,
            _ => FoodShape::Pebble,
        }
    }
}

impl FoodKind {
    pub fn shader_kind(self) -> f32 {
        match self {
            FoodKind::Grass => 0.0,
            FoodKind::Meat => -1.0,
        }
    }
}

impl WorldState {
    pub fn new(config: &SimConfig) -> Self {
        let mut rng = SmallRng::seed_from_u64(config.seed);
        let food_grower_count = config.food_growers.max(1);
        let floor_food_count = floor_food_count(config.food);
        let feeder_food_capacity =
            feeder_food_capacity(config.food, floor_food_count, food_grower_count);
        let mut cells = CellStore::new(
            config.cells,
            config.width,
            config.height,
            config.arena_shape,
            config.random_cell_geometry,
            config.segmented_cells,
            &config.cell_shape_weights,
            &mut rng,
        );
        cells.refresh_taxonomy();
        let food = FoodStore::new(
            floor_food_count,
            config.width,
            config.height,
            config.arena_shape,
            &mut rng,
        );
        let obstacles = ObstacleStore::new(
            config.obstacles,
            config.width,
            config.height,
            config.arena_shape,
            &mut rng,
        );
        let food_growers = FoodGrowerStore::new(
            food_grower_count,
            config.width,
            config.height,
            config.arena_shape,
            &mut rng,
        );
        let mut grid = SpatialGrid::new(config.width, config.height, GRID_CELL_SIZE);
        grid.rebuild(&food);
        let mut cell_grid = CellGrid::new(config.width, config.height, CELL_GRID_SIZE);
        cell_grid.rebuild(&cells);
        let mut obstacle_grid = CellGrid::new(config.width, config.height, OBSTACLE_GRID_SIZE);
        obstacle_grid.rebuild_points(&obstacles.x, &obstacles.y);
        let max_obstacle_radius = obstacles.radius.iter().copied().fold(0.0, f32::max);

        let mut world = Self {
            cells,
            food,
            obstacles,
            food_growers,
            visual_particles: VisualParticleStore::new(),
            width: config.width,
            height: config.height,
            arena_shape: config.arena_shape,
            grid,
            cell_grid,
            cell_grid_dirty: false,
            obstacle_grid,
            max_obstacle_radius,
            collision_pairs: Vec::with_capacity(config.cells.saturating_mul(4)),
            collision_bounds: Vec::with_capacity(config.cells),
            rng,
            elapsed: 0.0,
            max_feeder_food: feeder_food_capacity,
            max_carrion: config.cells.saturating_mul(2).clamp(8, 5_000),
            collision_stiffness: config.collision_stiffness.max(0.0),
            collision_damping: config.collision_damping.max(0.0),
            cell_sound_events: Vec::new(),
            energy_flow: EnergyFlowStats::default(),
            cell_count_delta: 0,
            food_count_delta: 0,
            energy_accumulator: EnergyFlowAccumulator::default(),
            count_baseline: (0, 0),
        };
        world.relocate_world_food_away_from_solids();
        world.seed_feeder_food(feeder_food_capacity);
        world.grid.rebuild(&world.food);
        world.energy_accumulator = EnergyFlowAccumulator::default();
        world.count_baseline = (world.cells.len(), world.food.active_count());

        world
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 1.0 / 20.0);
        self.elapsed += dt;
        self.update_visual_particles(dt);
        self.remove_dead_cells();
        if self.cell_grid_dirty {
            self.cell_grid.rebuild(&self.cells);
            self.cell_grid_dirty = false;
        }
        self.advect_obstacles(dt);
        self.obstacle_grid
            .rebuild_points(&self.obstacles.x, &self.obstacles.y);
        self.advect_food_growers(dt);
        self.resolve_obstacle_food_growers();
        self.decay_food(dt);
        self.grow_wild_food(dt);
        self.grow_food(dt);
        self.advect_food(dt);
        self.push_food_from_obstacles(dt);
        self.push_food_from_food_growers(dt);
        self.clamp_free_food_to_arena();
        self.grid.rebuild(&self.food);
        self.decay_visuals(dt);
        self.cells.relax_soft_body(dt);

        for i in 0..self.cells.len() {
            if self.cells.viability[i] <= 0.0 {
                continue;
            }
            self.cells.lysis_cooldown[i] = (self.cells.lysis_cooldown[i] - dt).max(0.0);
            for section in 0..self.cells.section_count[i] as usize {
                self.cells.lysis_deform_time[i][section] =
                    (self.cells.lysis_deform_time[i][section] - dt).max(0.0);
            }
            self.cells.hunt_pause[i] = (self.cells.hunt_pause[i] - dt).max(0.0);
            self.cells.hunt_recheck[i] = (self.cells.hunt_recheck[i] - dt).max(0.0);
            let x = self.cells.x[i];
            let y = self.cells.y[i];
            let speed = self.effective_cell_speed(i);
            let collision_bound = self.cells.collision_bound_radius(i);
            let target = self.update_cell_target(i, dt);

            let (desired_x, desired_y) = if let Some(target) = target {
                let delta = target.position - Vec2::new(x, y);
                let inv_len = target.distance_squared.max(0.0001).sqrt().recip();
                (delta.x * inv_len * speed, delta.y * inv_len * speed)
            } else {
                let phase = ((i as f32 * 12.9898 + x * 0.017 + y * 0.011).sin()) * WANDER_GAIN;
                let (s, c) = phase.sin_cos();
                (
                    (self.cells.vx[i] * c - self.cells.vy[i] * s).clamp(-speed, speed),
                    (self.cells.vx[i] * s + self.cells.vy[i] * c).clamp(-speed, speed),
                )
            };

            let desired_velocity = (Vec2::new(desired_x, desired_y)
                + self.cell_avoidance_velocity(
                    i,
                    Vec2::new(desired_x, desired_y),
                    speed,
                    collision_bound,
                ))
            .clamp_length_max(speed * 1.2);
            let current = liquid_current_at(Vec2::new(x, y), self.elapsed) * CELL_CURRENT_SPEED;
            self.drive_cell(i, desired_velocity, current, dt);

            let wake_speed = Vec2::new(self.cells.vx[i], self.cells.vy[i]).length();
            let wake_target = ((wake_speed - 4.0) / (speed * 0.48).max(12.0)).clamp(0.0, 1.0);
            let wake_follow_speed = if wake_target > self.cells.wake_strength[i] {
                2.4
            } else {
                5.0
            };
            let wake_follow = 1.0 - (-wake_follow_speed * dt).exp();
            self.cells.wake_strength[i] +=
                (wake_target - self.cells.wake_strength[i]) * wake_follow;

            self.cells.x[i] += self.cells.vx[i] * dt;
            self.cells.y[i] += self.cells.vy[i] * dt;

            self.bounce_cell(i);
            self.resolve_cell_obstacles(i, collision_bound);
            self.resolve_cell_food_growers(i, collision_bound);

            if let Some(CellTarget {
                kind: CellTargetKind::Food,
                index: food_index,
                remembered: false,
                ..
            }) = target
            {
                if cell_body_overlaps_circle(
                    &self.cells,
                    i,
                    Vec2::new(self.food.x[food_index], self.food.y[food_index]),
                    FOOD_RADIUS,
                ) && self.cells.viability[i] < self.cells.max_viability[i] - HUNGER_EPSILON
                {
                    let eaten_position =
                        Vec2::new(self.food.x[food_index], self.food.y[food_index]);
                    let eaten_kind = self.food.kind[food_index];
                    let eater_velocity = Vec2::new(self.cells.vx[i], self.cells.vy[i]);
                    self.spawn_food_particles(eaten_position, eaten_kind, eater_velocity);
                    let raw_food_energy =
                        self.food.energy[food_index] * self.food.growth[food_index].clamp(0.0, 1.0);
                    let food_energy = digested_food_energy(
                        eaten_kind,
                        raw_food_energy,
                        self.cells.aggressiveness[i],
                    );
                    let viability_before = self.cells.viability[i];
                    if self.food.is_feeder_food(food_index) {
                        self.clear_branchlet_food_association(food_index);
                    }
                    self.food.deactivate(food_index);
                    self.cells.add_viability(i, food_energy);
                    self.energy_accumulator.values.food_consumed +=
                        self.cells.viability[i] - viability_before;
                    self.cells.target_food[i] = -1;
                    self.cells.target_memory[i] = 0.0;
                    self.cells.target_search_failed[i] = false;
                    self.cell_sound_events
                        .push(Vec2::new(self.cells.x[i], self.cells.y[i]));
                }
            }

            if let Some(CellTarget {
                kind: CellTargetKind::Cell,
                index: victim_index,
                remembered: false,
                ..
            }) = target
            {
                self.try_lysis_attack(i, victim_index);
            }
        }
        self.update_tail_sections(dt);

        self.solve_cell_collisions(dt);
        self.decay_viability(dt);
        self.advance_mitosis(dt);
        self.process_cell_lifecycle();
        self.finish_energy_flow_window(dt);
    }

    fn finish_energy_flow_window(&mut self, dt: f32) {
        self.energy_accumulator.elapsed += dt;
        if self.energy_accumulator.elapsed < 0.5 {
            return;
        }
        let inv_window = self.energy_accumulator.elapsed.recip();
        let sample = EnergyFlowStats {
            wild_food_input: self.energy_accumulator.values.wild_food_input * inv_window,
            feeder_input: self.energy_accumulator.values.feeder_input * inv_window,
            carrion_transfer: self.energy_accumulator.values.carrion_transfer * inv_window,
            food_consumed: self.energy_accumulator.values.food_consumed * inv_window,
            metabolism: self.energy_accumulator.values.metabolism * inv_window,
            spoilage: self.energy_accumulator.values.spoilage * inv_window,
            mitosis_cost: self.energy_accumulator.values.mitosis_cost * inv_window,
            lysis_loss: self.energy_accumulator.values.lysis_loss * inv_window,
        };
        let follow = if self.energy_flow.metabolism <= 0.0 {
            1.0
        } else {
            0.32
        };
        self.energy_flow.wild_food_input +=
            (sample.wild_food_input - self.energy_flow.wild_food_input) * follow;
        self.energy_flow.feeder_input +=
            (sample.feeder_input - self.energy_flow.feeder_input) * follow;
        self.energy_flow.carrion_transfer +=
            (sample.carrion_transfer - self.energy_flow.carrion_transfer) * follow;
        self.energy_flow.food_consumed +=
            (sample.food_consumed - self.energy_flow.food_consumed) * follow;
        self.energy_flow.metabolism += (sample.metabolism - self.energy_flow.metabolism) * follow;
        self.energy_flow.spoilage += (sample.spoilage - self.energy_flow.spoilage) * follow;
        self.energy_flow.mitosis_cost +=
            (sample.mitosis_cost - self.energy_flow.mitosis_cost) * follow;
        self.energy_flow.lysis_loss += (sample.lysis_loss - self.energy_flow.lysis_loss) * follow;
        let counts = (self.cells.len(), self.food.active_count());
        self.cell_count_delta = signed_count_delta(counts.0, self.count_baseline.0);
        self.food_count_delta = signed_count_delta(counts.1, self.count_baseline.1);
        self.count_baseline = counts;
        self.energy_accumulator = EnergyFlowAccumulator::default();
    }

    fn spawn_food_particles(&mut self, position: Vec2, kind: FoodKind, eater_velocity: Vec2) {
        let available = MAX_VISUAL_PARTICLES.saturating_sub(self.visual_particles.len());
        let count = FOOD_PARTICLES_PER_BITE.min(available);
        let color = match kind {
            FoodKind::Grass => [0.46, 1.0, 0.42, 1.0],
            FoodKind::Meat => [1.0, 0.38, 0.30, 1.0],
        };
        for particle_index in 0..count {
            let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
            let direction = Vec2::from_angle(angle);
            let speed = if particle_index < 2 {
                self.rng.random_range(70.0..112.0)
            } else {
                self.rng.random_range(28.0..76.0)
            };
            let lifetime = self.rng.random_range(0.36..0.72);
            let offset = direction * self.rng.random_range(0.0..FOOD_RADIUS * 0.65);
            let velocity = direction * speed + eater_velocity * 0.16;
            self.visual_particles.x.push(position.x + offset.x);
            self.visual_particles.y.push(position.y + offset.y);
            self.visual_particles.vx.push(velocity.x);
            self.visual_particles.vy.push(velocity.y);
            self.visual_particles.radius.push(if particle_index < 2 {
                self.rng.random_range(1.4..2.4)
            } else {
                self.rng.random_range(0.65..1.45)
            });
            self.visual_particles.life.push(lifetime);
            self.visual_particles.lifetime.push(lifetime);
            self.visual_particles
                .phase
                .push(self.rng.random_range(0.0..std::f32::consts::TAU));
            self.visual_particles.color.push(color);
            self.visual_particles.style.push(0.0);
        }
    }

    fn spawn_mitosis_particles(&mut self, cell_index: usize, requested: usize, burst: f32) {
        let available = MAX_VISUAL_PARTICLES.saturating_sub(self.visual_particles.len());
        let count = requested.min(available);
        if count == 0 || cell_index >= self.cells.len() {
            return;
        }

        let center = Vec2::new(self.cells.x[cell_index], self.cells.y[cell_index]);
        let cell_velocity = Vec2::new(self.cells.vx[cell_index], self.cells.vy[cell_index]);
        let axis = Vec2::from_angle(self.cells.heading[cell_index] + std::f32::consts::FRAC_PI_2);
        let normal = Vec2::new(-axis.y, axis.x);
        let radius = self.cells.max_base_radius(cell_index);
        let base_color = species_color(self.cells.species[cell_index], 1.0);
        let color = [
            base_color[0] * 0.52 + 0.48,
            base_color[1] * 0.52 + 0.48,
            base_color[2] * 0.52 + 0.48,
            1.0,
        ];

        for _ in 0..count {
            let along = self.rng.random_range(-1.0..1.0);
            let sideways = self.rng.random_range(-0.72..0.72);
            let direction = (axis * along + normal * sideways)
                .try_normalize()
                .unwrap_or(axis);
            let offset = axis * self.rng.random_range(-radius * 0.34..radius * 0.34)
                + normal * self.rng.random_range(-radius * 0.12..radius * 0.12);
            let speed = self.rng.random_range(18.0..58.0) * burst;
            let lifetime = self.rng.random_range(0.48..0.92);
            let velocity = direction * speed + cell_velocity * 0.12;
            self.visual_particles.x.push(center.x + offset.x);
            self.visual_particles.y.push(center.y + offset.y);
            self.visual_particles.vx.push(velocity.x);
            self.visual_particles.vy.push(velocity.y);
            self.visual_particles
                .radius
                .push(self.rng.random_range(0.75..1.85) * (0.82 + burst * 0.18));
            self.visual_particles.life.push(lifetime);
            self.visual_particles.lifetime.push(lifetime);
            self.visual_particles
                .phase
                .push(self.rng.random_range(0.0..std::f32::consts::TAU));
            self.visual_particles.color.push(color);
            self.visual_particles.style.push(1.0);
        }
    }

    fn spawn_lysis_particles(
        &mut self,
        contact: Vec2,
        normal: Vec2,
        victim: usize,
        victim_section: u8,
    ) {
        let available = MAX_VISUAL_PARTICLES.saturating_sub(self.visual_particles.len());
        let count = LYSIS_PARTICLES_PER_HIT.min(available);
        if count == 0 {
            return;
        }

        let victim_velocity = self.cells.section_velocity(victim, victim_section);
        let color = cell_display_color(
            self.cells.species[victim],
            self.cells.viability_ratio(victim),
            self.cells.aggressiveness[victim],
            self.cells.lysis[victim],
        );
        let tangent = Vec2::new(-normal.y, normal.x);
        for particle_index in 0..count {
            let spread = self.rng.random_range(-0.92..0.92);
            let direction = (normal * self.rng.random_range(0.55..1.0) + tangent * spread)
                .try_normalize()
                .unwrap_or(normal);
            let speed = if particle_index < 2 {
                self.rng.random_range(78.0..126.0)
            } else {
                self.rng.random_range(34.0..88.0)
            };
            let lifetime = self.rng.random_range(0.30..0.62);
            let offset = direction * self.rng.random_range(0.0..2.5);
            let velocity = direction * speed + victim_velocity * 0.12;
            self.visual_particles.x.push(contact.x + offset.x);
            self.visual_particles.y.push(contact.y + offset.y);
            self.visual_particles.vx.push(velocity.x);
            self.visual_particles.vy.push(velocity.y);
            self.visual_particles.radius.push(if particle_index < 2 {
                self.rng.random_range(1.35..2.25)
            } else {
                self.rng.random_range(0.65..1.40)
            });
            self.visual_particles.life.push(lifetime);
            self.visual_particles.lifetime.push(lifetime);
            self.visual_particles
                .phase
                .push(self.rng.random_range(0.0..std::f32::consts::TAU));
            self.visual_particles.color.push(color);
            self.visual_particles.style.push(2.0);
        }
    }

    fn update_visual_particles(&mut self, dt: f32) {
        let mut index = 0;
        while index < self.visual_particles.len() {
            self.visual_particles.life[index] -= dt;
            if self.visual_particles.life[index] <= 0.0 {
                self.visual_particles.swap_remove(index);
                continue;
            }

            let position = Vec2::new(
                self.visual_particles.x[index],
                self.visual_particles.y[index],
            );
            let current = liquid_current_at(position, self.elapsed) * 0.16;
            let damping = (1.0 - dt * 3.8).max(0.0);
            self.visual_particles.vx[index] =
                self.visual_particles.vx[index] * damping + current.x * dt;
            self.visual_particles.vy[index] =
                self.visual_particles.vy[index] * damping + current.y * dt;
            self.visual_particles.x[index] += self.visual_particles.vx[index] * dt;
            self.visual_particles.y[index] += self.visual_particles.vy[index] * dt;
            index += 1;
        }
    }

    fn effective_cell_speed(&self, cell_index: usize) -> f32 {
        let mitosis_slowdown = if self.cells.mitosis_progress[cell_index] > 0.0 {
            0.28
        } else {
            1.0
        };
        self.cells.speed[cell_index]
            * (MIN_VIABILITY_MOVE_FACTOR + self.cells.viability_ratio(cell_index) * 0.72)
            * self.cells.morphology_speed_factor(cell_index)
            * mitosis_slowdown
    }

    fn is_lysis_capable(&self, cell_index: usize) -> bool {
        self.cells.lysis[cell_index] >= LYSIS_ACTIVE_THRESHOLD
    }

    fn best_lysis_target(&self, attacker: usize) -> Option<(usize, f32)> {
        if !self.is_lysis_capable(attacker) {
            return None;
        }
        let position = Vec2::new(self.cells.x[attacker], self.cells.y[attacker]);
        let perception = self.cells.perception[attacker].max(0.0);
        let aggression =
            (self.cells.aggressiveness[attacker] / CELL_AGGRESSIVENESS_DISPLAY_MAX).clamp(0.0, 1.0);
        let hunger = 1.0 - self.cells.viability_ratio(attacker);
        let attack_drive = aggression * (0.58 + hunger * 0.42);
        if attack_drive < 0.10 {
            return None;
        }

        let (min_x, max_x, min_y, max_y) = self.cell_grid.bucket_range(position, perception);
        let radius_sq = perception * perception;
        let attacker_size = self.cells.collision_bound_radius(attacker).max(0.1);
        let mut best = None;
        let mut best_score = 0.34;
        for gy in min_y..=max_y {
            for gx in min_x..=max_x {
                for &candidate in &self.cell_grid.buckets[gy * self.cell_grid.cols + gx] {
                    if candidate == attacker || self.cells.viability[candidate] <= 0.0 {
                        continue;
                    }
                    let target_position =
                        Vec2::new(self.cells.x[candidate], self.cells.y[candidate]);
                    let distance_squared = position.distance_squared(target_position);
                    if distance_squared > radius_sq {
                        continue;
                    }
                    let proximity = 1.0 - distance_squared.sqrt() / perception.max(0.001);
                    let vulnerability = 1.0 - self.cells.viability_ratio(candidate);
                    let prey_size = self.cells.collision_bound_radius(candidate).max(0.1);
                    let size_advantage =
                        ((attacker_size / prey_size - 0.72) / 0.85).clamp(0.0, 1.0);
                    let retaliation =
                        (self.cells.lysis[candidate] / CELL_LYSIS_DISPLAY_MAX).clamp(0.0, 1.0);
                    let score = attack_drive * 0.54
                        + proximity * 0.20
                        + vulnerability * 0.18
                        + size_advantage * 0.16
                        - retaliation * (0.08 + (1.0 - aggression) * 0.16);
                    if score > best_score {
                        best_score = score;
                        best = Some((candidate, distance_squared.max(0.0001)));
                    }
                }
            }
        }
        best
    }

    fn update_lysis_target(&mut self, attacker: usize, dt: f32) -> Option<CellTarget> {
        if !self.is_lysis_capable(attacker) || self.cells.hunt_pause[attacker] > 0.0 {
            return None;
        }
        let position = Vec2::new(self.cells.x[attacker], self.cells.y[attacker]);
        let perception_sq = self.cells.perception[attacker].powi(2);
        let persistence = (self.cells.persistence[attacker] / PERSISTENCE_GENE_MAX).clamp(0.0, 1.0);
        let memory_duration = 0.20 + persistence * 2.80;
        let stored = self.cells.target_cell[attacker];
        if stored >= 0 {
            let victim = stored as usize;
            let valid = victim < self.cells.len()
                && victim != attacker
                && self.cells.id[victim] == self.cells.target_cell_id[attacker]
                && self.cells.viability[victim] > 0.0;
            if valid {
                let target_position = Vec2::new(self.cells.x[victim], self.cells.y[victim]);
                let distance_squared = position.distance_squared(target_position);
                if distance_squared <= perception_sq {
                    self.cells.target_last_x[attacker] = target_position.x;
                    self.cells.target_last_y[attacker] = target_position.y;
                    self.cells.target_memory[attacker] = memory_duration;
                    return Some(CellTarget {
                        kind: CellTargetKind::Cell,
                        index: victim,
                        position: target_position,
                        distance_squared,
                        remembered: false,
                    });
                }
                self.cells.target_memory[attacker] -= dt;
                if self.cells.target_memory[attacker] > 0.0 {
                    let last = Vec2::new(
                        self.cells.target_last_x[attacker],
                        self.cells.target_last_y[attacker],
                    );
                    return Some(CellTarget {
                        kind: CellTargetKind::Cell,
                        index: victim,
                        position: last,
                        distance_squared: position.distance_squared(last).max(0.0001),
                        remembered: true,
                    });
                }
            } else {
                self.cells.hunt_pause[attacker] = LYSIS_HUNT_PAUSE_AFTER_KILL;
            }
            self.cells.target_cell[attacker] = -1;
            self.cells.target_cell_id[attacker] = NO_CELL_TARGET;
            self.cells.target_memory[attacker] = 0.0;
        }

        if self.cells.hunt_recheck[attacker] > 0.0 {
            return None;
        }
        let aggression =
            (self.cells.aggressiveness[attacker] / CELL_AGGRESSIVENESS_DISPLAY_MAX).clamp(0.0, 1.0);
        let stagger = (self.cells.id[attacker] % 17) as f32 * 0.004;
        self.cells.hunt_recheck[attacker] = (LYSIS_TARGET_RECHECK_MAX
            + (LYSIS_TARGET_RECHECK_MIN - LYSIS_TARGET_RECHECK_MAX) * aggression
            + stagger)
            .clamp(LYSIS_TARGET_RECHECK_MIN, LYSIS_TARGET_RECHECK_MAX);
        let (victim, distance_squared) = self.best_lysis_target(attacker)?;
        let target_position = Vec2::new(self.cells.x[victim], self.cells.y[victim]);
        self.cells.target_cell[attacker] = victim as i32;
        self.cells.target_cell_id[attacker] = self.cells.id[victim];
        self.cells.target_food[attacker] = -1;
        self.cells.target_last_x[attacker] = target_position.x;
        self.cells.target_last_y[attacker] = target_position.y;
        self.cells.target_memory[attacker] = memory_duration;
        Some(CellTarget {
            kind: CellTargetKind::Cell,
            index: victim,
            position: target_position,
            distance_squared,
            remembered: false,
        })
    }

    fn try_lysis_attack(&mut self, attacker: usize, victim: usize) -> bool {
        if attacker >= self.cells.len()
            || victim >= self.cells.len()
            || attacker == victim
            || !self.is_lysis_capable(attacker)
            || self.cells.lysis_cooldown[attacker] > 0.0
            || self.cells.viability[attacker] <= 0.0
            || self.cells.viability[victim] <= 0.0
        {
            return false;
        }

        let (base_damage, self_cost, cooldown, reach) =
            lysis_combat_profile(self.cells.lysis[attacker]);
        let Some(contact) = compound_cells_lysis_contact(&self.cells, attacker, victim, reach)
        else {
            return false;
        };
        let damage = base_damage
            * lysis_size_damage_multiplier(
                self.cells.biomass_sum(attacker),
                self.cells.biomass_sum(victim),
            );
        let victim_before = self.cells.viability[victim];
        let attacker_before = self.cells.viability[attacker];
        self.cells.viability[victim] = (self.cells.viability[victim] - damage).max(0.0);
        self.cells.viability[attacker] = (self.cells.viability[attacker] - self_cost).max(0.0);
        self.energy_accumulator.values.lysis_loss += victim_before - self.cells.viability[victim]
            + attacker_before
            - self.cells.viability[attacker];
        self.cells.lysis_cooldown[attacker] = cooldown;
        self.cells.begin_lysis_deformation(
            attacker,
            contact.section_a,
            contact.normal,
            LYSIS_ATTACK_DEFORM_DURATION,
            0.58,
        );
        self.cells.begin_lysis_deformation(
            victim,
            contact.section_b,
            -contact.normal,
            LYSIS_HIT_DEFORM_DURATION,
            -0.34,
        );
        self.spawn_lysis_particles(contact.point, contact.normal, victim, contact.section_b);
        self.cells.jelly_intensity[attacker] =
            (self.cells.jelly_intensity[attacker] + 0.16).min(1.0);
        self.cells.jelly_intensity[victim] = (self.cells.jelly_intensity[victim] + 0.38).min(1.0);
        if self.cells.viability[victim] <= 0.0 {
            self.cells.target_cell[attacker] = -1;
            self.cells.target_cell_id[attacker] = NO_CELL_TARGET;
            self.cells.hunt_pause[attacker] = LYSIS_HUNT_PAUSE_AFTER_KILL;
        }
        true
    }

    pub fn cell_target(&self, cell_index: usize) -> Option<CellTarget> {
        if cell_index >= self.cells.len() {
            return None;
        }
        let victim = self.cells.target_cell[cell_index];
        if victim >= 0 {
            let victim = victim as usize;
            if victim < self.cells.len()
                && self.cells.id[victim] == self.cells.target_cell_id[cell_index]
                && self.cells.viability[victim] > 0.0
            {
                let position = Vec2::new(self.cells.x[cell_index], self.cells.y[cell_index]);
                let target_position = Vec2::new(self.cells.x[victim], self.cells.y[victim]);
                return Some(CellTarget {
                    kind: CellTargetKind::Cell,
                    index: victim,
                    position: target_position,
                    distance_squared: position.distance_squared(target_position),
                    remembered: false,
                });
            }
        }
        if self.cells.viability[cell_index] >= self.cells.max_viability[cell_index] - HUNGER_EPSILON
        {
            return None;
        }

        let position = Vec2::new(self.cells.x[cell_index], self.cells.y[cell_index]);
        let target_index = self.cells.target_food[cell_index];
        if target_index >= 0 {
            let index = target_index as usize;
            let same_food = index < self.food.len()
                && self.food.active[index]
                && self.cells.target_food_generation[cell_index] == self.food.generation[index]
                && self.food_is_edible_for_cell(cell_index, index);
            if same_food {
                let target_position = Vec2::new(self.food.x[index], self.food.y[index]);
                let distance_squared = position.distance_squared(target_position);
                if distance_squared <= self.cells.perception[cell_index].powi(2) {
                    return Some(CellTarget {
                        kind: CellTargetKind::Food,
                        index,
                        position: target_position,
                        distance_squared,
                        remembered: false,
                    });
                }
            }
            if same_food && self.cells.target_memory[cell_index] > 0.0 {
                let target_position = Vec2::new(
                    self.cells.target_last_x[cell_index],
                    self.cells.target_last_y[cell_index],
                );
                return Some(CellTarget {
                    kind: CellTargetKind::Food,
                    index,
                    position: target_position,
                    distance_squared: position.distance_squared(target_position),
                    remembered: true,
                });
            }
        }
        self.nearest_edible_food(cell_index, position, self.cells.perception[cell_index])
            .map(|(index, dx, dy, distance_squared)| CellTarget {
                kind: CellTargetKind::Food,
                index,
                position: position + Vec2::new(dx, dy),
                distance_squared,
                remembered: false,
            })
    }

    fn food_is_edible_for_cell(&self, cell_index: usize, food_index: usize) -> bool {
        self.food.kind[food_index] != FoodKind::Meat
            || self.food.origin_species[food_index] < 0
            || self.food.origin_species[food_index] as u32 != self.cells.species[cell_index]
    }

    fn nearest_edible_food(
        &self,
        cell_index: usize,
        position: Vec2,
        perception: f32,
    ) -> Option<(usize, f32, f32, f32)> {
        self.grid.nearest_food_filtered(
            position.x,
            position.y,
            &self.food,
            perception,
            Some(self.cells.species[cell_index]),
        )
    }

    fn update_cell_target(&mut self, cell_index: usize, dt: f32) -> Option<CellTarget> {
        if let Some(target) = self.update_lysis_target(cell_index, dt) {
            return Some(target);
        }
        if self.cells.viability[cell_index] >= self.cells.max_viability[cell_index] - HUNGER_EPSILON
        {
            self.cells.target_food[cell_index] = -1;
            self.cells.target_memory[cell_index] = 0.0;
            self.cells.target_search_failed[cell_index] = false;
            return None;
        }

        let position = Vec2::new(self.cells.x[cell_index], self.cells.y[cell_index]);
        let perception = self.cells.perception[cell_index];
        let persistence =
            (self.cells.persistence[cell_index] / PERSISTENCE_GENE_MAX).clamp(0.0, 1.0);
        let memory_duration = 0.20 + persistence * 2.80;
        let recheck_interval = 0.08 + persistence * 0.28;
        let switch_ratio = 0.82 - persistence * 0.37;
        self.cells.target_recheck[cell_index] -= dt;

        let stored = self.cells.target_food[cell_index];
        if stored >= 0 {
            let index = stored as usize;
            let same_food = index < self.food.len()
                && self.food.active[index]
                && self.cells.target_food_generation[cell_index] == self.food.generation[index]
                && self.food_is_edible_for_cell(cell_index, index);
            if same_food {
                let target_position = Vec2::new(self.food.x[index], self.food.y[index]);
                let distance_squared = position.distance_squared(target_position);
                if distance_squared <= perception * perception {
                    self.cells.target_last_x[cell_index] = target_position.x;
                    self.cells.target_last_y[cell_index] = target_position.y;
                    self.cells.target_memory[cell_index] = memory_duration;
                    self.cells.target_search_failed[cell_index] = false;
                    if self.cells.target_recheck[cell_index] <= 0.0 {
                        self.cells.target_recheck[cell_index] = recheck_interval;
                        if let Some((new_index, dx, dy, new_distance_squared)) =
                            self.nearest_edible_food(cell_index, position, perception)
                        {
                            if new_index != index
                                && new_distance_squared
                                    < distance_squared * switch_ratio * switch_ratio
                            {
                                self.cells.target_food[cell_index] = new_index as i32;
                                self.cells.target_food_generation[cell_index] =
                                    self.food.generation[new_index];
                                self.cells.target_last_x[cell_index] = position.x + dx;
                                self.cells.target_last_y[cell_index] = position.y + dy;
                                return Some(CellTarget {
                                    kind: CellTargetKind::Food,
                                    index: new_index,
                                    position: position + Vec2::new(dx, dy),
                                    distance_squared: new_distance_squared,
                                    remembered: false,
                                });
                            }
                        }
                    }
                    return Some(CellTarget {
                        kind: CellTargetKind::Food,
                        index,
                        position: target_position,
                        distance_squared,
                        remembered: false,
                    });
                }
            }

            if same_food {
                self.cells.target_memory[cell_index] -= dt;
            } else {
                self.cells.target_memory[cell_index] = 0.0;
            }
            if same_food && self.cells.target_memory[cell_index] > 0.0 {
                let last = Vec2::new(
                    self.cells.target_last_x[cell_index],
                    self.cells.target_last_y[cell_index],
                );
                return Some(CellTarget {
                    kind: CellTargetKind::Food,
                    index,
                    position: last,
                    distance_squared: position.distance_squared(last).max(0.0001),
                    remembered: true,
                });
            }
            self.cells.target_food[cell_index] = -1;
        }

        if self.cells.target_search_failed[cell_index]
            && self.cells.target_recheck[cell_index] > 0.0
        {
            return None;
        }

        let Some((index, dx, dy, distance_squared)) =
            self.nearest_edible_food(cell_index, position, perception)
        else {
            let stagger = (self.cells.id[cell_index] % 19) as f32 * 0.006;
            self.cells.target_search_failed[cell_index] = true;
            self.cells.target_recheck[cell_index] =
                (recheck_interval * 1.35 + stagger).clamp(0.12, 0.64);
            return None;
        };
        let target_position = position + Vec2::new(dx, dy);
        self.cells.target_search_failed[cell_index] = false;
        self.cells.target_food[cell_index] = index as i32;
        self.cells.target_food_generation[cell_index] = self.food.generation[index];
        self.cells.target_last_x[cell_index] = target_position.x;
        self.cells.target_last_y[cell_index] = target_position.y;
        self.cells.target_memory[cell_index] = memory_duration;
        self.cells.target_recheck[cell_index] = recheck_interval;
        Some(CellTarget {
            kind: CellTargetKind::Food,
            index,
            position: target_position,
            distance_squared,
            remembered: false,
        })
    }

    fn drive_cell(&mut self, cell_index: usize, desired_velocity: Vec2, current: Vec2, dt: f32) {
        let current_heading = self.cells.heading[cell_index];
        let forward = Vec2::new(current_heading.cos(), current_heading.sin());
        let desired_dir = desired_velocity.try_normalize().unwrap_or(forward);
        let desired_angle = desired_dir.y.atan2(desired_dir.x);
        let alignment = forward.dot(desired_dir);
        let velocity = Vec2::new(self.cells.vx[cell_index], self.cells.vy[cell_index]);
        let drive_speed = self.effective_cell_speed(cell_index);
        let controlled_forward_speed = (velocity - current).dot(forward);
        let stalled = alignment < STUCK_ALIGNMENT
            && controlled_forward_speed.abs() < drive_speed * STUCK_SPEED_FACTOR;

        if self.cells.reverse_time[cell_index] > 0.0 {
            self.cells.reverse_time[cell_index] =
                (self.cells.reverse_time[cell_index] - dt).max(0.0);
        } else if stalled {
            self.cells.stuck_time[cell_index] += dt;
            if self.cells.stuck_time[cell_index] >= STUCK_REVERSE_DELAY {
                self.cells.reverse_time[cell_index] = EMERGENCY_REVERSE_DURATION;
                self.cells.stuck_time[cell_index] = 0.0;
            }
        } else {
            self.cells.stuck_time[cell_index] =
                (self.cells.stuck_time[cell_index] - dt * 2.0).max(0.0);
        }

        let reversing = self.cells.reverse_time[cell_index] > 0.0;
        let turn_delta = angle_delta(desired_angle, current_heading);
        let turn_step = self.cells.turn_speed[cell_index]
            * self.cells.morphology_turn_factor(cell_index)
            * self.cells.turn_agility_factor(cell_index, turn_delta)
            * CELL_TURN_RATE_MULTIPLIER
            * dt;
        let new_heading = wrap_angle(current_heading + turn_delta.clamp(-turn_step, turn_step));
        self.cells.heading[cell_index] = new_heading;

        let front = Vec2::new(new_heading.cos(), new_heading.sin());
        let abs_turn = turn_delta.abs();
        let throttle = if reversing {
            -EMERGENCY_REVERSE_THROTTLE
        } else if abs_turn > TURN_IN_PLACE_ANGLE {
            TURN_IN_PLACE_THROTTLE
        } else {
            1.0 - (abs_turn / TURN_IN_PLACE_ANGLE).clamp(0.0, 1.0) * 0.45
        };
        let target_velocity = front * drive_speed * throttle + current;
        let side = Vec2::new(-front.y, front.x);
        let acceleration_response = 1.0
            - (-CELL_ACCELERATION_GAIN
                * self.cells.morphology_acceleration_factor(cell_index)
                * dt)
                .exp();
        let lateral_grip =
            CELL_LATERAL_GRIP * (0.72 + self.cells.morphology_turn_factor(cell_index) * 0.28);
        let lateral_response = 1.0 - (-lateral_grip * dt).exp();
        let linear_drag = (-CELL_LINEAR_DRAG * dt).exp();

        let forward_speed = velocity.dot(front);
        let target_forward_speed = target_velocity.dot(front);
        let new_forward_speed =
            forward_speed + (target_forward_speed - forward_speed) * acceleration_response;

        let lateral_speed = velocity.dot(side);
        let target_lateral_speed = current.dot(side);
        let new_lateral_speed =
            lateral_speed + (target_lateral_speed - lateral_speed) * lateral_response;

        let new_velocity = (front * new_forward_speed + side * new_lateral_speed) * linear_drag;
        self.cells.vx[cell_index] = new_velocity.x;
        self.cells.vy[cell_index] = new_velocity.y;
    }

    fn update_tail_sections(&mut self, dt: f32) {
        for index in 0..self.cells.len() {
            if self.cells.section_count[index] < 2 {
                continue;
            }
            let heading = self.cells.heading[index];
            let spacing = self.cells.section_spacing[index];
            for section in 1..self.cells.section_count[index] {
                let parent_section = self.cells.section_parents[index][section as usize - 1];
                let parent = self.cells.section_center(index, parent_section);
                let parent_velocity = self.cells.section_velocity(index, parent_section);
                let position = self.cells.section_center(index, section);
                let velocity = self.cells.section_velocity(index, section);
                let direction = Vec2::from_angle(
                    heading + self.cells.section_angles[index][section as usize - 1],
                );
                let side = Vec2::new(-direction.y, direction.x);
                let desired = parent + direction * spacing;
                let position_error = desired - position;
                let velocity_error = parent_velocity - velocity;
                let spring =
                    direction * position_error.dot(direction) * TAIL_LONGITUDINAL_STIFFNESS
                        + side * position_error.dot(side) * TAIL_LATERAL_STIFFNESS;
                let damping = direction * velocity_error.dot(direction) * TAIL_LONGITUDINAL_DAMPING
                    + side * velocity_error.dot(side) * TAIL_LATERAL_DAMPING;
                let current = liquid_current_at(position, self.elapsed) * CELL_CURRENT_SPEED * 0.68;
                let mut new_velocity = velocity + (spring + damping + current) * dt;
                new_velocity = new_velocity
                    .clamp_length_max(self.effective_cell_speed(index) * 1.6 + CELL_CURRENT_SPEED);
                let mut new_position = position + new_velocity * dt;
                let from_parent = new_position - parent;
                let distance = from_parent.length();
                if distance > spacing * 1.42 && distance > 0.001 {
                    new_position = parent + from_parent * (spacing * 1.42 / distance);
                } else if distance < spacing * 0.50 && distance > 0.001 {
                    new_position = parent + from_parent * (spacing * 0.50 / distance);
                }
                bounce_point_in_arena(
                    &mut new_position.x,
                    &mut new_position.y,
                    &mut new_velocity.x,
                    &mut new_velocity.y,
                    self.width,
                    self.height,
                    self.arena_shape,
                    self.cells.section_collision_radius(index, section),
                );
                self.cells
                    .set_section_state(index, section, new_position, new_velocity);
            }
            for edge in 0..(self.cells.section_count[index] as usize - 1) {
                let child = edge as u8 + 1;
                let parent = self.cells.section_parents[index][edge];
                let parent_position = self.cells.section_center(index, parent);
                let child_position = self.cells.section_center(index, child);
                let axis = child_position - parent_position;
                let side = axis
                    .try_normalize()
                    .map(|direction| Vec2::new(-direction.y, direction.x))
                    .unwrap_or(Vec2::Y);
                let relative_velocity = self.cells.section_velocity(index, child)
                    - self.cells.section_velocity(index, parent);
                let idle =
                    (self.elapsed * 0.74 + self.cells.jelly_phase[index] + edge as f32 * 1.9).sin()
                        * spacing
                        * 0.10;
                self.cells.edge_curve_offsets[index][edge] = (relative_velocity.dot(side) * 0.045
                    + idle
                    + self.cells.section_angles[index][edge].sin() * spacing * 0.12)
                    .clamp(-spacing * 0.42, spacing * 0.42);
            }
            self.resolve_tail_solids(index);
        }
    }

    fn resolve_tail_solids(&mut self, cell_index: usize) {
        for section in 1..self.cells.section_count[cell_index] {
            let section_radius = self.cells.section_collision_radius(cell_index, section);
            for obstacle_index in 0..self.obstacles.len() {
                let center = Vec2::new(
                    self.obstacles.x[obstacle_index],
                    self.obstacles.y[obstacle_index],
                );
                self.push_section_from_solid(
                    cell_index,
                    section,
                    center,
                    self.obstacles.radius[obstacle_index] + section_radius,
                );
            }
            for grower_index in 0..self.food_growers.len() {
                let center = Vec2::new(
                    self.food_growers.x[grower_index],
                    self.food_growers.y[grower_index],
                );
                let before_push = self.cells.section_center(cell_index, section);
                let broad_radius = self.food_growers.extent_radius(grower_index) + section_radius;
                if before_push.distance_squared(center) > broad_radius * broad_radius {
                    continue;
                }
                self.push_section_from_solid(
                    cell_index,
                    section,
                    center,
                    self.food_growers.radius[grower_index] + section_radius,
                );
                for branch_index in self.food_growers.branch_range(grower_index) {
                    if !self.food_growers.branch_has_collision(branch_index) {
                        continue;
                    }
                    let position = self.cells.section_center(cell_index, section);
                    let (closest, t) = self
                        .food_growers
                        .closest_point_on_branch(branch_index, position);
                    let influence = self.food_growers.branch_collision_width_at(branch_index, t)
                        + section_radius;
                    self.push_section_from_solid(cell_index, section, closest, influence);
                }
            }
        }
    }

    fn push_section_from_solid(
        &mut self,
        cell_index: usize,
        section: u8,
        solid_center: Vec2,
        influence: f32,
    ) {
        let position = self.cells.section_center(cell_index, section);
        let delta = position - solid_center;
        let dist_sq = delta.length_squared();
        if dist_sq >= influence * influence {
            return;
        }
        let (normal, distance) = if dist_sq > 0.0001 {
            let distance = dist_sq.sqrt();
            (delta / distance, distance)
        } else {
            (Vec2::X, 0.001)
        };
        let penetration = influence - distance;
        let push = penetration
            .mul_add(SOFT_BODY_SOLID_PUSH_FACTOR, 0.0)
            .min(SOFT_BODY_SOLID_PUSH_MAX);
        let pushed_position = position + normal * push;
        let mut velocity = self.cells.section_velocity(cell_index, section);
        self.cells
            .compress_section_contact(cell_index, section, -normal, penetration * 0.45);
        let inward = velocity.dot(normal);
        if inward < 0.0 {
            velocity -= normal * inward * 1.15;
        }
        self.cells
            .set_section_state(cell_index, section, pushed_position, velocity);
    }

    fn advect_obstacles(&mut self, dt: f32) {
        for i in 0..self.obstacles.len() {
            let position = Vec2::new(self.obstacles.x[i], self.obstacles.y[i]);
            let current = liquid_current_at(position, self.elapsed + self.obstacles.phase[i])
                * OBSTACLE_CURRENT_SPEED;
            self.obstacles.vx[i] = self.obstacles.vx[i] * 0.985 + current.x * 0.015;
            self.obstacles.vy[i] = self.obstacles.vy[i] * 0.985 + current.y * 0.015;
            let velocity = Vec2::new(self.obstacles.vx[i], self.obstacles.vy[i])
                .clamp_length_max(OBSTACLE_CURRENT_SPEED);
            self.obstacles.vx[i] = velocity.x;
            self.obstacles.vy[i] = velocity.y;
            self.obstacles.x[i] += self.obstacles.vx[i] * dt;
            self.obstacles.y[i] += self.obstacles.vy[i] * dt;
            self.obstacles.rotation[i] += self.obstacles.spin[i] * dt;
            bounce_point_in_arena(
                &mut self.obstacles.x[i],
                &mut self.obstacles.y[i],
                &mut self.obstacles.vx[i],
                &mut self.obstacles.vy[i],
                self.width,
                self.height,
                self.arena_shape,
                self.obstacles.radius[i],
            );
        }
    }

    fn advect_food_growers(&mut self, dt: f32) {
        for i in 0..self.food_growers.len() {
            let extent = self.food_growers.extent_radius(i);
            let position = Vec2::new(self.food_growers.x[i], self.food_growers.y[i]);
            let current = liquid_current_at(position, self.elapsed + self.food_growers.phase[i])
                * GROWER_CURRENT_SPEED;
            self.food_growers.vx[i] = self.food_growers.vx[i] * 0.99 + current.x * 0.01;
            self.food_growers.vy[i] = self.food_growers.vy[i] * 0.99 + current.y * 0.01;
            self.food_growers.x[i] += self.food_growers.vx[i] * dt;
            self.food_growers.y[i] += self.food_growers.vy[i] * dt;
            self.food_growers.rotation[i] += self.food_growers.spin[i] * dt;
            bounce_point_in_arena(
                &mut self.food_growers.x[i],
                &mut self.food_growers.y[i],
                &mut self.food_growers.vx[i],
                &mut self.food_growers.vy[i],
                self.width,
                self.height,
                self.arena_shape,
                extent,
            );
        }

        self.food_growers
            .rebuild_branch_world_geometry(self.elapsed);
    }

    fn resolve_obstacle_food_growers(&mut self) {
        let mut moved_grower = false;

        for obstacle_index in 0..self.obstacles.len() {
            let obstacle_radius = self.obstacles.radius[obstacle_index];

            for grower_index in 0..self.food_growers.len() {
                let obstacle_pos = Vec2::new(
                    self.obstacles.x[obstacle_index],
                    self.obstacles.y[obstacle_index],
                );
                let grower_pos = Vec2::new(
                    self.food_growers.x[grower_index],
                    self.food_growers.y[grower_index],
                );
                let broad_phase =
                    self.food_growers.extent_radius(grower_index) + obstacle_radius + 10.0;
                if obstacle_pos.distance_squared(grower_pos) >= broad_phase * broad_phase {
                    continue;
                }

                let core_min_dist = obstacle_radius + self.food_growers.radius[grower_index];
                self.push_obstacle_from_grower(
                    obstacle_index,
                    grower_index,
                    obstacle_pos - grower_pos,
                    core_min_dist,
                    &mut moved_grower,
                );

                let obstacle_pos = Vec2::new(
                    self.obstacles.x[obstacle_index],
                    self.obstacles.y[obstacle_index],
                );
                for branch_index in self.food_growers.branch_range(grower_index) {
                    if !self.food_growers.branch_has_collision(branch_index) {
                        continue;
                    }

                    let (closest, t) = self
                        .food_growers
                        .closest_point_on_branch(branch_index, obstacle_pos);
                    let branch_width = self.food_growers.branch_collision_width_at(branch_index, t);
                    self.push_obstacle_from_grower(
                        obstacle_index,
                        grower_index,
                        obstacle_pos - closest,
                        obstacle_radius + branch_width,
                        &mut moved_grower,
                    );
                }
            }

            bounce_point_in_arena(
                &mut self.obstacles.x[obstacle_index],
                &mut self.obstacles.y[obstacle_index],
                &mut self.obstacles.vx[obstacle_index],
                &mut self.obstacles.vy[obstacle_index],
                self.width,
                self.height,
                self.arena_shape,
                obstacle_radius,
            );
        }

        if moved_grower {
            for grower_index in 0..self.food_growers.len() {
                let extent = self.food_growers.extent_radius(grower_index);
                bounce_point_in_arena(
                    &mut self.food_growers.x[grower_index],
                    &mut self.food_growers.y[grower_index],
                    &mut self.food_growers.vx[grower_index],
                    &mut self.food_growers.vy[grower_index],
                    self.width,
                    self.height,
                    self.arena_shape,
                    extent,
                );
            }
            self.food_growers
                .rebuild_branch_world_geometry(self.elapsed);
        }
    }

    fn push_obstacle_from_grower(
        &mut self,
        obstacle_index: usize,
        grower_index: usize,
        delta: Vec2,
        min_dist: f32,
        moved_grower: &mut bool,
    ) {
        let dist_sq = delta.length_squared();
        if dist_sq >= min_dist * min_dist {
            return;
        }

        let (normal, dist) = if dist_sq > 0.0001 {
            let dist = dist_sq.sqrt();
            (delta / dist, dist)
        } else {
            (Vec2::X, 0.001)
        };
        let push = min_dist - dist;
        let obstacle_share = 0.72;
        let grower_share = 1.0 - obstacle_share;

        self.obstacles.x[obstacle_index] += normal.x * push * obstacle_share;
        self.obstacles.y[obstacle_index] += normal.y * push * obstacle_share;
        self.food_growers.x[grower_index] -= normal.x * push * grower_share;
        self.food_growers.y[grower_index] -= normal.y * push * grower_share;
        *moved_grower = true;

        let obstacle_into = self.obstacles.vx[obstacle_index] * normal.x
            + self.obstacles.vy[obstacle_index] * normal.y;
        if obstacle_into < 0.0 {
            self.obstacles.vx[obstacle_index] -= obstacle_into * normal.x * 1.35;
            self.obstacles.vy[obstacle_index] -= obstacle_into * normal.y * 1.35;
        }

        let grower_into = self.food_growers.vx[grower_index] * normal.x
            + self.food_growers.vy[grower_index] * normal.y;
        if grower_into > 0.0 {
            self.food_growers.vx[grower_index] -= grower_into * normal.x * 0.55;
            self.food_growers.vy[grower_index] -= grower_into * normal.y * 0.55;
        }
    }

    fn grow_food(&mut self, dt: f32) {
        for i in 0..self.food_growers.len() {
            self.food_growers.timer[i] -= dt;
            if self.food_growers.timer[i] > 0.0
                || self.food.active_count_for(FoodSource::Feeder) >= self.max_feeder_food
            {
                continue;
            }

            self.food_growers.timer[i] = self.food_growers.interval[i];
            let branch_range = self.food_growers.branch_range(i);
            if branch_range.is_empty() {
                continue;
            }

            for _ in 0..FOOD_GROWER_BATCH_SIZE {
                if !self.try_spawn_feeder_food(i) {
                    break;
                }
            }
        }
    }

    fn grow_wild_food(&mut self, dt: f32) {
        for index in 0..self.food.len() {
            if self.food.active[index] || self.food.source[index] != FoodSource::Wild {
                continue;
            }
            self.food.regrow_timer[index] -= dt;
            if self.food.regrow_timer[index] > 0.0 {
                continue;
            }
            let position = self.safe_random_food_position();
            self.food.respawn_wild_at(index, position, &mut self.rng);
            self.energy_accumulator.values.wild_food_input +=
                self.food.energy[index] * self.food.growth[index];
        }
    }

    fn decay_food(&mut self, dt: f32) {
        let mut expired = Vec::new();
        for index in 0..self.food.len() {
            if !self.food.active[index] {
                continue;
            }
            self.food.age[index] += dt;
            let spoilage_rate = match self.food.kind[index] {
                FoodKind::Grass => GRASS_SPOILAGE_RATE,
                FoodKind::Meat => MEAT_SPOILAGE_RATE,
            };
            let growth = self.food.growth[index].clamp(0.0, 1.0);
            let before = self.food.energy[index] * growth;
            self.food.energy[index] *= (-spoilage_rate * dt).exp();
            let after = self.food.energy[index] * growth;
            self.energy_accumulator.values.spoilage += (before - after).max(0.0);
            if self.food.age[index] >= self.food.lifetime[index] || self.food.energy[index] < 0.05 {
                self.energy_accumulator.values.spoilage += after;
                expired.push(index);
            }
        }

        for index in expired {
            if self.food.is_feeder_food(index) {
                self.clear_branchlet_food_association(index);
            }
            self.food.deactivate(index);
        }
    }

    fn seed_feeder_food(&mut self, feeder_food_capacity: usize) {
        if feeder_food_capacity == 0 || self.food_growers.len() == 0 {
            return;
        }

        let mut spawned = 0;
        let max_attempts = feeder_food_capacity.saturating_mul(24).max(96);
        for attempt in 0..max_attempts {
            if spawned >= feeder_food_capacity {
                break;
            }

            let grower_index = if attempt % 3 == 0 {
                0
            } else {
                self.rng.random_range(0..self.food_growers.len())
            };

            if self.try_spawn_feeder_food(grower_index) {
                spawned += 1;
            }
        }
    }

    fn try_spawn_feeder_food(&mut self, grower_index: usize) -> bool {
        if self.food.active_count_for(FoodSource::Feeder) >= self.max_feeder_food {
            return false;
        }

        let mut inactive_branchlet_indices = Vec::new();
        for idx in 0..self.food_growers.branchlet_grower_index.len() {
            if self.food_growers.branchlet_grower_index[idx] == grower_index {
                let mut is_active = false;
                if let Some(food_idx) = self.food_growers.branchlet_food_index[idx]
                    && food_idx < self.food.len()
                    && self.food.active[food_idx]
                {
                    is_active = true;
                }
                if !is_active {
                    inactive_branchlet_indices.push(idx);
                }
            }
        }

        let mut reuse_branchlet = false;
        if !inactive_branchlet_indices.is_empty() && self.rng.random_bool(0.5) {
            reuse_branchlet = true;
        }

        let mut spawn = None;
        let mut chosen_branchlet_idx = None;

        if reuse_branchlet {
            for _ in 0..12 {
                let branchlet_index = inactive_branchlet_indices
                    [self.rng.random_range(0..inactive_branchlet_indices.len())];
                let branch_index = self.food_growers.branchlet_branch_index[branchlet_index];
                let branch_t = self.food_growers.branchlet_t[branchlet_index];
                let side = self.food_growers.branchlet_side[branchlet_index];
                let branchlet_length = self.food_growers.branchlet_length[branchlet_index];
                let dev_angle = self.food_growers.branchlet_angle_dev[branchlet_index];

                let branch_width = self
                    .food_growers
                    .branch_collision_width_at(branch_index, branch_t);
                let base = self.food_growers.branch_center_at(branch_index, branch_t);
                let normal = self.food_growers.branch_normal_at(branch_index, branch_t);

                let (sin_dev, cos_dev) = dev_angle.sin_cos();
                let stem_dir = Vec2::new(
                    normal.x * cos_dev - normal.y * sin_dev,
                    normal.x * sin_dev + normal.y * cos_dev,
                );

                let lateral = side
                    * (branch_width / cos_dev
                        + branchlet_length
                        + FOOD_RADIUS
                        + FEEDER_FOOD_SURFACE_GAP);
                let x = base.x + stem_dir.x * lateral;
                let y = base.y + stem_dir.y * lateral;
                let point = Vec2::new(x, y);

                let sway_margin = branch_t * self.food_growers.branch_length[branch_index] * 0.12;
                let parent_overlaps = if self.food_growers.branch_has_collision(branch_index) {
                    let (closest, t) = self
                        .food_growers
                        .closest_point_on_branch(branch_index, point);
                    let min_dist =
                        self.food_growers.branch_collision_width_at(branch_index, t) + FOOD_RADIUS;
                    point.distance_squared(closest) < min_dist * min_dist
                } else {
                    false
                };

                if point_inside_arena(
                    point,
                    self.width,
                    self.height,
                    self.arena_shape,
                    FOOD_RADIUS,
                ) && !parent_overlaps
                    && !self.point_overlaps_other_solids(
                        point,
                        FOOD_RADIUS + sway_margin,
                        branch_index,
                    )
                {
                    let distance = branch_t * self.food_growers.branch_length[branch_index];
                    spawn = Some((branch_index, x, y, dev_angle, distance, lateral));
                    chosen_branchlet_idx = Some(branchlet_index);
                    break;
                }
            }
        }

        if spawn.is_none() {
            let branch_range = self.food_growers.branch_range(grower_index);
            if !branch_range.is_empty() {
                for _ in 0..12 {
                    let branch_index = self.rng.random_range(branch_range.clone());
                    let branch_t = self.rng.random_range(0.36..0.92);
                    let side = if self.rng.random_bool(0.5) { -1.0 } else { 1.0 };
                    let branch_width = self
                        .food_growers
                        .branch_collision_width_at(branch_index, branch_t);
                    let branchlet_length = self.rng.random_range(3.5..7.0) + branch_width * 0.15;
                    let dev_angle = self.rng.random_range(-0.5_f32..0.5_f32);

                    let base = self.food_growers.branch_center_at(branch_index, branch_t);
                    let normal = self.food_growers.branch_normal_at(branch_index, branch_t);

                    let (sin_dev, cos_dev) = dev_angle.sin_cos();
                    let stem_dir = Vec2::new(
                        normal.x * cos_dev - normal.y * sin_dev,
                        normal.x * sin_dev + normal.y * cos_dev,
                    );

                    let lateral = side
                        * (branch_width / cos_dev
                            + branchlet_length
                            + FOOD_RADIUS
                            + FEEDER_FOOD_SURFACE_GAP);
                    let x = base.x + stem_dir.x * lateral;
                    let y = base.y + stem_dir.y * lateral;
                    let point = Vec2::new(x, y);

                    let sway_margin =
                        branch_t * self.food_growers.branch_length[branch_index] * 0.12;
                    let parent_overlaps = if self.food_growers.branch_has_collision(branch_index) {
                        let (closest, t) = self
                            .food_growers
                            .closest_point_on_branch(branch_index, point);
                        let min_dist = self.food_growers.branch_collision_width_at(branch_index, t)
                            + FOOD_RADIUS;
                        point.distance_squared(closest) < min_dist * min_dist
                    } else {
                        false
                    };

                    if point_inside_arena(
                        point,
                        self.width,
                        self.height,
                        self.arena_shape,
                        FOOD_RADIUS,
                    ) && !parent_overlaps
                        && !self.point_overlaps_other_solids(
                            point,
                            FOOD_RADIUS + sway_margin,
                            branch_index,
                        )
                    {
                        self.food_growers.branchlet_grower_index.push(grower_index);
                        self.food_growers.branchlet_branch_index.push(branch_index);
                        self.food_growers.branchlet_t.push(branch_t);
                        self.food_growers.branchlet_side.push(side);
                        self.food_growers.branchlet_length.push(branchlet_length);
                        self.food_growers.branchlet_angle_dev.push(dev_angle);
                        self.food_growers.branchlet_food_index.push(None);

                        let branchlet_index = self.food_growers.branchlet_branch_index.len() - 1;
                        let distance = branch_t * self.food_growers.branch_length[branch_index];
                        spawn = Some((branch_index, x, y, dev_angle, distance, lateral));
                        chosen_branchlet_idx = Some(branchlet_index);
                        break;
                    }
                }
            }
        }

        if let Some((branch_index, x, y, dev_angle, distance, lateral)) = spawn {
            let food_idx = self.food.push_feeder_at(
                grower_index as i32,
                branch_index as i32,
                x,
                y,
                dev_angle,
                distance,
                lateral,
                self.width,
                self.height,
                self.arena_shape,
                &mut self.rng,
            );
            self.energy_accumulator.values.feeder_input +=
                self.food.energy[food_idx] * self.food.growth[food_idx];
            if let Some(branchlet_index) = chosen_branchlet_idx {
                self.food_growers.branchlet_food_index[branchlet_index] = Some(food_idx);
            }
            return true;
        }

        false
    }

    fn advect_food(&mut self, dt: f32) {
        for i in 0..self.food.len() {
            if !self.food.active[i] {
                continue;
            }

            if let Some(grower_index) = self.food.feeder_index(i) {
                if grower_index < self.food_growers.len() {
                    let branch_index = self.food.anchor_branch[i];
                    if branch_index >= 0 {
                        let branch_index = branch_index as usize;
                        if branch_index < self.food_growers.branch_total() {
                            let branch_length =
                                self.food_growers.branch_length[branch_index].max(1.0);
                            let branch_t =
                                (self.food.anchor_distance[i] / branch_length).clamp(0.0, 1.0);
                            let base = self.food_growers.branch_center_at(branch_index, branch_t);
                            let normal = self.food_growers.branch_normal_at(branch_index, branch_t);
                            let anchor_angle = self.food.anchor_angle[i];
                            let (sin_dev, cos_dev) = anchor_angle.sin_cos();
                            let stem_dir = Vec2::new(
                                normal.x * cos_dev - normal.y * sin_dev,
                                normal.x * sin_dev + normal.y * cos_dev,
                            );
                            self.food.x[i] = base.x + stem_dir.x * self.food.anchor_lateral[i];
                            self.food.y[i] = base.y + stem_dir.y * self.food.anchor_lateral[i];
                        } else {
                            self.clear_branchlet_food_association(i);
                            self.food.deactivate(i);
                            continue;
                        }
                    } else {
                        let angle =
                            self.food_growers.rotation[grower_index] + self.food.anchor_angle[i];
                        let (s, c) = angle.sin_cos();
                        self.food.x[i] = self.food_growers.x[grower_index]
                            + c * self.food.anchor_distance[i]
                            - s * self.food.anchor_lateral[i];
                        self.food.y[i] = self.food_growers.y[grower_index]
                            + s * self.food.anchor_distance[i]
                            + c * self.food.anchor_lateral[i];
                    }
                    let previous_growth = self.food.growth[i];
                    self.food.growth[i] = (previous_growth + dt * 1.7).min(1.0);
                    self.energy_accumulator.values.feeder_input +=
                        (self.food.growth[i] - previous_growth) * self.food.energy[i];
                    self.food.rotation[i] += self.food.spin[i] * dt;
                } else {
                    self.clear_branchlet_food_association(i);
                    self.food.deactivate(i);
                }
                continue;
            }

            if self.food.source[i] == FoodSource::Wild {
                let previous_growth = self.food.growth[i];
                self.food.growth[i] = (previous_growth + dt * 0.72).min(1.0);
                self.energy_accumulator.values.wild_food_input +=
                    (self.food.growth[i] - previous_growth) * self.food.energy[i];
            }
            let position = Vec2::new(self.food.x[i], self.food.y[i]);
            let current = liquid_current_at(position, self.elapsed) * FOOD_CURRENT_SPEED;
            self.food.x[i] += current.x * dt;
            self.food.y[i] += current.y * dt;
            self.food.rotation[i] += self.food.spin[i] * dt;

            let clamped = clamp_point_to_arena(
                Vec2::new(self.food.x[i], self.food.y[i]),
                self.width,
                self.height,
                self.arena_shape,
                FOOD_RADIUS,
            );
            self.food.x[i] = clamped.x;
            self.food.y[i] = clamped.y;
        }
    }

    fn push_food_from_obstacles(&mut self, _dt: f32) {
        for obstacle_index in 0..self.obstacles.len() {
            let center = Vec2::new(
                self.obstacles.x[obstacle_index],
                self.obstacles.y[obstacle_index],
            );
            let hard_radius = self.obstacles.radius[obstacle_index] + FOOD_RADIUS + 2.0;

            for food_index in 0..self.food.len() {
                if !self.food.active[food_index] || self.food.is_feeder_food(food_index) {
                    continue;
                }

                let food_pos = Vec2::new(self.food.x[food_index], self.food.y[food_index]);
                let delta = food_pos - center;
                let dist_sq = delta.length_squared();
                if dist_sq >= hard_radius * hard_radius {
                    continue;
                }

                let (dir, dist) = if dist_sq > 0.0001 {
                    let dist = dist_sq.sqrt();
                    (delta / dist, dist)
                } else {
                    (Vec2::X, 0.001)
                };
                let push = hard_radius - dist;
                self.food.x[food_index] += dir.x * push;
                self.food.y[food_index] += dir.y * push;
            }
        }
    }

    fn push_food_from_food_growers(&mut self, _dt: f32) {
        for grower_index in 0..self.food_growers.len() {
            let center = Vec2::new(
                self.food_growers.x[grower_index],
                self.food_growers.y[grower_index],
            );
            let hard_radius = self.food_growers.radius[grower_index] + FOOD_RADIUS + 2.0;
            let extent = self.food_growers.extent_radius(grower_index) + FOOD_RADIUS + 20.0;

            for food_index in 0..self.food.len() {
                if !self.food.active[food_index] || self.food.is_feeder_food(food_index) {
                    continue;
                }

                let food_pos = Vec2::new(self.food.x[food_index], self.food.y[food_index]);
                let delta_center = food_pos - center;
                let dist_center_sq = delta_center.length_squared();

                // Hard collision with grower core
                if dist_center_sq < hard_radius * hard_radius {
                    let (dir, dist) = if dist_center_sq > 0.0001 {
                        let dist = dist_center_sq.sqrt();
                        (delta_center / dist, dist)
                    } else {
                        (Vec2::X, 0.001)
                    };
                    let push = hard_radius - dist;
                    self.food.x[food_index] += dir.x * push;
                    self.food.y[food_index] += dir.y * push;
                }

                // Skip branch checks if food is far from this grower
                if dist_center_sq >= extent * extent {
                    continue;
                }

                // Hard collision with solid branches
                for branch_index in self.food_growers.branch_range(grower_index) {
                    if !self.food_growers.branch_has_collision(branch_index) {
                        continue;
                    }

                    let food_pos = Vec2::new(self.food.x[food_index], self.food.y[food_index]);
                    let (closest, t) = self
                        .food_growers
                        .closest_point_on_branch(branch_index, food_pos);
                    let delta = food_pos - closest;
                    let branch_width = self.food_growers.branch_collision_width_at(branch_index, t);
                    let min_dist = branch_width + FOOD_RADIUS + 1.0;
                    let dist_sq = delta.length_squared();
                    if dist_sq >= min_dist * min_dist {
                        continue;
                    }

                    let (dir, dist) = if dist_sq > 0.0001 {
                        let dist = dist_sq.sqrt();
                        (delta / dist, dist)
                    } else {
                        (Vec2::X, 0.001)
                    };
                    let push = min_dist - dist;
                    self.food.x[food_index] += dir.x * push;
                    self.food.y[food_index] += dir.y * push;
                }
            }
        }
    }

    fn clamp_free_food_to_arena(&mut self) {
        for food_index in 0..self.food.len() {
            if !self.food.active[food_index] || self.food.is_feeder_food(food_index) {
                continue;
            }

            let point = clamp_point_to_arena(
                Vec2::new(self.food.x[food_index], self.food.y[food_index]),
                self.width,
                self.height,
                self.arena_shape,
                FOOD_RADIUS,
            );
            self.food.x[food_index] = point.x;
            self.food.y[food_index] = point.y;
        }
    }

    fn resolve_cell_obstacles(&mut self, cell_index: usize, cell_bound: f32) {
        let center = Vec2::new(self.cells.x[cell_index], self.cells.y[cell_index]);
        let search_radius = cell_bound + self.max_obstacle_radius;
        let (min_x, max_x, min_y, max_y) = self.obstacle_grid.bucket_range(center, search_radius);
        for gy in min_y..=max_y {
            for gx in min_x..=max_x {
                let bucket = gy * self.obstacle_grid.cols + gx;
                for &obstacle_index in &self.obstacle_grid.buckets[bucket] {
                    let dx = self.cells.x[cell_index] - self.obstacles.x[obstacle_index];
                    let dy = self.cells.y[cell_index] - self.obstacles.y[obstacle_index];
                    let dist_sq = dx * dx + dy * dy;
                    let obstacle_radius = self.obstacles.radius[obstacle_index];
                    let broad_min_dist = cell_bound + obstacle_radius;
                    if dist_sq >= broad_min_dist * broad_min_dist {
                        continue;
                    }

                    let (nx, ny, dist) = if dist_sq > 0.0001 {
                        let dist = dist_sq.sqrt();
                        (dx / dist, dy / dist, dist)
                    } else {
                        (1.0, 0.0, 0.001)
                    };
                    let normal = Vec2::new(nx, ny);
                    let ray_index = self.cells.soft_ray_index_for_direction(cell_index, normal);
                    let cell_radius = self.cells.current_radii[cell_index][ray_index];
                    let min_dist = cell_radius + obstacle_radius;
                    if dist_sq >= min_dist * min_dist {
                        continue;
                    }

                    let compression =
                        self.cells
                            .compress_ray(cell_index, ray_index, dist - obstacle_radius);
                    let push = ((min_dist - dist) * SOFT_BODY_SOLID_PUSH_FACTOR)
                        .min(SOFT_BODY_SOLID_PUSH_MAX);
                    self.cells.x[cell_index] += nx * push;
                    self.cells.y[cell_index] += ny * push;
                    if compression > 0.0 {
                        self.cells.vx[cell_index] +=
                            nx * compression * SOFT_BODY_COMPRESSION_IMPULSE;
                        self.cells.vy[cell_index] +=
                            ny * compression * SOFT_BODY_COMPRESSION_IMPULSE;
                    }

                    let into_obstacle =
                        self.cells.vx[cell_index] * nx + self.cells.vy[cell_index] * ny;
                    if into_obstacle < 0.0 {
                        self.cells.vx[cell_index] -=
                            into_obstacle * nx * (1.0 + CELL_OBSTACLE_RESTITUTION);
                        self.cells.vy[cell_index] -=
                            into_obstacle * ny * (1.0 + CELL_OBSTACLE_RESTITUTION);
                        self.cells.jelly_intensity[cell_index] =
                            (self.cells.jelly_intensity[cell_index] + 0.35).min(1.0);
                        self.cells.jelly_dir_x[cell_index] = nx;
                        self.cells.jelly_dir_y[cell_index] = ny;
                    }
                }
            }
        }
    }

    fn resolve_cell_food_growers(&mut self, cell_index: usize, broad_cell_radius: f32) {
        let mut cell_x = self.cells.x[cell_index];
        let mut cell_y = self.cells.y[cell_index];

        for grower_index in 0..self.food_growers.len() {
            let grower_x = self.food_growers.x[grower_index];
            let grower_y = self.food_growers.y[grower_index];
            let dx = cell_x - grower_x;
            let dy = cell_y - grower_y;
            let extent = self.food_growers.extent_radius(grower_index) + broad_cell_radius;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq >= extent * extent {
                continue;
            }

            let core_normal = if dist_sq > 0.0001 {
                Vec2::new(dx, dy) * dist_sq.sqrt().recip()
            } else {
                Vec2::X
            };
            let grower_radius = self.food_growers.radius[grower_index];
            let broad_min_dist = broad_cell_radius + grower_radius;
            if dist_sq < broad_min_dist * broad_min_dist {
                let ray_index = self
                    .cells
                    .soft_ray_index_for_direction(cell_index, core_normal);
                let cell_radius = self.cells.current_radii[cell_index][ray_index];
                let min_dist = cell_radius + grower_radius;
                if dist_sq < min_dist * min_dist {
                    let (nx, ny, dist) = if dist_sq > 0.0001 {
                        let dist = dist_sq.sqrt();
                        (dx / dist, dy / dist, dist)
                    } else {
                        (1.0, 0.0, 0.001)
                    };
                    self.push_cell_from_grower_surface(
                        cell_index,
                        nx,
                        ny,
                        dist,
                        min_dist,
                        grower_radius,
                    );
                    cell_x = self.cells.x[cell_index];
                    cell_y = self.cells.y[cell_index];
                }
            }

            for branch_index in self.food_growers.branch_range(grower_index) {
                if !self.food_growers.branch_has_collision(branch_index) {
                    continue;
                }

                let cell_point = Vec2::new(cell_x, cell_y);
                let (closest, t) = self
                    .food_growers
                    .closest_point_on_branch(branch_index, cell_point);
                let delta = cell_point - closest;
                let dx = delta.x;
                let dy = delta.y;
                let branch_width = self.food_growers.branch_collision_width_at(branch_index, t);
                let dist_sq = dx * dx + dy * dy;
                let broad_min_dist = broad_cell_radius + branch_width;
                if dist_sq >= broad_min_dist * broad_min_dist {
                    continue;
                }

                let branch_normal = if delta.length_squared() > 0.0001 {
                    delta.normalize()
                } else {
                    Vec2::X
                };
                let ray_index = self
                    .cells
                    .soft_ray_index_for_direction(cell_index, branch_normal);
                let cell_radius = self.cells.current_radii[cell_index][ray_index];
                let min_dist = cell_radius + branch_width;
                if dist_sq >= min_dist * min_dist {
                    continue;
                }

                let (nx, ny, dist) = if dist_sq > 0.0001 {
                    let dist = dist_sq.sqrt();
                    (dx / dist, dy / dist, dist)
                } else {
                    (1.0, 0.0, 0.001)
                };
                self.push_cell_from_grower_surface(
                    cell_index,
                    nx,
                    ny,
                    dist,
                    min_dist,
                    branch_width,
                );
                cell_x = self.cells.x[cell_index];
                cell_y = self.cells.y[cell_index];
            }
        }
    }

    fn push_cell_from_grower_surface(
        &mut self,
        cell_index: usize,
        nx: f32,
        ny: f32,
        dist: f32,
        min_dist: f32,
        solid_radius: f32,
    ) {
        let push = ((min_dist - dist) * SOFT_BODY_SOLID_PUSH_FACTOR).min(SOFT_BODY_SOLID_PUSH_MAX);
        let ray_index = self
            .cells
            .soft_ray_index_for_direction(cell_index, Vec2::new(nx, ny));
        let compression = self
            .cells
            .compress_ray(cell_index, ray_index, dist - solid_radius);
        self.cells.x[cell_index] += nx * push;
        self.cells.y[cell_index] += ny * push;
        if compression > 0.0 {
            self.cells.vx[cell_index] += nx * compression * SOFT_BODY_COMPRESSION_IMPULSE;
            self.cells.vy[cell_index] += ny * compression * SOFT_BODY_COMPRESSION_IMPULSE;
        }

        let into_grower = self.cells.vx[cell_index] * nx + self.cells.vy[cell_index] * ny;
        if into_grower < 0.0 {
            self.cells.vx[cell_index] -= into_grower * nx * (1.0 + CELL_OBSTACLE_RESTITUTION);
            self.cells.vy[cell_index] -= into_grower * ny * (1.0 + CELL_OBSTACLE_RESTITUTION);
            self.cells.jelly_intensity[cell_index] =
                (self.cells.jelly_intensity[cell_index] + 0.35).min(1.0);
            self.cells.jelly_dir_x[cell_index] = nx;
            self.cells.jelly_dir_y[cell_index] = ny;

            // Redirect heading along the branch surface tangent so cells slide instead of getting stuck
            let tangent_x = -ny;
            let tangent_y = nx;
            let vel_along_tangent =
                self.cells.vx[cell_index] * tangent_x + self.cells.vy[cell_index] * tangent_y;
            if vel_along_tangent.abs() > 0.5 {
                let slide_angle = tangent_y.atan2(tangent_x);
                let heading_delta = wrap_angle(slide_angle - self.cells.heading[cell_index]);
                self.cells.heading[cell_index] =
                    wrap_angle(self.cells.heading[cell_index] + heading_delta * 0.35);
            }
        }
    }

    fn relocate_world_food_away_from_solids(&mut self) {
        for food_index in 0..self.food.len() {
            if self.food.is_feeder_food(food_index) {
                continue;
            }

            let point = Vec2::new(self.food.x[food_index], self.food.y[food_index]);
            if self.point_overlaps_solid(point, FOOD_RADIUS + FOOD_SOLID_SPAWN_MARGIN) {
                let safe = self.safe_random_food_position();
                self.food.x[food_index] = safe.x;
                self.food.y[food_index] = safe.y;
            }
        }
    }

    fn safe_random_food_position(&mut self) -> Vec2 {
        for _ in 0..96 {
            let point = random_point_in_arena(
                self.width,
                self.height,
                self.arena_shape,
                FOOD_RADIUS,
                &mut self.rng,
            );
            if !self.point_overlaps_solid(point, FOOD_RADIUS + FOOD_SOLID_SPAWN_MARGIN) {
                return point;
            }
        }

        random_point_in_arena(
            self.width,
            self.height,
            self.arena_shape,
            FOOD_RADIUS,
            &mut self.rng,
        )
    }

    fn point_overlaps_solid(&self, point: Vec2, radius: f32) -> bool {
        self.point_overlaps_other_solids(point, radius, usize::MAX)
    }

    fn point_overlaps_other_solids(&self, point: Vec2, radius: f32, ignore_branch: usize) -> bool {
        if !point_inside_arena(point, self.width, self.height, self.arena_shape, radius) {
            return true;
        }

        let search_radius = radius + self.max_obstacle_radius;
        let (min_x, max_x, min_y, max_y) = self.obstacle_grid.bucket_range(point, search_radius);
        for gy in min_y..=max_y {
            for gx in min_x..=max_x {
                let bucket = gy * self.obstacle_grid.cols + gx;
                for &obstacle_index in &self.obstacle_grid.buckets[bucket] {
                    let center = Vec2::new(
                        self.obstacles.x[obstacle_index],
                        self.obstacles.y[obstacle_index],
                    );
                    let min_dist = self.obstacles.radius[obstacle_index] + radius;
                    if point.distance_squared(center) < min_dist * min_dist {
                        return true;
                    }
                }
            }
        }

        for grower_index in 0..self.food_growers.len() {
            let center = Vec2::new(
                self.food_growers.x[grower_index],
                self.food_growers.y[grower_index],
            );
            let min_dist = self.food_growers.radius[grower_index] + radius;
            let dist_sq = point.distance_squared(center);
            if dist_sq < min_dist * min_dist {
                return true;
            }

            let extent = self.food_growers.extent_radius(grower_index) + radius;
            if dist_sq >= extent * extent {
                continue;
            }

            for branch_index in self.food_growers.branch_range(grower_index) {
                if branch_index == ignore_branch
                    || !self.food_growers.branch_has_collision(branch_index)
                {
                    continue;
                }

                let (closest, t) = self
                    .food_growers
                    .closest_point_on_branch(branch_index, point);
                let min_dist =
                    self.food_growers.branch_collision_width_at(branch_index, t) + radius;
                if point.distance_squared(closest) < min_dist * min_dist {
                    return true;
                }
            }
        }

        false
    }

    fn cell_avoidance_velocity(
        &self,
        cell_index: usize,
        desired_velocity: Vec2,
        speed: f32,
        cell_radius: f32,
    ) -> Vec2 {
        let position = Vec2::new(self.cells.x[cell_index], self.cells.y[cell_index]);
        let desired_dir = if desired_velocity.length_squared() > 0.0001 {
            desired_velocity.normalize()
        } else {
            Vec2::new(self.cells.vx[cell_index], self.cells.vy[cell_index])
                .try_normalize()
                .unwrap_or(Vec2::X)
        };
        let mut avoidance = Vec2::ZERO;

        let search_radius = cell_radius + CELL_AVOIDANCE_MARGIN + self.max_obstacle_radius;
        let (min_x, max_x, min_y, max_y) = self.obstacle_grid.bucket_range(position, search_radius);
        for gy in min_y..=max_y {
            for gx in min_x..=max_x {
                let bucket = gy * self.obstacle_grid.cols + gx;
                for &obstacle_index in &self.obstacle_grid.buckets[bucket] {
                    let center = Vec2::new(
                        self.obstacles.x[obstacle_index],
                        self.obstacles.y[obstacle_index],
                    );
                    let influence =
                        self.obstacles.radius[obstacle_index] + cell_radius + CELL_AVOIDANCE_MARGIN;
                    add_avoidance(
                        &mut avoidance,
                        position - center,
                        influence,
                        desired_dir,
                        speed,
                    );
                }
            }
        }

        for grower_index in 0..self.food_growers.len() {
            let center = Vec2::new(
                self.food_growers.x[grower_index],
                self.food_growers.y[grower_index],
            );
            let grower_delta = position - center;
            let grower_influence =
                self.food_growers.extent_radius(grower_index) + cell_radius + CELL_AVOIDANCE_MARGIN;
            if grower_delta.length_squared() >= grower_influence * grower_influence {
                continue;
            }

            let influence =
                self.food_growers.radius[grower_index] + cell_radius + CELL_AVOIDANCE_MARGIN;
            add_avoidance(&mut avoidance, grower_delta, influence, desired_dir, speed);

            for branch_index in self.food_growers.branch_range(grower_index) {
                if !self.food_growers.branch_has_collision(branch_index) {
                    continue;
                }

                let (closest, t) = self
                    .food_growers
                    .closest_point_on_branch(branch_index, position);
                let influence = self.food_growers.branch_collision_width_at(branch_index, t)
                    + cell_radius
                    + CELL_AVOIDANCE_MARGIN;
                add_avoidance(
                    &mut avoidance,
                    position - closest,
                    influence,
                    desired_dir,
                    speed,
                );
            }
        }

        avoidance
    }

    fn decay_visuals(&mut self, dt: f32) {
        let decay = (1.0 - JELLY_DECAY * dt).clamp(0.0, 1.0);

        for i in 0..self.cells.len() {
            self.cells.jelly_phase[i] += dt * (6.0 + self.cells.jelly_intensity[i] * 3.0);
            self.cells.jelly_intensity[i] *= decay;
        }
    }

    fn decay_viability(&mut self, dt: f32) {
        let mut total_drain = 0.0;
        for i in 0..self.cells.len() {
            let drain = self.cell_metabolic_drain_rate(i) * dt;
            let before = self.cells.viability[i];
            self.cells.viability[i] = (before - drain).max(0.0);
            total_drain += before - self.cells.viability[i];
        }
        self.energy_accumulator.values.metabolism += total_drain;
    }

    fn cell_metabolic_drain_rate(&self, index: usize) -> f32 {
        let speed_cost = (self.cells.speed[index] / CELL_SPEED_DISPLAY_MAX).clamp(0.0, 1.5);
        let biomass_cost = self.cells.biomass_sum(index) * SOFT_BODY_BIOMASS_DRAIN_RATE;
        let lysis_cost = (self.cells.lysis[index] / CELL_LYSIS_DISPLAY_MAX).clamp(0.0, 1.0) * 0.012;
        (VIABILITY_DECAY_BASE + speed_cost * VIABILITY_DECAY_SPEED + biomass_cost + lysis_cost)
            * self.cells.morphology_metabolism_factor(index)
    }

    fn remove_dead_cells(&mut self) {
        let mut dead_indices = Vec::new();
        for i in 0..self.cells.len() {
            if self.cells.viability[i] <= 0.0 {
                dead_indices.push(i);
            }
        }

        for &cell_index in dead_indices.iter().rev() {
            self.spawn_meat_from_cell(cell_index);
            self.cells.swap_remove(cell_index);
        }
        if !dead_indices.is_empty() {
            self.cell_grid_dirty = true;
        }
    }

    fn process_cell_lifecycle(&mut self) {
        self.remove_dead_cells();

        let initial_len = self.cells.len();
        for i in 0..initial_len {
            if self.cells.viability[i] <= 0.0 {
                continue;
            }

            let threshold = self.cells.max_viability[i] * self.cells.division_threshold[i] / 100.0;
            if self.cells.viability[i] >= threshold
                && self.cells.mitosis_progress[i] <= 0.0
                && self.cells.mitosis_recovery[i] <= 0.0
            {
                self.cells.mitosis_progress[i] = f32::EPSILON;
                self.spawn_mitosis_particles(i, 4, 0.45);
            }
        }
    }

    fn advance_mitosis(&mut self, dt: f32) {
        let initial_len = self.cells.len();
        for i in 0..initial_len {
            self.cells.mitosis_recovery[i] = (self.cells.mitosis_recovery[i] - dt).max(0.0);
            if self.cells.mitosis_progress[i] <= 0.0 {
                continue;
            }
            let previous_progress = self.cells.mitosis_progress[i];
            self.cells.mitosis_progress[i] += dt / MITOSIS_DURATION;
            if previous_progress < 0.48 && self.cells.mitosis_progress[i] >= 0.48 {
                self.spawn_mitosis_particles(i, 7, 0.72);
            }
            if self.cells.mitosis_progress[i] >= 1.0 {
                self.spawn_mitosis_particles(i, 15, 1.25);
                self.cells.mitosis_progress[i] = 0.0;
                self.cells.mitosis_recovery[i] = MITOSIS_RECOVERY_DURATION;
                self.divide_cell(i);
            }
        }
    }

    fn divide_cell(&mut self, parent_index: usize) {
        let division_axis =
            Vec2::from_angle(self.cells.heading[parent_index] + std::f32::consts::FRAC_PI_2);
        let division_offset =
            self.cells.collision_bound_radius(parent_index) * DIVISION_CHILD_OFFSET;
        let parent_shift = -division_axis * (division_offset * 0.5);
        self.cells.x[parent_index] += parent_shift.x;
        self.cells.y[parent_index] += parent_shift.y;
        if self.cells.section_count[parent_index] >= 2 {
            self.cells.tail_x[parent_index] += parent_shift.x;
            self.cells.tail_y[parent_index] += parent_shift.y;
            for extra_index in 0..(self.cells.section_count[parent_index] as usize - 2) {
                self.cells.extra_sections[parent_index][extra_index].x += parent_shift.x;
                self.cells.extra_sections[parent_index][extra_index].y += parent_shift.y;
            }
        }
        let construction_cost = (self.cells.biomass_sum(parent_index)
            * CELL_STRUCTURE_ENERGY_PER_BIOMASS)
            .min(self.cells.viability[parent_index] * 0.25);
        self.energy_accumulator.values.mitosis_cost += construction_cost;
        let split_viability = ((self.cells.viability[parent_index] - construction_cost) * 0.5)
            .clamp(0.0, self.cells.max_viability[parent_index]);
        self.cells.viability[parent_index] = split_viability;
        self.cells.push_child_from(
            parent_index,
            split_viability,
            self.width,
            self.height,
            self.arena_shape,
            division_axis,
            division_offset,
            &mut self.rng,
        );
        self.cell_grid_dirty = true;
        self.cell_sound_events.push(Vec2::new(
            self.cells.x[parent_index],
            self.cells.y[parent_index],
        ));
    }

    fn spawn_meat_from_cell(&mut self, cell_index: usize) {
        let recoverable_energy = self.cells.viability[cell_index] * DEATH_VIABILITY_RECOVERY
            + self.cells.biomass_sum(cell_index)
                * CELL_STRUCTURE_ENERGY_PER_BIOMASS
                * DEATH_STRUCTURE_RECOVERY;
        let available_slots = self
            .max_carrion
            .saturating_sub(self.food.active_count_for(FoodSource::Carrion));
        let chunk_count = ((recoverable_energy / MEAT_CHUNK_ENERGY_MAX).ceil() as usize)
            .clamp(1, MEAT_CHUNKS_MAX)
            .min(available_slots);
        let origin = Vec2::new(self.cells.x[cell_index], self.cells.y[cell_index]);
        self.cell_sound_events.push(origin);
        if chunk_count == 0 || recoverable_energy <= 0.05 {
            return;
        }
        let chunk_energy = (recoverable_energy / chunk_count as f32).min(MEAT_CHUNK_ENERGY_MAX);
        self.energy_accumulator.values.carrion_transfer += chunk_energy * chunk_count as f32;
        let spread = (self.cells.radius[cell_index] * 3.2).max(18.0);

        for _ in 0..chunk_count {
            let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
            let distance = self.rng.random_range(0.0..spread);
            let (s, c) = angle.sin_cos();
            let point = origin + Vec2::new(c, s) * distance;
            self.food.push_meat_at(
                point.x,
                point.y,
                self.width,
                self.height,
                self.arena_shape,
                chunk_energy,
                self.cells.species[cell_index],
                &mut self.rng,
            );
        }
    }

    pub fn cell_index_by_id(&self, cell_id: u64) -> Option<usize> {
        self.cells.id.iter().position(|id| *id == cell_id)
    }

    pub fn active_food_counts(&self) -> (usize, usize, usize) {
        (
            self.food.active_count_for(FoodSource::Wild),
            self.food.active_count_for(FoodSource::Feeder),
            self.food.active_count_for(FoodSource::Carrion),
        )
    }

    #[allow(dead_code)]
    pub fn feeder_food_stem_points(&self, food_index: usize) -> Option<(Vec2, Vec2)> {
        if food_index >= self.food.len() || !self.food.active[food_index] {
            return None;
        }
        let branchlet_index = self
            .food_growers
            .branchlet_food_index
            .iter()
            .position(|idx| *idx == Some(food_index))?;
        self.branchlet_stem_points(branchlet_index)
    }

    pub fn branchlet_stem_points(&self, branchlet_index: usize) -> Option<(Vec2, Vec2)> {
        if branchlet_index >= self.food_growers.branchlet_branch_index.len() {
            return None;
        }
        let branch_index = self.food_growers.branchlet_branch_index[branchlet_index];
        let branch_t = self.food_growers.branchlet_t[branchlet_index];
        let base = self.food_growers.branch_center_at(branch_index, branch_t);
        let normal = self.food_growers.branch_normal_at(branch_index, branch_t);
        let side = self.food_growers.branchlet_side[branchlet_index];
        let branch_width = self
            .food_growers
            .branch_collision_width_at(branch_index, branch_t);

        let anchor_angle = self.food_growers.branchlet_angle_dev[branchlet_index];
        let (sin_dev, cos_dev) = anchor_angle.sin_cos();
        let stem_dir = Vec2::new(
            normal.x * cos_dev - normal.y * sin_dev,
            normal.x * sin_dev + normal.y * cos_dev,
        );

        let branchlet_length = self.food_growers.branchlet_length[branchlet_index];
        let start = base + stem_dir * side * (branch_width / cos_dev);
        let end = base + stem_dir * side * ((branch_width + branchlet_length) / cos_dev);
        Some((start, end))
    }

    pub fn clear_branchlet_food_association(&mut self, food_index: usize) {
        for idx in 0..self.food_growers.branchlet_food_index.len() {
            if self.food_growers.branchlet_food_index[idx] == Some(food_index) {
                self.food_growers.branchlet_food_index[idx] = None;
            }
        }
    }

    fn solve_cell_collisions(&mut self, dt: f32) {
        self.cell_grid.rebuild(&self.cells);
        self.cell_grid_dirty = false;
        self.collision_pairs.clear();
        self.collision_bounds.clear();
        self.collision_bounds.reserve(
            self.cells
                .len()
                .saturating_sub(self.collision_bounds.capacity()),
        );
        let mut max_bound = 0.0_f32;
        for index in 0..self.cells.len() {
            let bound = self.cells.collision_bound_radius(index);
            self.collision_bounds.push(bound);
            max_bound = max_bound.max(bound);
        }

        for a in 0..self.cells.len() {
            let bound_a = self.collision_bounds[a];
            let search_radius = bound_a + max_bound;
            let center_a = Vec2::new(self.cells.x[a], self.cells.y[a]);
            let (min_x, max_x, min_y, max_y) = self.cell_grid.bucket_range(center_a, search_radius);
            for gy in min_y..=max_y {
                for gx in min_x..=max_x {
                    let bucket_index = gy * self.cell_grid.cols + gx;
                    for &b in &self.cell_grid.buckets[bucket_index] {
                        if b <= a {
                            continue;
                        }
                        let bound_b = self.collision_bounds[b];
                        let broad_distance = bound_a + bound_b;
                        let dx = self.cells.x[b] - self.cells.x[a];
                        let dy = self.cells.y[b] - self.cells.y[a];
                        if dx * dx + dy * dy < broad_distance * broad_distance {
                            self.collision_pairs.push((a, b));
                        }
                    }
                }
            }
        }

        for pair_index in 0..self.collision_pairs.len() {
            let (a, b) = self.collision_pairs[pair_index];
            self.resolve_cell_pair(a, b, dt);
        }

        for i in 0..self.cells.len() {
            self.bounce_cell(i);
        }
    }

    fn resolve_cell_pair(&mut self, a: usize, b: usize, dt: f32) {
        let both_single = self.cells.section_count[a] == 1 && self.cells.section_count[b] == 1;
        if !both_single {
            self.resolve_compound_cell_pair(a, b, dt);
            return;
        }
        let (section_a, section_b) = if both_single {
            let dx = self.cells.x[b] - self.cells.x[a];
            let dy = self.cells.y[b] - self.cells.y[a];
            let min_distance = self.cells.collision_radius[a] + self.cells.collision_radius[b];
            if dx * dx + dy * dy >= min_distance * min_distance {
                return;
            }
            (0, 0)
        } else {
            closest_section_pair(&self.cells, a, b)
        };
        let center_a = self.cells.section_center(a, section_a);
        let center_b = self.cells.section_center(b, section_b);
        let section_delta = center_b - center_a;
        let section_dist_sq = section_delta.length_squared();
        let section_radius_a = self.cells.section_collision_radius(a, section_a);
        let section_radius_b = self.cells.section_collision_radius(b, section_b);
        let broad_min_dist = section_radius_a + section_radius_b;
        if section_dist_sq >= broad_min_dist * broad_min_dist {
            return;
        }
        let center_distance = section_dist_sq.sqrt();
        let normal = if section_dist_sq > 0.0001 {
            section_delta * center_distance.recip()
        } else {
            let angle = ((a as f32 * 12.9898 + b as f32 * 78.233).sin()) * std::f32::consts::TAU;
            let (ny, nx) = angle.sin_cos();
            Vec2::new(nx, ny)
        };

        let (contact_a, contact_b) = if section_a == 0 && section_b == 0 {
            (
                sample_membrane_contact(&self.cells, a, b, normal),
                sample_membrane_contact(&self.cells, b, a, -normal),
            )
        } else {
            let penetration = (section_radius_a + section_radius_b - center_distance).max(0.0);
            let mut contact_a = MembraneContact::default();
            let mut contact_b = MembraneContact::default();
            if penetration > 0.0 {
                contact_a.depth_sum = penetration;
                contact_a.count = 1;
                contact_b.depth_sum = penetration;
                contact_b.count = 1;
            }
            (contact_a, contact_b)
        };
        let contact_count = contact_a.count + contact_b.count;
        let core_distance = self.cells.section_core_radius(a, section_a)
            + self.cells.section_core_radius(b, section_b);
        let core_penetration = (core_distance - center_distance).max(0.0);
        if contact_count == 0 && core_penetration <= 0.0 {
            return;
        }

        if section_a == 0 && section_b == 0 {
            self.cells.compress_rays_by_depth(a, &contact_a.ray_depths);
            self.cells.compress_rays_by_depth(b, &contact_b.ray_depths);
        } else if contact_count > 0 {
            self.cells
                .compress_section_contact(a, section_a, normal, contact_a.depth_sum * 0.5);
            self.cells
                .compress_section_contact(b, section_b, -normal, contact_b.depth_sum * 0.5);
        }

        let membrane_penetration = if contact_count > 0 {
            (contact_a.depth_sum + contact_b.depth_sum) / contact_count as f32
        } else {
            0.0
        };
        let penetration = membrane_penetration.max(core_penetration);
        let stiffness = if core_penetration > 0.0 {
            self.collision_stiffness * HARD_CORE_STIFFNESS_MULTIPLIER
        } else {
            self.collision_stiffness
        };
        let relative_velocity =
            self.cells.section_velocity(b, section_b) - self.cells.section_velocity(a, section_a);
        let relative_normal_speed = relative_velocity.dot(normal);
        let force =
            (stiffness * penetration - self.collision_damping * relative_normal_speed).max(0.0);
        let force_step = normal * force * dt;
        let inverse_mass_a = section_radius_a.powi(2).max(1.0).recip();
        let inverse_mass_b = section_radius_b.powi(2).max(1.0).recip();
        self.cells
            .apply_section_impulse(a, section_a, -force_step * inverse_mass_a);
        self.cells
            .apply_section_impulse(b, section_b, force_step * inverse_mass_b);

        let contact_scale = broad_min_dist.max(0.001);
        let wobble = ((penetration / contact_scale).clamp(0.0, 1.0)
            + relative_normal_speed.abs() / 160.0)
            .min(1.0)
            * JELLY_HIT_GAIN;
        self.cells.jelly_intensity[a] = (self.cells.jelly_intensity[a] + wobble).min(1.0);
        self.cells.jelly_intensity[b] = (self.cells.jelly_intensity[b] + wobble).min(1.0);
        self.cells.jelly_dir_x[a] = -normal.x;
        self.cells.jelly_dir_y[a] = -normal.y;
        self.cells.jelly_dir_x[b] = normal.x;
        self.cells.jelly_dir_y[b] = normal.y;
    }

    fn resolve_compound_cell_pair(&mut self, a: usize, b: usize, dt: f32) {
        let Some(contact) = find_compound_contact(&self.cells, a, b) else {
            return;
        };
        self.cells.compress_curve_contact(
            a,
            contact.t_a,
            contact.normal,
            contact.penetration * 0.5,
        );
        self.cells.compress_curve_contact(
            b,
            contact.t_b,
            -contact.normal,
            contact.penetration * 0.5,
        );

        let stiffness = if contact.core_penetration > 0.0 {
            self.collision_stiffness * HARD_CORE_STIFFNESS_MULTIPLIER
        } else {
            self.collision_stiffness
        };
        let relative_normal_speed = (contact.velocity_b - contact.velocity_a).dot(contact.normal);
        let penetration = contact.penetration.max(contact.core_penetration);
        let force =
            (stiffness * penetration - self.collision_damping * relative_normal_speed).max(0.0);
        let force_step = contact.normal * force * dt;
        let inverse_mass_a = contact.radius_a.powi(2).max(1.0).recip();
        let inverse_mass_b = contact.radius_b.powi(2).max(1.0).recip();
        self.cells
            .apply_curve_impulse(a, contact.t_a, -force_step * inverse_mass_a);
        self.cells
            .apply_curve_impulse(b, contact.t_b, force_step * inverse_mass_b);

        if contact.core_penetration > 0.0 && relative_normal_speed < 0.0 {
            let inverse_mass_sum = (inverse_mass_a + inverse_mass_b).max(0.0001);
            let stop_speed =
                -relative_normal_speed + (contact.core_penetration * 6.0).clamp(0.0, 36.0);
            let constraint_impulse = contact.normal * (stop_speed / inverse_mass_sum);
            self.cells
                .apply_curve_impulse(a, contact.t_a, -constraint_impulse * inverse_mass_a);
            self.cells
                .apply_curve_impulse(b, contact.t_b, constraint_impulse * inverse_mass_b);
        }

        let contact_scale = (contact.radius_a + contact.radius_b).max(0.001);
        let wobble = ((penetration / contact_scale).clamp(0.0, 1.0)
            + relative_normal_speed.abs() / 160.0)
            .min(1.0)
            * JELLY_HIT_GAIN;
        self.cells.jelly_intensity[a] = (self.cells.jelly_intensity[a] + wobble).min(1.0);
        self.cells.jelly_intensity[b] = (self.cells.jelly_intensity[b] + wobble).min(1.0);
        self.cells.jelly_dir_x[a] = -contact.normal.x;
        self.cells.jelly_dir_y[a] = -contact.normal.y;
        self.cells.jelly_dir_x[b] = contact.normal.x;
        self.cells.jelly_dir_y[b] = contact.normal.y;
    }

    fn bounce_cell(&mut self, i: usize) {
        let r = self.cells.collision_radius[i];
        let bounced = bounce_point_in_arena(
            &mut self.cells.x[i],
            &mut self.cells.y[i],
            &mut self.cells.vx[i],
            &mut self.cells.vy[i],
            self.width,
            self.height,
            self.arena_shape,
            r,
        );

        if bounced
            && self.cells.vx[i] * self.cells.vx[i] + self.cells.vy[i] * self.cells.vy[i] > 1.0
        {
            self.cells.heading[i] = self.cells.vy[i].atan2(self.cells.vx[i]);
        }
    }
}

#[derive(Default)]
struct MembraneContact {
    ray_depths: [f32; SOFT_BODY_POINTS],
    depth_sum: f32,
    count: usize,
}

#[derive(Clone, Copy)]
struct CompoundCurveSample {
    center: Vec2,
    velocity: Vec2,
    radius: f32,
    core_radius: f32,
}

impl CompoundCurveSample {
    fn lerp(self, other: Self, t: f32, center: Vec2) -> Self {
        Self {
            center,
            velocity: self.velocity.lerp(other.velocity, t),
            radius: self.radius + (other.radius - self.radius) * t,
            core_radius: self.core_radius + (other.core_radius - self.core_radius) * t,
        }
    }
}

#[derive(Clone, Copy)]
struct CompoundContact {
    t_a: f32,
    t_b: f32,
    normal: Vec2,
    penetration: f32,
    core_penetration: f32,
    radius_a: f32,
    radius_b: f32,
    velocity_a: Vec2,
    velocity_b: Vec2,
}

fn compound_curve_sample(cells: &CellStore, index: usize, t: f32) -> CompoundCurveSample {
    if cells.section_count[index] < 2 {
        return CompoundCurveSample {
            center: Vec2::new(cells.x[index], cells.y[index]),
            velocity: Vec2::new(cells.vx[index], cells.vy[index]),
            radius: cells.collision_radius[index],
            core_radius: cells.core_radius[index],
        };
    }

    let edge_count = cells.section_count[index] as usize - 1;
    let scaled = t.clamp(0.0, 1.0) * edge_count as f32;
    let edge = scaled.floor().min((edge_count - 1) as f32) as usize;
    let local = (scaled - edge as f32).clamp(0.0, 1.0);
    let first_section = cells.section_parents[index][edge];
    let second_section = edge as u8 + 1;
    let first_center = cells.section_center(index, first_section);
    let second_center = cells.section_center(index, second_section);
    let edge_axis = second_center - first_center;
    let edge_side = edge_axis
        .try_normalize()
        .map(|direction| Vec2::new(-direction.y, direction.x))
        .unwrap_or(Vec2::Y);
    let control =
        (first_center + second_center) * 0.5 + edge_side * cells.edge_curve_offsets[index][edge];
    let center = first_center
        .lerp(control, local)
        .lerp(control.lerp(second_center, local), local);
    let velocity = cells
        .section_velocity(index, first_section)
        .lerp(cells.section_velocity(index, second_section), local);
    let first_radius = cells.section_collision_radius(index, first_section);
    let second_radius = cells.section_collision_radius(index, second_section);
    let radius = first_radius + (second_radius - first_radius) * local;
    let first_core = cells.section_core_radius(index, first_section);
    let second_core = cells.section_core_radius(index, second_section);
    let core_radius = first_core + (second_core - first_core) * local;
    CompoundCurveSample {
        center,
        velocity,
        radius,
        core_radius,
    }
}

fn find_compound_contact(cells: &CellStore, a: usize, b: usize) -> Option<CompoundContact> {
    let segments_a = if cells.section_count[a] >= 2 {
        (cells.section_count[a] as usize - 1) * 2
    } else {
        1
    };
    let segments_b = if cells.section_count[b] >= 2 {
        (cells.section_count[b] as usize - 1) * 2
    } else {
        1
    };
    let samples_a = compound_curve_samples(cells, a, segments_a);
    let samples_b = compound_curve_samples(cells, b, segments_b);
    let mut best: Option<CompoundContact> = None;

    for segment_a in 0..segments_a {
        let a_t0 = if cells.section_count[a] >= 2 {
            segment_a as f32 / segments_a as f32
        } else {
            0.0
        };
        let a_t1 = if cells.section_count[a] >= 2 {
            (segment_a + 1) as f32 / segments_a as f32
        } else {
            0.0
        };
        let a0 = samples_a[segment_a];
        let a1 = samples_a[segment_a + 1];

        for segment_b in 0..segments_b {
            let b_t0 = if cells.section_count[b] >= 2 {
                segment_b as f32 / segments_b as f32
            } else {
                0.0
            };
            let b_t1 = if cells.section_count[b] >= 2 {
                (segment_b + 1) as f32 / segments_b as f32
            } else {
                0.0
            };
            let b0 = samples_b[segment_b];
            let b1 = samples_b[segment_b + 1];
            let (local_a, local_b, point_a, point_b) =
                closest_segment_points(a0.center, a1.center, b0.center, b1.center);
            let t_a = a_t0 + (a_t1 - a_t0) * local_a;
            let t_b = b_t0 + (b_t1 - b_t0) * local_b;
            let sample_a = a0.lerp(a1, local_a, point_a);
            let sample_b = b0.lerp(b1, local_b, point_b);
            let delta = point_b - point_a;
            let distance_sq = delta.length_squared();
            let sample_radius = sample_a.radius + sample_b.radius;
            if distance_sq >= sample_radius * sample_radius {
                continue;
            }
            let distance = distance_sq.sqrt();
            let normal = if distance_sq > 0.0001 {
                delta / distance
            } else {
                (sample_b.center - sample_a.center)
                    .try_normalize()
                    .unwrap_or_else(|| {
                        let tangent = (a1.center - a0.center).try_normalize().unwrap_or(Vec2::X);
                        Vec2::new(-tangent.y, tangent.x)
                    })
            };
            let radius_a = compound_membrane_radius(cells, a, t_a, normal);
            let radius_b = compound_membrane_radius(cells, b, t_b, -normal);
            let penetration = radius_a + radius_b - distance;
            if penetration <= 0.0 {
                continue;
            }
            let core_penetration =
                (sample_a.core_radius + sample_b.core_radius - distance).max(0.0);
            let contact = CompoundContact {
                t_a,
                t_b,
                normal,
                penetration,
                core_penetration,
                radius_a,
                radius_b,
                velocity_a: sample_a.velocity,
                velocity_b: sample_b.velocity,
            };
            if best
                .as_ref()
                .is_none_or(|previous| penetration > previous.penetration)
            {
                best = Some(contact);
            }
        }
    }
    best
}

#[derive(Clone, Copy)]
struct LysisContact {
    section_a: u8,
    section_b: u8,
    normal: Vec2,
    point: Vec2,
}

fn section_near_body_t(cells: &CellStore, index: usize, t: f32) -> u8 {
    if cells.section_count[index] < 2 {
        return 0;
    }
    let edge_count = cells.section_count[index] as usize - 1;
    let scaled = t.clamp(0.0, 1.0) * edge_count as f32;
    let edge = scaled.floor().min((edge_count - 1) as f32) as usize;
    let local = (scaled - edge as f32).clamp(0.0, 1.0);
    if local < 0.5 {
        cells.section_parents[index][edge]
    } else {
        edge as u8 + 1
    }
}

fn compound_cells_lysis_contact(
    cells: &CellStore,
    a: usize,
    b: usize,
    margin: f32,
) -> Option<LysisContact> {
    let segments_a = if cells.section_count[a] >= 2 {
        (cells.section_count[a] as usize - 1) * 2
    } else {
        1
    };
    let segments_b = if cells.section_count[b] >= 2 {
        (cells.section_count[b] as usize - 1) * 2
    } else {
        1
    };
    let samples_a = compound_curve_samples(cells, a, segments_a);
    let samples_b = compound_curve_samples(cells, b, segments_b);
    let mut best: Option<(f32, LysisContact)> = None;
    for segment_a in 0..segments_a {
        for segment_b in 0..segments_b {
            let (local_a, local_b, point_a, point_b) = closest_segment_points(
                samples_a[segment_a].center,
                samples_a[segment_a + 1].center,
                samples_b[segment_b].center,
                samples_b[segment_b + 1].center,
            );
            let delta = point_b - point_a;
            let distance = delta.length();
            let direction = delta.try_normalize().unwrap_or(Vec2::X);
            let t_a = (segment_a as f32 + local_a) / segments_a as f32;
            let t_b = (segment_b as f32 + local_b) / segments_b as f32;
            let radius_a = compound_membrane_radius(cells, a, t_a, direction);
            let radius_b = compound_membrane_radius(cells, b, t_b, -direction);
            let surface_distance = radius_a + radius_b + margin.max(0.0);
            if distance <= surface_distance {
                let surface_a = point_a + direction * radius_a;
                let surface_b = point_b - direction * radius_b;
                let contact = LysisContact {
                    section_a: section_near_body_t(cells, a, t_a),
                    section_b: section_near_body_t(cells, b, t_b),
                    normal: direction,
                    point: (surface_a + surface_b) * 0.5,
                };
                let gap = distance - radius_a - radius_b;
                if best.is_none_or(|(previous_gap, _)| gap < previous_gap) {
                    best = Some((gap, contact));
                }
            }
        }
    }
    best.map(|(_, contact)| contact)
}

fn compound_membrane_radius(cells: &CellStore, index: usize, t: f32, direction: Vec2) -> f32 {
    if cells.section_count[index] < 2 {
        return cells.section_membrane_radius(index, 0, direction.y.atan2(direction.x));
    }
    let edge_count = cells.section_count[index] as usize - 1;
    let scaled = t.clamp(0.0, 1.0) * edge_count as f32;
    let edge = scaled.floor().min((edge_count - 1) as f32) as usize;
    let local = (scaled - edge as f32).clamp(0.0, 1.0);
    let parent = cells.section_parents[index][edge];
    let child = edge as u8 + 1;
    let angle = direction.y.atan2(direction.x);
    let parent_radius = cells.section_membrane_radius(index, parent, angle);
    let child_radius = cells.section_membrane_radius(index, child, angle);
    let end_blend = (local * 2.0 - 1.0).powi(2);
    (parent_radius + (child_radius - parent_radius) * local) * (0.78 + end_blend * 0.22)
}

fn compound_curve_samples(
    cells: &CellStore,
    index: usize,
    segments: usize,
) -> [CompoundCurveSample; MAX_COMPOUND_COLLISION_SEGMENTS + 1] {
    let first = compound_curve_sample(cells, index, 0.0);
    let mut samples = [first; MAX_COMPOUND_COLLISION_SEGMENTS + 1];
    for (sample_index, sample) in samples.iter_mut().enumerate().take(segments + 1) {
        let t = if cells.section_count[index] >= 2 {
            sample_index as f32 / segments as f32
        } else {
            0.0
        };
        *sample = compound_curve_sample(cells, index, t);
    }
    samples
}

fn cell_body_overlaps_circle(
    cells: &CellStore,
    index: usize,
    circle_center: Vec2,
    circle_radius: f32,
) -> bool {
    if cells.section_count[index] < 2 {
        let center = Vec2::new(cells.x[index], cells.y[index]);
        let contact_radius = cells.collision_radius[index] + circle_radius;
        return center.distance_squared(circle_center) <= contact_radius * contact_radius;
    }

    let segment_count = (cells.section_count[index] as usize - 1) * 2;
    let samples = compound_curve_samples(cells, index, segment_count);
    for segment in 0..segment_count {
        let start = samples[segment];
        let end = samples[segment + 1];
        let axis = end.center - start.center;
        let axis_length_sq = axis.length_squared();
        let t = if axis_length_sq > 0.000001 {
            ((circle_center - start.center).dot(axis) / axis_length_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let closest = start.center + axis * t;
        let direction = (circle_center - closest).try_normalize().unwrap_or(Vec2::X);
        let body_t = (segment as f32 + t) / segment_count as f32;
        let body_radius = compound_membrane_radius(cells, index, body_t, direction);
        let contact_radius = body_radius + circle_radius;
        if closest.distance_squared(circle_center) <= contact_radius * contact_radius {
            return true;
        }
    }
    false
}

fn closest_segment_points(p1: Vec2, q1: Vec2, p2: Vec2, q2: Vec2) -> (f32, f32, Vec2, Vec2) {
    let first = q1 - p1;
    let second = q2 - p2;
    let offset = p1 - p2;
    let first_len_sq = first.length_squared();
    let second_len_sq = second.length_squared();
    let mut first_t;
    let second_t;

    if first_len_sq <= 0.000001 && second_len_sq <= 0.000001 {
        return (0.0, 0.0, p1, p2);
    }
    if first_len_sq <= 0.000001 {
        first_t = 0.0;
        second_t = (second.dot(-offset) / second_len_sq).clamp(0.0, 1.0);
    } else {
        let first_offset = first.dot(offset);
        if second_len_sq <= 0.000001 {
            second_t = 0.0;
            first_t = (-first_offset / first_len_sq).clamp(0.0, 1.0);
        } else {
            let second_offset = second.dot(offset);
            let cross = first.dot(second);
            let denominator = first_len_sq * second_len_sq - cross * cross;
            first_t = if denominator.abs() > 0.000001 {
                ((cross * second_offset - first_offset * second_len_sq) / denominator)
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            let projected_second = (cross * first_t + second_offset) / second_len_sq;
            if projected_second < 0.0 {
                second_t = 0.0;
                first_t = (-first_offset / first_len_sq).clamp(0.0, 1.0);
            } else if projected_second > 1.0 {
                second_t = 1.0;
                first_t = ((cross - first_offset) / first_len_sq).clamp(0.0, 1.0);
            } else {
                second_t = projected_second;
            }
        }
    }
    (
        first_t,
        second_t,
        p1 + first * first_t,
        p2 + second * second_t,
    )
}

fn closest_section_pair(cells: &CellStore, a: usize, b: usize) -> (u8, u8) {
    let mut best = (0, 0);
    let mut best_distance_sq = f32::INFINITY;
    for section_a in 0..cells.section_count[a] {
        for section_b in 0..cells.section_count[b] {
            let distance_sq = cells
                .section_center(a, section_a)
                .distance_squared(cells.section_center(b, section_b));
            if distance_sq < best_distance_sq {
                best_distance_sq = distance_sq;
                best = (section_a, section_b);
            }
        }
    }
    best
}

fn sample_membrane_contact(
    cells: &CellStore,
    source: usize,
    target: usize,
    source_to_target: Vec2,
) -> MembraneContact {
    let source_center = Vec2::new(cells.x[source], cells.y[source]);
    let target_center = Vec2::new(cells.x[target], cells.y[target]);
    let (source_heading_s, source_heading_c) = cells.heading[source].sin_cos();
    let target_heading = cells.heading[target];
    let mut contact = MembraneContact {
        ray_depths: [0.0; SOFT_BODY_POINTS],
        depth_sum: 0.0,
        count: 0,
    };

    for ray_index in 0..SOFT_BODY_POINTS {
        let local_x = cells.ray_dir_x[source][ray_index];
        let local_y = cells.ray_dir_y[source][ray_index];
        let ray_direction = Vec2::new(
            local_x * source_heading_c - local_y * source_heading_s,
            local_x * source_heading_s + local_y * source_heading_c,
        );
        if ray_direction.dot(source_to_target) <= 0.0 {
            continue;
        }

        let tip = source_center + ray_direction * cells.current_radii[source][ray_index];
        let target_to_tip = tip - target_center;
        let distance_sq = target_to_tip.length_squared();
        let distance = distance_sq.sqrt();
        let sample_direction = if distance_sq > 0.0001 {
            target_to_tip / distance
        } else {
            -source_to_target
        };
        let local_angle = sample_direction.y.atan2(sample_direction.x) - target_heading;
        let membrane_radius = cells.virtual_membrane_radius_local(target, local_angle);
        let penetration = membrane_radius - distance;
        if penetration <= 0.0 {
            continue;
        }

        contact.ray_depths[ray_index] = penetration;
        contact.depth_sum += penetration;
        contact.count += 1;
    }

    contact
}

fn liquid_current_at(position: Vec2, time: f32) -> Vec2 {
    let t = time * LIQUID_FLOW_SPEED * 8.0;
    let p = position / LIQUID_FLOW_SCALE;
    let drift = Vec2::new((t * 0.23).sin(), (t * 0.19).cos()) * 0.35;
    let p = p + drift;

    let wave_a = (p.x * 1.7 + (p.y * 0.9 + t * 0.7).sin() + t).sin();
    let wave_b = ((p.x + p.y) * 1.15 - t * 0.63).sin();
    let wave_c = ((p.x * 0.62 - p.y * 1.31) + t * 0.37).cos();
    let wave_d = ((p.x * 1.41 + p.y * 0.73) - t * 0.29).sin();

    let direction = Vec2::new(wave_b - wave_c * 0.7, wave_a + wave_d * 0.7);
    let len_sq = direction.length_squared();
    if len_sq < 0.0001 {
        return Vec2::ZERO;
    }

    let pulse = 0.55 + 0.45 * ((p.x * 0.8 - p.y * 0.6 + t * 0.42).sin() * 0.5 + 0.5);
    direction * len_sq.sqrt().recip() * pulse
}

fn clamp_bounce_axis(value: &mut f32, velocity: &mut f32, half_extent: f32, margin: f32) {
    let min = -half_extent + margin;
    let max = half_extent - margin;

    if *value < min {
        *value = min;
        *velocity = velocity.abs() * 0.35;
    } else if *value > max {
        *value = max;
        *velocity = -velocity.abs() * 0.35;
    }
}

fn wrap_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn angle_delta(target: f32, current: f32) -> f32 {
    wrap_angle(target - current)
}

fn mutation_chance(susceptibility: f32) -> f32 {
    let t = (susceptibility / CELL_MUTATION_DISPLAY_MAX).clamp(0.0, 1.0);
    MUTATION_CHANCE_MIN + (MUTATION_CHANCE_MAX - MUTATION_CHANCE_MIN) * t
}

fn mutation_power(susceptibility: f32) -> f32 {
    let t = (susceptibility / CELL_MUTATION_DISPLAY_MAX).clamp(0.0, 1.0);
    MUTATION_POWER_MIN + (MUTATION_POWER_MAX - MUTATION_POWER_MIN) * t
}

fn mutate_gene(value: f32, min: f32, max: f32, susceptibility: f32, rng: &mut SmallRng) -> f32 {
    let mut mutated = value;
    if rng.random_bool(mutation_chance(susceptibility) as f64) {
        let sign = if rng.random_bool(0.5) { -1.0 } else { 1.0 };
        let strength = rng.random_range(MUTATION_STRENGTH_MIN..MUTATION_STRENGTH_MAX)
            * mutation_power(susceptibility);
        mutated += value * strength * sign;
    }

    mutated.clamp(min, max)
}

#[derive(Clone, Copy, Debug)]
enum SeedGeometryMode {
    Uniform,
    AxialStretch,
    ExtremeAxis,
    AlternatingBend,
    OneSidedCurve,
    CenterWaist,
    AxisPinch,
    DiagonalExpansion,
    Triangular,
    Cruciform,
    Lancet,
    PlacoidShield,
    Chaotic,
}

const ALL_SEED_GEOMETRY_MODES: [SeedGeometryMode; 13] = [
    SeedGeometryMode::Uniform,
    SeedGeometryMode::AxialStretch,
    SeedGeometryMode::ExtremeAxis,
    SeedGeometryMode::AlternatingBend,
    SeedGeometryMode::OneSidedCurve,
    SeedGeometryMode::CenterWaist,
    SeedGeometryMode::AxisPinch,
    SeedGeometryMode::DiagonalExpansion,
    SeedGeometryMode::Triangular,
    SeedGeometryMode::Cruciform,
    SeedGeometryMode::Lancet,
    SeedGeometryMode::PlacoidShield,
    SeedGeometryMode::Chaotic,
];

#[derive(Component, Clone, Debug)]
pub struct SoftBodyCell {
    pub speed: f32,
    pub energy: f32,
    pub agility: f32,
    #[allow(dead_code)]
    pub perception: f32,
    #[allow(dead_code)]
    pub persistence: f32,
    pub mutation_factor: f32,
    pub size: f32,
    pub base_radii: [f32; SOFT_BODY_POINTS],
    pub current_radii: [f32; SOFT_BODY_POINTS],
    pub angle_offsets: [f32; SOFT_BODY_POINTS],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeMutationEvent {
    Single,
    Axial,
    Sector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellShapeClass {
    Coccus,
    Bacillus,
    Filament,
    Spirillum,
    Vibrio,
    Diplococcus,
    Fusiform,
    Cuboid,
    Triquetrum,
    Stauromorph,
    Lancetiform,
    Placoid,
    Lobatum,
}

impl CellShapeClass {
    pub const fn label_ru(self) -> &'static str {
        match self {
            Self::Triquetrum => "Триквитрум / тригональная клетка",
            Self::Stauromorph => "Ставроморф / круциформ",
            Self::Lancetiform => "Ланцетовидная клетка",
            Self::Placoid => "Плакоид / кутикулярный щит",
            Self::Coccus => "Кокк",
            Self::Bacillus => "Бацилла",
            Self::Filament => "Филамент / нематода",
            Self::Spirillum => "Спирилла",
            Self::Vibrio => "Вибрион",
            Self::Diplococcus => "Диплококк",
            Self::Fusiform => "Веретено",
            Self::Cuboid => "Кубоид",
            Self::Lobatum => "Лобатум / амеба",
        }
    }
}

fn random_seed_shape(
    size: f32,
    required_mode: Option<SeedGeometryMode>,
    weights: &[f32; CELL_SHAPE_COUNT],
    rng: &mut SmallRng,
) -> ([f32; SOFT_BODY_POINTS], [f32; SOFT_BODY_POINTS]) {
    let mode = required_mode.unwrap_or_else(|| {
        let total_weight = weights.iter().sum::<f32>().max(0.0001);
        let mut roll = rng.random_range(0.0..total_weight);
        for (index, weight) in weights.iter().copied().enumerate() {
            if roll < weight {
                return ALL_SEED_GEOMETRY_MODES[index];
            }
            roll -= weight;
        }
        SeedGeometryMode::Chaotic
    });

    let jitter = |rng: &mut SmallRng, scale: f32| rng.random_range(-scale..scale);
    let scale_radii = |values: [f32; 8]| values.map(|value| value * size);
    let (mut radii, mut offsets) = match mode {
        SeedGeometryMode::Uniform => (
            scale_radii([0.82, 0.83, 0.81, 0.84, 0.82, 0.83, 0.81, 0.84]),
            [0.0; 8],
        ),
        SeedGeometryMode::AxialStretch => (
            scale_radii([1.0, 0.53, 0.46, 0.53, 1.0, 0.53, 0.46, 0.53]),
            [0.0; 8],
        ),
        SeedGeometryMode::ExtremeAxis => (
            scale_radii([1.0, 0.31, 0.30, 0.31, 1.0, 0.31, 0.30, 0.31]),
            [0.0; 8],
        ),
        SeedGeometryMode::AlternatingBend => (
            scale_radii([1.0, 0.31, 0.30, 0.31, 1.0, 0.31, 0.30, 0.31]),
            [0.08, -0.08, 0.08, -0.08, 0.08, -0.08, 0.08, -0.08],
        ),
        SeedGeometryMode::OneSidedCurve => (
            scale_radii([0.95, 0.92, 0.62, 0.42, 0.40, 0.42, 0.62, 0.92]),
            [0.0, -0.11, 0.0, 0.11, 0.0, -0.11, 0.0, 0.11],
        ),
        SeedGeometryMode::CenterWaist => (
            scale_radii([1.0, 0.90, 0.31, 0.90, 1.0, 0.90, 0.31, 0.90]),
            [0.0; 8],
        ),
        SeedGeometryMode::AxisPinch => (
            scale_radii([1.0, 0.60, 0.50, 0.60, 1.0, 0.60, 0.50, 0.60]),
            [0.0, 0.13, 0.0, -0.13, 0.0, 0.13, 0.0, -0.13],
        ),
        SeedGeometryMode::DiagonalExpansion => (
            scale_radii([0.68, 0.95, 0.68, 0.95, 0.68, 0.95, 0.68, 0.95]),
            [0.0; 8],
        ),
        SeedGeometryMode::Triangular => (
            scale_radii([1.0, 0.72, 0.58, 0.95, 0.70, 0.95, 0.58, 0.72]),
            [
                0.0,
                0.0,
                0.0,
                -SOFT_BODY_MAX_ANGLE_OFFSET,
                0.0,
                SOFT_BODY_MAX_ANGLE_OFFSET,
                0.0,
                0.0,
            ],
        ),
        SeedGeometryMode::Cruciform => (
            scale_radii([1.0, 0.42, 1.0, 0.42, 1.0, 0.42, 1.0, 0.42]),
            [0.0; 8],
        ),
        SeedGeometryMode::Lancet => (
            scale_radii([1.0, 0.62, 0.31, 0.62, 1.0, 0.62, 0.31, 0.62]),
            [0.0, 0.15, 0.0, -0.15, 0.0, 0.15, 0.0, -0.15],
        ),
        SeedGeometryMode::PlacoidShield => (
            scale_radii([1.0, 0.94, 0.76, 0.58, 0.54, 0.58, 0.76, 0.94]),
            [0.0; 8],
        ),
        SeedGeometryMode::Chaotic => (
            scale_radii([0.96, 0.43, 0.78, 0.54, 0.64, 0.91, 0.47, 0.82]),
            [0.14, -0.03, 0.09, 0.02, -0.13, 0.05, -0.08, 0.11],
        ),
    };

    let global_scale = 1.0 + jitter(rng, 0.025);
    let broad_phase = rng.random_range(0.0..std::f32::consts::TAU);
    let fine_phase = rng.random_range(0.0..std::f32::consts::TAU);
    let broad_amplitude = rng.random_range(0.018..0.045);
    let fine_amplitude = rng.random_range(0.008..0.022);

    for (index, radius) in radii.iter_mut().enumerate() {
        let angle = index as f32 * SOFT_BODY_SECTOR_ANGLE;
        let organic_wave = (angle * 2.0 + broad_phase).sin() * broad_amplitude
            + (angle * 3.0 + fine_phase).sin() * fine_amplitude;
        let local_variation = jitter(rng, 0.014);
        *radius = (*radius * (global_scale + organic_wave + local_variation))
            .clamp(size * SOFT_BODY_BASE_MIN_FACTOR, size);
    }

    let angle_phase = rng.random_range(0.0..std::f32::consts::TAU);
    for (index, offset) in offsets.iter_mut().enumerate() {
        let angle = index as f32 * SOFT_BODY_SECTOR_ANGLE;
        let organic_drift = (angle * 2.0 + angle_phase).sin() * 0.012;
        *offset = (*offset + organic_drift + jitter(rng, 0.009))
            .clamp(-SOFT_BODY_MAX_ANGLE_OFFSET, SOFT_BODY_MAX_ANGLE_OFFSET);
    }

    (radii, offsets)
}

fn random_free_shape(
    size: f32,
    rng: &mut SmallRng,
) -> ([f32; SOFT_BODY_POINTS], [f32; SOFT_BODY_POINTS]) {
    let mut raw = [0.0; SOFT_BODY_POINTS];
    for radius in &mut raw {
        *radius = rng.random_range(0.38..1.0);
    }
    let mut radii = [0.0; SOFT_BODY_POINTS];
    for index in 0..SOFT_BODY_POINTS {
        radii[index] =
            ((raw[(index + 7) % 8] + raw[index] * 2.0 + raw[(index + 1) % 8]) * 0.25 * size)
                .clamp(size * SOFT_BODY_BASE_MIN_FACTOR, size);
    }
    let offsets = std::array::from_fn(|_| {
        rng.random_range(-SOFT_BODY_MAX_ANGLE_OFFSET..SOFT_BODY_MAX_ANGLE_OFFSET)
    });
    (radii, offsets)
}

pub fn mutate_soft_body_shape(
    cell: &mut SoftBodyCell,
    rng: &mut SmallRng,
) -> Option<ShapeMutationEvent> {
    if !rng.random_bool(mutation_chance(cell.mutation_factor) as f64) {
        cell.current_radii = clamp_current_radii(cell.base_radii, cell.current_radii, cell.size);
        return None;
    }

    let event = match rng.random_range(0.0..1.0) {
        roll if roll < 0.60 => ShapeMutationEvent::Single,
        roll if roll < 0.90 => ShapeMutationEvent::Axial,
        _ => ShapeMutationEvent::Sector,
    };
    let anchor = rng.random_range(0..SOFT_BODY_POINTS);
    let mutate_angle = rng.random_bool(0.5);
    let power = mutation_power(cell.mutation_factor);
    let length_delta =
        rng.random_range(-1.0..1.0) * cell.size * SHAPE_MUTATION_LENGTH_SCALE * power;
    let angle_delta =
        rng.random_range(-SOFT_BODY_MUTATION_ANGLE_DELTA..SOFT_BODY_MUTATION_ANGLE_DELTA) * power;

    let mut indices = [anchor; 3];
    let count = match event {
        ShapeMutationEvent::Single => 1,
        ShapeMutationEvent::Axial => {
            indices[1] = (anchor + SOFT_BODY_POINTS / 2) % SOFT_BODY_POINTS;
            2
        }
        ShapeMutationEvent::Sector => {
            indices[0] = (anchor + SOFT_BODY_POINTS - 1) % SOFT_BODY_POINTS;
            indices[1] = anchor;
            indices[2] = (anchor + 1) % SOFT_BODY_POINTS;
            3
        }
    };

    for &index in &indices[..count] {
        if mutate_angle {
            cell.angle_offsets[index] = (cell.angle_offsets[index] + angle_delta)
                .clamp(-SOFT_BODY_MAX_ANGLE_OFFSET, SOFT_BODY_MAX_ANGLE_OFFSET);
        } else {
            cell.base_radii[index] = (cell.base_radii[index] + length_delta)
                .clamp(cell.size * SOFT_BODY_BASE_MIN_FACTOR, cell.size);
        }
    }

    cell.current_radii = clamp_current_radii(cell.base_radii, cell.current_radii, cell.size);
    Some(event)
}

fn clamp_current_radii(
    base_radii: [f32; SOFT_BODY_POINTS],
    mut current_radii: [f32; SOFT_BODY_POINTS],
    size: f32,
) -> [f32; SOFT_BODY_POINTS] {
    let core_radius = size * CORE_RADIUS_FACTOR;
    for index in 0..SOFT_BODY_POINTS {
        current_radii[index] = current_radii[index].clamp(core_radius, base_radii[index]);
    }
    current_radii
}

pub fn analyze_cell_shape(cell: &SoftBodyCell) -> String {
    analyze_cell_shape_class(cell).label_ru().to_string()
}

pub fn analyze_cell_shape_class(cell: &SoftBodyCell) -> CellShapeClass {
    debug_assert!(
        cell.speed.is_finite()
            && cell.energy.is_finite()
            && cell.agility.is_finite()
            && cell.mutation_factor.is_finite()
            && cell.size.is_finite()
    );
    let radii = cell.base_radii;
    let offsets = cell.angle_offsets;
    let mean = radii.iter().sum::<f32>() / SOFT_BODY_POINTS as f32;
    let min = radii.iter().copied().fold(f32::INFINITY, f32::min);
    let max = radii.iter().copied().fold(0.0, f32::max);
    let spread = (max - min) / mean.max(0.0001);
    let mean_angle = offsets.iter().map(|value| value.abs()).sum::<f32>() / SOFT_BODY_POINTS as f32;

    let mut best_axis = 0;
    let mut best_axis_mean = 0.0;
    for axis in 0..4 {
        let pair_mean = (radii[axis] + radii[axis + 4]) * 0.5;
        if pair_mean > best_axis_mean {
            best_axis = axis;
            best_axis_mean = pair_mean;
        }
    }

    let mut other_sum = 0.0;
    for (index, radius) in radii.iter().copied().enumerate() {
        if index != best_axis && index != best_axis + 4 {
            other_sum += radius;
        }
    }
    let other_mean = other_sum / 6.0;
    let elongation = best_axis_mean / other_mean.max(0.0001);
    let perpendicular_mean = (radii[(best_axis + 2) % 8] + radii[(best_axis + 6) % 8]) * 0.5;
    let perpendicular_ratio = perpendicular_mean / best_axis_mean.max(0.0001);

    let alternating_signs = (0..SOFT_BODY_POINTS)
        .filter(|&index| {
            let next = (index + 1) % SOFT_BODY_POINTS;
            offsets[index] * offsets[next] < 0.0
                && offsets[index].abs().min(offsets[next].abs()) > 0.035
        })
        .count();

    let axial_mean = (radii[0] + radii[2] + radii[4] + radii[6]) * 0.25;
    let diagonal_mean = (radii[1] + radii[3] + radii[5] + radii[7]) * 0.25;
    let axial_variation = [radii[0], radii[2], radii[4], radii[6]]
        .into_iter()
        .map(|radius| (radius - axial_mean).abs())
        .sum::<f32>()
        / (axial_mean * 4.0).max(0.0001);
    let diagonal_variation = [radii[1], radii[3], radii[5], radii[7]]
        .into_iter()
        .map(|radius| (radius - diagonal_mean).abs())
        .sum::<f32>()
        / (diagonal_mean * 4.0).max(0.0001);
    let square_ratio = diagonal_mean / axial_mean.max(0.0001);
    if axial_mean / diagonal_mean.max(0.0001) > 1.65 && axial_variation < 0.12 {
        return CellShapeClass::Stauromorph;
    }

    if elongation > 3.0 && perpendicular_ratio < 0.48 {
        return if alternating_signs >= 5 {
            CellShapeClass::Spirillum
        } else {
            CellShapeClass::Filament
        };
    }

    let side_a = radii[7] + radii[0] + radii[1];
    let side_b = radii[3] + radii[4] + radii[5];
    let side_imbalance = (side_a - side_b).abs() / (side_a + side_b).max(0.0001);
    let facing_angle_drift = offsets[7] - offsets[1] + offsets[3] - offsets[5];
    if side_imbalance > 0.20 && facing_angle_drift.abs() < 0.055 && mean_angle < 0.045 {
        return CellShapeClass::Placoid;
    }
    if side_imbalance > 0.22 && facing_angle_drift.abs() > 0.10 {
        return CellShapeClass::Vibrio;
    }

    let opposite_lobes = (0..4).any(|axis| {
        let lobes = (radii[axis] + radii[axis + 4]) * 0.5;
        let waist = (radii[(axis + 2) % 8] + radii[(axis + 6) % 8]) * 0.5;
        let shoulders = (radii[(axis + 1) % 8]
            + radii[(axis + 3) % 8]
            + radii[(axis + 5) % 8]
            + radii[(axis + 7) % 8])
            * 0.25;
        lobes > mean * 1.18 && shoulders > mean * 1.05 && waist < mean * 0.72
    });
    if opposite_lobes {
        return CellShapeClass::Diplococcus;
    }

    if (1.28..=1.52).contains(&square_ratio) && diagonal_variation < 0.12 {
        return CellShapeClass::Cuboid;
    }

    let prominent_peaks = (0..SOFT_BODY_POINTS)
        .filter(|&index| {
            let previous = radii[(index + 7) % 8];
            let next = radii[(index + 1) % 8];
            radii[index] > mean * 1.14
                && radii[index] > previous * 1.22
                && radii[index] > next * 1.22
        })
        .count();
    if prominent_peaks == 3 && mean_angle < 0.11 {
        return CellShapeClass::Triquetrum;
    }

    let adjacent_left = (best_axis + 7) % 8;
    let adjacent_right = (best_axis + 1) % 8;
    let opposite_adjacent_left = (best_axis + 3) % 8;
    let opposite_adjacent_right = (best_axis + 5) % 8;
    let pinched_to_axis = offsets[adjacent_left] < -0.09
        && offsets[adjacent_right] > 0.09
        && offsets[opposite_adjacent_left] < -0.09
        && offsets[opposite_adjacent_right] > 0.09;
    if elongation > 1.65 && perpendicular_ratio < 0.38 && pinched_to_axis {
        return CellShapeClass::Lancetiform;
    }
    if elongation > 1.45 && pinched_to_axis {
        return CellShapeClass::Fusiform;
    }

    if elongation > 1.55 && mean_angle < 0.055 {
        return CellShapeClass::Bacillus;
    }

    if spread < 0.15 && mean_angle < 0.06 {
        return CellShapeClass::Coccus;
    }

    CellShapeClass::Lobatum
}

fn mutate_child_soft_body(
    parent_base: [f32; SOFT_BODY_POINTS],
    parent_current: [f32; SOFT_BODY_POINTS],
    parent_offsets: [f32; SOFT_BODY_POINTS],
    size: f32,
    susceptibility: f32,
    rng: &mut SmallRng,
) -> (
    [f32; SOFT_BODY_POINTS],
    [f32; SOFT_BODY_POINTS],
    [f32; SOFT_BODY_POINTS],
) {
    let mut cell = SoftBodyCell {
        speed: 0.0,
        energy: 0.0,
        agility: 0.0,
        perception: 0.0,
        persistence: 0.0,
        mutation_factor: susceptibility,
        size,
        base_radii: parent_base,
        current_radii: parent_current,
        angle_offsets: parent_offsets,
    };
    mutate_soft_body_shape(&mut cell, rng);
    (cell.base_radii, cell.current_radii, cell.angle_offsets)
}

#[derive(Clone, Copy)]
struct SegmentSoftBodyProfile {
    size: f32,
    core_radius: f32,
    collision_radius: f32,
    base_radii: [f32; SOFT_BODY_POINTS],
    current_radii: [f32; SOFT_BODY_POINTS],
    visual_radii: [f32; SOFT_BODY_POINTS],
    angle_offsets: [f32; SOFT_BODY_POINTS],
}

impl SegmentSoftBodyProfile {
    fn from_shape(
        base_radii: [f32; SOFT_BODY_POINTS],
        current_radii: [f32; SOFT_BODY_POINTS],
        angle_offsets: [f32; SOFT_BODY_POINTS],
    ) -> Self {
        let size = soft_body_max_radius(&base_radii).max(0.1);
        let core_radius = size * CORE_RADIUS_FACTOR;
        let base_radii = base_radii.map(|radius| radius.clamp(core_radius, size));
        let current_radii = current_radii.map(|radius| radius.clamp(core_radius, size));
        let collision_radius = soft_body_max_radius(&current_radii).max(core_radius);
        Self {
            size,
            core_radius,
            collision_radius,
            base_radii,
            current_radii,
            visual_radii: current_radii,
            angle_offsets,
        }
    }

    fn dormant(position_radius: f32) -> Self {
        let radius = position_radius.max(0.1);
        Self::from_shape(
            [radius; SOFT_BODY_POINTS],
            [radius; SOFT_BODY_POINTS],
            [0.0; SOFT_BODY_POINTS],
        )
    }
}

fn soft_body_max_radius(radii: &[f32; SOFT_BODY_POINTS]) -> f32 {
    radii.iter().copied().fold(0.0, f32::max)
}

fn scaled_soft_body_shape(
    source_radii: [f32; SOFT_BODY_POINTS],
    source_offsets: [f32; SOFT_BODY_POINTS],
    target_size: f32,
    rng: &mut SmallRng,
) -> ([f32; SOFT_BODY_POINTS], [f32; SOFT_BODY_POINTS]) {
    let source_size = soft_body_max_radius(&source_radii).max(0.1);
    let core = target_size * CORE_RADIUS_FACTOR;
    let scale = target_size / source_size;
    let mut radii = source_radii.map(|radius| (radius * scale).clamp(core, target_size));
    let mut offsets = source_offsets;
    for ray in 0..SOFT_BODY_POINTS {
        radii[ray] = (radii[ray] * rng.random_range(0.94..1.06)).clamp(core, target_size);
        offsets[ray] = (offsets[ray] + rng.random_range(-0.025..0.025))
            .clamp(-SOFT_BODY_MAX_ANGLE_OFFSET, SOFT_BODY_MAX_ANGLE_OFFSET);
    }
    (radii, offsets)
}

fn random_segment_soft_body(
    source_radii: [f32; SOFT_BODY_POINTS],
    source_offsets: [f32; SOFT_BODY_POINTS],
    source_size: f32,
    random_geometry: bool,
    shape_weights: &[f32; CELL_SHAPE_COUNT],
    rng: &mut SmallRng,
) -> SegmentSoftBodyProfile {
    let segment_size = (source_size
        * rng.random_range(SEGMENT_SIZE_MIN_FACTOR..SEGMENT_SIZE_MAX_FACTOR))
    .clamp(CELL_SIZE_GENE_MIN * 0.55, CELL_SIZE_GENE_MAX * 1.25);
    let (base_radii, angle_offsets) = if rng.random_bool(SEGMENT_INHERIT_SHAPE_CHANCE) {
        scaled_soft_body_shape(source_radii, source_offsets, segment_size, rng)
    } else if random_geometry && rng.random_bool(0.68) {
        random_free_shape(segment_size, rng)
    } else {
        random_seed_shape(segment_size, None, shape_weights, rng)
    };
    SegmentSoftBodyProfile::from_shape(base_radii, base_radii, angle_offsets)
}

fn mutate_child_segment_soft_body(
    parent_base: [f32; SOFT_BODY_POINTS],
    parent_current: [f32; SOFT_BODY_POINTS],
    parent_offsets: [f32; SOFT_BODY_POINTS],
    susceptibility: f32,
    rng: &mut SmallRng,
) -> SegmentSoftBodyProfile {
    let parent_size = soft_body_max_radius(&parent_base).max(0.1);
    let child_size = mutate_gene(
        parent_size,
        CELL_SIZE_GENE_MIN * 0.55,
        CELL_SIZE_GENE_MAX * 1.25,
        susceptibility,
        rng,
    );
    let source_size = soft_body_max_radius(&parent_current).max(parent_size);
    let scale = child_size / source_size.max(0.1);
    let core = child_size * CORE_RADIUS_FACTOR;
    let scaled_base = parent_base.map(|radius| (radius * scale).clamp(core, child_size));
    let scaled_current = parent_current.map(|radius| (radius * scale).clamp(core, child_size));
    let (base_radii, current_radii, angle_offsets) = mutate_child_soft_body(
        scaled_base,
        scaled_current,
        parent_offsets,
        child_size,
        susceptibility,
        rng,
    );
    SegmentSoftBodyProfile::from_shape(base_radii, current_radii, angle_offsets)
}

#[derive(Clone, Copy)]
pub struct ExtraSection {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub core_radius: f32,
    pub collision_radius: f32,
    pub base_radii: [f32; SOFT_BODY_POINTS],
    pub current_radii: [f32; SOFT_BODY_POINTS],
    pub visual_radii: [f32; SOFT_BODY_POINTS],
    #[allow(dead_code)]
    pub angle_offsets: [f32; SOFT_BODY_POINTS],
}

impl ExtraSection {
    fn dormant(position: Vec2, radius: f32) -> Self {
        let profile = SegmentSoftBodyProfile::dormant(radius);
        Self::from_profile(position, Vec2::ZERO, profile)
    }

    fn from_profile(position: Vec2, velocity: Vec2, profile: SegmentSoftBodyProfile) -> Self {
        Self {
            x: position.x,
            y: position.y,
            vx: velocity.x,
            vy: velocity.y,
            core_radius: profile.core_radius,
            collision_radius: profile.collision_radius,
            base_radii: profile.base_radii,
            current_radii: profile.current_radii,
            visual_radii: profile.visual_radii,
            angle_offsets: profile.angle_offsets,
        }
    }
}

pub struct CellStore {
    pub id: Vec<u64>,
    next_id: u64,
    segmented_enabled: bool,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub heading: Vec<f32>,
    pub radius: Vec<f32>,
    pub core_radius: Vec<f32>,
    pub speed: Vec<f32>,
    pub turn_speed: Vec<f32>,
    pub perception: Vec<f32>,
    pub persistence: Vec<f32>,
    pub aggressiveness: Vec<f32>,
    pub lysis: Vec<f32>,
    lysis_cooldown: Vec<f32>,
    lysis_deform_time: Vec<[f32; 4]>,
    lysis_deform_duration: Vec<[f32; 4]>,
    lysis_deform_angle: Vec<[f32; 4]>,
    lysis_deform_amount: Vec<[f32; 4]>,
    hunt_pause: Vec<f32>,
    hunt_recheck: Vec<f32>,
    target_food: Vec<i32>,
    target_food_generation: Vec<u32>,
    target_last_x: Vec<f32>,
    target_last_y: Vec<f32>,
    target_memory: Vec<f32>,
    target_search_failed: Vec<bool>,
    target_recheck: Vec<f32>,
    target_cell: Vec<i32>,
    target_cell_id: Vec<u64>,
    pub section_count: Vec<u8>,
    pub section_spacing: Vec<f32>,
    pub section_bend: Vec<f32>,
    pub section_angles: Vec<[f32; 3]>,
    pub section_parents: Vec<[u8; 3]>,
    pub edge_curve_offsets: Vec<[f32; 3]>,
    pub extra_sections: Vec<[ExtraSection; 2]>,
    pub tail_x: Vec<f32>,
    pub tail_y: Vec<f32>,
    pub tail_vx: Vec<f32>,
    pub tail_vy: Vec<f32>,
    pub tail_core_radius: Vec<f32>,
    pub tail_base_radii: Vec<[f32; SOFT_BODY_POINTS]>,
    pub tail_current_radii: Vec<[f32; SOFT_BODY_POINTS]>,
    pub tail_visual_radii: Vec<[f32; SOFT_BODY_POINTS]>,
    pub tail_angle_offsets: Vec<[f32; SOFT_BODY_POINTS]>,
    tail_collision_radius: Vec<f32>,
    tail_ray_dir_x: Vec<[f32; SOFT_BODY_POINTS]>,
    tail_ray_dir_y: Vec<[f32; SOFT_BODY_POINTS]>,
    stuck_time: Vec<f32>,
    reverse_time: Vec<f32>,
    pub species: Vec<u32>,
    pub viability: Vec<f32>,
    pub max_viability: Vec<f32>,
    pub mutation_susceptibility: Vec<f32>,
    pub division_threshold: Vec<f32>,
    pub mitosis_progress: Vec<f32>,
    mitosis_recovery: Vec<f32>,
    pub base_radii: Vec<[f32; SOFT_BODY_POINTS]>,
    pub current_radii: Vec<[f32; SOFT_BODY_POINTS]>,
    pub visual_radii: Vec<[f32; SOFT_BODY_POINTS]>,
    pub angle_offsets: Vec<[f32; SOFT_BODY_POINTS]>,
    collision_radius: Vec<f32>,
    biomass: Vec<f32>,
    asymmetry_x: Vec<f32>,
    asymmetry_y: Vec<f32>,
    shape_drag: Vec<f32>,
    morphology_acceleration: Vec<f32>,
    morphology_turn: Vec<f32>,
    morphology_viability: Vec<f32>,
    morphology_metabolism: Vec<f32>,
    ray_dir_x: Vec<[f32; SOFT_BODY_POINTS]>,
    ray_dir_y: Vec<[f32; SOFT_BODY_POINTS]>,
    pub shape_wave_a: Vec<f32>,
    pub shape_wave_b: Vec<f32>,
    pub shape_phase: Vec<f32>,
    pub shape_softness: Vec<f32>,
    pub nucleus_offset_x: Vec<f32>,
    pub nucleus_offset_y: Vec<f32>,
    pub nucleus_radius: Vec<f32>,
    pub jelly_phase: Vec<f32>,
    pub jelly_intensity: Vec<f32>,
    pub jelly_dir_x: Vec<f32>,
    pub jelly_dir_y: Vec<f32>,
    pub wake_strength: Vec<f32>,
}

impl CellStore {
    #[inline]
    pub(crate) fn section_center(&self, index: usize, section: u8) -> Vec2 {
        match section {
            0 => Vec2::new(self.x[index], self.y[index]),
            1 => Vec2::new(self.tail_x[index], self.tail_y[index]),
            _ => {
                let extra = self.extra_sections[index][section as usize - 2];
                Vec2::new(extra.x, extra.y)
            }
        }
    }

    #[inline]
    pub(crate) fn section_velocity(&self, index: usize, section: u8) -> Vec2 {
        match section {
            0 => Vec2::new(self.vx[index], self.vy[index]),
            1 => Vec2::new(self.tail_vx[index], self.tail_vy[index]),
            _ => {
                let extra = self.extra_sections[index][section as usize - 2];
                Vec2::new(extra.vx, extra.vy)
            }
        }
    }

    #[inline]
    fn section_collision_radius(&self, index: usize, section: u8) -> f32 {
        match section {
            0 => self.collision_radius[index],
            1 => self.tail_collision_radius[index],
            _ => self.extra_sections[index][section as usize - 2].collision_radius,
        }
    }

    #[inline]
    fn section_soft_body_source(
        &self,
        index: usize,
        section: u8,
    ) -> (
        [f32; SOFT_BODY_POINTS],
        [f32; SOFT_BODY_POINTS],
        [f32; SOFT_BODY_POINTS],
    ) {
        match section {
            0 => (
                self.base_radii[index],
                self.current_radii[index],
                self.angle_offsets[index],
            ),
            1 => (
                self.tail_base_radii[index],
                self.tail_current_radii[index],
                self.tail_angle_offsets[index],
            ),
            _ => {
                let extra = self.extra_sections[index][section as usize - 2];
                (extra.base_radii, extra.current_radii, extra.angle_offsets)
            }
        }
    }

    #[inline]
    fn section_core_radius(&self, index: usize, section: u8) -> f32 {
        match section {
            0 => self.core_radius[index],
            1 => self.tail_core_radius[index],
            _ => self.extra_sections[index][section as usize - 2].core_radius,
        }
    }

    fn section_membrane_radius(&self, index: usize, section: u8, world_angle: f32) -> f32 {
        let (radii, offsets) = match section {
            0 => (self.current_radii[index], self.angle_offsets[index]),
            1 => (
                self.tail_current_radii[index],
                self.tail_angle_offsets[index],
            ),
            _ => {
                let extra = self.extra_sections[index][section as usize - 2];
                (extra.current_radii, extra.angle_offsets)
            }
        };
        let local_angle =
            (world_angle - self.section_heading(index, section)).rem_euclid(std::f32::consts::TAU);
        let sector = local_angle / SOFT_BODY_SECTOR_ANGLE;
        let first = sector.floor() as usize % SOFT_BODY_POINTS;
        let second = (first + 1) % SOFT_BODY_POINTS;
        let first_angle = first as f32 * SOFT_BODY_SECTOR_ANGLE + offsets[first];
        let second_angle = (first + 1) as f32 * SOFT_BODY_SECTOR_ANGLE + offsets[second];
        let span = (second_angle - first_angle).max(0.001);
        let blend = ((local_angle - first_angle) / span).clamp(0.0, 1.0);
        radii[first] + (radii[second] - radii[first]) * blend
    }

    fn section_heading(&self, index: usize, section: u8) -> f32 {
        if section == 0 {
            return self.heading[index];
        }
        let center = self.section_center(index, section);
        let parent_section = self.section_parents[index][section as usize - 1];
        let parent = self.section_center(index, parent_section);
        (parent.y - center.y).atan2(parent.x - center.x)
    }

    fn begin_lysis_deformation(
        &mut self,
        index: usize,
        section: u8,
        direction: Vec2,
        duration: f32,
        amount: f32,
    ) {
        let slot = section.min(3) as usize;
        self.lysis_deform_time[index][slot] = duration;
        self.lysis_deform_duration[index][slot] = duration.max(0.001);
        self.lysis_deform_angle[index][slot] = direction.y.atan2(direction.x);
        self.lysis_deform_amount[index][slot] = amount;
    }

    pub(crate) fn lysis_visual_radii(&self, index: usize, section: u8) -> [f32; SOFT_BODY_POINTS] {
        let (mut radii, offsets, core_radius) = match section {
            0 => (
                self.visual_radii[index],
                self.angle_offsets[index],
                self.core_radius[index],
            ),
            1 => (
                self.tail_visual_radii[index],
                self.tail_angle_offsets[index],
                self.tail_core_radius[index],
            ),
            _ => {
                let extra = self.extra_sections[index][section as usize - 2];
                (extra.visual_radii, extra.angle_offsets, extra.core_radius)
            }
        };
        let slot = section.min(3) as usize;
        let remaining = self.lysis_deform_time[index][slot];
        if remaining <= 0.0 {
            return radii;
        }

        let duration = self.lysis_deform_duration[index][slot].max(0.001);
        let progress = (1.0 - remaining / duration).clamp(0.0, 1.0);
        let amount = self.lysis_deform_amount[index][slot];
        let base_pulse = (progress * std::f32::consts::PI).sin().max(0.0).powf(0.72);
        let spring = if amount > 0.0 {
            1.0 + (progress * std::f32::consts::TAU * 1.5).sin() * 0.16 * (1.0 - progress)
        } else {
            1.0
        };
        let pulse = base_pulse * spring;
        let impact_angle = self.lysis_deform_angle[index][slot];
        let heading = self.section_heading(index, section);
        let width = if amount > 0.0 { 1.02 } else { 1.28 };
        let scale = radii.iter().copied().fold(core_radius, f32::max);
        for ray in 0..SOFT_BODY_POINTS {
            let ray_angle = heading + SOFT_BODY_BASE_ANGLES[ray] + offsets[ray];
            let delta = (ray_angle - impact_angle + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            let linear = (1.0 - delta.abs() / width).clamp(0.0, 1.0);
            let weight = linear * linear * (3.0 - 2.0 * linear);
            radii[ray] = (radii[ray] + scale * amount * pulse * weight).max(core_radius);
        }
        radii
    }

    fn apply_section_impulse(&mut self, index: usize, section: u8, impulse: Vec2) {
        match section {
            0 => {
                self.vx[index] += impulse.x;
                self.vy[index] += impulse.y;
            }
            1 => {
                self.tail_vx[index] += impulse.x;
                self.tail_vy[index] += impulse.y;
            }
            _ => {
                let extra = &mut self.extra_sections[index][section as usize - 2];
                extra.vx += impulse.x;
                extra.vy += impulse.y;
            }
        }
    }

    fn set_section_state(&mut self, index: usize, section: u8, position: Vec2, velocity: Vec2) {
        match section {
            1 => {
                self.tail_x[index] = position.x;
                self.tail_y[index] = position.y;
                self.tail_vx[index] = velocity.x;
                self.tail_vy[index] = velocity.y;
            }
            2..=3 => {
                let extra = &mut self.extra_sections[index][section as usize - 2];
                extra.x = position.x;
                extra.y = position.y;
                extra.vx = velocity.x;
                extra.vy = velocity.y;
            }
            _ => {}
        }
    }

    fn apply_curve_impulse(&mut self, index: usize, t: f32, impulse: Vec2) {
        if self.section_count[index] < 2 {
            self.apply_section_impulse(index, 0, impulse);
            return;
        }
        let edge_count = self.section_count[index] as usize - 1;
        let scaled = t.clamp(0.0, 1.0) * edge_count as f32;
        let edge = scaled.floor().min((edge_count - 1) as f32) as usize;
        let local = (scaled - edge as f32).clamp(0.0, 1.0);
        let child = edge as u8 + 1;
        let parent = self.section_parents[index][edge];
        self.apply_section_impulse(index, parent, impulse * (1.0 - local));
        self.apply_section_impulse(index, child, impulse * local);
    }

    fn compress_curve_contact(&mut self, index: usize, t: f32, direction: Vec2, depth: f32) {
        if self.section_count[index] < 2 {
            self.compress_section_contact(index, 0, direction, depth);
            return;
        }
        let edge_count = self.section_count[index] as usize - 1;
        let scaled = t.clamp(0.0, 1.0) * edge_count as f32;
        let edge = scaled.floor().min((edge_count - 1) as f32) as usize;
        let local = (scaled - edge as f32).clamp(0.0, 1.0);
        let child = edge as u8 + 1;
        let parent = self.section_parents[index][edge];
        if 1.0 - local > 0.05 {
            self.compress_section_contact(index, parent, direction, depth * (1.0 - local));
        }
        if local > 0.05 {
            self.compress_section_contact(index, child, direction, depth * local);
        }
    }

    fn compress_section_contact(&mut self, index: usize, section: u8, direction: Vec2, depth: f32) {
        let heading = self.section_heading(index, section);
        let local_angle = direction.y.atan2(direction.x) - heading;
        let ray_index = (local_angle / SOFT_BODY_SECTOR_ANGLE).round() as i32;
        let ray_index = ray_index.rem_euclid(SOFT_BODY_POINTS as i32) as usize;
        if section == 0 {
            let min = self.core_radius[index];
            self.current_radii[index][ray_index] =
                (self.current_radii[index][ray_index] - depth).max(min);
            self.refresh_current_radius_cache(index);
        } else if section == 1 {
            let min = self.tail_core_radius[index];
            self.tail_current_radii[index][ray_index] =
                (self.tail_current_radii[index][ray_index] - depth).max(min);
            self.tail_collision_radius[index] = self.tail_current_radii[index]
                .iter()
                .copied()
                .fold(min, f32::max);
        } else {
            let extra = &mut self.extra_sections[index][section as usize - 2];
            extra.current_radii[ray_index] =
                (extra.current_radii[ray_index] - depth).max(extra.core_radius);
            extra.collision_radius = extra
                .current_radii
                .iter()
                .copied()
                .fold(extra.core_radius, f32::max);
        }
    }

    fn new(
        count: usize,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        random_geometry: bool,
        segmented_cells: bool,
        shape_weights: &[f32; CELL_SHAPE_COUNT],
        rng: &mut SmallRng,
    ) -> Self {
        let mut store = Self {
            id: Vec::with_capacity(count),
            next_id: 0,
            segmented_enabled: segmented_cells,
            x: Vec::with_capacity(count),
            y: Vec::with_capacity(count),
            vx: Vec::with_capacity(count),
            vy: Vec::with_capacity(count),
            heading: Vec::with_capacity(count),
            radius: Vec::with_capacity(count),
            core_radius: Vec::with_capacity(count),
            speed: Vec::with_capacity(count),
            turn_speed: Vec::with_capacity(count),
            perception: Vec::with_capacity(count),
            persistence: Vec::with_capacity(count),
            aggressiveness: Vec::with_capacity(count),
            lysis: Vec::with_capacity(count),
            lysis_cooldown: Vec::with_capacity(count),
            lysis_deform_time: Vec::with_capacity(count),
            lysis_deform_duration: Vec::with_capacity(count),
            lysis_deform_angle: Vec::with_capacity(count),
            lysis_deform_amount: Vec::with_capacity(count),
            hunt_pause: Vec::with_capacity(count),
            hunt_recheck: Vec::with_capacity(count),
            target_food: Vec::with_capacity(count),
            target_food_generation: Vec::with_capacity(count),
            target_last_x: Vec::with_capacity(count),
            target_last_y: Vec::with_capacity(count),
            target_memory: Vec::with_capacity(count),
            target_search_failed: Vec::with_capacity(count),
            target_recheck: Vec::with_capacity(count),
            target_cell: Vec::with_capacity(count),
            target_cell_id: Vec::with_capacity(count),
            section_count: Vec::with_capacity(count),
            section_spacing: Vec::with_capacity(count),
            section_bend: Vec::with_capacity(count),
            section_angles: Vec::with_capacity(count),
            section_parents: Vec::with_capacity(count),
            edge_curve_offsets: Vec::with_capacity(count),
            extra_sections: Vec::with_capacity(count),
            tail_x: Vec::with_capacity(count),
            tail_y: Vec::with_capacity(count),
            tail_vx: Vec::with_capacity(count),
            tail_vy: Vec::with_capacity(count),
            tail_core_radius: Vec::with_capacity(count),
            tail_base_radii: Vec::with_capacity(count),
            tail_current_radii: Vec::with_capacity(count),
            tail_visual_radii: Vec::with_capacity(count),
            tail_angle_offsets: Vec::with_capacity(count),
            tail_collision_radius: Vec::with_capacity(count),
            tail_ray_dir_x: Vec::with_capacity(count),
            tail_ray_dir_y: Vec::with_capacity(count),
            stuck_time: Vec::with_capacity(count),
            reverse_time: Vec::with_capacity(count),
            species: Vec::with_capacity(count),
            viability: Vec::with_capacity(count),
            max_viability: Vec::with_capacity(count),
            mutation_susceptibility: Vec::with_capacity(count),
            division_threshold: Vec::with_capacity(count),
            mitosis_progress: Vec::with_capacity(count),
            mitosis_recovery: Vec::with_capacity(count),
            base_radii: Vec::with_capacity(count),
            current_radii: Vec::with_capacity(count),
            visual_radii: Vec::with_capacity(count),
            angle_offsets: Vec::with_capacity(count),
            collision_radius: Vec::with_capacity(count),
            biomass: Vec::with_capacity(count),
            asymmetry_x: Vec::with_capacity(count),
            asymmetry_y: Vec::with_capacity(count),
            shape_drag: Vec::with_capacity(count),
            morphology_acceleration: Vec::with_capacity(count),
            morphology_turn: Vec::with_capacity(count),
            morphology_viability: Vec::with_capacity(count),
            morphology_metabolism: Vec::with_capacity(count),
            ray_dir_x: Vec::with_capacity(count),
            ray_dir_y: Vec::with_capacity(count),
            shape_wave_a: Vec::with_capacity(count),
            shape_wave_b: Vec::with_capacity(count),
            shape_phase: Vec::with_capacity(count),
            shape_softness: Vec::with_capacity(count),
            nucleus_offset_x: Vec::with_capacity(count),
            nucleus_offset_y: Vec::with_capacity(count),
            nucleus_radius: Vec::with_capacity(count),
            jelly_phase: Vec::with_capacity(count),
            jelly_intensity: Vec::with_capacity(count),
            jelly_dir_x: Vec::with_capacity(count),
            jelly_dir_y: Vec::with_capacity(count),
            wake_strength: Vec::with_capacity(count),
        };

        for _cell_index in 0..count {
            let size_gene = rng.random_range(0.0_f32..1.0).powf(1.12);
            let radius = CELL_SIZE_GENE_MIN + (CELL_SIZE_GENE_MAX - CELL_SIZE_GENE_MIN) * size_gene;
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let speed = rng.random_range(42.0..72.0);
            let turn_speed = rng.random_range(0.85..2.35);
            let (s, c) = angle.sin_cos();
            let nucleus_angle = rng.random_range(0.0..std::f32::consts::TAU);
            let nucleus_distance = rng.random_range(0.0..0.36) * radius;
            let (nucleus_s, nucleus_c) = nucleus_angle.sin_cos();
            let (base_radii, angle_offsets) = if random_geometry {
                random_free_shape(radius, rng)
            } else {
                random_seed_shape(radius, None, shape_weights, rng)
            };

            let position = random_point_in_arena(arena_w, arena_h, arena_shape, radius, rng);
            store.x.push(position.x);
            store.y.push(position.y);
            store.vx.push(c * speed);
            store.vy.push(s * speed);
            store.heading.push(angle);
            store.radius.push(radius);
            store.core_radius.push(radius * CORE_RADIUS_FACTOR);
            store.speed.push(speed);
            store.turn_speed.push(turn_speed);
            store.perception.push(rng.random_range(260.0..520.0));
            store.persistence.push(rng.random_range(18.0..62.0));
            let lysis = if rng.random_bool(INITIAL_LYSIS_CHANCE) {
                rng.random_range(28.0..72.0)
            } else {
                0.0
            };
            store
                .aggressiveness
                .push(if lysis >= LYSIS_ACTIVE_THRESHOLD {
                    rng.random_range(32.0..82.0)
                } else {
                    rng.random_range(0.0..42.0)
                });
            store.lysis.push(lysis);
            store.lysis_cooldown.push(0.0);
            store.lysis_deform_time.push([0.0; 4]);
            store.lysis_deform_duration.push([1.0; 4]);
            store.lysis_deform_angle.push([0.0; 4]);
            store.lysis_deform_amount.push([0.0; 4]);
            store.hunt_pause.push(0.0);
            store
                .hunt_recheck
                .push(rng.random_range(0.0..LYSIS_TARGET_RECHECK_MAX));
            store.target_food.push(-1);
            store.target_food_generation.push(0);
            store.target_last_x.push(position.x);
            store.target_last_y.push(position.y);
            store.target_memory.push(0.0);
            store.target_search_failed.push(false);
            store.target_recheck.push(rng.random_range(0.0..0.24));
            store.target_cell.push(-1);
            store.target_cell_id.push(NO_CELL_TARGET);
            let section_count = if segmented_cells && rng.random_bool(INITIAL_SEGMENTED_CHANCE) {
                let topology_roll = rng.random::<f32>();
                if topology_roll < 0.70 {
                    2
                } else if topology_roll < 0.92 {
                    3
                } else {
                    4
                }
            } else {
                1
            };
            let spacing =
                radius * rng.random_range(TAIL_MIN_SPACING_FACTOR..TAIL_MAX_SPACING_FACTOR);
            let bend = if section_count >= 2 {
                rng.random_range(-0.24..0.24)
            } else {
                0.0
            };
            let tail_profile = random_segment_soft_body(
                base_radii,
                angle_offsets,
                radius,
                random_geometry,
                shape_weights,
                rng,
            );
            let section_angles = [
                rng.random_range(0.0..std::f32::consts::TAU),
                rng.random_range(0.0..std::f32::consts::TAU),
                rng.random_range(0.0..std::f32::consts::TAU),
            ];
            let section_parents = [0, rng.random_range(0..2), rng.random_range(0..3)];
            let first_direction = Vec2::from_angle(angle + section_angles[0]);
            let tail_position = if section_count >= 2 {
                clamp_point_to_arena(
                    position + first_direction * spacing,
                    arena_w,
                    arena_h,
                    arena_shape,
                    tail_profile.collision_radius,
                )
            } else {
                position
            };
            store.section_count.push(section_count);
            store.section_spacing.push(spacing);
            store.section_bend.push(bend);
            store.section_angles.push(section_angles);
            store.section_parents.push(section_parents);
            store.edge_curve_offsets.push([0.0; 3]);
            store.tail_x.push(tail_position.x);
            store.tail_y.push(tail_position.y);
            store.tail_vx.push(c * speed);
            store.tail_vy.push(s * speed);
            store.tail_core_radius.push(tail_profile.core_radius);
            store.tail_base_radii.push(tail_profile.base_radii);
            store.tail_current_radii.push(tail_profile.current_radii);
            store.tail_visual_radii.push(tail_profile.visual_radii);
            store.tail_angle_offsets.push(tail_profile.angle_offsets);
            store
                .tail_collision_radius
                .push(tail_profile.collision_radius);
            let mut tail_dirs_x = [0.0; SOFT_BODY_POINTS];
            let mut tail_dirs_y = [0.0; SOFT_BODY_POINTS];
            for ray_index in 0..SOFT_BODY_POINTS {
                let angle =
                    SOFT_BODY_BASE_ANGLES[ray_index] + tail_profile.angle_offsets[ray_index];
                tail_dirs_x[ray_index] = angle.cos();
                tail_dirs_y[ray_index] = angle.sin();
            }
            store.tail_ray_dir_x.push(tail_dirs_x);
            store.tail_ray_dir_y.push(tail_dirs_y);
            let mut extras = [
                ExtraSection::dormant(tail_position, tail_profile.size),
                ExtraSection::dormant(tail_position, tail_profile.size),
            ];
            let mut generated_positions = [position, tail_position, tail_position, tail_position];
            for extra_index in 0..2 {
                if section_count as usize > extra_index + 2 {
                    let section = extra_index + 2;
                    let parent = section_parents[section - 1] as usize;
                    let direction = Vec2::from_angle(angle + section_angles[extra_index + 1]);
                    let extra_profile = random_segment_soft_body(
                        tail_profile.base_radii,
                        tail_profile.angle_offsets,
                        tail_profile.size,
                        random_geometry,
                        shape_weights,
                        rng,
                    );
                    let generated = clamp_point_to_arena(
                        generated_positions[parent] + direction * spacing,
                        arena_w,
                        arena_h,
                        arena_shape,
                        extra_profile.collision_radius,
                    );
                    extras[extra_index] = ExtraSection::from_profile(
                        generated,
                        Vec2::new(c * speed, s * speed),
                        extra_profile,
                    );
                    generated_positions[section] = generated;
                }
            }
            store.extra_sections.push(extras);
            store.stuck_time.push(0.0);
            store.reverse_time.push(0.0);
            store.species.push(0);
            store.max_viability.push(CELL_VIABILITY_MAX);
            store
                .viability
                .push(rng.random_range((CELL_VIABILITY_MAX * 0.50)..(CELL_VIABILITY_MAX * 0.68)));
            store
                .mutation_susceptibility
                .push(rng.random_range(18.0..45.0));
            store.division_threshold.push(rng.random_range(78.0..90.0));
            store.mitosis_progress.push(0.0);
            store.mitosis_recovery.push(0.0);
            store.base_radii.push(base_radii);
            store.current_radii.push(base_radii);
            store.visual_radii.push(base_radii);
            store.angle_offsets.push(angle_offsets);
            store.collision_radius.push(radius);
            store.biomass.push(0.0);
            store.asymmetry_x.push(0.0);
            store.asymmetry_y.push(0.0);
            store.shape_drag.push(1.0);
            store.morphology_acceleration.push(1.0);
            store.morphology_turn.push(1.0);
            store.morphology_viability.push(1.0);
            store.morphology_metabolism.push(1.0);
            store.ray_dir_x.push([0.0; SOFT_BODY_POINTS]);
            store.ray_dir_y.push([0.0; SOFT_BODY_POINTS]);
            let store_index = store.len() - 1;
            store.rebuild_soft_body_cache(store_index);
            store.rebuild_tail_cache(store_index);
            store.shape_wave_a.push(0.0);
            store.shape_wave_b.push(0.0);
            store
                .shape_phase
                .push(rng.random_range(0.0..std::f32::consts::TAU));
            store.shape_softness.push(0.0);
            store.nucleus_offset_x.push(nucleus_c * nucleus_distance);
            store.nucleus_offset_y.push(nucleus_s * nucleus_distance);
            store
                .nucleus_radius
                .push(radius * rng.random_range(0.2..0.42));
            store
                .jelly_phase
                .push(rng.random_range(0.0..std::f32::consts::TAU));
            store.jelly_intensity.push(0.0);
            store.jelly_dir_x.push(c);
            store.jelly_dir_y.push(s);
            store.wake_strength.push(0.0);
            store.id.push(store.next_id);
            store.next_id += 1;
        }

        store
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn viability_ratio(&self, index: usize) -> f32 {
        (self.viability[index] / self.max_viability[index].max(1.0)).clamp(0.0, 1.0)
    }

    pub fn add_viability(&mut self, index: usize, amount: f32) {
        self.viability[index] = (self.viability[index] + amount).min(self.max_viability[index]);
    }

    pub fn soft_body_profile(&self, index: usize) -> SoftBodyCell {
        SoftBodyCell {
            speed: self.speed[index],
            energy: self.viability[index],
            agility: self.turn_speed[index],
            perception: self.perception[index],
            persistence: self.persistence[index],
            mutation_factor: self.mutation_susceptibility[index],
            size: self.radius[index],
            base_radii: self.base_radii[index],
            current_radii: self.current_radii[index],
            angle_offsets: self.angle_offsets[index],
        }
    }

    pub fn refresh_taxonomy(&mut self) {
        for index in 0..self.len() {
            self.species[index] = self.taxonomy_species_id(index);
        }
    }

    fn taxonomy_hash_step(hash: &mut u32, value: u32) {
        *hash ^= value;
        *hash = hash.wrapping_mul(16777619);
    }

    fn taxonomy_species_id(&self, index: usize) -> u32 {
        let avg = (self.base_radii[index].iter().sum::<f32>() / SOFT_BODY_POINTS as f32).max(0.1);
        let class = if self.section_count[index] >= 2 {
            let aspect = self.section_spacing[index]
                / (self.max_base_radius(index) + self.tail_collision_radius[index]).max(0.1);
            if self.section_bend[index].abs() > 0.20 {
                CellShapeClass::Spirillum
            } else if aspect > 1.55 {
                CellShapeClass::Filament
            } else {
                CellShapeClass::Bacillus
            }
        } else {
            analyze_cell_shape_class(&self.soft_body_profile(index))
        };
        let class_bin: u32 = match class {
            CellShapeClass::Coccus => 0,
            CellShapeClass::Bacillus => 1,
            CellShapeClass::Filament => 2,
            CellShapeClass::Spirillum => 3,
            CellShapeClass::Vibrio => 4,
            CellShapeClass::Diplococcus => 5,
            CellShapeClass::Fusiform => 6,
            CellShapeClass::Cuboid => 7,
            CellShapeClass::Triquetrum => 8,
            CellShapeClass::Stauromorph => 9,
            CellShapeClass::Lancetiform => 10,
            CellShapeClass::Placoid => 11,
            CellShapeClass::Lobatum => 12,
        };
        let mut species_hash = 2166136261u32;
        let mut genus_hash = 2166136261u32;
        for radius in self.base_radii[index] {
            let normalized = radius / avg;
            let quantized = (normalized * 12.0).round().clamp(4.0, 24.0) as u32;
            let genus_quantized = (normalized * 9.0).round().clamp(3.0, 18.0) as u32;
            Self::taxonomy_hash_step(&mut species_hash, quantized);
            Self::taxonomy_hash_step(&mut genus_hash, genus_quantized);
        }
        for offset in self.angle_offsets[index] {
            let quantized =
                ((offset + SOFT_BODY_MAX_ANGLE_OFFSET) / (SOFT_BODY_MAX_ANGLE_OFFSET * 2.0) * 8.0)
                    .round()
                    .clamp(0.0, 8.0) as u32;
            let genus_quantized =
                ((offset + SOFT_BODY_MAX_ANGLE_OFFSET) / (SOFT_BODY_MAX_ANGLE_OFFSET * 2.0) * 5.0)
                    .round()
                    .clamp(0.0, 5.0) as u32;
            Self::taxonomy_hash_step(&mut species_hash, quantized + 31);
            Self::taxonomy_hash_step(&mut genus_hash, genus_quantized + 31);
        }
        let trophic_bin = if self.aggressiveness[index] < 40.0 {
            0
        } else if self.aggressiveness[index] < 70.0 {
            1
        } else {
            2
        };
        let lysis_bin = if self.lysis[index] >= LYSIS_ACTIVE_THRESHOLD {
            1 + strict_gene_bin(
                self.lysis[index],
                LYSIS_ACTIVE_THRESHOLD,
                CELL_LYSIS_DISPLAY_MAX,
                7,
            )
        } else {
            0
        };
        let gene_bins = [
            strict_gene_bin(self.speed[index], SPEED_GENE_MIN, SPEED_GENE_MAX, 12),
            strict_gene_bin(self.turn_speed[index], TURN_GENE_MIN, TURN_GENE_MAX, 12),
            strict_gene_bin(
                self.perception[index],
                PERCEPTION_GENE_MIN,
                PERCEPTION_GENE_MAX,
                10,
            ),
            strict_gene_bin(
                self.persistence[index],
                PERSISTENCE_GENE_MIN,
                PERSISTENCE_GENE_MAX,
                10,
            ),
            trophic_bin,
            lysis_bin,
            strict_gene_bin(
                self.mutation_susceptibility[index],
                0.0,
                CELL_MUTATION_DISPLAY_MAX,
                8,
            ),
            strict_gene_bin(
                self.max_base_radius(index),
                CELL_SIZE_GENE_MIN,
                CELL_SIZE_GENE_MAX,
                12,
            ),
            self.section_count[index] as u32,
        ];
        for (slot, bin) in gene_bins.into_iter().enumerate() {
            Self::taxonomy_hash_step(&mut species_hash, bin.clamp(0, 31) + 97 + slot as u32 * 37);
        }
        let genus_bins = [
            strict_gene_bin(self.speed[index], SPEED_GENE_MIN, SPEED_GENE_MAX, 6),
            strict_gene_bin(self.turn_speed[index], TURN_GENE_MIN, TURN_GENE_MAX, 6),
            strict_gene_bin(
                self.perception[index],
                PERCEPTION_GENE_MIN,
                PERCEPTION_GENE_MAX,
                5,
            ),
            strict_gene_bin(
                self.persistence[index],
                PERSISTENCE_GENE_MIN,
                PERSISTENCE_GENE_MAX,
                5,
            ),
            lysis_bin,
            strict_gene_bin(
                self.mutation_susceptibility[index],
                0.0,
                CELL_MUTATION_DISPLAY_MAX,
                4,
            ),
            strict_gene_bin(
                self.max_base_radius(index),
                CELL_SIZE_GENE_MIN,
                CELL_SIZE_GENE_MAX,
                6,
            ),
            self.section_count[index] as u32,
        ];
        for bin in genus_bins {
            Self::taxonomy_hash_step(&mut genus_hash, bin + 97);
        }
        let section_spacing_bin = if self.section_count[index] >= 2 {
            strict_gene_bin(
                self.section_spacing[index],
                CELL_SIZE_GENE_MIN * 0.15,
                CELL_SIZE_GENE_MAX * 4.8,
                31,
            )
        } else {
            0
        };
        let genus_section_spacing_bin = if self.section_count[index] >= 2 {
            strict_gene_bin(
                self.section_spacing[index],
                CELL_SIZE_GENE_MIN * 0.15,
                CELL_SIZE_GENE_MAX * 4.8,
                15,
            )
        } else {
            0
        };
        let section_bend_bin = if self.section_count[index] >= 2 {
            strict_gene_bin(self.section_bend[index].abs(), 0.0, 1.4, 15)
        } else {
            0
        };
        let genus_section_bend_bin = if self.section_count[index] >= 2 {
            strict_gene_bin(self.section_bend[index].abs(), 0.0, 1.4, 7)
        } else {
            0
        };
        Self::taxonomy_hash_step(&mut species_hash, section_spacing_bin + 151);
        Self::taxonomy_hash_step(&mut genus_hash, genus_section_spacing_bin + 151);
        Self::taxonomy_hash_step(&mut species_hash, section_bend_bin + 193);
        Self::taxonomy_hash_step(&mut genus_hash, genus_section_bend_bin + 193);

        let genus = genus_hash % SPECIES_GENUS_SLOTS;
        let epithet_slots_per_troph = (SPECIES_EPITHET_SLOTS / 3).max(1);
        let epithet = (species_hash % epithet_slots_per_troph) * 3 + trophic_bin;
        class_bin * SPECIES_CLASS_STRIDE + genus * SPECIES_EPITHET_SLOTS + epithet
    }

    pub fn shape_name(&self, index: usize) -> String {
        if self.section_count[index] >= 2 {
            let aspect = self.section_spacing[index]
                / (self.max_base_radius(index) + self.tail_collision_radius[index]).max(0.1);
            return if self.section_bend[index].abs() > 0.20 {
                "Спирилла / многосегментная клетка".to_string()
            } else if aspect > 1.55 {
                "Филамент / нематода".to_string()
            } else {
                "Бацилла".to_string()
            };
        }
        analyze_cell_shape(&self.soft_body_profile(index))
    }

    fn push_child_from(
        &mut self,
        parent_index: usize,
        viability: f32,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        division_axis: Vec2,
        division_offset: f32,
        rng: &mut SmallRng,
    ) {
        let parent_heading = self.heading[parent_index];
        let side = if rng.random_bool(0.5) { -1.0 } else { 1.0 };
        let offset_c = division_axis.x;
        let offset_s = division_axis.y;
        let radius = self.radius[parent_index];
        let offset = division_offset;
        let position = clamp_point_to_arena(
            Vec2::new(
                self.x[parent_index] + offset_c * offset,
                self.y[parent_index] + offset_s * offset,
            ),
            arena_w,
            arena_h,
            arena_shape,
            radius,
        );
        let susceptibility = self.mutation_susceptibility[parent_index];

        self.id.push(self.next_id);
        self.next_id += 1;
        self.x.push(position.x);
        self.y.push(position.y);
        self.vx.push(self.vx[parent_index] * 0.35 + offset_c * 8.0);
        self.vy.push(self.vy[parent_index] * 0.35 + offset_s * 8.0);
        self.heading.push(wrap_angle(parent_heading + side * 0.28));
        self.radius.push(radius);
        self.core_radius.push(radius * CORE_RADIUS_FACTOR);
        self.speed.push(mutate_gene(
            self.speed[parent_index],
            SPEED_GENE_MIN,
            SPEED_GENE_MAX,
            susceptibility,
            rng,
        ));
        self.turn_speed.push(mutate_gene(
            self.turn_speed[parent_index],
            TURN_GENE_MIN,
            TURN_GENE_MAX,
            susceptibility,
            rng,
        ));
        self.perception.push(mutate_gene(
            self.perception[parent_index],
            PERCEPTION_GENE_MIN,
            PERCEPTION_GENE_MAX,
            susceptibility,
            rng,
        ));
        self.persistence.push(mutate_gene(
            self.persistence[parent_index],
            PERSISTENCE_GENE_MIN,
            PERSISTENCE_GENE_MAX,
            susceptibility,
            rng,
        ));
        self.aggressiveness.push(mutate_gene(
            self.aggressiveness[parent_index],
            0.0,
            CELL_AGGRESSIVENESS_DISPLAY_MAX,
            susceptibility,
            rng,
        ));
        let parent_lysis = self.lysis[parent_index];
        let child_lysis = if parent_lysis >= LYSIS_ACTIVE_THRESHOLD {
            mutate_gene(
                parent_lysis,
                0.0,
                CELL_LYSIS_DISPLAY_MAX,
                susceptibility,
                rng,
            )
        } else if rng.random_bool(0.002 + susceptibility as f64 * 0.00008) {
            rng.random_range(18.0..38.0)
        } else {
            0.0
        };
        self.lysis.push(child_lysis);
        self.lysis_cooldown.push(0.0);
        self.lysis_deform_time.push([0.0; 4]);
        self.lysis_deform_duration.push([1.0; 4]);
        self.lysis_deform_angle.push([0.0; 4]);
        self.lysis_deform_amount.push([0.0; 4]);
        self.hunt_pause.push(0.0);
        self.hunt_recheck
            .push(rng.random_range(0.0..LYSIS_TARGET_RECHECK_MAX));
        self.target_food.push(-1);
        self.target_food_generation.push(0);
        self.target_last_x.push(position.x);
        self.target_last_y.push(position.y);
        self.target_memory.push(0.0);
        self.target_search_failed.push(false);
        self.target_recheck.push(rng.random_range(0.0..0.24));
        self.target_cell.push(-1);
        self.target_cell_id.push(NO_CELL_TARGET);
        let mut child_section_count = self.section_count[parent_index];
        let topology_mutation_chance = 0.015 + susceptibility as f64 / 100.0 * 0.055;
        if self.segmented_enabled && rng.random_bool(topology_mutation_chance) {
            child_section_count = if rng.random_bool(0.62) {
                (child_section_count + 1).min(4)
            } else {
                child_section_count.saturating_sub(1).max(1)
            };
        }
        let spacing = mutate_gene(
            self.section_spacing[parent_index],
            radius * TAIL_MIN_SPACING_FACTOR,
            radius * TAIL_MAX_SPACING_FACTOR,
            susceptibility,
            rng,
        );
        let bend = if child_section_count >= 2 {
            (self.section_bend[parent_index]
                + rng.random_range(-0.09..0.09) * mutation_power(susceptibility))
            .clamp(-0.38, 0.38)
        } else {
            0.0
        };
        let tail_source_section = if self.section_count[parent_index] >= 2 {
            1
        } else {
            0
        };
        let (tail_source_base, tail_source_current, tail_source_offsets) =
            self.section_soft_body_source(parent_index, tail_source_section);
        let tail_profile = mutate_child_segment_soft_body(
            tail_source_base,
            tail_source_current,
            tail_source_offsets,
            susceptibility,
            rng,
        );
        let child_heading = wrap_angle(parent_heading + side * 0.28);
        let mut section_angles = self.section_angles[parent_index];
        for angle in &mut section_angles {
            *angle =
                wrap_angle(*angle + rng.random_range(-0.22..0.22) * mutation_power(susceptibility));
        }
        let mut section_parents = self.section_parents[parent_index];
        for edge in 0..3 {
            section_parents[edge] = section_parents[edge].min(edge as u8);
        }
        if child_section_count > self.section_count[parent_index] {
            for child in self.section_count[parent_index]..child_section_count {
                if child > 0 {
                    section_parents[child as usize - 1] = rng.random_range(0..child);
                }
            }
        } else if child_section_count > 2 && rng.random_bool(topology_mutation_chance * 0.35) {
            let child = rng.random_range(2..child_section_count);
            section_parents[child as usize - 1] = rng.random_range(0..child);
        }
        let tail_direction = Vec2::from_angle(child_heading + section_angles[0]);
        let tail_position = if child_section_count >= 2 {
            clamp_point_to_arena(
                position + tail_direction * spacing,
                arena_w,
                arena_h,
                arena_shape,
                tail_profile.collision_radius,
            )
        } else {
            position
        };
        self.section_count.push(child_section_count);
        self.section_spacing.push(spacing);
        self.section_bend.push(bend);
        self.section_angles.push(section_angles);
        self.section_parents.push(section_parents);
        self.edge_curve_offsets.push([0.0; 3]);
        self.tail_x.push(tail_position.x);
        self.tail_y.push(tail_position.y);
        self.tail_vx.push(self.vx[parent_index] * 0.35);
        self.tail_vy.push(self.vy[parent_index] * 0.35);
        self.tail_core_radius.push(tail_profile.core_radius);
        self.tail_base_radii.push(tail_profile.base_radii);
        self.tail_current_radii.push(tail_profile.current_radii);
        self.tail_visual_radii.push(tail_profile.visual_radii);
        self.tail_angle_offsets.push(tail_profile.angle_offsets);
        self.tail_collision_radius
            .push(tail_profile.collision_radius);
        self.tail_ray_dir_x.push([0.0; SOFT_BODY_POINTS]);
        self.tail_ray_dir_y.push([0.0; SOFT_BODY_POINTS]);
        let mut extras = [
            ExtraSection::dormant(tail_position, tail_profile.size),
            ExtraSection::dormant(tail_position, tail_profile.size),
        ];
        let mut generated_positions = [position, tail_position, tail_position, tail_position];
        for extra_index in 0..2 {
            if child_section_count as usize > extra_index + 2 {
                let section = extra_index + 2;
                let parent = section_parents[section - 1] as usize;
                let source_section = if self.section_count[parent_index] as usize > section {
                    section as u8
                } else if parent < child_section_count as usize && parent > 0 {
                    parent as u8
                } else {
                    tail_source_section
                };
                let (source_base, source_current, source_offsets) =
                    self.section_soft_body_source(parent_index, source_section);
                let extra_profile = mutate_child_segment_soft_body(
                    source_base,
                    source_current,
                    source_offsets,
                    susceptibility,
                    rng,
                );
                let generated = clamp_point_to_arena(
                    generated_positions[parent]
                        + Vec2::from_angle(child_heading + section_angles[extra_index + 1])
                            * spacing,
                    arena_w,
                    arena_h,
                    arena_shape,
                    extra_profile.collision_radius,
                );
                extras[extra_index] = ExtraSection::from_profile(
                    generated,
                    Vec2::new(self.vx[parent_index] * 0.35, self.vy[parent_index] * 0.35),
                    extra_profile,
                );
                generated_positions[section] = generated;
            }
        }
        self.extra_sections.push(extras);
        self.stuck_time.push(0.0);
        self.reverse_time.push(0.0);
        self.species.push(self.species[parent_index]);
        self.viability
            .push(viability.clamp(0.0, self.max_viability[parent_index]));
        self.max_viability.push(self.max_viability[parent_index]);
        let mut child_mutation_susceptibility = mutate_gene(
            self.mutation_susceptibility[parent_index],
            MUTATION_GENE_MIN,
            MUTATION_GENE_MAX,
            susceptibility,
            rng,
        );
        child_mutation_susceptibility = (child_mutation_susceptibility
            + rng.random_range(-0.01..0.01) * MUTATION_FACTOR_DELTA_SCALE)
            .clamp(MUTATION_GENE_MIN, MUTATION_GENE_MAX);
        self.mutation_susceptibility
            .push(child_mutation_susceptibility);
        self.division_threshold.push(mutate_gene(
            self.division_threshold[parent_index],
            DIVISION_THRESHOLD_MIN,
            DIVISION_THRESHOLD_MAX,
            susceptibility,
            rng,
        ));
        self.mitosis_progress.push(0.0);
        self.mitosis_recovery.push(MITOSIS_RECOVERY_DURATION);
        let (child_base_radii, child_current_radii, child_angle_offsets) = mutate_child_soft_body(
            self.base_radii[parent_index],
            self.current_radii[parent_index],
            self.angle_offsets[parent_index],
            radius,
            susceptibility,
            rng,
        );
        self.base_radii.push(child_base_radii);
        self.current_radii.push(child_current_radii);
        self.visual_radii.push(child_current_radii);
        self.angle_offsets.push(child_angle_offsets);
        self.collision_radius.push(radius);
        self.biomass.push(0.0);
        self.asymmetry_x.push(0.0);
        self.asymmetry_y.push(0.0);
        self.shape_drag.push(1.0);
        self.morphology_acceleration.push(1.0);
        self.morphology_turn.push(1.0);
        self.morphology_viability.push(1.0);
        self.morphology_metabolism.push(1.0);
        self.ray_dir_x.push([0.0; SOFT_BODY_POINTS]);
        self.ray_dir_y.push([0.0; SOFT_BODY_POINTS]);
        let child_index = self.len() - 1;
        self.rebuild_soft_body_cache(child_index);
        self.rebuild_tail_cache(child_index);
        // Morphology changes capacity, but may not mint or erase the inherited energy split.
        self.viability[child_index] = viability.clamp(0.0, self.max_viability[child_index]);
        self.shape_wave_a.push(self.shape_wave_a[parent_index]);
        self.shape_wave_b.push(self.shape_wave_b[parent_index]);
        self.shape_phase
            .push(rng.random_range(0.0..std::f32::consts::TAU));
        self.shape_softness.push(self.shape_softness[parent_index]);
        self.nucleus_offset_x
            .push(self.nucleus_offset_x[parent_index]);
        self.nucleus_offset_y
            .push(self.nucleus_offset_y[parent_index]);
        self.nucleus_radius.push(self.nucleus_radius[parent_index]);
        self.jelly_phase
            .push(rng.random_range(0.0..std::f32::consts::TAU));
        self.jelly_intensity.push(0.35);
        self.jelly_dir_x.push(offset_c);
        self.jelly_dir_y.push(offset_s);
        self.wake_strength.push(0.0);
        self.species[child_index] = self.taxonomy_species_id(child_index);
    }

    fn swap_remove(&mut self, index: usize) {
        self.id.swap_remove(index);
        self.x.swap_remove(index);
        self.y.swap_remove(index);
        self.vx.swap_remove(index);
        self.vy.swap_remove(index);
        self.heading.swap_remove(index);
        self.radius.swap_remove(index);
        self.core_radius.swap_remove(index);
        self.speed.swap_remove(index);
        self.turn_speed.swap_remove(index);
        self.perception.swap_remove(index);
        self.persistence.swap_remove(index);
        self.aggressiveness.swap_remove(index);
        self.lysis.swap_remove(index);
        self.lysis_cooldown.swap_remove(index);
        self.lysis_deform_time.swap_remove(index);
        self.lysis_deform_duration.swap_remove(index);
        self.lysis_deform_angle.swap_remove(index);
        self.lysis_deform_amount.swap_remove(index);
        self.hunt_pause.swap_remove(index);
        self.hunt_recheck.swap_remove(index);
        self.target_food.swap_remove(index);
        self.target_food_generation.swap_remove(index);
        self.target_last_x.swap_remove(index);
        self.target_last_y.swap_remove(index);
        self.target_memory.swap_remove(index);
        self.target_search_failed.swap_remove(index);
        self.target_recheck.swap_remove(index);
        self.target_cell.swap_remove(index);
        self.target_cell_id.swap_remove(index);
        self.section_count.swap_remove(index);
        self.section_spacing.swap_remove(index);
        self.section_bend.swap_remove(index);
        self.section_angles.swap_remove(index);
        self.section_parents.swap_remove(index);
        self.edge_curve_offsets.swap_remove(index);
        self.extra_sections.swap_remove(index);
        self.tail_x.swap_remove(index);
        self.tail_y.swap_remove(index);
        self.tail_vx.swap_remove(index);
        self.tail_vy.swap_remove(index);
        self.tail_core_radius.swap_remove(index);
        self.tail_base_radii.swap_remove(index);
        self.tail_current_radii.swap_remove(index);
        self.tail_visual_radii.swap_remove(index);
        self.tail_angle_offsets.swap_remove(index);
        self.tail_collision_radius.swap_remove(index);
        self.tail_ray_dir_x.swap_remove(index);
        self.tail_ray_dir_y.swap_remove(index);
        self.stuck_time.swap_remove(index);
        self.reverse_time.swap_remove(index);
        self.species.swap_remove(index);
        self.viability.swap_remove(index);
        self.max_viability.swap_remove(index);
        self.mutation_susceptibility.swap_remove(index);
        self.division_threshold.swap_remove(index);
        self.mitosis_progress.swap_remove(index);
        self.mitosis_recovery.swap_remove(index);
        self.base_radii.swap_remove(index);
        self.current_radii.swap_remove(index);
        self.visual_radii.swap_remove(index);
        self.angle_offsets.swap_remove(index);
        self.collision_radius.swap_remove(index);
        self.biomass.swap_remove(index);
        self.asymmetry_x.swap_remove(index);
        self.asymmetry_y.swap_remove(index);
        self.shape_drag.swap_remove(index);
        self.morphology_acceleration.swap_remove(index);
        self.morphology_turn.swap_remove(index);
        self.morphology_viability.swap_remove(index);
        self.morphology_metabolism.swap_remove(index);
        self.ray_dir_x.swap_remove(index);
        self.ray_dir_y.swap_remove(index);
        self.shape_wave_a.swap_remove(index);
        self.shape_wave_b.swap_remove(index);
        self.shape_phase.swap_remove(index);
        self.shape_softness.swap_remove(index);
        self.nucleus_offset_x.swap_remove(index);
        self.nucleus_offset_y.swap_remove(index);
        self.nucleus_radius.swap_remove(index);
        self.jelly_phase.swap_remove(index);
        self.jelly_intensity.swap_remove(index);
        self.jelly_dir_x.swap_remove(index);
        self.jelly_dir_y.swap_remove(index);
        self.wake_strength.swap_remove(index);
    }

    #[allow(dead_code)]
    pub fn shape_radius_at(&self, index: usize, angle: f32) -> f32 {
        let wave_a = self.shape_wave_a[index];
        let wave_b = self.shape_wave_b[index];
        let phase = self.shape_phase[index];
        let radius =
            1.0 + wave_a * (angle * 3.0 + phase).sin() + wave_b * (angle * 5.0 - phase * 0.7).sin();

        radius.clamp(0.55, 1.0)
    }

    pub fn collision_bound_radius(&self, index: usize) -> f32 {
        let head = self.section_center(index, 0);
        let mut bound = self.collision_radius[index];
        for section in 1..self.section_count[index] {
            bound = bound.max(
                head.distance(self.section_center(index, section))
                    + self.section_collision_radius(index, section),
            );
        }
        bound
    }

    pub fn max_base_radius(&self, index: usize) -> f32 {
        self.base_radii[index]
            .iter()
            .copied()
            .fold(self.radius[index] * SOFT_BODY_BASE_MIN_FACTOR, f32::max)
    }

    pub(crate) fn section_wake_half_width(&self, index: usize, section: u8) -> f32 {
        let radii = match section {
            0 => self.visual_radii[index],
            1 => self.tail_visual_radii[index],
            _ => self.extra_sections[index][section as usize - 2].visual_radii,
        };
        ((radii[2] + radii[6]) * 0.5)
            .max(self.section_core_radius(index, section))
            .max(0.1)
    }

    pub(crate) fn connection_wake_sample(
        &self,
        index: usize,
        edge: usize,
        t: f32,
    ) -> (Vec2, Vec2, f32) {
        let parent = self.section_parents[index][edge];
        let child = edge as u8 + 1;
        let parent_center = self.section_center(index, parent);
        let child_center = self.section_center(index, child);
        let axis = child_center - parent_center;
        let side = axis
            .try_normalize()
            .map(|direction| Vec2::new(-direction.y, direction.x))
            .unwrap_or(Vec2::Y);
        let control =
            (parent_center + child_center) * 0.5 + side * self.edge_curve_offsets[index][edge];
        let t = t.clamp(0.0, 1.0);
        let center = parent_center
            .lerp(control, t)
            .lerp(control.lerp(child_center, t), t);
        let velocity = self
            .section_velocity(index, parent)
            .lerp(self.section_velocity(index, child), t);
        let parent_width = self.section_wake_half_width(index, parent);
        let child_width = self.section_wake_half_width(index, child);
        let end_blend = (t * 2.0 - 1.0).powi(2);
        let width = (parent_width + (child_width - parent_width) * t) * (0.78 + end_blend * 0.22);
        (center, velocity, width)
    }

    fn biomass_sum(&self, index: usize) -> f32 {
        self.biomass[index]
    }

    #[cfg(test)]
    fn virtual_membrane_radius(&self, index: usize, world_angle: f32) -> f32 {
        self.virtual_membrane_radius_local(index, world_angle - self.heading[index])
    }

    fn virtual_membrane_radius_local(&self, index: usize, local_angle: f32) -> f32 {
        let first_angle = SOFT_BODY_BASE_ANGLES[0] + self.angle_offsets[index][0];
        let local_angle =
            first_angle + (local_angle - first_angle).rem_euclid(std::f32::consts::TAU);

        for (left, base_angle) in SOFT_BODY_BASE_ANGLES.iter().copied().enumerate() {
            let right = (left + 1) % SOFT_BODY_POINTS;
            let left_angle = base_angle + self.angle_offsets[index][left];
            let right_angle = if right == 0 {
                first_angle + std::f32::consts::TAU
            } else {
                SOFT_BODY_BASE_ANGLES[right] + self.angle_offsets[index][right]
            };

            if local_angle <= right_angle {
                let span = (right_angle - left_angle).max(0.0001);
                let t = ((local_angle - left_angle) / span).clamp(0.0, 1.0);
                return self.current_radii[index][left]
                    + (self.current_radii[index][right] - self.current_radii[index][left]) * t;
            }
        }

        self.current_radii[index][0]
    }

    fn compress_rays_by_depth(&mut self, index: usize, ray_depths: &[f32; SOFT_BODY_POINTS]) {
        let mut changed = false;
        for (ray_index, depth) in ray_depths.iter().copied().enumerate() {
            if depth <= 0.0 {
                continue;
            }

            let min_radius = self.core_radius[index];
            let old_radius = self.current_radii[index][ray_index];
            self.current_radii[index][ray_index] = (old_radius - depth).max(min_radius);
            changed |= self.current_radii[index][ray_index] < old_radius;
        }

        if changed {
            self.refresh_current_radius_cache(index);
        }
    }

    fn soft_ray_index_for_direction(&self, index: usize, dir: Vec2) -> usize {
        let (heading_s, heading_c) = self.heading[index].sin_cos();
        let local_dir_x = dir.x * heading_c + dir.y * heading_s;
        let local_dir_y = -dir.x * heading_s + dir.y * heading_c;
        let approx = (local_dir_y.atan2(local_dir_x) / SOFT_BODY_SECTOR_ANGLE).round() as i32;
        let mut best_index = 0;
        let mut best_dot = f32::NEG_INFINITY;

        for ray_index in [
            approx.rem_euclid(SOFT_BODY_POINTS as i32) as usize,
            (approx - 1).rem_euclid(SOFT_BODY_POINTS as i32) as usize,
            (approx + 1).rem_euclid(SOFT_BODY_POINTS as i32) as usize,
        ] {
            let dot = self.ray_dir_x[index][ray_index] * local_dir_x
                + self.ray_dir_y[index][ray_index] * local_dir_y;
            if dot > best_dot {
                best_dot = dot;
                best_index = ray_index;
            }
        }

        best_index
    }

    fn compress_ray(&mut self, index: usize, ray_index: usize, target_radius: f32) -> f32 {
        let base = self.base_radii[index][ray_index];
        let min_radius = self.core_radius[index];
        let target = target_radius.clamp(min_radius, base);
        let old = self.current_radii[index][ray_index];
        if target < old {
            self.current_radii[index][ray_index] =
                old + (target - old) * SOFT_BODY_COMPRESSION_RESPONSE;
            self.refresh_current_radius_cache(index);
            old - self.current_radii[index][ray_index]
        } else {
            0.0
        }
    }

    fn relax_soft_body(&mut self, dt: f32) {
        for index in 0..self.len() {
            let elasticity =
                (SOFT_BODY_ELASTICITY_SPEED * self.viability_ratio(index) * dt).clamp(0.0, 1.0);
            let visual_follow = (SOFT_BODY_VISUAL_FOLLOW_SPEED * dt).clamp(0.0, 1.0);
            let mut max_current = self.core_radius[index];
            let mut biomass = 0.0;
            for ray_index in 0..SOFT_BODY_POINTS {
                let base = self.base_radii[index][ray_index].min(self.radius[index]);
                self.base_radii[index][ray_index] = base;
                let current =
                    self.current_radii[index][ray_index].clamp(self.core_radius[index], base);
                let relaxed = current + (base - current) * elasticity;
                self.current_radii[index][ray_index] = relaxed;
                let visual = self.visual_radii[index][ray_index].min(self.radius[index]);
                self.visual_radii[index][ray_index] = visual + (relaxed - visual) * visual_follow;
                max_current = max_current.max(relaxed);
                biomass += base;
            }
            self.collision_radius[index] = max_current;
            self.biomass[index] = biomass;

            if self.section_count[index] >= 2 {
                let mut tail_max = self.tail_core_radius[index];
                for ray_index in 0..SOFT_BODY_POINTS {
                    let base =
                        self.tail_base_radii[index][ray_index].max(self.tail_core_radius[index]);
                    self.tail_base_radii[index][ray_index] = base;
                    let current = self.tail_current_radii[index][ray_index]
                        .clamp(self.tail_core_radius[index], base);
                    let relaxed = current + (base - current) * elasticity;
                    self.tail_current_radii[index][ray_index] = relaxed;
                    let visual = self.tail_visual_radii[index][ray_index].min(base);
                    self.tail_visual_radii[index][ray_index] =
                        visual + (relaxed - visual) * visual_follow;
                    tail_max = tail_max.max(relaxed);
                    biomass += base;
                }
                self.tail_collision_radius[index] = tail_max;
                for extra_index in 0..(self.section_count[index] as usize - 2) {
                    let extra = &mut self.extra_sections[index][extra_index];
                    let mut extra_max = extra.core_radius;
                    for ray_index in 0..SOFT_BODY_POINTS {
                        let base = extra.base_radii[ray_index].max(extra.core_radius);
                        extra.base_radii[ray_index] = base;
                        let current = extra.current_radii[ray_index].clamp(extra.core_radius, base);
                        let relaxed = current + (base - current) * elasticity;
                        extra.current_radii[ray_index] = relaxed;
                        let visual = extra.visual_radii[ray_index].min(base);
                        extra.visual_radii[ray_index] = visual + (relaxed - visual) * visual_follow;
                        extra_max = extra_max.max(relaxed);
                        biomass += base;
                    }
                    extra.collision_radius = extra_max;
                }
                self.biomass[index] = biomass;
            }
        }
    }

    fn rebuild_soft_body_cache(&mut self, index: usize) {
        self.refresh_current_radius_cache(index);
        self.visual_radii[index] = self.current_radii[index];

        let mut biomass = 0.0;
        let mut vector = Vec2::ZERO;
        for (ray_index, base_angle) in SOFT_BODY_BASE_ANGLES.iter().copied().enumerate() {
            biomass += self.base_radii[index][ray_index];
            let offset = self.angle_offsets[index][ray_index];
            let (ray_s, ray_c) = (base_angle + offset).sin_cos();
            self.ray_dir_x[index][ray_index] = ray_c;
            self.ray_dir_y[index][ray_index] = ray_s;
            let (s, c) = base_angle.sin_cos();
            vector += Vec2::new(c, s) * offset;
        }
        let normalized = vector / (SOFT_BODY_POINTS as f32 * SOFT_BODY_MAX_ANGLE_OFFSET);
        self.biomass[index] = biomass;
        self.asymmetry_x[index] = normalized.x;
        self.asymmetry_y[index] = normalized.y;
        self.rebuild_morphology_profile(index);
    }

    fn rebuild_morphology_profile(&mut self, index: usize) {
        let radii = self.base_radii[index];
        let mean = radii.iter().sum::<f32>() / SOFT_BODY_POINTS as f32;
        let variance = radii
            .iter()
            .map(|radius| (radius - mean).powi(2))
            .sum::<f32>()
            / SOFT_BODY_POINTS as f32;
        let radial_cv = variance.sqrt() / mean.max(0.001);

        let axis_mean = (radii[0] + radii[2] + radii[4] + radii[6]) * 0.25;
        let diagonal_mean = (radii[1] + radii[3] + radii[5] + radii[7]) * 0.25;
        let corner_ratio = diagonal_mean / axis_mean.max(0.001);
        let opposite_symmetry = (1.0
            - (0..4)
                .map(|ray| (radii[ray] - radii[ray + 4]).abs())
                .sum::<f32>()
                / (mean * 4.0).max(0.001))
        .clamp(0.0, 1.0);
        let corner_score = (1.0 - (corner_ratio - std::f32::consts::SQRT_2).abs() / 0.32)
            .clamp(0.0, 1.0)
            * opposite_symmetry;

        let mut vertices = [Vec2::ZERO; SOFT_BODY_POINTS];
        for ray in 0..SOFT_BODY_POINTS {
            vertices[ray] =
                Vec2::from_angle(SOFT_BODY_BASE_ANGLES[ray] + self.angle_offsets[index][ray])
                    * radii[ray];
        }
        let mut twice_area = 0.0;
        let mut perimeter = 0.0;
        for ray in 0..SOFT_BODY_POINTS {
            let next = (ray + 1) % SOFT_BODY_POINTS;
            twice_area += vertices[ray].perp_dot(vertices[next]);
            perimeter += vertices[ray].distance(vertices[next]);
        }
        let area = twice_area.abs() * 0.5;
        let mut compactness =
            (4.0 * std::f32::consts::PI * area / perimeter.max(0.001).powi(2)).clamp(0.0, 1.0);

        let mut min_axis = f32::INFINITY;
        let mut max_axis: f32 = 0.0;
        for ray in 0..4 {
            let extent = radii[ray] + radii[ray + 4];
            min_axis = min_axis.min(extent);
            max_axis = max_axis.max(extent);
        }
        let ray_elongation =
            (((max_axis / min_axis.max(0.001) - 1.0) / 2.0) - corner_score * 0.22).clamp(0.0, 1.0);
        let segment_elongation = if self.section_count[index] > 1 {
            let count_factor = (self.section_count[index] as f32 - 1.0) / 3.0;
            let spacing_factor =
                (self.section_spacing[index] / (mean * 2.0).max(0.001)).clamp(0.55, 1.8);
            (count_factor * spacing_factor * 0.82).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let elongation = ray_elongation.max(segment_elongation);
        compactness *= 1.0 - elongation * 0.42;

        let angle_disorder = self.angle_offsets[index]
            .iter()
            .map(|angle| angle.abs())
            .sum::<f32>()
            / (SOFT_BODY_POINTS as f32 * SOFT_BODY_MAX_ANGLE_OFFSET);
        let irregularity = (((radial_cv - corner_score * 0.15).max(0.0) / 0.42)
            + angle_disorder * 0.55
            + (1.0 - opposite_symmetry) * 0.35)
            .clamp(0.0, 1.0);
        let asymmetry = self.asymmetry_vector(index).length().clamp(0.0, 1.0);
        let effective_corner = corner_score * (1.0 - elongation * 0.45);

        self.shape_drag[index] = (1.0 + elongation * 0.25
            - irregularity * 0.13
            - effective_corner * 0.06
            - asymmetry * SOFT_BODY_SHAPE_DRAG)
            .clamp(0.72, 1.28);
        self.morphology_acceleration[index] =
            (1.0 + elongation * 0.32 - irregularity * 0.13 - effective_corner * 0.04)
                .clamp(0.72, 1.32);
        self.morphology_turn[index] =
            (1.0 - elongation * 0.48 - effective_corner * 0.08 - irregularity * 0.10
                + (compactness - 0.72).max(0.0) * 0.18)
                .clamp(0.48, 1.16);
        self.morphology_viability[index] = (0.92 + compactness * 0.12 + effective_corner * 0.12
            - elongation * 0.08
            - irregularity * 0.07)
            .clamp(0.78, 1.20);
        self.morphology_metabolism[index] =
            (1.04 + elongation * 0.12 + irregularity * 0.10 - compactness * 0.08).clamp(0.88, 1.26);

        let old_max = self.max_viability[index].max(1.0);
        let viability_ratio = (self.viability[index] / old_max).clamp(0.0, 1.0);
        self.max_viability[index] = CELL_VIABILITY_MAX * self.morphology_viability[index];
        self.viability[index] = viability_ratio * self.max_viability[index];
    }

    fn rebuild_tail_cache(&mut self, index: usize) {
        let mut max_current = self.tail_core_radius[index];
        for ray_index in 0..SOFT_BODY_POINTS {
            let angle =
                SOFT_BODY_BASE_ANGLES[ray_index] + self.tail_angle_offsets[index][ray_index];
            self.tail_ray_dir_x[index][ray_index] = angle.cos();
            self.tail_ray_dir_y[index][ray_index] = angle.sin();
            max_current = max_current.max(self.tail_current_radii[index][ray_index]);
        }
        self.tail_collision_radius[index] = max_current;
        self.tail_visual_radii[index] = self.tail_current_radii[index];
    }

    fn refresh_current_radius_cache(&mut self, index: usize) {
        self.collision_radius[index] = self.current_radii[index]
            .iter()
            .copied()
            .fold(self.core_radius[index], f32::max);
    }

    fn asymmetry_vector(&self, index: usize) -> Vec2 {
        Vec2::new(self.asymmetry_x[index], self.asymmetry_y[index])
    }

    pub fn morphology_speed_factor(&self, index: usize) -> f32 {
        self.shape_drag[index]
    }

    #[cfg(test)]
    fn shape_drag_factor(&self, index: usize) -> f32 {
        self.morphology_speed_factor(index)
    }

    pub fn morphology_acceleration_factor(&self, index: usize) -> f32 {
        self.morphology_acceleration[index]
    }

    pub fn morphology_turn_factor(&self, index: usize) -> f32 {
        self.morphology_turn[index]
    }

    pub fn morphology_viability_factor(&self, index: usize) -> f32 {
        self.morphology_viability[index]
    }

    pub fn morphology_metabolism_factor(&self, index: usize) -> f32 {
        self.morphology_metabolism[index]
    }

    fn turn_agility_factor(&self, index: usize, turn_delta: f32) -> f32 {
        let asymmetry = self.asymmetry_vector(index);
        let amount = asymmetry.length().clamp(0.0, 1.0);
        if amount <= 0.001 || turn_delta.abs() <= 0.001 {
            return 1.0;
        }

        let bend_angle = asymmetry.y.atan2(asymmetry.x);
        let bend_side = angle_delta(bend_angle, 0.0).signum();
        if bend_side == turn_delta.signum() {
            1.0 + amount * SOFT_BODY_TURN_BONUS
        } else {
            1.0
        }
    }
}

pub struct CellGrid {
    cols: usize,
    rows: usize,
    cell_size: f32,
    width: f32,
    height: f32,
    buckets: Vec<Vec<usize>>,
    occupied_buckets: Vec<usize>,
}

impl CellGrid {
    fn new(width: f32, height: f32, cell_size: f32) -> Self {
        let cols = (width / cell_size).ceil() as usize;
        let rows = (height / cell_size).ceil() as usize;
        let buckets = (0..cols * rows).map(|_| Vec::with_capacity(2)).collect();

        Self {
            cols,
            rows,
            cell_size,
            width,
            height,
            buckets,
            occupied_buckets: Vec::new(),
        }
    }

    fn rebuild(&mut self, cells: &CellStore) {
        self.rebuild_points(&cells.x, &cells.y);
    }

    fn rebuild_points(&mut self, x: &[f32], y: &[f32]) {
        for bucket in self.occupied_buckets.drain(..) {
            self.buckets[bucket].clear();
        }

        for i in 0..x.len().min(y.len()) {
            let bucket = self.bucket_index(x[i], y[i]);
            if self.buckets[bucket].is_empty() {
                self.occupied_buckets.push(bucket);
            }
            self.buckets[bucket].push(i);
        }
    }

    fn bucket_index(&self, x: f32, y: f32) -> usize {
        let gx = ((x + self.width * 0.5) / self.cell_size)
            .floor()
            .clamp(0.0, self.cols as f32 - 1.0) as usize;
        let gy = ((y + self.height * 0.5) / self.cell_size)
            .floor()
            .clamp(0.0, self.rows as f32 - 1.0) as usize;
        gy * self.cols + gx
    }

    fn bucket_range(&self, center: Vec2, radius: f32) -> (usize, usize, usize, usize) {
        let min_x = (((center.x - radius + self.width * 0.5) / self.cell_size).floor() as i32)
            .clamp(0, self.cols as i32 - 1) as usize;
        let max_x = (((center.x + radius + self.width * 0.5) / self.cell_size).floor() as i32)
            .clamp(0, self.cols as i32 - 1) as usize;
        let min_y = (((center.y - radius + self.height * 0.5) / self.cell_size).floor() as i32)
            .clamp(0, self.rows as i32 - 1) as usize;
        let max_y = (((center.y + radius + self.height * 0.5) / self.cell_size).floor() as i32)
            .clamp(0, self.rows as i32 - 1) as usize;
        (min_x, max_x, min_y, max_y)
    }
}

pub struct FoodStore {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub kind: Vec<FoodKind>,
    pub shape: Vec<FoodShape>,
    pub phase: Vec<f32>,
    pub rotation: Vec<f32>,
    pub spin: Vec<f32>,
    pub growth: Vec<f32>,
    pub energy: Vec<f32>,
    pub origin_species: Vec<i32>,
    pub age: Vec<f32>,
    pub lifetime: Vec<f32>,
    source: Vec<FoodSource>,
    regrow_timer: Vec<f32>,
    pub active: Vec<bool>,
    generation: Vec<u32>,
    pub feeder: Vec<i32>,
    pub anchor_branch: Vec<i32>,
    pub anchor_angle: Vec<f32>,
    pub anchor_distance: Vec<f32>,
    pub anchor_lateral: Vec<f32>,
}

impl FoodStore {
    fn new(
        count: usize,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        rng: &mut SmallRng,
    ) -> Self {
        let mut store = Self {
            x: Vec::with_capacity(count),
            y: Vec::with_capacity(count),
            kind: Vec::with_capacity(count),
            shape: Vec::with_capacity(count),
            phase: Vec::with_capacity(count),
            rotation: Vec::with_capacity(count),
            spin: Vec::with_capacity(count),
            growth: Vec::with_capacity(count),
            energy: Vec::with_capacity(count),
            origin_species: Vec::with_capacity(count),
            age: Vec::with_capacity(count),
            lifetime: Vec::with_capacity(count),
            source: Vec::with_capacity(count),
            regrow_timer: Vec::with_capacity(count),
            active: Vec::with_capacity(count),
            generation: Vec::with_capacity(count),
            feeder: Vec::with_capacity(count),
            anchor_branch: Vec::with_capacity(count),
            anchor_angle: Vec::with_capacity(count),
            anchor_distance: Vec::with_capacity(count),
            anchor_lateral: Vec::with_capacity(count),
        };

        for _ in 0..count {
            store.push_random(arena_w, arena_h, arena_shape, rng);
        }

        store
    }

    fn push_random(
        &mut self,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        rng: &mut SmallRng,
    ) {
        let kind = FoodKind::Grass;
        let position = random_point_in_arena(arena_w, arena_h, arena_shape, FOOD_RADIUS, rng);
        self.x.push(position.x);
        self.y.push(position.y);
        self.kind.push(kind);
        self.shape.push(FoodShape::random(rng));
        self.phase
            .push(rng.random_range(0.0..std::f32::consts::TAU));
        self.rotation
            .push(rng.random_range(0.0..std::f32::consts::TAU));
        self.spin.push(random_food_spin(rng));
        self.growth.push(1.0);
        self.energy.push(WORLD_GRASS_ENERGY);
        self.origin_species.push(-1);
        self.age.push(0.0);
        self.lifetime.push(food_lifetime(kind, rng));
        self.source.push(FoodSource::Wild);
        self.regrow_timer.push(0.0);
        self.active.push(true);
        self.generation.push(1);
        self.feeder.push(-1);
        self.anchor_branch.push(-1);
        self.anchor_angle.push(0.0);
        self.anchor_distance.push(0.0);
        self.anchor_lateral.push(0.0);
    }

    fn push_feeder_at(
        &mut self,
        grower_index: i32,
        branch_index: i32,
        x: f32,
        y: f32,
        anchor_angle: f32,
        anchor_distance: f32,
        anchor_lateral: f32,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        rng: &mut SmallRng,
    ) -> usize {
        let point =
            clamp_point_to_arena(Vec2::new(x, y), arena_w, arena_h, arena_shape, FOOD_RADIUS);
        let x = point.x;
        let y = point.y;

        if let Some(index) = self.inactive_slot_for(FoodSource::Feeder) {
            self.generation[index] = self.generation[index].wrapping_add(1);
            self.x[index] = x;
            self.y[index] = y;
            self.kind[index] = FoodKind::Grass;
            self.shape[index] = FoodShape::random_feeder_food(rng);
            self.phase[index] = rng.random_range(0.0..std::f32::consts::TAU);
            self.rotation[index] = anchor_angle;
            self.spin[index] = 0.0;
            self.growth[index] = 0.24;
            self.energy[index] = FEEDER_FOOD_ENERGY;
            self.origin_species[index] = -1;
            self.age[index] = 0.0;
            self.lifetime[index] = rng.random_range(55.0..90.0);
            self.source[index] = FoodSource::Feeder;
            self.regrow_timer[index] = 0.0;
            self.active[index] = true;
            self.feeder[index] = grower_index;
            self.anchor_branch[index] = branch_index;
            self.anchor_angle[index] = anchor_angle;
            self.anchor_distance[index] = anchor_distance;
            self.anchor_lateral[index] = anchor_lateral;
            return index;
        }

        self.x.push(x);
        self.y.push(y);
        self.kind.push(FoodKind::Grass);
        self.shape.push(FoodShape::random_feeder_food(rng));
        self.phase
            .push(rng.random_range(0.0..std::f32::consts::TAU));
        self.rotation.push(anchor_angle);
        self.spin.push(0.0);
        self.growth.push(0.24);
        self.energy.push(FEEDER_FOOD_ENERGY);
        self.origin_species.push(-1);
        self.age.push(0.0);
        self.lifetime.push(rng.random_range(55.0..90.0));
        self.source.push(FoodSource::Feeder);
        self.regrow_timer.push(0.0);
        self.active.push(true);
        self.generation.push(1);
        self.feeder.push(grower_index);
        self.anchor_branch.push(branch_index);
        self.anchor_angle.push(anchor_angle);
        self.anchor_distance.push(anchor_distance);
        self.anchor_lateral.push(anchor_lateral);
        self.x.len() - 1
    }

    fn push_meat_at(
        &mut self,
        x: f32,
        y: f32,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        energy: f32,
        origin_species: u32,
        rng: &mut SmallRng,
    ) {
        let point =
            clamp_point_to_arena(Vec2::new(x, y), arena_w, arena_h, arena_shape, FOOD_RADIUS);
        let x = point.x;
        let y = point.y;

        if let Some(index) = self.inactive_slot_for(FoodSource::Carrion) {
            self.generation[index] = self.generation[index].wrapping_add(1);
            self.x[index] = x;
            self.y[index] = y;
            self.kind[index] = FoodKind::Meat;
            self.shape[index] = FoodShape::random(rng);
            self.phase[index] = rng.random_range(0.0..std::f32::consts::TAU);
            self.rotation[index] = rng.random_range(0.0..std::f32::consts::TAU);
            self.spin[index] = random_food_spin(rng);
            self.growth[index] = 1.0;
            self.energy[index] = energy.max(0.0);
            self.origin_species[index] = origin_species as i32;
            self.age[index] = 0.0;
            self.lifetime[index] = food_lifetime(FoodKind::Meat, rng);
            self.source[index] = FoodSource::Carrion;
            self.regrow_timer[index] = 0.0;
            self.active[index] = true;
            self.feeder[index] = -1;
            self.anchor_branch[index] = -1;
            self.anchor_angle[index] = 0.0;
            self.anchor_distance[index] = 0.0;
            self.anchor_lateral[index] = 0.0;
            return;
        }

        self.x.push(x);
        self.y.push(y);
        self.kind.push(FoodKind::Meat);
        self.shape.push(FoodShape::random(rng));
        self.phase
            .push(rng.random_range(0.0..std::f32::consts::TAU));
        self.rotation
            .push(rng.random_range(0.0..std::f32::consts::TAU));
        self.spin.push(random_food_spin(rng));
        self.growth.push(1.0);
        self.energy.push(energy.max(0.0));
        self.origin_species.push(origin_species as i32);
        self.age.push(0.0);
        self.lifetime.push(food_lifetime(FoodKind::Meat, rng));
        self.source.push(FoodSource::Carrion);
        self.regrow_timer.push(0.0);
        self.active.push(true);
        self.generation.push(1);
        self.feeder.push(-1);
        self.anchor_branch.push(-1);
        self.anchor_angle.push(0.0);
        self.anchor_distance.push(0.0);
        self.anchor_lateral.push(0.0);
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn active_count(&self) -> usize {
        self.active.iter().filter(|active| **active).count()
    }

    fn active_count_for(&self, source: FoodSource) -> usize {
        self.active
            .iter()
            .zip(&self.source)
            .filter(|(active, item_source)| **active && **item_source == source)
            .count()
    }

    fn inactive_slot_for(&self, source: FoodSource) -> Option<usize> {
        self.active
            .iter()
            .zip(&self.source)
            .position(|(active, item_source)| !*active && *item_source == source)
    }

    fn is_feeder_food(&self, index: usize) -> bool {
        self.feeder[index] >= 0
    }

    fn feeder_index(&self, index: usize) -> Option<usize> {
        (self.feeder[index] >= 0).then_some(self.feeder[index] as usize)
    }

    fn deactivate(&mut self, index: usize) {
        self.generation[index] = self.generation[index].wrapping_add(1);
        self.active[index] = false;
        self.growth[index] = 0.0;
        self.energy[index] = 0.0;
        self.age[index] = 0.0;
        self.lifetime[index] = 0.0;
        self.regrow_timer[index] = if self.source[index] == FoodSource::Wild {
            WILD_GRASS_REGROW_MIN + self.phase[index].sin().abs() * WILD_GRASS_REGROW_SPREAD
        } else {
            0.0
        };
        self.feeder[index] = -1;
        self.anchor_branch[index] = -1;
        self.anchor_angle[index] = 0.0;
        self.anchor_distance[index] = 0.0;
        self.anchor_lateral[index] = 0.0;
    }

    fn respawn_wild_at(&mut self, index: usize, position: Vec2, rng: &mut SmallRng) {
        debug_assert_eq!(self.source[index], FoodSource::Wild);
        self.generation[index] = self.generation[index].wrapping_add(1);
        self.x[index] = position.x;
        self.y[index] = position.y;
        self.kind[index] = FoodKind::Grass;
        self.shape[index] = FoodShape::random(rng);
        self.phase[index] = rng.random_range(0.0..std::f32::consts::TAU);
        self.rotation[index] = rng.random_range(0.0..std::f32::consts::TAU);
        self.spin[index] = random_food_spin(rng);
        self.growth[index] = rng.random_range(0.18..0.34);
        self.energy[index] = WORLD_GRASS_ENERGY;
        self.origin_species[index] = -1;
        self.age[index] = 0.0;
        self.lifetime[index] = food_lifetime(FoodKind::Grass, rng);
        self.regrow_timer[index] = 0.0;
        self.active[index] = true;
        self.feeder[index] = -1;
        self.anchor_branch[index] = -1;
        self.anchor_angle[index] = 0.0;
        self.anchor_distance[index] = 0.0;
        self.anchor_lateral[index] = 0.0;
    }
}

fn food_lifetime(kind: FoodKind, rng: &mut SmallRng) -> f32 {
    match kind {
        FoodKind::Grass => rng.random_range(70.0..120.0),
        FoodKind::Meat => rng.random_range(22.0..42.0),
    }
}

fn random_food_spin(rng: &mut SmallRng) -> f32 {
    let speed = rng.random_range(0.7..2.6);
    if rng.random_bool(0.5) { -speed } else { speed }
}

fn add_avoidance(avoidance: &mut Vec2, delta: Vec2, influence: f32, desired_dir: Vec2, speed: f32) {
    let dist_sq = delta.length_squared();
    if dist_sq >= influence * influence {
        return;
    }

    let (normal, dist) = if dist_sq > 0.0001 {
        let dist = dist_sq.sqrt();
        (delta / dist, dist)
    } else {
        (Vec2::X, 0.001)
    };
    let pressure = (1.0 - dist / influence).clamp(0.0, 1.0);
    let mut tangent = Vec2::new(-normal.y, normal.x);
    if tangent.dot(desired_dir) < 0.0 {
        tangent = -tangent;
    }
    *avoidance +=
        (normal * 1.18 + tangent * 0.72) * pressure * pressure * speed * CELL_AVOIDANCE_STRENGTH;
}

pub struct ObstacleStore {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub radius: Vec<f32>,
    pub phase: Vec<f32>,
    pub rotation: Vec<f32>,
    pub spin: Vec<f32>,
    pub spokes: Vec<f32>,
    pub rings: Vec<f32>,
}

impl ObstacleStore {
    fn new(
        count: usize,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        rng: &mut SmallRng,
    ) -> Self {
        let mut store = Self {
            x: Vec::with_capacity(count),
            y: Vec::with_capacity(count),
            vx: Vec::with_capacity(count),
            vy: Vec::with_capacity(count),
            radius: Vec::with_capacity(count),
            phase: Vec::with_capacity(count),
            rotation: Vec::with_capacity(count),
            spin: Vec::with_capacity(count),
            spokes: Vec::with_capacity(count),
            rings: Vec::with_capacity(count),
        };

        for _ in 0..count {
            let radius = 14.0 + (76.0 - 14.0) * rng.random_range(0.0_f32..1.0).powf(1.65);
            let position = random_point_in_arena(arena_w, arena_h, arena_shape, radius, rng);
            store.x.push(position.x);
            store.y.push(position.y);
            store.vx.push(rng.random_range(-4.0..4.0));
            store.vy.push(rng.random_range(-4.0..4.0));
            store.radius.push(radius);
            store
                .phase
                .push(rng.random_range(0.0..std::f32::consts::TAU));
            store
                .rotation
                .push(rng.random_range(0.0..std::f32::consts::TAU));
            store.spin.push(rng.random_range(-0.085..0.085));
            store.spokes.push(rng.random_range(18.0..35.0));
            store.rings.push(rng.random_range(2.0..5.0));
        }

        store
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }
}

pub struct FoodGrowerStore {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub radius: Vec<f32>,
    pub phase: Vec<f32>,
    pub rotation: Vec<f32>,
    pub spin: Vec<f32>,
    pub branch_start: Vec<usize>,
    pub branch_count: Vec<usize>,
    pub branch_angle: Vec<f32>,
    pub branch_length: Vec<f32>,
    pub branch_width: Vec<f32>,
    pub branch_curve: Vec<f32>,
    pub branch_layer: Vec<f32>,
    pub branch_solid: Vec<bool>,
    pub branch_phase: Vec<f32>,
    pub branch_world_angle: Vec<f32>,
    pub branch_start_x: Vec<f32>,
    pub branch_start_y: Vec<f32>,
    pub branch_end_x: Vec<f32>,
    pub branch_end_y: Vec<f32>,
    pub branch_hue_shift: Vec<f32>,
    pub branch_lightness_shift: Vec<f32>,
    pub branch_saturation_shift: Vec<f32>,
    pub branch_width_scale: Vec<f32>,
    pub branch_sway_speed: Vec<f32>,
    pub extent: Vec<f32>,
    pub timer: Vec<f32>,
    pub interval: Vec<f32>,

    // Branchlets
    pub branchlet_grower_index: Vec<usize>,
    pub branchlet_branch_index: Vec<usize>,
    pub branchlet_t: Vec<f32>,
    pub branchlet_side: Vec<f32>,
    pub branchlet_length: Vec<f32>,
    pub branchlet_angle_dev: Vec<f32>,
    pub branchlet_food_index: Vec<Option<usize>>,
}

fn random_non_overlapping_grower_position(
    arena_w: f32,
    arena_h: f32,
    arena_shape: ArenaShape,
    extent: f32,
    gap: f32,
    existing_x: &[f32],
    existing_y: &[f32],
    existing_extents: &[f32],
    rng: &mut SmallRng,
) -> Vec2 {
    let mut best = Vec2::ZERO;
    let mut best_clearance = f32::NEG_INFINITY;
    for _ in 0..192 {
        let candidate = random_point_in_arena(arena_w, arena_h, arena_shape, extent, rng);
        let clearance = existing_x
            .iter()
            .zip(existing_y)
            .zip(existing_extents)
            .map(|((&x, &y), &other_extent)| {
                candidate.distance(Vec2::new(x, y)) - extent - other_extent - gap
            })
            .fold(f32::INFINITY, f32::min);
        if clearance >= 0.0 {
            return candidate;
        }
        if clearance > best_clearance {
            best = candidate;
            best_clearance = clearance;
        }
    }
    best
}

impl FoodGrowerStore {
    fn new(
        count: usize,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        rng: &mut SmallRng,
    ) -> Self {
        let count = count.max(1);
        let mut store = Self {
            x: Vec::with_capacity(count),
            y: Vec::with_capacity(count),
            vx: Vec::with_capacity(count),
            vy: Vec::with_capacity(count),
            radius: Vec::with_capacity(count),
            phase: Vec::with_capacity(count),
            rotation: Vec::with_capacity(count),
            spin: Vec::with_capacity(count),
            branch_start: Vec::with_capacity(count),
            branch_count: Vec::with_capacity(count),
            branch_angle: Vec::with_capacity(count.saturating_mul(10)),
            branch_length: Vec::with_capacity(count.saturating_mul(10)),
            branch_width: Vec::with_capacity(count.saturating_mul(10)),
            branch_curve: Vec::with_capacity(count.saturating_mul(10)),
            branch_layer: Vec::with_capacity(count.saturating_mul(10)),
            branch_solid: Vec::with_capacity(count.saturating_mul(10)),
            branch_phase: Vec::with_capacity(count.saturating_mul(10)),
            branch_world_angle: Vec::with_capacity(count.saturating_mul(10)),
            branch_start_x: Vec::with_capacity(count.saturating_mul(10)),
            branch_start_y: Vec::with_capacity(count.saturating_mul(10)),
            branch_end_x: Vec::with_capacity(count.saturating_mul(10)),
            branch_end_y: Vec::with_capacity(count.saturating_mul(10)),
            branch_hue_shift: Vec::with_capacity(count.saturating_mul(10)),
            branch_lightness_shift: Vec::with_capacity(count.saturating_mul(10)),
            branch_saturation_shift: Vec::with_capacity(count.saturating_mul(10)),
            branch_width_scale: Vec::with_capacity(count.saturating_mul(10)),
            branch_sway_speed: Vec::with_capacity(count.saturating_mul(10)),
            extent: Vec::with_capacity(count),
            timer: Vec::with_capacity(count),
            interval: Vec::with_capacity(count),

            branchlet_grower_index: Vec::with_capacity(count.saturating_mul(20)),
            branchlet_branch_index: Vec::with_capacity(count.saturating_mul(20)),
            branchlet_t: Vec::with_capacity(count.saturating_mul(20)),
            branchlet_side: Vec::with_capacity(count.saturating_mul(20)),
            branchlet_length: Vec::with_capacity(count.saturating_mul(20)),
            branchlet_angle_dev: Vec::with_capacity(count.saturating_mul(20)),
            branchlet_food_index: Vec::with_capacity(count.saturating_mul(20)),
        };

        let arena_min_dimension = arena_w.min(arena_h).max(1_000.0);
        let grower_scale = (arena_min_dimension / 10_000.0).clamp(0.35, 3.0);
        let titanic_radius = 280.0 * grower_scale;
        let grower_gap = 42.0 * grower_scale;

        for grower_index in 0..count {
            let titanic = grower_index == 0;
            let giant = !titanic && count > 6 && rng.random_bool(0.06);
            let radius = if titanic {
                titanic_radius
            } else if giant {
                rng.random_range(110.0..150.0) * grower_scale
            } else {
                rng.random_range(50.0..86.0) * grower_scale
            };
            let branch_reach: f32 = if titanic {
                rng.random_range(1.92..2.18)
            } else if giant {
                rng.random_range(1.42..1.70)
            } else {
                rng.random_range(1.34..1.58)
            };
            let extent = radius * branch_reach;
            if titanic {
                let position =
                    clamp_point_to_arena(Vec2::ZERO, arena_w, arena_h, arena_shape, extent);
                store.x.push(position.x);
                store.y.push(position.y);
                store.vx.push(rng.random_range(-0.45..0.45));
                store.vy.push(rng.random_range(-0.45..0.45));
            } else {
                let spawn_extent = extent + radius * 0.18;
                let position = random_non_overlapping_grower_position(
                    arena_w,
                    arena_h,
                    arena_shape,
                    spawn_extent,
                    grower_gap,
                    &store.x,
                    &store.y,
                    &store.extent,
                    rng,
                );
                store.x.push(position.x);
                store.y.push(position.y);
                store.vx.push(rng.random_range(-2.0..2.0));
                store.vy.push(rng.random_range(-2.0..2.0));
            }
            store.radius.push(radius);
            store
                .phase
                .push(rng.random_range(0.0..std::f32::consts::TAU));
            store
                .rotation
                .push(rng.random_range(0.0..std::f32::consts::TAU));
            store.spin.push(if titanic {
                rng.random_range(-0.007..0.007)
            } else {
                rng.random_range(-0.018..0.018)
            });
            let branch_count = if titanic {
                rng.random_range(24..33)
            } else if giant {
                rng.random_range(12..19)
            } else {
                rng.random_range(6..11)
            };
            let branch_start = store.branch_angle.len();
            store.branch_start.push(branch_start);
            store.branch_count.push(branch_count);
            let solid_offset = rng.random_range(0..2);
            let solid_count = branch_count / 2;

            let mut max_extent = radius;
            for branch_index in 0..branch_count {
                let branch_step = std::f32::consts::TAU / branch_count as f32;
                let angle = branch_step * branch_index as f32 + rng.random_range(-0.16..0.16);
                store.branch_angle.push(angle);
                let length_variance = if titanic { 0.86..1.08 } else { 0.88..1.02 };
                let branch_length = extent * rng.random_range(length_variance);
                let width_range = if titanic { 0.052..0.086 } else { 0.065..0.11 };
                let branch_width = radius * rng.random_range(width_range);
                max_extent = max_extent.max(branch_length + branch_width);
                store.branch_length.push(branch_length);
                store.branch_width.push(branch_width);
                let curve_range = if titanic { -0.62..0.62 } else { -0.52..0.52 };
                store.branch_curve.push(rng.random_range(curve_range));
                let solid_slot = branch_index >= solid_offset
                    && (branch_index - solid_offset) % 2 == 0
                    && (branch_index - solid_offset) / 2 < solid_count;
                let layer = if solid_slot {
                    rng.random_range(0.68..0.96)
                } else {
                    rng.random_range(0.18..0.46)
                };
                store.branch_layer.push(layer);
                store.branch_solid.push(solid_slot);
                store
                    .branch_phase
                    .push(rng.random_range(0.0..std::f32::consts::TAU));
                store.branch_world_angle.push(0.0);
                store.branch_start_x.push(0.0);
                store.branch_start_y.push(0.0);
                store.branch_end_x.push(0.0);
                store.branch_end_y.push(0.0);
                store.branch_hue_shift.push(rng.random_range(-0.08..0.08));
                store
                    .branch_lightness_shift
                    .push(rng.random_range(-0.12..0.12));
                store
                    .branch_saturation_shift
                    .push(rng.random_range(-0.10..0.10));
                store.branch_width_scale.push(rng.random_range(0.85..1.15));
                store.branch_sway_speed.push(rng.random_range(0.18..0.44));
            }

            store.extent.push(max_extent);
            store.timer.push(if titanic {
                rng.random_range(0.05..0.9)
            } else {
                rng.random_range(0.2..3.0)
            });
            store.interval.push(if titanic {
                rng.random_range(0.55..1.15)
            } else if giant {
                rng.random_range(1.2..2.4)
            } else {
                rng.random_range(1.8..3.6)
            });
        }

        store.rebuild_branch_world_geometry(0.0);
        store
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn branch_total(&self) -> usize {
        self.branch_angle.len()
    }

    pub fn extent_radius(&self, index: usize) -> f32 {
        self.extent[index]
    }

    pub fn branch_range(&self, index: usize) -> std::ops::Range<usize> {
        let start = self.branch_start[index];
        start..start + self.branch_count[index]
    }

    pub fn branch_collision_width_at(&self, branch_index: usize, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        self.branch_width[branch_index]
            * (1.28 + (0.58 - 1.28) * t)
            * self.branch_width_scale[branch_index]
    }

    pub fn branch_has_collision(&self, branch_index: usize) -> bool {
        self.branch_solid[branch_index]
    }

    pub fn branch_center_at(&self, branch_index: usize, t: f32) -> Vec2 {
        let t = t.clamp(0.0, 1.0);
        let start = Vec2::new(
            self.branch_start_x[branch_index],
            self.branch_start_y[branch_index],
        );
        let end = Vec2::new(
            self.branch_end_x[branch_index],
            self.branch_end_y[branch_index],
        );
        let segment = end - start;
        let tangent = segment.try_normalize().unwrap_or(Vec2::X);
        let normal = Vec2::new(-tangent.y, tangent.x);
        let curve = self.branch_curve[branch_index]
            * self.branch_length[branch_index]
            * 0.16
            * (std::f32::consts::PI * t).sin();

        start + segment * t + normal * curve
    }

    pub fn branch_normal_at(&self, branch_index: usize, t: f32) -> Vec2 {
        let before = self.branch_center_at(branch_index, t - 0.025);
        let after = self.branch_center_at(branch_index, t + 0.025);
        let tangent = (after - before).try_normalize().unwrap_or(Vec2::X);
        Vec2::new(-tangent.y, tangent.x)
    }

    pub fn closest_point_on_branch(&self, branch_index: usize, point: Vec2) -> (Vec2, f32) {
        let samples = 10;
        let mut best_point = self.branch_center_at(branch_index, 0.0);
        let mut best_t = 0.0;
        let mut best_dist_sq = point.distance_squared(best_point);
        let mut previous = best_point;
        let mut previous_t = 0.0;

        for sample in 1..=samples {
            let t = sample as f32 / samples as f32;
            let current = self.branch_center_at(branch_index, t);
            let segment = current - previous;
            let segment_len_sq = segment.length_squared().max(0.0001);
            let local_t = ((point - previous).dot(segment) / segment_len_sq).clamp(0.0, 1.0);
            let candidate = previous + segment * local_t;
            let candidate_t = previous_t + (t - previous_t) * local_t;
            let dist_sq = point.distance_squared(candidate);
            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_point = candidate;
                best_t = candidate_t;
            }
            previous = current;
            previous_t = t;
        }

        (best_point, best_t)
    }

    pub fn total_branches(&self) -> usize {
        self.branch_angle.len()
    }

    pub fn rebuild_branch_world_geometry(&mut self, elapsed: f32) {
        for grower_index in 0..self.len() {
            let center_x = self.x[grower_index];
            let center_y = self.y[grower_index];
            let start_distance = self.radius[grower_index] * 0.56;
            let start = self.branch_start[grower_index];
            let end = start + self.branch_count[grower_index];

            for branch_index in start..end {
                let sway = (self.branch_phase[branch_index]
                    + elapsed * self.branch_sway_speed[branch_index])
                    .sin()
                    * 0.06;
                let angle = self.rotation[grower_index] + self.branch_angle[branch_index] + sway;
                let (s, c) = angle.sin_cos();
                let end_distance = self.branch_length[branch_index];

                self.branch_world_angle[branch_index] = angle;
                self.branch_start_x[branch_index] = center_x + c * start_distance;
                self.branch_start_y[branch_index] = center_y + s * start_distance;
                self.branch_end_x[branch_index] = center_x + c * end_distance;
                self.branch_end_y[branch_index] = center_y + s * end_distance;
            }
        }
    }
}

pub struct SpatialGrid {
    cols: usize,
    rows: usize,
    cell_size: f32,
    width: f32,
    height: f32,
    buckets: Vec<Vec<usize>>,
}

impl SpatialGrid {
    fn new(width: f32, height: f32, cell_size: f32) -> Self {
        let cols = (width / cell_size).ceil() as usize;
        let rows = (height / cell_size).ceil() as usize;
        let buckets = (0..cols * rows).map(|_| Vec::with_capacity(8)).collect();

        Self {
            cols,
            rows,
            cell_size,
            width,
            height,
            buckets,
        }
    }

    fn rebuild(&mut self, food: &FoodStore) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }

        for i in 0..food.len() {
            if !food.active[i] {
                continue;
            }

            let bucket = self.bucket_index(food.x[i], food.y[i]);
            self.buckets[bucket].push(i);
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn nearest_food(
        &self,
        x: f32,
        y: f32,
        food: &FoodStore,
        perception_radius: f32,
    ) -> Option<(usize, f32, f32, f32)> {
        self.nearest_food_filtered(x, y, food, perception_radius, None)
    }

    #[inline]
    fn nearest_food_filtered(
        &self,
        x: f32,
        y: f32,
        food: &FoodStore,
        perception_radius: f32,
        forbidden_meat_species: Option<u32>,
    ) -> Option<(usize, f32, f32, f32)> {
        let (cx, cy) = self.grid_coords(x, y);
        let mut best = None;
        let radius_sq = perception_radius.max(0.0).powi(2);
        let mut best_dist_sq = f32::INFINITY;
        let max_ring = (perception_radius.max(0.0) / self.cell_size).ceil() as i32 + 1;

        for ring in 0..=max_ring {
            if ring == 0 {
                self.scan_food_bucket(
                    cx,
                    cy,
                    x,
                    y,
                    food,
                    forbidden_meat_species,
                    radius_sq,
                    &mut best_dist_sq,
                    &mut best,
                );
            } else {
                for ox in -ring..=ring {
                    self.scan_food_bucket(
                        cx + ox,
                        cy - ring,
                        x,
                        y,
                        food,
                        forbidden_meat_species,
                        radius_sq,
                        &mut best_dist_sq,
                        &mut best,
                    );
                    self.scan_food_bucket(
                        cx + ox,
                        cy + ring,
                        x,
                        y,
                        food,
                        forbidden_meat_species,
                        radius_sq,
                        &mut best_dist_sq,
                        &mut best,
                    );
                }
                for oy in (-ring + 1)..ring {
                    self.scan_food_bucket(
                        cx - ring,
                        cy + oy,
                        x,
                        y,
                        food,
                        forbidden_meat_species,
                        radius_sq,
                        &mut best_dist_sq,
                        &mut best,
                    );
                    self.scan_food_bucket(
                        cx + ring,
                        cy + oy,
                        x,
                        y,
                        food,
                        forbidden_meat_species,
                        radius_sq,
                        &mut best_dist_sq,
                        &mut best,
                    );
                }
            }

            if best.is_some() && best_dist_sq <= self.unsearched_distance_sq(x, y, cx, cy, ring) {
                break;
            }
        }

        best
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn scan_food_bucket(
        &self,
        gx: i32,
        gy: i32,
        x: f32,
        y: f32,
        food: &FoodStore,
        forbidden_meat_species: Option<u32>,
        radius_sq: f32,
        best_dist_sq: &mut f32,
        best: &mut Option<(usize, f32, f32, f32)>,
    ) {
        if gx < 0 || gy < 0 || gx >= self.cols as i32 || gy >= self.rows as i32 {
            return;
        }
        if self.bucket_distance_sq(x, y, gx, gy) > radius_sq.min(*best_dist_sq) {
            return;
        }

        let bucket = gy as usize * self.cols + gx as usize;
        for &food_index in &self.buckets[bucket] {
            if let Some(species) = forbidden_meat_species {
                if food.kind[food_index] == FoodKind::Meat
                    && food.origin_species[food_index] >= 0
                    && food.origin_species[food_index] as u32 == species
                {
                    continue;
                }
            }
            let dx = food.x[food_index] - x;
            let dy = food.y[food_index] - y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= radius_sq && dist_sq < *best_dist_sq {
                *best_dist_sq = dist_sq;
                *best = Some((food_index, dx, dy, dist_sq.max(0.0001)));
            }
        }
    }

    #[inline]
    fn bucket_distance_sq(&self, x: f32, y: f32, gx: i32, gy: i32) -> f32 {
        let min_x = -self.width * 0.5 + gx as f32 * self.cell_size;
        let min_y = -self.height * 0.5 + gy as f32 * self.cell_size;
        let max_x = (min_x + self.cell_size).min(self.width * 0.5);
        let max_y = (min_y + self.cell_size).min(self.height * 0.5);
        let dx = if x < min_x {
            min_x - x
        } else if x > max_x {
            x - max_x
        } else {
            0.0
        };
        let dy = if y < min_y {
            min_y - y
        } else if y > max_y {
            y - max_y
        } else {
            0.0
        };
        dx * dx + dy * dy
    }

    #[inline]
    fn unsearched_distance_sq(&self, x: f32, y: f32, cx: i32, cy: i32, ring: i32) -> f32 {
        let min_gx = (cx - ring).max(0);
        let max_gx = (cx + ring).min(self.cols as i32 - 1);
        let min_gy = (cy - ring).max(0);
        let max_gy = (cy + ring).min(self.rows as i32 - 1);
        let mut distance = f32::INFINITY;

        if min_gx > 0 {
            distance = distance.min(x - (-self.width * 0.5 + min_gx as f32 * self.cell_size));
        }
        if max_gx < self.cols as i32 - 1 {
            distance = distance.min(-self.width * 0.5 + (max_gx + 1) as f32 * self.cell_size - x);
        }
        if min_gy > 0 {
            distance = distance.min(y - (-self.height * 0.5 + min_gy as f32 * self.cell_size));
        }
        if max_gy < self.rows as i32 - 1 {
            distance = distance.min(-self.height * 0.5 + (max_gy + 1) as f32 * self.cell_size - y);
        }

        distance.max(0.0).powi(2)
    }

    fn bucket_index(&self, x: f32, y: f32) -> usize {
        let (gx, gy) = self.grid_coords(x, y);
        gy as usize * self.cols + gx as usize
    }

    fn grid_coords(&self, x: f32, y: f32) -> (i32, i32) {
        let gx = ((x + self.width * 0.5) / self.cell_size)
            .floor()
            .clamp(0.0, self.cols as f32 - 1.0) as i32;
        let gy = ((y + self.height * 0.5) / self.cell_size)
            .floor()
            .clamp(0.0, self.rows as f32 - 1.0) as i32;
        (gx, gy)
    }
}

#[derive(Resource, Default)]
pub struct FrameStats {
    pub sim_time: Duration,
    pub upload_time: Duration,
}

pub fn species_color(species: u32, viability_ratio: f32) -> [f32; 4] {
    const PALETTE: [[f32; 3]; 12] = [
        [0.82, 0.90, 0.98],
        [0.97, 0.84, 0.90],
        [0.84, 0.96, 0.87],
        [0.98, 0.93, 0.81],
        [0.90, 0.84, 0.98],
        [0.81, 0.95, 0.95],
        [0.98, 0.86, 0.81],
        [0.90, 0.97, 0.82],
        [0.84, 0.88, 0.98],
        [0.98, 0.85, 0.96],
        [0.93, 0.98, 0.84],
        [0.83, 0.98, 0.91],
    ];
    let tint = PALETTE[species as usize % PALETTE.len()];
    let brightness = 0.92 + viability_ratio.clamp(0.0, 1.0) * 0.08;
    [
        tint[0] * brightness,
        tint[1] * brightness,
        tint[2] * brightness,
        1.0,
    ]
}

pub fn aggression_spectrum_color(aggressiveness: f32) -> [f32; 3] {
    let aggression = (aggressiveness / CELL_AGGRESSIVENESS_DISPLAY_MAX).clamp(0.0, 1.0);
    if aggression < 0.5 {
        let t = aggression * 2.0;
        [0.50 + 0.45 * t, 0.96 - 0.10 * t, 0.56 * (1.0 - t)]
    } else {
        let t = (aggression - 0.5) * 2.0;
        [0.95 + 0.05 * t, 0.86 * (1.0 - t) + 0.28 * t, 0.0]
    }
}

pub fn cell_display_color(
    species: u32,
    viability_ratio: f32,
    aggressiveness: f32,
    lysis: f32,
) -> [f32; 4] {
    let active_predator_color = lysis >= LYSIS_ACTIVE_THRESHOLD && aggressiveness > 0.01;
    let color_driver = if active_predator_color {
        aggressiveness
    } else {
        0.0
    };
    let spectrum = aggression_spectrum_color(color_driver);
    let tint_variation = (species as f32 * 0.618_034).fract();
    let tint_strength = 0.70 + tint_variation * 0.20;
    let base = [0.98, 1.0, 0.985];
    let vitality = viability_ratio.clamp(0.0, 1.0);
    let brightness = 0.90 + vitality * 0.10;
    let saturation = 0.25 + vitality * 0.75;
    let tinted = [
        (base[0] * (1.0 - tint_strength) + spectrum[0] * tint_strength) * brightness,
        (base[1] * (1.0 - tint_strength) + spectrum[1] * tint_strength) * brightness,
        (base[2] * (1.0 - tint_strength) + spectrum[2] * tint_strength) * brightness,
    ];
    let gray = (tinted[0] * 0.299 + tinted[1] * 0.587 + tinted[2] * 0.114) * 0.94;
    [
        gray * (1.0 - saturation) + tinted[0] * saturation,
        gray * (1.0 - saturation) + tinted[1] * saturation,
        gray * (1.0 - saturation) + tinted[2] * saturation,
        1.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_soft_body(
        base_radii: [f32; SOFT_BODY_POINTS],
        angle_offsets: [f32; SOFT_BODY_POINTS],
    ) -> SoftBodyCell {
        SoftBodyCell {
            speed: 60.0,
            energy: 50.0,
            agility: 3.0,
            perception: 500.0,
            persistence: 50.0,
            mutation_factor: 50.0,
            size: 10.0,
            base_radii,
            current_radii: base_radii,
            angle_offsets,
        }
    }

    fn set_test_cell_soft_radius(world: &mut WorldState, index: usize, radius: f32) {
        world.cells.radius[index] = radius;
        world.cells.core_radius[index] = radius * CORE_RADIUS_FACTOR;
        world.cells.base_radii[index] = [radius; SOFT_BODY_POINTS];
        world.cells.current_radii[index] = [radius; SOFT_BODY_POINTS];
        world.cells.visual_radii[index] = [radius; SOFT_BODY_POINTS];
        world.cells.angle_offsets[index] = [0.0; SOFT_BODY_POINTS];
        world.cells.rebuild_soft_body_cache(index);
    }

    #[test]
    fn simulator_10k_is_the_default_balance_profile() {
        let config = SimConfig::default();
        assert_eq!(config.cells, 10_000);
        assert_eq!(config.food, 3_000);
        assert_eq!((config.width, config.height), (18_000.0, 10_000.0));
        assert_eq!(config.obstacles, 30);
        assert_eq!(config.food_growers, 6);
        assert!(config.segmented_cells);
    }

    #[test]
    fn simulator_population_starts_inside_balanced_gene_core() {
        let world = WorldState::new(&SimConfig {
            cells: 1_024,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });

        assert!(
            world
                .cells
                .speed
                .iter()
                .all(|value| (42.0..72.0).contains(value))
        );
        assert!(
            world
                .cells
                .turn_speed
                .iter()
                .all(|value| (0.85..2.35).contains(value))
        );
        assert!(
            world
                .cells
                .perception
                .iter()
                .all(|value| (260.0..520.0).contains(value))
        );
        assert!(
            world
                .cells
                .mutation_susceptibility
                .iter()
                .all(|value| (18.0..45.0).contains(value))
        );
        assert!(
            world
                .cells
                .division_threshold
                .iter()
                .all(|value| (78.0..90.0).contains(value))
        );
    }

    #[test]
    fn simulator_10k_primary_production_matches_metabolic_demand() {
        let world = WorldState::new(&SimConfig::default());
        let demand = (0..world.cells.len())
            .map(|index| world.cell_metabolic_drain_rate(index))
            .sum::<f32>();
        let feeder_production = world
            .food_growers
            .interval
            .iter()
            .map(|interval| {
                FOOD_GROWER_BATCH_SIZE as f32 * FEEDER_FOOD_ENERGY / interval.max(0.001)
            })
            .sum::<f32>();
        let average_wild_regrow =
            WILD_GRASS_REGROW_MIN + WILD_GRASS_REGROW_SPREAD * 2.0 / std::f32::consts::PI;
        let wild_production = world
            .food
            .source
            .iter()
            .filter(|source| **source == FoodSource::Wild)
            .count() as f32
            * WORLD_GRASS_ENERGY
            / average_wild_regrow;
        let production = feeder_production + wild_production;
        let ratio = production / demand.max(0.001);

        assert!(
            (1.02..=1.20).contains(&ratio),
            "10k energy ratio must stay near equilibrium: production={production:.2}, demand={demand:.2}, ratio={ratio:.3}"
        );
    }

    fn complete_test_mitosis(world: &mut WorldState) {
        world.process_cell_lifecycle();
        assert!(world.cells.mitosis_progress[0] > 0.0);
        world.advance_mitosis(MITOSIS_DURATION);
    }

    #[test]
    fn keeps_cells_inside_arena() {
        let config = SimConfig {
            cells: 1_000,
            food: 200,
            ..default()
        };
        let mut world = WorldState::new(&config);

        for _ in 0..180 {
            world.update(1.0 / 60.0);
        }

        for i in 0..world.cells.len() {
            for section in 0..world.cells.section_count[i] {
                let point = world.cells.section_center(i, section);
                let radius = world.cells.section_collision_radius(i, section);
                assert!(point.x >= -world.width * 0.5 + radius);
                assert!(point.x <= world.width * 0.5 - radius);
                assert!(point.y >= -world.height * 0.5 + radius);
                assert!(point.y <= world.height * 0.5 - radius);
            }
        }
    }

    #[test]
    fn circle_arena_keeps_spawned_objects_inside() {
        let config = SimConfig {
            cells: 220,
            food: 120,
            obstacles: 16,
            food_growers: 4,
            width: 2_400.0,
            height: 2_400.0,
            arena_shape: ArenaShape::Circle,
            ..default()
        };
        let world = WorldState::new(&config);

        for i in 0..world.cells.len() {
            for section in 0..world.cells.section_count[i] {
                assert!(point_inside_arena(
                    world.cells.section_center(i, section),
                    world.width,
                    world.height,
                    world.arena_shape,
                    world.cells.section_collision_radius(i, section),
                ));
            }
        }
        for i in 0..world.food.len() {
            let point = Vec2::new(world.food.x[i], world.food.y[i]);
            assert!(point_inside_arena(
                point,
                world.width,
                world.height,
                world.arena_shape,
                FOOD_RADIUS,
            ));
        }
        for i in 0..world.obstacles.len() {
            let point = Vec2::new(world.obstacles.x[i], world.obstacles.y[i]);
            assert!(point_inside_arena(
                point,
                world.width,
                world.height,
                world.arena_shape,
                world.obstacles.radius[i],
            ));
        }
        for i in 0..world.food_growers.len() {
            let point = Vec2::new(world.food_growers.x[i], world.food_growers.y[i]);
            assert!(point_inside_arena(
                point,
                world.width,
                world.height,
                world.arena_shape,
                world.food_growers.extent_radius(i),
            ));
        }
    }

    #[test]
    fn circle_arena_boundary_clamps_cells_food_obstacles_and_growers() {
        let config = SimConfig {
            cells: 1,
            food: 1,
            obstacles: 1,
            food_growers: 2,
            width: 1_200.0,
            height: 1_200.0,
            arena_shape: ArenaShape::Circle,
            ..default()
        };
        let mut world = WorldState::new(&config);

        world.cells.x[0] = 900.0;
        world.cells.y[0] = 0.0;
        world.cells.vx[0] = 100.0;
        world.bounce_cell(0);
        assert!(point_inside_arena(
            Vec2::new(world.cells.x[0], world.cells.y[0]),
            world.width,
            world.height,
            world.arena_shape,
            world.cells.collision_bound_radius(0),
        ));

        world.food.x[0] = 900.0;
        world.food.y[0] = 0.0;
        world.food.feeder[0] = -1;
        world.advect_food(0.0);
        assert!(point_inside_arena(
            Vec2::new(world.food.x[0], world.food.y[0]),
            world.width,
            world.height,
            world.arena_shape,
            FOOD_RADIUS,
        ));

        world.obstacles.x[0] = 900.0;
        world.obstacles.y[0] = 0.0;
        world.obstacles.vx[0] = 80.0;
        world.advect_obstacles(0.0);
        assert!(point_inside_arena(
            Vec2::new(world.obstacles.x[0], world.obstacles.y[0]),
            world.width,
            world.height,
            world.arena_shape,
            world.obstacles.radius[0],
        ));

        world.food_growers.x[0] = 900.0;
        world.food_growers.y[0] = 0.0;
        world.food_growers.vx[0] = 80.0;
        world.advect_food_growers(0.0);
        assert!(point_inside_arena(
            Vec2::new(world.food_growers.x[0], world.food_growers.y[0]),
            world.width,
            world.height,
            world.arena_shape,
            world.food_growers.extent_radius(0),
        ));
    }

    #[test]
    fn food_count_stays_within_capacity() {
        let config = SimConfig {
            cells: 500,
            food: 50,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        let initial_food = world.food.len();
        assert!(world.food.kind.iter().all(|kind| *kind == FoodKind::Grass));
        let max_food = config.food + world.food_growers.len() * 80;

        for _ in 0..120 {
            world.update(1.0 / 60.0);
        }

        assert!(world.food.len() >= initial_food);
        assert!(world.food.len() <= max_food);
    }

    #[test]
    fn initial_and_grown_food_is_grass_only() {
        let config = SimConfig {
            cells: 0,
            food: 12,
            ..default()
        };
        let mut world = WorldState::new(&config);

        assert!(world.food.kind.iter().all(|kind| *kind == FoodKind::Grass));
        let wild_index = world
            .food
            .source
            .iter()
            .position(|source| *source == FoodSource::Wild)
            .expect("wild grass slot");
        world.food.deactivate(wild_index);
        world.food.regrow_timer[wild_index] = 0.0;
        world.grow_wild_food(0.0);

        assert!(world.food.active[wild_index]);
        assert_eq!(world.food.kind[wild_index], FoodKind::Grass);
    }

    #[test]
    fn resting_cell_always_spends_viability() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        world.cells.vx[0] = 0.0;
        world.cells.vy[0] = 0.0;
        let before = world.cells.viability[0];

        world.decay_viability(1.0);

        let spent = before - world.cells.viability[0];
        assert!(spent >= VIABILITY_DECAY_BASE * 0.75);
    }

    #[test]
    fn aggressiveness_controls_whether_lysis_seeks_prey() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.x[1] = 40.0;
        world.cells.y[1] = 0.0;
        world.cells.perception[0] = 300.0;
        world.cells.lysis[0] = 70.0;
        world.cells.aggressiveness[0] = 100.0;
        world.cell_grid.rebuild(&world.cells);

        assert_eq!(world.best_lysis_target(0).map(|target| target.0), Some(1));
        world.cells.aggressiveness[0] = 0.0;
        assert!(world.best_lysis_target(0).is_none());
    }

    #[test]
    fn aggressiveness_trades_grass_efficiency_for_meat_efficiency() {
        let raw_energy = 8.0;
        let biotroph_grass = digested_food_energy(FoodKind::Grass, raw_energy, 0.0);
        let biotroph_meat = digested_food_energy(FoodKind::Meat, raw_energy, 0.0);
        let hemi_grass = digested_food_energy(
            FoodKind::Grass,
            raw_energy,
            CELL_AGGRESSIVENESS_DISPLAY_MAX * 0.5,
        );
        let hemi_meat = digested_food_energy(
            FoodKind::Meat,
            raw_energy,
            CELL_AGGRESSIVENESS_DISPLAY_MAX * 0.5,
        );
        let necro_grass =
            digested_food_energy(FoodKind::Grass, raw_energy, CELL_AGGRESSIVENESS_DISPLAY_MAX);
        let necro_meat =
            digested_food_energy(FoodKind::Meat, raw_energy, CELL_AGGRESSIVENESS_DISPLAY_MAX);

        assert!(biotroph_grass > hemi_grass && hemi_grass > necro_grass);
        assert!(biotroph_meat < hemi_meat && hemi_meat < necro_meat);
        assert!((biotroph_grass - raw_energy * 1.25).abs() < 0.001);
        assert!((necro_grass - raw_energy * 0.30).abs() < 0.001);
        assert!((necro_meat - raw_energy * 4.0).abs() < 0.001);
    }

    #[test]
    fn aggression_tints_only_lysis_capable_cells() {
        fn chroma(color: [f32; 4]) -> f32 {
            let min = color[0].min(color[1]).min(color[2]);
            let max = color[0].max(color[1]).max(color[2]);
            max - min
        }

        let default_green = cell_display_color(0, 1.0, 0.0, 0.0);
        let aggressive_without_lysis =
            cell_display_color(0, 1.0, CELL_AGGRESSIVENESS_DISPLAY_MAX, 0.0);
        let yellow = cell_display_color(
            4,
            1.0,
            CELL_AGGRESSIVENESS_DISPLAY_MAX * 0.5,
            CELL_LYSIS_DISPLAY_MAX,
        );
        let red = cell_display_color(
            8,
            1.0,
            CELL_AGGRESSIVENESS_DISPLAY_MAX,
            CELL_LYSIS_DISPLAY_MAX,
        );
        let fading_red = cell_display_color(
            8,
            0.0,
            CELL_AGGRESSIVENESS_DISPLAY_MAX,
            CELL_LYSIS_DISPLAY_MAX,
        );

        assert_eq!(default_green, aggressive_without_lysis);
        assert!(default_green[1] > default_green[0]);
        assert!(yellow[0] > default_green[0]);
        assert!(yellow[1] > red[1]);
        assert!(red[0] > red[1]);
        assert!(chroma(fading_red) < chroma(red));
        assert!(chroma(fading_red) > chroma(red) * 0.20);
    }

    #[test]
    fn larger_predators_deal_more_lysis_damage_to_smaller_prey() {
        let large_to_small = lysis_size_damage_multiplier(160.0, 40.0);
        let equal_size = lysis_size_damage_multiplier(80.0, 80.0);
        let small_to_large = lysis_size_damage_multiplier(40.0, 160.0);

        assert!(large_to_small > equal_size);
        assert!(small_to_large < equal_size);
        assert!((equal_size - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn contact_lysis_damages_both_cells_and_respects_cooldown() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        set_test_cell_soft_radius(&mut world, 1, 8.0);
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.x[1] = 14.0;
        world.cells.y[1] = 0.0;
        world.cells.heading[0] = 0.0;
        world.cells.heading[1] = 0.0;
        world.cells.viability[0] = 50.0;
        world.cells.viability[1] = 50.0;
        world.cells.lysis[0] = 100.0;

        assert!(world.try_lysis_attack(0, 1));
        let attacker_after = world.cells.viability[0];
        let victim_after = world.cells.viability[1];
        assert!(50.0 - victim_after > 50.0 - attacker_after);
        assert!(world.cells.lysis_cooldown[0] > 0.0);
        assert_eq!(world.visual_particles.len(), LYSIS_PARTICLES_PER_HIT);
        world.cells.lysis_deform_time[0][0] *= 0.55;
        world.cells.lysis_deform_time[1][0] *= 0.55;
        let attacker_radii = world.cells.lysis_visual_radii(0, 0);
        let victim_radii = world.cells.lysis_visual_radii(1, 0);
        assert!(attacker_radii[0] > world.cells.visual_radii[0][0]);
        assert!(victim_radii[4] < world.cells.visual_radii[1][4]);
        assert!(victim_radii[4] >= world.cells.core_radius[1]);
        assert!(!world.try_lysis_attack(0, 1));
        assert_eq!(world.cells.viability[0], attacker_after);
        assert_eq!(world.cells.viability[1], victim_after);
    }

    #[test]
    fn developed_lysis_attacks_faster_but_at_shorter_range() {
        let weak = lysis_combat_profile(20.0);
        let strong = lysis_combat_profile(100.0);
        assert!(strong.2 < weak.2);
        assert!(strong.3 < weak.3);

        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        set_test_cell_soft_radius(&mut world, 1, 8.0);
        world.cells.x[0] = 0.0;
        world.cells.x[1] = 19.0;
        world.cells.y[0] = 0.0;
        world.cells.y[1] = 0.0;
        world.cells.lysis[0] = 20.0;
        assert!(world.try_lysis_attack(0, 1));

        world.cells.lysis_cooldown[0] = 0.0;
        world.cells.lysis[0] = 100.0;
        assert!(!world.try_lysis_attack(0, 1));
    }

    #[test]
    fn size_gene_has_a_visibly_exaggerated_physical_range() {
        assert!(CELL_SIZE_GENE_MAX / CELL_SIZE_GENE_MIN >= 2.5);
        let world = WorldState::new(&SimConfig {
            cells: 2_048,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        let min = world
            .cells
            .radius
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);
        let max = world.cells.radius.iter().copied().fold(0.0, f32::max);
        assert!(max / min > 2.4);
    }

    #[test]
    fn energy_flow_reports_total_resting_metabolism_per_second() {
        let mut world = WorldState::new(&SimConfig {
            cells: 32,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        let expected = (0..world.cells.len())
            .map(|index| world.cell_metabolic_drain_rate(index))
            .sum::<f32>();

        world.decay_viability(0.5);
        world.finish_energy_flow_window(0.5);

        assert!((world.energy_flow.metabolism - expected).abs() < 0.01);
        assert_eq!(world.energy_flow.external_input(), 0.0);
    }

    #[test]
    fn food_uses_multiple_shapes() {
        let config = SimConfig {
            cells: 0,
            food: 40,
            ..default()
        };
        let world = WorldState::new(&config);
        let first = world.food.shape[0];

        assert!(world.food.shape.iter().any(|shape| *shape != first));
    }

    #[test]
    fn soft_body_radii_start_with_eight_clamped_points() {
        let config = SimConfig {
            cells: 16,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let world = WorldState::new(&config);

        for cell_index in 0..world.cells.len() {
            let size = world.cells.radius[cell_index];
            for ray_index in 0..SOFT_BODY_POINTS {
                let base = world.cells.base_radii[cell_index][ray_index];
                let current = world.cells.current_radii[cell_index][ray_index];
                let offset = world.cells.angle_offsets[cell_index][ray_index];
                assert!((size * SOFT_BODY_BASE_MIN_FACTOR..=size).contains(&base));
                assert_eq!(base, current);
                assert!(
                    (-SOFT_BODY_MAX_ANGLE_OFFSET..=SOFT_BODY_MAX_ANGLE_OFFSET).contains(&offset)
                );
            }
        }
    }

    #[test]
    fn soft_body_relaxes_faster_with_high_energy() {
        let config = SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        for index in 0..2 {
            set_test_cell_soft_radius(&mut world, index, 8.0);
            world.cells.current_radii[index] = [4.0; SOFT_BODY_POINTS];
            world.cells.max_viability[index] = 100.0;
        }
        world.cells.viability[0] = 100.0;
        world.cells.viability[1] = 10.0;

        world.cells.relax_soft_body(1.0 / 60.0);

        assert!(world.cells.current_radii[0][0] > world.cells.current_radii[1][0]);
    }

    #[test]
    fn soft_body_visual_radii_follow_physics_without_snapping() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        world.cells.current_radii[0] = [4.0; SOFT_BODY_POINTS];
        world.cells.visual_radii[0] = [8.0; SOFT_BODY_POINTS];
        world.cells.viability[0] = 100.0;

        world.cells.relax_soft_body(1.0 / 60.0);

        assert!(world.cells.visual_radii[0][0] < 8.0);
        assert!(world.cells.visual_radii[0][0] > world.cells.current_radii[0][0]);
    }

    #[test]
    fn obstacle_collision_compresses_nearest_soft_ray() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 1,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        world.obstacles.x[0] = 0.0;
        world.obstacles.y[0] = 0.0;
        world.obstacles.radius[0] = 80.0;
        world.cells.x[0] = 85.0;
        world.cells.y[0] = 0.0;
        world.cells.vx[0] = -20.0;
        world.cells.vy[0] = 0.0;
        world.max_obstacle_radius = 80.0;
        world
            .obstacle_grid
            .rebuild_points(&world.obstacles.x, &world.obstacles.y);

        let cell_bound = world.cells.collision_bound_radius(0);
        world.resolve_cell_obstacles(0, cell_bound);

        assert!(
            world.cells.current_radii[0]
                .iter()
                .any(|radius| *radius < 8.0)
        );
        assert!(world.cells.vx[0] > -20.0);
    }

    #[test]
    fn cell_pair_collision_compresses_both_soft_rays() {
        let config = SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        set_test_cell_soft_radius(&mut world, 1, 8.0);
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.x[1] = 14.0;
        world.cells.y[1] = 0.0;
        world.cells.vx[0] = 10.0;
        world.cells.vx[1] = -10.0;

        world.solve_cell_collisions(1.0 / 60.0);

        let ray_a = world.cells.soft_ray_index_for_direction(0, Vec2::X);
        let ray_b = world.cells.soft_ray_index_for_direction(1, -Vec2::X);
        assert!(world.cells.current_radii[0][ray_a] < 8.0);
        assert!(world.cells.current_radii[1][ray_b] < 8.0);
    }

    #[test]
    fn cell_core_radius_is_thirty_percent_of_size() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 10.0);

        assert!((world.cells.core_radius[0] - 3.0).abs() < 0.0001);
    }

    #[test]
    fn shape_analyzer_recognizes_reference_profiles() {
        let cases = [
            (CellShapeClass::Coccus, test_soft_body([8.0; 8], [0.0; 8])),
            (
                CellShapeClass::Bacillus,
                test_soft_body([10.0, 5.0, 4.5, 5.0, 10.0, 5.0, 4.5, 5.0], [0.0; 8]),
            ),
            (
                CellShapeClass::Filament,
                test_soft_body([10.0, 3.0, 3.0, 3.0, 10.0, 3.0, 3.0, 3.0], [0.0; 8]),
            ),
            (
                CellShapeClass::Spirillum,
                test_soft_body(
                    [10.0, 3.0, 3.0, 3.0, 10.0, 3.0, 3.0, 3.0],
                    [0.08, -0.08, 0.08, -0.08, 0.08, -0.08, 0.08, -0.08],
                ),
            ),
            (
                CellShapeClass::Vibrio,
                test_soft_body(
                    [9.0, 9.0, 6.0, 4.0, 4.0, 4.0, 6.0, 9.0],
                    [0.0, -0.10, 0.0, 0.10, 0.0, -0.10, 0.0, 0.10],
                ),
            ),
            (
                CellShapeClass::Diplococcus,
                test_soft_body([10.0, 9.0, 3.0, 9.0, 10.0, 9.0, 3.0, 9.0], [0.0; 8]),
            ),
            (
                CellShapeClass::Fusiform,
                test_soft_body(
                    [10.0, 6.0, 5.0, 6.0, 10.0, 6.0, 5.0, 6.0],
                    [0.0, 0.12, 0.0, -0.12, 0.0, 0.12, 0.0, -0.12],
                ),
            ),
            (
                CellShapeClass::Cuboid,
                test_soft_body([6.0, 8.4, 6.0, 8.4, 6.0, 8.4, 6.0, 8.4], [0.0; 8]),
            ),
            (
                CellShapeClass::Triquetrum,
                test_soft_body(
                    [10.0, 7.2, 5.8, 9.5, 7.0, 9.5, 5.8, 7.2],
                    [
                        0.0,
                        0.0,
                        0.0,
                        -SOFT_BODY_MAX_ANGLE_OFFSET,
                        0.0,
                        SOFT_BODY_MAX_ANGLE_OFFSET,
                        0.0,
                        0.0,
                    ],
                ),
            ),
            (
                CellShapeClass::Stauromorph,
                test_soft_body([10.0, 4.2, 10.0, 4.2, 10.0, 4.2, 10.0, 4.2], [0.0; 8]),
            ),
            (
                CellShapeClass::Lancetiform,
                test_soft_body(
                    [10.0, 6.2, 3.1, 6.2, 10.0, 6.2, 3.1, 6.2],
                    [0.0, 0.15, 0.0, -0.15, 0.0, 0.15, 0.0, -0.15],
                ),
            ),
            (
                CellShapeClass::Placoid,
                test_soft_body([10.0, 9.4, 7.6, 5.8, 5.4, 5.8, 7.6, 9.4], [0.0; 8]),
            ),
            (
                CellShapeClass::Lobatum,
                test_soft_body(
                    [9.5, 4.0, 7.5, 5.0, 6.0, 9.0, 4.5, 8.0],
                    [0.14, -0.03, 0.09, 0.02, -0.13, 0.05, -0.08, 0.11],
                ),
            ),
        ];

        for (expected, cell) in cases {
            assert_eq!(
                analyze_cell_shape_class(&cell),
                expected,
                "profile should be recognized as {}",
                expected.label_ru()
            );
        }
    }

    #[test]
    fn seed_shapes_keep_identity_with_individual_variation() {
        let cases = [
            (SeedGeometryMode::Uniform, CellShapeClass::Coccus),
            (SeedGeometryMode::AxialStretch, CellShapeClass::Bacillus),
            (SeedGeometryMode::ExtremeAxis, CellShapeClass::Filament),
            (SeedGeometryMode::AlternatingBend, CellShapeClass::Spirillum),
            (SeedGeometryMode::OneSidedCurve, CellShapeClass::Vibrio),
            (SeedGeometryMode::CenterWaist, CellShapeClass::Diplococcus),
            (SeedGeometryMode::AxisPinch, CellShapeClass::Fusiform),
            (SeedGeometryMode::DiagonalExpansion, CellShapeClass::Cuboid),
            (SeedGeometryMode::Triangular, CellShapeClass::Triquetrum),
            (SeedGeometryMode::Cruciform, CellShapeClass::Stauromorph),
            (SeedGeometryMode::Lancet, CellShapeClass::Lancetiform),
            (SeedGeometryMode::PlacoidShield, CellShapeClass::Placoid),
            (SeedGeometryMode::Chaotic, CellShapeClass::Lobatum),
        ];
        let mut rng = SmallRng::seed_from_u64(0x0B6A_2026);

        for (mode, expected) in cases {
            let (first_radii, _) =
                random_seed_shape(10.0, Some(mode), &DEFAULT_CELL_SHAPE_WEIGHTS, &mut rng);
            let mut differs_from_first = false;

            for _ in 0..8 {
                let (radii, offsets) =
                    random_seed_shape(10.0, Some(mode), &DEFAULT_CELL_SHAPE_WEIGHTS, &mut rng);
                differs_from_first |= radii
                    .iter()
                    .zip(first_radii)
                    .any(|(radius, first)| (radius - first).abs() > 0.05);
                assert_eq!(
                    analyze_cell_shape_class(&test_soft_body(radii, offsets)),
                    expected,
                    "mode profile radii={radii:?}, offsets={offsets:?}"
                );
            }

            assert!(differs_from_first);
        }
    }

    #[test]
    fn shape_weight_edit_keeps_total_at_one_hundred() {
        let mut config = SimConfig::default();
        config.set_cell_shape_weight(0, 42.0);
        assert!((config.cell_shape_weights.iter().sum::<f32>() - 100.0).abs() < 0.001);
        assert!((config.cell_shape_weights[0] - 42.0).abs() < 0.001);
    }

    #[test]
    fn taxonomy_keeps_distinct_shape_classes_in_separate_species() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });

        world.cells.section_count[0] = 1;
        world.cells.section_count[1] = 1;
        world.cells.aggressiveness[0] = 20.0;
        world.cells.aggressiveness[1] = 20.0;
        world.cells.angle_offsets[0] = [0.0; SOFT_BODY_POINTS];
        world.cells.angle_offsets[1] = [0.0; SOFT_BODY_POINTS];
        world.cells.base_radii[0] = [10.0, 3.0, 3.0, 3.0, 10.0, 3.0, 3.0, 3.0];
        world.cells.base_radii[1] = [6.0, 8.4, 6.0, 8.4, 6.0, 8.4, 6.0, 8.4];
        world.cells.rebuild_soft_body_cache(0);
        world.cells.rebuild_soft_body_cache(1);
        world.cells.refresh_taxonomy();

        assert_eq!(
            analyze_cell_shape_class(&world.cells.soft_body_profile(0)),
            CellShapeClass::Filament
        );
        assert_eq!(
            analyze_cell_shape_class(&world.cells.soft_body_profile(1)),
            CellShapeClass::Cuboid
        );
        assert_ne!(world.cells.species[0], world.cells.species[1]);
    }

    #[test]
    fn taxonomy_uses_core_genes_not_only_shape() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });

        for index in 0..2 {
            world.cells.section_count[index] = 1;
            world.cells.base_radii[index] = [8.0; SOFT_BODY_POINTS];
            world.cells.angle_offsets[index] = [0.0; SOFT_BODY_POINTS];
        }
        world.cells.speed[0] = SPEED_GENE_MIN;
        world.cells.speed[1] = SPEED_GENE_MAX;
        world.cells.perception[0] = PERCEPTION_GENE_MIN;
        world.cells.perception[1] = PERCEPTION_GENE_MAX;
        world.cells.aggressiveness[0] = 5.0;
        world.cells.aggressiveness[1] = 95.0;
        world.cells.lysis[0] = 0.0;
        world.cells.lysis[1] = CELL_LYSIS_DISPLAY_MAX;
        world.cells.refresh_taxonomy();

        assert_eq!(
            analyze_cell_shape_class(&world.cells.soft_body_profile(0)),
            CellShapeClass::Coccus
        );
        assert_eq!(
            analyze_cell_shape_class(&world.cells.soft_body_profile(1)),
            CellShapeClass::Coccus
        );
        assert_ne!(world.cells.species[0], world.cells.species[1]);
    }

    #[test]
    fn taxonomy_splits_trophic_bands_without_splitting_genus() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });

        for index in 0..2 {
            world.cells.section_count[index] = 1;
            world.cells.base_radii[index] = [7.0; SOFT_BODY_POINTS];
            world.cells.angle_offsets[index] = [0.0; SOFT_BODY_POINTS];
            world.cells.speed[index] = 62.0;
            world.cells.turn_speed[index] = 2.0;
            world.cells.perception[index] = 420.0;
            world.cells.persistence[index] = 45.0;
            world.cells.lysis[index] = 0.0;
            world.cells.mutation_susceptibility[index] = 30.0;
            world.cells.rebuild_soft_body_cache(index);
        }
        world.cells.aggressiveness[0] = 35.0;
        world.cells.aggressiveness[1] = 45.0;
        world.cells.refresh_taxonomy();

        assert_ne!(world.cells.species[0], world.cells.species[1]);
        assert_eq!(
            world.cells.species[0] / SPECIES_EPITHET_SLOTS,
            world.cells.species[1] / SPECIES_EPITHET_SLOTS
        );
    }

    #[test]
    fn taxonomy_keeps_minor_gene_noise_inside_species() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });

        for index in 0..2 {
            world.cells.section_count[index] = 1;
            world.cells.base_radii[index] = [7.0; SOFT_BODY_POINTS];
            world.cells.angle_offsets[index] = [0.0; SOFT_BODY_POINTS];
            world.cells.lysis[index] = 0.0;
            world.cells.rebuild_soft_body_cache(index);
        }
        world.cells.speed[0] = 61.0;
        world.cells.speed[1] = 62.0;
        world.cells.turn_speed[0] = 2.00;
        world.cells.turn_speed[1] = 2.03;
        world.cells.perception[0] = 450.0;
        world.cells.perception[1] = 456.0;
        world.cells.persistence[0] = 35.0;
        world.cells.persistence[1] = 36.0;
        world.cells.aggressiveness[0] = 30.0;
        world.cells.aggressiveness[1] = 31.0;
        world.cells.mutation_susceptibility[0] = 30.0;
        world.cells.mutation_susceptibility[1] = 31.0;
        world.cells.refresh_taxonomy();

        assert_eq!(world.cells.species[0], world.cells.species[1]);
    }

    #[test]
    fn taxonomy_splits_same_coccus_shape_when_genes_diverge_hard() {
        let mut world = WorldState::new(&SimConfig {
            cells: 4,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });
        let profiles = [
            (5.6, 61.0, 2.2, 336.0, 60.0, 32.0, 0.0, 38.0),
            (5.2, 64.0, 2.0, 480.0, 31.0, 37.0, 0.0, 36.0),
            (7.1, 67.0, 1.9, 490.0, 55.0, 70.0, 57.0, 26.0),
            (4.3, 62.0, 1.2, 302.0, 32.0, 1.0, 0.0, 32.0),
        ];

        for (index, profile) in profiles.iter().copied().enumerate() {
            let (size, speed, turn, perception, persistence, aggression, lysis, mutation) = profile;
            world.cells.section_count[index] = 1;
            world.cells.base_radii[index] = [size; SOFT_BODY_POINTS];
            world.cells.angle_offsets[index] = [0.0; SOFT_BODY_POINTS];
            world.cells.speed[index] = speed;
            world.cells.turn_speed[index] = turn;
            world.cells.perception[index] = perception;
            world.cells.persistence[index] = persistence;
            world.cells.aggressiveness[index] = aggression;
            world.cells.lysis[index] = lysis;
            world.cells.mutation_susceptibility[index] = mutation;
            world.cells.rebuild_soft_body_cache(index);
        }
        world.cells.refresh_taxonomy();

        let species = world
            .cells
            .species
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(species.len(), profiles.len());
    }

    #[test]
    fn full_random_geometry_is_classified_after_generation() {
        let config = SimConfig {
            cells: 256,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            random_cell_geometry: true,
            ..default()
        };
        let world = WorldState::new(&config);
        let unique_names = (0..world.cells.len())
            .map(|index| world.cells.shape_name(index))
            .collect::<std::collections::HashSet<_>>();
        assert!(unique_names.len() >= 2);
    }

    #[test]
    fn initial_population_contains_every_analyzed_shape_without_stored_types() {
        let config = SimConfig {
            cells: 128,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let world = WorldState::new(&config);
        let mut found = [false; 13];
        let classes = [
            CellShapeClass::Coccus,
            CellShapeClass::Bacillus,
            CellShapeClass::Filament,
            CellShapeClass::Spirillum,
            CellShapeClass::Vibrio,
            CellShapeClass::Diplococcus,
            CellShapeClass::Fusiform,
            CellShapeClass::Cuboid,
            CellShapeClass::Triquetrum,
            CellShapeClass::Stauromorph,
            CellShapeClass::Lancetiform,
            CellShapeClass::Placoid,
            CellShapeClass::Lobatum,
        ];

        for index in 0..world.cells.len() {
            let class = analyze_cell_shape_class(&world.cells.soft_body_profile(index));
            found[classes
                .iter()
                .position(|candidate| *candidate == class)
                .unwrap()] = true;
        }

        assert!(found.into_iter().all(|present| present));
    }

    #[test]
    fn weighted_initial_geometry_produces_a_mixed_population() {
        let config = SimConfig {
            cells: 2_000,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            seed: 0x5EED_2026,
            ..default()
        };
        let world = WorldState::new(&config);
        let lobatum_count = (0..world.cells.len())
            .filter(|&index| {
                analyze_cell_shape_class(&world.cells.soft_body_profile(index))
                    == CellShapeClass::Lobatum
            })
            .count();

        assert!(lobatum_count < world.cells.len() / 3);
    }

    #[test]
    fn grouped_shape_mutations_follow_expected_distribution_and_bounds() {
        let mut rng = SmallRng::seed_from_u64(0x5A4E_2026);
        let mut counts = [0usize; 3];
        let mut mutations = 0usize;

        for _ in 0..20_000 {
            let mut cell = test_soft_body([8.0; 8], [0.0; 8]);
            cell.mutation_factor = 100.0;
            if let Some(event) = mutate_soft_body_shape(&mut cell, &mut rng) {
                mutations += 1;
                counts[match event {
                    ShapeMutationEvent::Single => 0,
                    ShapeMutationEvent::Axial => 1,
                    ShapeMutationEvent::Sector => 2,
                }] += 1;
            }
            assert!(cell.angle_offsets.iter().all(|offset| {
                (-SOFT_BODY_MAX_ANGLE_OFFSET..=SOFT_BODY_MAX_ANGLE_OFFSET).contains(offset)
            }));
            assert!(cell.base_radii.iter().all(|radius| {
                (cell.size * SOFT_BODY_BASE_MIN_FACTOR..=cell.size).contains(radius)
            }));
        }

        let ratios = counts.map(|count| count as f32 / mutations as f32);
        assert!((ratios[0] - 0.60).abs() < 0.025);
        assert!((ratios[1] - 0.30).abs() < 0.025);
        assert!((ratios[2] - 0.10).abs() < 0.02);
    }

    #[test]
    fn collision_compression_never_crosses_hard_core() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 10.0);
        world
            .cells
            .compress_rays_by_depth(0, &[100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0]);

        assert!(
            world.cells.current_radii[0]
                .iter()
                .all(|radius| (*radius - 3.0).abs() < 0.0001)
        );
    }

    #[test]
    fn overlapping_hard_cores_use_strong_repulsion() {
        let config = SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            collision_damping: 0.0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 10.0);
        set_test_cell_soft_radius(&mut world, 1, 10.0);
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.x[1] = 0.0;
        world.cells.y[1] = 0.0;
        world.cells.vx[0] = 0.0;
        world.cells.vy[0] = 0.0;
        world.cells.vx[1] = 0.0;
        world.cells.vy[1] = 0.0;

        world.solve_cell_collisions(1.0 / 60.0);

        let speed_a = Vec2::new(world.cells.vx[0], world.cells.vy[0]).length();
        let speed_b = Vec2::new(world.cells.vx[1], world.cells.vy[1]).length();
        assert!(speed_a > 4.0);
        assert!(speed_b > 4.0);
    }

    #[test]
    fn virtual_membrane_interpolates_between_neighboring_rays() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        world.cells.heading[0] = 0.0;
        world.cells.current_radii[0][0] = 4.0;
        world.cells.current_radii[0][1] = 8.0;

        let radius = world
            .cells
            .virtual_membrane_radius(0, std::f32::consts::FRAC_PI_8);

        assert!((radius - 6.0).abs() < 0.0001);
    }

    #[test]
    fn virtual_membrane_detects_contact_between_ray_directions() {
        let config = SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        set_test_cell_soft_radius(&mut world, 1, 8.0);
        world.cells.heading[0] = 0.0;
        world.cells.heading[1] = 0.0;
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.x[1] = 12.0;
        world.cells.y[1] = 5.0;
        let normal = Vec2::new(12.0, 5.0).normalize();

        let contact = sample_membrane_contact(&world.cells, 0, 1, normal);

        assert!(contact.count > 0);
        assert!(contact.depth_sum > 0.0);
    }

    #[test]
    fn collision_damping_does_not_pull_separating_cells_together() {
        let config = SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        set_test_cell_soft_radius(&mut world, 1, 8.0);
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.x[1] = 14.0;
        world.cells.y[1] = 0.0;
        world.cells.vx[0] = -100.0;
        world.cells.vx[1] = 100.0;

        world.solve_cell_collisions(1.0 / 60.0);

        assert_eq!(world.cells.vx[0], -100.0);
        assert_eq!(world.cells.vx[1], 100.0);
    }

    #[test]
    fn biomass_energy_cost_scales_with_base_radii_sum() {
        let config = SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.viability[0] = 100.0;
        world.cells.viability[1] = 100.0;
        world.cells.speed[0] = 60.0;
        world.cells.speed[1] = 60.0;
        world.cells.base_radii[0] = [4.0; SOFT_BODY_POINTS];
        world.cells.base_radii[1] = [8.0; SOFT_BODY_POINTS];
        world.cells.angle_offsets[0] = [0.0; SOFT_BODY_POINTS];
        world.cells.angle_offsets[1] = [0.0; SOFT_BODY_POINTS];
        world.cells.section_count[0] = 1;
        world.cells.section_count[1] = 1;
        world.cells.rebuild_soft_body_cache(0);
        world.cells.rebuild_soft_body_cache(1);
        world.cells.viability[0] = 100.0;
        world.cells.viability[1] = 100.0;

        world.decay_viability(1.0);

        assert!(world.cells.viability[1] < world.cells.viability[0]);
    }

    #[test]
    fn obstacles_and_food_growers_spawn() {
        let config = SimConfig {
            cells: 0,
            food: 0,
            obstacles: 7,
            food_growers: 3,
            ..default()
        };
        let world = WorldState::new(&config);

        assert_eq!(world.obstacles.len(), 7);
        assert_eq!(world.food_growers.len(), 3);
    }

    #[test]
    fn titanic_food_grower_is_guaranteed() {
        let config = SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            width: 24_000.0,
            height: 13_500.0,
            ..default()
        };
        let world = WorldState::new(&config);

        assert_eq!(world.food_growers.len(), 1);
        assert!(world.food_growers.radius[0] >= 300.0);
        assert!(world.food_growers.branch_count[0] >= 24);
        assert!(world.food_growers.x[0].abs() < 0.001);
        assert!(world.food_growers.y[0].abs() < 0.001);
    }

    #[test]
    fn titanic_food_grower_scales_with_arena() {
        let small = WorldState::new(&SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 2,
            width: 2_000.0,
            height: 2_000.0,
            ..default()
        });
        let large = WorldState::new(&SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 2,
            width: 30_000.0,
            height: 20_000.0,
            ..default()
        });

        assert!(large.food_growers.radius[0] > small.food_growers.radius[0]);
        assert!(large.food_growers.extent_radius(0) > small.food_growers.extent_radius(0));
        assert!(large.food_growers.radius[1] > small.food_growers.radius[1]);
    }

    #[test]
    fn small_food_growers_spawn_outside_the_mega_grower() {
        let world = WorldState::new(&SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 12,
            seed: 0xFEED_1234,
            ..default()
        });
        let mega = Vec2::new(world.food_growers.x[0], world.food_growers.y[0]);
        let mega_extent = world.food_growers.extent_radius(0);
        for index in 1..world.food_growers.len() {
            let position = Vec2::new(world.food_growers.x[index], world.food_growers.y[index]);
            assert!(
                position.distance(mega) >= mega_extent + world.food_growers.extent_radius(index),
                "grower {index} overlaps the central mega grower"
            );
        }
    }

    #[test]
    fn food_growers_spawn_food_over_time() {
        let config = SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 1,
            ..default()
        };
        let mut world = WorldState::new(&config);

        for _ in 0..240 {
            world.update(1.0 / 60.0);
        }

        assert!(world.food.len() > 0);
        assert!(world.food.kind.iter().all(|kind| *kind == FoodKind::Grass));
        assert!(world.food.feeder.iter().all(|feeder| *feeder >= 0));
    }

    #[test]
    fn default_food_is_mostly_on_food_growers() {
        let config = SimConfig {
            cells: 0,
            ..default()
        };
        let world = WorldState::new(&config);
        let feeder_food = world
            .food
            .feeder
            .iter()
            .filter(|feeder| **feeder >= 0)
            .count();
        let floor_food = world.food.len() - feeder_food;

        assert!(feeder_food > floor_food);
    }

    #[test]
    fn world_food_spawns_outside_solid_feeders() {
        let config = SimConfig {
            cells: 0,
            food: 180,
            obstacles: 6,
            food_growers: 4,
            width: 2_600.0,
            height: 2_000.0,
            ..default()
        };
        let world = WorldState::new(&config);

        for i in 0..world.food.len() {
            if world.food.is_feeder_food(i) {
                continue;
            }

            let point = Vec2::new(world.food.x[i], world.food.y[i]);
            assert!(!world.point_overlaps_solid(point, FOOD_RADIUS));
        }
    }

    #[test]
    fn feeder_food_sits_outside_branch_collision() {
        let config = SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 1,
            ..default()
        };
        let mut world = WorldState::new(&config);

        for _ in 0..240 {
            world.update(1.0 / 60.0);
        }

        assert!(world.food.len() > 0);
        for i in 0..world.food.len() {
            let point = Vec2::new(world.food.x[i], world.food.y[i]);
            if world.point_overlaps_solid(point, FOOD_RADIUS) {
                let grower_idx = world.food.feeder[i] as usize;
                let parent_branch = world.food.anchor_branch[i];
                println!(
                    "[DEBUG] Food index {} (parent branch: {}) overlaps solid!",
                    i, parent_branch
                );
                println!(
                    "  Grower center: x: {:.3}, y: {:.3}",
                    world.food_growers.x[grower_idx], world.food_growers.y[grower_idx]
                );
                println!("  Food pos: x: {:.3}, y: {:.3}", point.x, point.y);

                if parent_branch >= 0 {
                    let pb = parent_branch as usize;
                    println!(
                        "  Parent branch {} info: start: ({:.3}, {:.3}), end: ({:.3}, {:.3}), len: {:.3}",
                        pb,
                        world.food_growers.branch_start_x[pb],
                        world.food_growers.branch_start_y[pb],
                        world.food_growers.branch_end_x[pb],
                        world.food_growers.branch_end_y[pb],
                        world.food_growers.branch_length[pb]
                    );
                    let branch_t = world.food.anchor_distance[i]
                        / world.food_growers.branch_length[pb].max(1.0);
                    println!(
                        "    food anchor_distance: {:.3}, branch_t: {:.3}, anchor_lateral: {:.3}",
                        world.food.anchor_distance[i], branch_t, world.food.anchor_lateral[i]
                    );
                }

                for branch_index in world.food_growers.branch_range(grower_idx) {
                    if !world.food_growers.branch_has_collision(branch_index) {
                        continue;
                    }
                    let (closest, t) = world
                        .food_growers
                        .closest_point_on_branch(branch_index, point);
                    let min_dist = world
                        .food_growers
                        .branch_collision_width_at(branch_index, t)
                        + FOOD_RADIUS;
                    let dist = point.distance(closest);
                    if dist < min_dist {
                        println!(
                            "  Overlaps branch {}! dist: {:.3}, min_dist: {:.3} (width: {:.3}, t: {:.3})",
                            branch_index,
                            dist,
                            min_dist,
                            world
                                .food_growers
                                .branch_collision_width_at(branch_index, t),
                            t
                        );
                        println!(
                            "    Branch {} info: start: ({:.3}, {:.3}), end: ({:.3}, {:.3}), len: {:.3}",
                            branch_index,
                            world.food_growers.branch_start_x[branch_index],
                            world.food_growers.branch_start_y[branch_index],
                            world.food_growers.branch_end_x[branch_index],
                            world.food_growers.branch_end_y[branch_index],
                            world.food_growers.branch_length[branch_index]
                        );
                        println!(
                            "    Closest point on branch {}: ({:.3}, {:.3})",
                            branch_index, closest.x, closest.y
                        );
                    }
                }
                panic!("feeder food overlaps solid");
            }
        }
    }

    #[test]
    fn food_grower_solid_branches_are_half_and_not_adjacent() {
        let config = SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 4,
            ..default()
        };
        let world = WorldState::new(&config);

        for grower_index in 0..world.food_growers.len() {
            let branches: Vec<_> = world.food_growers.branch_range(grower_index).collect();
            let solid_count = branches
                .iter()
                .filter(|branch| world.food_growers.branch_has_collision(**branch))
                .count();
            assert_eq!(solid_count, branches.len() / 2);

            for local_index in 0..branches.len() {
                let current = branches[local_index];
                let next = branches[(local_index + 1) % branches.len()];
                assert!(
                    !(world.food_growers.branch_has_collision(current)
                        && world.food_growers.branch_has_collision(next))
                );
            }
        }
    }

    #[test]
    fn feeder_food_grows_on_decorative_side_stem_without_spin() {
        let config = SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 1,
            ..default()
        };
        let mut world = WorldState::new(&config);

        for _ in 0..240 {
            world.update(1.0 / 60.0);
        }

        let food_index = (0..world.food.len())
            .find(|index| world.food.is_feeder_food(*index))
            .expect("feeder food spawned");
        let (stem_start, stem_end) = world
            .feeder_food_stem_points(food_index)
            .expect("feeder food has decorative stem");
        let food_point = Vec2::new(world.food.x[food_index], world.food.y[food_index]);

        assert_eq!(world.food.spin[food_index], 0.0);
        assert!(stem_start.distance_squared(stem_end) > 4.0);
        assert!(food_point.distance(stem_end) <= FOOD_RADIUS + FEEDER_FOOD_SURFACE_GAP + 1.5);
        assert!(!world.point_overlaps_solid(food_point, FOOD_RADIUS));
    }

    #[test]
    fn obstacles_push_cells_out() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 1,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.obstacles.x[0] = 0.0;
        world.obstacles.y[0] = 0.0;
        world.obstacles.radius[0] = 80.0;
        world.cells.x[0] = 10.0;
        world.cells.y[0] = 0.0;
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        world.max_obstacle_radius = 80.0;
        world
            .obstacle_grid
            .rebuild_points(&world.obstacles.x, &world.obstacles.y);

        let before = Vec2::new(world.cells.x[0], world.cells.y[0]).length();
        let cell_bound = world.cells.collision_bound_radius(0);
        world.resolve_cell_obstacles(0, cell_bound);

        let dist = Vec2::new(world.cells.x[0], world.cells.y[0]).length();
        assert!(dist > before);
        assert!(dist - before <= SOFT_BODY_SOLID_PUSH_MAX + 0.001);
        assert!(
            world.cells.current_radii[0]
                .iter()
                .any(|radius| *radius < 8.0)
        );
    }

    #[test]
    fn food_growers_push_cells_out() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 1,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.food_growers.x[0] = 0.0;
        world.food_growers.y[0] = 0.0;
        world.food_growers.radius[0] = 80.0;
        world.cells.x[0] = 10.0;
        world.cells.y[0] = 0.0;
        set_test_cell_soft_radius(&mut world, 0, 8.0);

        let before = Vec2::new(world.cells.x[0], world.cells.y[0]).length();
        let cell_bound = world.cells.collision_bound_radius(0);
        world.resolve_cell_food_growers(0, cell_bound);

        let dist = Vec2::new(world.cells.x[0], world.cells.y[0]).length();
        assert!(dist > before);
        assert!(dist - before <= SOFT_BODY_SOLID_PUSH_MAX + 0.001);
        assert!(
            world.cells.current_radii[0]
                .iter()
                .any(|radius| *radius < 8.0)
        );
    }

    #[test]
    fn only_upper_food_grower_branches_block_cells() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 1,
            ..default()
        };
        let mut world = WorldState::new(&config);
        let branch = world.food_growers.branch_range(0).next().unwrap();
        for branch_index in world.food_growers.branch_range(0) {
            world.food_growers.branch_solid[branch_index] = false;
        }
        world.food_growers.branch_solid[branch] = false;
        world.food_growers.branch_curve[branch] = 0.0;
        world.food_growers.branch_start_x[branch] = 0.0;
        world.food_growers.branch_start_y[branch] = 0.0;
        world.food_growers.branch_end_x[branch] = 120.0;
        world.food_growers.branch_end_y[branch] = 0.0;
        world.food_growers.branch_width[branch] = 24.0;
        world.food_growers.x[0] = 10_000.0;
        world.food_growers.y[0] = 10_000.0;
        world.food_growers.radius[0] = 1.0;
        world.food_growers.extent[0] = 20_000.0;
        world.cells.x[0] = 60.0;
        world.cells.y[0] = 1.0;
        set_test_cell_soft_radius(&mut world, 0, 8.0);

        let cell_bound = world.cells.collision_bound_radius(0);
        world.resolve_cell_food_growers(0, cell_bound);
        assert!((world.cells.y[0] - 1.0).abs() < 0.001);

        world.food_growers.branch_solid[branch] = true;
        let cell_bound = world.cells.collision_bound_radius(0);
        world.resolve_cell_food_growers(0, cell_bound);
        assert!(world.cells.y[0].abs() > 1.0);
        assert!(world.cells.y[0].abs() <= 1.0 + SOFT_BODY_SOLID_PUSH_MAX + 0.001);
    }

    #[test]
    fn obstacles_collide_with_food_growers() {
        let config = SimConfig {
            cells: 0,
            food: 0,
            obstacles: 1,
            food_growers: 1,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.obstacles.x[0] = 10.0;
        world.obstacles.y[0] = 0.0;
        world.obstacles.radius[0] = 30.0;
        world.food_growers.x[0] = 0.0;
        world.food_growers.y[0] = 0.0;
        world.food_growers.radius[0] = 80.0;

        world.resolve_obstacle_food_growers();

        let dist = Vec2::new(
            world.obstacles.x[0] - world.food_growers.x[0],
            world.obstacles.y[0] - world.food_growers.y[0],
        )
        .length();
        assert!(dist >= 110.0);
    }

    #[test]
    fn zero_viability_cell_dies_into_meat() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        let initial_food = world.food.len();
        let recoverable = world.cells.biomass_sum(0)
            * CELL_STRUCTURE_ENERGY_PER_BIOMASS
            * DEATH_STRUCTURE_RECOVERY;
        world.cells.viability[0] = 0.0;

        world.remove_dead_cells();

        assert_eq!(world.cells.len(), 0);
        assert!(world.food.len() > initial_food);
        assert!(
            world.food.kind[initial_food..]
                .iter()
                .all(|kind| *kind == FoodKind::Meat)
        );
        assert!(
            world.food.source[initial_food..]
                .iter()
                .all(|source| *source == FoodSource::Carrion)
        );
        let carrion_energy = world.food.energy[initial_food..].iter().sum::<f32>();
        assert!(carrion_energy > 0.0);
        assert!(carrion_energy <= recoverable + 0.001);
    }

    #[test]
    fn full_viability_cell_does_not_eat() {
        let config = SimConfig {
            cells: 1,
            food: 1,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.viability[0] = world.cells.max_viability[0];
        world.cells.division_threshold[0] = 1_000.0;

        for i in 0..world.food.len() {
            world.food.active[i] = false;
        }
        world.food.active[0] = true;
        world.food.x[0] = 0.0;
        world.food.y[0] = 0.0;
        world.food.feeder[0] = -1;

        world.update(1.0 / 60.0);

        assert!(world.food.active[0]);
        assert!(world.cells.viability[0] <= world.cells.max_viability[0]);
    }

    #[test]
    fn cell_divides_at_threshold_and_splits_viability() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.viability[0] = 80.0;
        world.cells.division_threshold[0] = 80.0;
        world.cells.mutation_susceptibility[0] = 0.0;
        let construction_cost = (world.cells.biomass_sum(0) * CELL_STRUCTURE_ENERGY_PER_BIOMASS)
            .min(world.cells.viability[0] * 0.25);
        let expected_split = (80.0 - construction_cost) * 0.5;

        world.process_cell_lifecycle();
        assert_eq!(world.cells.len(), 1);
        assert!(world.cells.mitosis_progress[0] > 0.0);
        assert_eq!(world.visual_particles.len(), 4);
        world.advance_mitosis(MITOSIS_DURATION * 0.55);
        assert_eq!(world.cells.len(), 1);
        assert!(world.cells.mitosis_progress[0] > 0.5);
        assert_eq!(world.visual_particles.len(), 11);
        world.advance_mitosis(MITOSIS_DURATION * 0.45);

        assert_eq!(world.cells.len(), 2);
        assert_eq!(world.visual_particles.len(), 26);
        assert!((world.cells.viability[0] - expected_split).abs() < 0.001);
        assert!((world.cells.viability[1] - expected_split).abs() < 0.001);
    }

    #[test]
    fn child_genes_stay_in_allowed_ranges() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.viability[0] = 90.0;
        world.cells.division_threshold[0] = 80.0;
        world.cells.speed[0] = SPEED_GENE_MAX;
        world.cells.turn_speed[0] = TURN_GENE_MAX;
        world.cells.perception[0] = PERCEPTION_GENE_MAX;
        world.cells.mutation_susceptibility[0] = MUTATION_GENE_MAX;

        complete_test_mitosis(&mut world);
        let child = 1;

        assert!((SPEED_GENE_MIN..=SPEED_GENE_MAX).contains(&world.cells.speed[child]));
        assert!((TURN_GENE_MIN..=TURN_GENE_MAX).contains(&world.cells.turn_speed[child]));
        assert!(
            (PERCEPTION_GENE_MIN..=PERCEPTION_GENE_MAX).contains(&world.cells.perception[child])
        );
        assert!(
            (PERSISTENCE_GENE_MIN..=PERSISTENCE_GENE_MAX).contains(&world.cells.persistence[child])
        );
        assert!(
            (MUTATION_GENE_MIN..=MUTATION_GENE_MAX)
                .contains(&world.cells.mutation_susceptibility[child])
        );
        assert!(
            (DIVISION_THRESHOLD_MIN..=DIVISION_THRESHOLD_MAX)
                .contains(&world.cells.division_threshold[child])
        );
    }

    #[test]
    fn segmented_feature_flag_preserves_single_section_cells() {
        let world = WorldState::new(&SimConfig {
            cells: 256,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        assert!(world.cells.section_count.iter().all(|count| *count == 1));
    }

    #[test]
    fn segmented_population_contains_full_second_soft_bodies() {
        let world = WorldState::new(&SimConfig {
            cells: 512,
            food: 0,
            segmented_cells: true,
            seed: 42,
            ..default()
        });
        assert!(world.cells.section_count.iter().any(|count| *count == 2));
        assert_eq!(world.cells.tail_base_radii.len(), world.cells.len());
        assert_eq!(world.cells.tail_current_radii.len(), world.cells.len());
        assert_eq!(world.cells.tail_visual_radii.len(), world.cells.len());
    }

    #[test]
    fn segmented_sections_can_have_independent_shapes_and_sizes() {
        let world = WorldState::new(&SimConfig {
            cells: 2_000,
            food: 0,
            segmented_cells: true,
            seed: 137,
            ..default()
        });

        let varied_tail = world
            .cells
            .section_count
            .iter()
            .enumerate()
            .any(|(index, count)| {
                if *count < 2 {
                    return false;
                }
                let head_size = soft_body_max_radius(&world.cells.base_radii[index]).max(0.1);
                let tail_size = soft_body_max_radius(&world.cells.tail_base_radii[index]).max(0.1);
                let size_ratio = tail_size / head_size;
                let shape_delta = (0..SOFT_BODY_POINTS)
                    .map(|ray| {
                        (world.cells.base_radii[index][ray] / head_size
                            - world.cells.tail_base_radii[index][ray] / tail_size)
                            .abs()
                    })
                    .sum::<f32>()
                    / SOFT_BODY_POINTS as f32;
                !(0.92..=1.08).contains(&size_ratio) || shape_delta > 0.045
            });

        assert!(varied_tail);
        assert!(
            world
                .cells
                .section_count
                .iter()
                .enumerate()
                .any(|(index, count)| *count >= 3
                    && soft_body_max_radius(&world.cells.extra_sections[index][0].base_radii)
                        != soft_body_max_radius(&world.cells.tail_base_radii[index]))
        );
    }

    #[test]
    fn tail_section_size_is_not_clamped_to_head_size() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.section_count[0] = 2;
        world.cells.radius[0] = 8.0;
        world.cells.core_radius[0] = 8.0 * CORE_RADIUS_FACTOR;
        world.cells.base_radii[0] = [8.0; SOFT_BODY_POINTS];
        world.cells.current_radii[0] = [8.0; SOFT_BODY_POINTS];
        world.cells.tail_core_radius[0] = 13.0 * CORE_RADIUS_FACTOR;
        world.cells.tail_base_radii[0] = [13.0; SOFT_BODY_POINTS];
        world.cells.tail_current_radii[0] = [13.0; SOFT_BODY_POINTS];
        world.cells.tail_visual_radii[0] = [13.0; SOFT_BODY_POINTS];

        world.cells.relax_soft_body(1.0 / 60.0);

        assert!(world.cells.tail_base_radii[0][0] > world.cells.radius[0]);
        assert!(world.cells.tail_collision_radius[0] > world.cells.collision_radius[0]);
    }

    #[test]
    fn segmented_population_reaches_four_sections_with_free_angles() {
        let world = WorldState::new(&SimConfig {
            cells: 2_000,
            food: 0,
            segmented_cells: true,
            seed: 91,
            ..default()
        });
        assert!(world.cells.section_count.iter().all(|count| *count <= 4));
        assert!(world.cells.section_count.contains(&3));
        assert!(world.cells.section_count.contains(&4));
        assert!(
            world
                .cells
                .section_angles
                .iter()
                .flatten()
                .any(|angle| angle.sin().abs() > 0.65)
        );
        assert!(
            world
                .cells
                .section_count
                .iter()
                .enumerate()
                .any(|(index, count)| *count >= 3 && world.cells.section_parents[index][1] == 0)
        );
    }

    #[test]
    fn fourth_section_follows_without_driving_the_head() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.section_count[0] = 4;
        world.cells.section_spacing[0] = 20.0;
        world.cells.section_angles[0] = [std::f32::consts::PI, 1.4, -1.2];
        world.cells.section_parents[0] = [0, 0, 1];
        world.cells.x[0] = 8_000.0;
        world.cells.y[0] = 0.0;
        world.cells.heading[0] = 0.0;
        world.cells.vx[0] = 35.0;
        world.cells.vy[0] = 0.0;
        world.cells.tail_x[0] = 7_980.0;
        world.cells.tail_y[0] = 0.0;
        world.cells.extra_sections[0][0].x = 7_980.0;
        world.cells.extra_sections[0][0].y = 20.0;
        world.cells.extra_sections[0][1].x = 7_980.0;
        world.cells.extra_sections[0][1].y = 40.0;
        let last_before = world.cells.section_center(0, 3);

        world.update_tail_sections(1.0 / 60.0);

        assert_eq!(world.cells.vx[0], 35.0);
        assert_eq!(world.cells.heading[0], 0.0);
        assert_ne!(world.cells.section_center(0, 3), last_before);
        assert!(cell_body_overlaps_circle(
            &world.cells,
            0,
            world.cells.section_center(0, 3),
            FOOD_RADIUS,
        ));
    }

    #[test]
    fn extra_section_membrane_uses_its_eight_directional_radii() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.section_count[0] = 3;
        world.cells.section_parents[0] = [0, 1, 0];
        world.cells.extra_sections[0][0].current_radii =
            [14.0, 10.0, 5.0, 6.0, 13.0, 9.0, 5.0, 7.0];
        world.cells.extra_sections[0][0].angle_offsets = [0.0; SOFT_BODY_POINTS];
        let heading = world.cells.section_heading(0, 2);

        let along = world.cells.section_membrane_radius(0, 2, heading);
        let across =
            world
                .cells
                .section_membrane_radius(0, 2, heading + std::f32::consts::FRAC_PI_2);

        assert!(along > across * 2.0);
    }

    #[test]
    fn collision_follows_curved_connection_instead_of_straight_chord() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.section_count[0] = 2;
        world.cells.section_parents[0] = [0, 0, 0];
        world.cells.x[0] = -30.0;
        world.cells.y[0] = 0.0;
        world.cells.tail_x[0] = 30.0;
        world.cells.tail_y[0] = 0.0;
        world.cells.edge_curve_offsets[0][0] = 20.0;
        set_test_cell_soft_radius(&mut world, 0, 4.0);
        world.cells.tail_current_radii[0] = [4.0; SOFT_BODY_POINTS];
        world.cells.tail_collision_radius[0] = 4.0;

        world.cells.section_count[1] = 1;
        world.cells.x[1] = 0.0;
        world.cells.y[1] = 10.0;
        set_test_cell_soft_radius(&mut world, 1, 3.0);

        let contact = find_compound_contact(&world.cells, 0, 1)
            .expect("coccus must touch the curved midpoint");
        assert!((contact.t_a - 0.5).abs() < 0.12);
    }

    #[test]
    fn tail_has_no_drive_and_follows_the_head_spring() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.section_count[0] = 2;
        world.cells.section_spacing[0] = 30.0;
        world.cells.section_bend[0] = 0.0;
        world.cells.section_angles[0][0] = std::f32::consts::PI;
        world.cells.x[0] = 8_020.0;
        world.cells.y[0] = 0.0;
        world.cells.heading[0] = 0.0;
        world.cells.vx[0] = 40.0;
        world.cells.vy[0] = 0.0;
        world.cells.tail_x[0] = 7_970.0;
        world.cells.tail_y[0] = 0.0;
        world.cells.tail_vx[0] = 0.0;
        world.cells.tail_vy[0] = 0.0;

        world.update_tail_sections(1.0 / 60.0);

        assert!(world.cells.tail_vx[0] > 0.0);
        assert_eq!(world.cells.heading[0], 0.0);
        assert_eq!(world.cells.vx[0], 40.0);
    }

    #[test]
    fn tail_lags_behind_lateral_head_motion() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.section_count[0] = 2;
        world.cells.section_spacing[0] = 30.0;
        world.cells.section_bend[0] = 0.0;
        world.cells.section_angles[0][0] = std::f32::consts::PI;
        world.cells.x[0] = 8_020.0;
        world.cells.y[0] = 0.0;
        world.cells.heading[0] = 0.0;
        world.cells.vx[0] = 0.0;
        world.cells.vy[0] = 40.0;
        world.cells.tail_x[0] = 7_990.0;
        world.cells.tail_y[0] = 0.0;
        world.cells.tail_vx[0] = 0.0;
        world.cells.tail_vy[0] = 0.0;

        world.update_tail_sections(1.0 / 60.0);

        assert!(world.cells.tail_vy[0] > 0.0);
        assert!(world.cells.tail_vy[0] < world.cells.vy[0] * 0.08);
    }

    #[test]
    fn tail_to_tail_collision_is_resolved_without_moving_centers() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        for index in 0..2 {
            world.cells.section_count[index] = 2;
            set_test_cell_soft_radius(&mut world, index, 8.0);
            world.cells.tail_current_radii[index] = [8.0; SOFT_BODY_POINTS];
            world.cells.tail_base_radii[index] = [8.0; SOFT_BODY_POINTS];
            world.cells.tail_collision_radius[index] = 8.0;
            world.cells.tail_vx[index] = 0.0;
            world.cells.tail_vy[index] = 0.0;
        }
        world.cells.x[0] = 0.0;
        world.cells.x[1] = 80.0;
        world.cells.tail_x[0] = 40.0;
        world.cells.tail_y[0] = 0.0;
        world.cells.tail_x[1] = 44.0;
        world.cells.tail_y[1] = 0.0;

        world.resolve_cell_pair(0, 1, 1.0 / 60.0);

        assert_eq!(world.cells.x[0], 0.0);
        assert_eq!(world.cells.x[1], 80.0);
        assert!(world.cells.tail_vx[0] < 0.0);
        assert!(world.cells.tail_vx[1] > 0.0);
    }

    #[test]
    fn curved_connections_collide_when_all_end_sections_are_separate() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });
        for index in 0..2 {
            world.cells.section_count[index] = 2;
            world.cells.section_bend[index] = 0.0;
            world.cells.vx[index] = 0.0;
            world.cells.vy[index] = 0.0;
            world.cells.tail_vx[index] = 0.0;
            world.cells.tail_vy[index] = 0.0;
            world.cells.collision_radius[index] = 4.0;
            world.cells.tail_collision_radius[index] = 4.0;
            world.cells.core_radius[index] = 1.0;
            world.cells.tail_core_radius[index] = 1.0;
            world.cells.current_radii[index] = [4.0; SOFT_BODY_POINTS];
            world.cells.tail_current_radii[index] = [4.0; SOFT_BODY_POINTS];
        }
        world.cells.x[0] = 30.0;
        world.cells.y[0] = 0.0;
        world.cells.tail_x[0] = -30.0;
        world.cells.tail_y[0] = 0.0;
        world.cells.x[1] = 0.0;
        world.cells.y[1] = 30.0;
        world.cells.tail_x[1] = 0.0;
        world.cells.tail_y[1] = -30.0;

        for section_a in 0..2 {
            for section_b in 0..2 {
                assert!(
                    world
                        .cells
                        .section_center(0, section_a)
                        .distance(world.cells.section_center(1, section_b))
                        > 8.0
                );
            }
        }
        let contact = find_compound_contact(&world.cells, 0, 1).expect("curves must intersect");
        assert!((contact.t_a - 0.5).abs() < 0.1);
        assert!((contact.t_b - 0.5).abs() < 0.1);

        world.resolve_cell_pair(0, 1, 1.0 / 60.0);

        let first_speed = Vec2::new(world.cells.vx[0], world.cells.vy[0]).length()
            + Vec2::new(world.cells.tail_vx[0], world.cells.tail_vy[0]).length();
        let second_speed = Vec2::new(world.cells.vx[1], world.cells.vy[1]).length()
            + Vec2::new(world.cells.tail_vx[1], world.cells.tail_vy[1]).length();
        assert!(first_speed > 0.0);
        assert!(second_speed > 0.0);
    }

    #[test]
    fn driven_coccus_cannot_push_through_a_connection_core() {
        let mut world = WorldState::new(&SimConfig {
            cells: 2,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.section_count[0] = 2;
        world.cells.section_bend[0] = 0.0;
        world.cells.x[0] = 30.0;
        world.cells.y[0] = 0.0;
        world.cells.tail_x[0] = -30.0;
        world.cells.tail_y[0] = 0.0;
        world.cells.vx[0] = 0.0;
        world.cells.vy[0] = 0.0;
        world.cells.tail_vx[0] = 0.0;
        world.cells.tail_vy[0] = 0.0;
        world.cells.collision_radius[0] = 5.0;
        world.cells.tail_collision_radius[0] = 5.0;
        world.cells.core_radius[0] = 2.0;
        world.cells.tail_core_radius[0] = 2.0;

        world.cells.section_count[1] = 1;
        world.cells.x[1] = 0.0;
        world.cells.y[1] = -14.0;
        world.cells.vx[1] = 0.0;
        world.cells.vy[1] = 80.0;
        world.cells.collision_radius[1] = 4.0;
        world.cells.core_radius[1] = 1.5;

        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            world.cells.vy[1] += (80.0 - world.cells.vy[1]) * 0.08;
            world.cells.x[1] += world.cells.vx[1] * dt;
            world.cells.y[1] += world.cells.vy[1] * dt;
            world.resolve_cell_pair(0, 1, dt);
        }

        assert!(world.cells.y[1] < 0.0);
    }

    #[test]
    fn segmented_cell_only_eats_food_touching_its_real_body() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.section_count[0] = 2;
        world.cells.x[0] = 30.0;
        world.cells.y[0] = 0.0;
        world.cells.tail_x[0] = -30.0;
        world.cells.tail_y[0] = 0.0;
        world.cells.collision_radius[0] = 5.0;
        world.cells.tail_collision_radius[0] = 5.0;

        let distant_food = Vec2::new(30.0, 30.0);
        let old_broad_radius = world.cells.collision_bound_radius(0) + FOOD_RADIUS;
        assert!(
            Vec2::new(world.cells.x[0], world.cells.y[0]).distance_squared(distant_food)
                < old_broad_radius * old_broad_radius
        );
        assert!(!cell_body_overlaps_circle(
            &world.cells,
            0,
            distant_food,
            FOOD_RADIUS
        ));
        assert!(cell_body_overlaps_circle(
            &world.cells,
            0,
            Vec2::new(0.0, 4.0),
            FOOD_RADIUS
        ));
    }

    #[test]
    fn solid_pushes_tail_section_without_moving_the_head() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 0,
            segmented_cells: false,
            ..default()
        });
        world.cells.section_count[0] = 2;
        world.cells.x[0] = 80.0;
        world.cells.y[0] = 0.0;
        world.cells.tail_x[0] = 10.0;
        world.cells.tail_y[0] = 0.0;
        world.cells.tail_vx[0] = -12.0;
        world.cells.tail_vy[0] = 0.0;
        world.cells.tail_core_radius[0] = 2.0;
        world.cells.tail_base_radii[0] = [8.0; SOFT_BODY_POINTS];
        world.cells.tail_current_radii[0] = [8.0; SOFT_BODY_POINTS];
        world.cells.tail_collision_radius[0] = 8.0;

        world.push_section_from_solid(0, 1, Vec2::ZERO, 20.0);

        assert_eq!(world.cells.x[0], 80.0);
        assert!(world.cells.tail_x[0] > 10.0);
        assert!(world.cells.tail_vx[0] >= 0.0);
        assert!(world.cells.tail_current_radii[0][4] < 8.0);
        assert!(world.cells.tail_current_radii[0][4] >= world.cells.tail_core_radius[0]);
    }

    #[test]
    fn perception_strictly_limits_food_visibility() {
        let config = SimConfig {
            cells: 1,
            food: 1,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        for food_index in 0..world.food.len() {
            world.food.active[food_index] = false;
        }
        world.food.active[0] = true;
        world.food.x[0] = 500.0;
        world.food.y[0] = 0.0;
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.viability[0] = 10.0;
        world.cells.perception[0] = 499.0;
        world.grid.rebuild(&world.food);

        assert!(
            world
                .grid
                .nearest_food(0.0, 0.0, &world.food, 499.0)
                .is_none()
        );
        assert!(world.cell_target(0).is_none());

        world.cells.perception[0] = 500.0;
        let visible = world
            .grid
            .nearest_food(0.0, 0.0, &world.food, 500.0)
            .expect("food on the perception boundary is visible");
        assert_eq!(visible.0, 0);
        assert!((visible.3 - 250_000.0).abs() < 0.001);
        let target = world
            .cell_target(0)
            .expect("cell acquires visible food target");
        assert_eq!(target.kind, CellTargetKind::Food);
        assert_eq!(target.index, 0);
        assert_eq!(target.position, Vec2::new(500.0, 0.0));
    }

    #[test]
    fn perception_search_returns_nearest_visible_food_across_grid_rings() {
        let config = SimConfig {
            cells: 1,
            food: 2,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        for food_index in 0..world.food.len() {
            world.food.active[food_index] = false;
        }
        world.food.active[0] = true;
        world.food.x[0] = 700.0;
        world.food.y[0] = 0.0;
        world.food.active[1] = true;
        world.food.x[1] = 260.0;
        world.food.y[1] = 0.0;
        world.grid.rebuild(&world.food);

        let nearest = world
            .grid
            .nearest_food(0.0, 0.0, &world.food, 800.0)
            .expect("visible food exists");
        assert_eq!(nearest.0, 1);
    }

    #[test]
    fn persistence_controls_target_switch_hysteresis() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 8,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        for active in &mut world.food.active {
            *active = false;
        }
        world.food.active[0] = true;
        world.food.active[1] = true;
        world.food.x[0] = 200.0;
        world.food.y[0] = 0.0;
        world.food.x[1] = 220.0;
        world.food.y[1] = 0.0;
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.viability[0] = 10.0;
        world.cells.perception[0] = 500.0;
        world.cells.persistence[0] = 100.0;
        world.grid.rebuild(&world.food);
        assert_eq!(world.update_cell_target(0, 0.01).unwrap().index, 0);

        world.food.x[1] = 150.0;
        world.cells.target_recheck[0] = 0.0;
        world.grid.rebuild(&world.food);
        assert_eq!(world.update_cell_target(0, 0.01).unwrap().index, 0);

        world.cells.persistence[0] = 0.0;
        world.cells.target_recheck[0] = 0.0;
        assert_eq!(world.update_cell_target(0, 0.01).unwrap().index, 1);
    }

    #[test]
    fn persistent_cell_chases_last_known_position_after_losing_food() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 4,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        for active in &mut world.food.active {
            *active = false;
        }
        world.food.active[0] = true;
        world.food.x[0] = 200.0;
        world.food.y[0] = 30.0;
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.viability[0] = 10.0;
        world.cells.perception[0] = 400.0;
        world.cells.persistence[0] = 100.0;
        world.grid.rebuild(&world.food);
        world.update_cell_target(0, 0.01).unwrap();

        world.food.x[0] = 900.0;
        world.grid.rebuild(&world.food);
        let remembered = world.update_cell_target(0, 0.1).unwrap();
        assert!(remembered.remembered);
        assert_eq!(remembered.position, Vec2::new(200.0, 30.0));

        for _ in 0..31 {
            world.update_cell_target(0, 0.1);
        }
        assert!(world.update_cell_target(0, 0.1).is_none());
    }

    #[test]
    fn replaced_food_is_not_chased_at_its_old_position() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 1,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        world.food.x[0] = 200.0;
        world.food.y[0] = 30.0;
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.viability[0] = 10.0;
        world.cells.perception[0] = 400.0;
        world.cells.persistence[0] = 100.0;
        world.grid.rebuild(&world.food);
        world.update_cell_target(0, 0.01).unwrap();

        world.food.deactivate(0);
        world.food.active[0] = true;
        world.food.x[0] = 900.0;
        world.food.y[0] = 0.0;
        world.grid.rebuild(&world.food);

        assert!(world.update_cell_target(0, 0.1).is_none());
        assert_eq!(world.cells.target_food[0], -1);
        assert_eq!(world.cells.target_memory[0], 0.0);
    }

    #[test]
    fn cells_ignore_meat_from_their_own_species() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 1,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        world.cells.viability[0] = 10.0;
        world.cells.max_viability[0] = 100.0;
        world.cells.perception[0] = 400.0;
        world.cells.species[0] = 17;
        world.food.kind[0] = FoodKind::Meat;
        world.food.origin_species[0] = 17;
        world.food.x[0] = 20.0;
        world.food.y[0] = 0.0;
        world.grid.rebuild(&world.food);

        assert!(
            world
                .nearest_edible_food(0, Vec2::new(0.0, 0.0), 400.0)
                .is_none()
        );

        world.food.origin_species[0] = 18;
        world.grid.rebuild(&world.food);
        assert!(
            world
                .nearest_edible_food(0, Vec2::new(0.0, 0.0), 400.0)
                .is_some()
        );
    }

    #[test]
    fn child_soft_body_shape_stays_in_allowed_ranges() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        world.cells.viability[0] = 90.0;
        world.cells.division_threshold[0] = 80.0;
        world.cells.mutation_susceptibility[0] = MUTATION_GENE_MAX;

        complete_test_mitosis(&mut world);
        let child = 1;

        for ray_index in 0..SOFT_BODY_POINTS {
            assert!(
                (world.cells.radius[child] * SOFT_BODY_BASE_MIN_FACTOR..=world.cells.radius[child])
                    .contains(&world.cells.base_radii[child][ray_index])
            );
            assert!(
                world.cells.current_radii[child][ray_index]
                    <= world.cells.base_radii[child][ray_index]
            );
            assert!(
                (-SOFT_BODY_MAX_ANGLE_OFFSET..=SOFT_BODY_MAX_ANGLE_OFFSET)
                    .contains(&world.cells.angle_offsets[child][ray_index])
            );
        }
        assert!(
            (MUTATION_GENE_MIN..=MUTATION_GENE_MAX)
                .contains(&world.cells.mutation_susceptibility[child])
        );
    }

    #[test]
    fn mutation_susceptibility_controls_mutation_parameters() {
        assert!(mutation_chance(100.0) > mutation_chance(0.0));
        assert!(mutation_power(100.0) > mutation_power(0.0));
    }

    #[test]
    fn asymmetric_soft_body_shape_applies_drag_and_turn_bias() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.angle_offsets[0] = [0.0; SOFT_BODY_POINTS];
        world.cells.rebuild_soft_body_cache(0);
        let baseline_speed_factor = world.cells.shape_drag_factor(0);

        world.cells.angle_offsets[0][2] = SOFT_BODY_MAX_ANGLE_OFFSET;
        world.cells.rebuild_soft_body_cache(0);
        assert!(world.cells.shape_drag_factor(0) < baseline_speed_factor);
        assert!(world.cells.turn_agility_factor(0, 0.5) > 1.0);
        assert_eq!(world.cells.turn_agility_factor(0, -0.5), 1.0);
    }

    #[test]
    fn geometry_continuously_controls_morphology_tradeoffs() {
        let mut world = WorldState::new(&SimConfig {
            cells: 3,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            segmented_cells: false,
            ..default()
        });
        for index in 0..3 {
            world.cells.angle_offsets[index] = [0.0; SOFT_BODY_POINTS];
            world.cells.section_count[index] = 1;
        }

        world.cells.base_radii[0] = [6.0; SOFT_BODY_POINTS];
        world.cells.base_radii[1] = [6.0, 8.485, 6.0, 8.485, 6.0, 8.485, 6.0, 8.485];
        world.cells.base_radii[2] = [14.0, 6.0, 4.5, 6.0, 14.0, 6.0, 4.5, 6.0];
        for index in 0..3 {
            world.cells.rebuild_soft_body_cache(index);
        }

        assert!(
            world.cells.morphology_viability_factor(1) > world.cells.morphology_viability_factor(0)
        );
        assert!(world.cells.morphology_speed_factor(2) > world.cells.morphology_speed_factor(0));
        assert!(
            world.cells.morphology_acceleration_factor(2)
                > world.cells.morphology_acceleration_factor(0)
        );
        assert!(world.cells.morphology_turn_factor(2) < world.cells.morphology_turn_factor(0));
        assert!(
            world.cells.morphology_viability_factor(1) > world.cells.morphology_viability_factor(2)
        );
    }

    #[test]
    fn cell_ids_survive_swap_remove_selection_case() {
        let config = SimConfig {
            cells: 3,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        let selected_id = world.cells.id[1];
        world.cells.base_radii[1] = [7.25; SOFT_BODY_POINTS];
        world.cells.rebuild_soft_body_cache(1);
        world.cells.viability[0] = 0.0;

        world.remove_dead_cells();

        let selected_index = world
            .cell_index_by_id(selected_id)
            .expect("selected id remains");
        assert_eq!(world.cells.id[selected_index], selected_id);
        assert_eq!(
            world.cells.base_radii[selected_index],
            [7.25; SOFT_BODY_POINTS]
        );
    }

    #[test]
    fn food_restores_viability() {
        let config = SimConfig {
            cells: 1,
            food: 1,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.x[0] = 5000.0;
        world.cells.y[0] = 5000.0;
        world.cells.viability[0] = 10.0;

        for i in 0..world.food.len() {
            world.food.active[i] = false;
        }
        world.food.active[0] = true;
        world.food.x[0] = 5000.0;
        world.food.y[0] = 5000.0;
        world.food.feeder[0] = -1;
        world.food.kind[0] = FoodKind::Grass;
        world.food.growth[0] = 1.0;
        world.cells.aggressiveness[0] = 0.0;
        let available_energy = digested_food_energy(
            world.food.kind[0],
            world.food.energy[0] * world.food.growth[0],
            world.cells.aggressiveness[0],
        );

        world.update(1.0 / 60.0);

        assert!(world.cells.viability[0] > 10.0);
        assert!(world.cells.viability[0] <= 10.0 + available_energy);
        assert!(!world.food.active[0], "eaten world food must not respawn");
        assert_eq!(world.visual_particles.len(), FOOD_PARTICLES_PER_BITE);
        assert!(world.visual_particles.life.iter().all(|life| *life > 0.0));

        world.update_visual_particles(1.0);
        assert_eq!(world.visual_particles.len(), 0);
    }

    #[test]
    fn uneaten_food_spoils_and_despawns() {
        let mut world = WorldState::new(&SimConfig {
            cells: 0,
            food: 1,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        world.food.lifetime[0] = 0.5;
        let initial_energy = world.food.energy[0];

        world.decay_food(0.25);
        assert!(world.food.active[0]);
        assert!(world.food.energy[0] < initial_energy);
        world.decay_food(0.25);

        assert!(!world.food.active[0]);
        assert_eq!(world.food.energy[0], 0.0);
    }

    #[test]
    fn food_particle_budget_is_bounded() {
        let mut world = WorldState::new(&SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        for _ in 0..(MAX_VISUAL_PARTICLES / FOOD_PARTICLES_PER_BITE + 20) {
            world.spawn_food_particles(Vec2::ZERO, FoodKind::Grass, Vec2::ZERO);
        }
        assert_eq!(world.visual_particles.len(), MAX_VISUAL_PARTICLES);
    }

    #[test]
    fn cell_turning_is_limited_by_turn_speed() {
        let config = SimConfig {
            cells: 1,
            food: 1,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.heading[0] = 0.0;
        world.cells.vx[0] = 0.0;
        world.cells.vy[0] = 0.0;
        world.cells.turn_speed[0] = 0.6;
        world.cells.angle_offsets[0] = [0.0; SOFT_BODY_POINTS];
        world.cells.rebuild_soft_body_cache(0);

        for i in 0..world.food.len() {
            world.food.active[i] = false;
        }
        world.food.active[0] = true;
        world.food.x[0] = 0.0;
        world.food.y[0] = 500.0;
        world.food.feeder[0] = -1;

        world.update(1.0 / 60.0);

        assert!(world.cells.heading[0] > 0.0);
        assert!(world.cells.heading[0] <= 0.6 / 60.0 + 0.001);
    }

    #[test]
    fn cell_acceleration_is_gradual() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.heading[0] = 0.0;
        world.cells.vx[0] = 0.0;
        world.cells.vy[0] = 0.0;
        world.cells.speed[0] = 100.0;
        world.cells.viability[0] = world.cells.max_viability[0];

        world.drive_cell(0, Vec2::X * 100.0, Vec2::ZERO, 1.0 / 60.0);

        assert!(world.cells.vx[0] > 0.0);
        assert!(world.cells.vx[0] < 5.0);
    }

    #[test]
    fn cell_wake_strength_fades_in_instead_of_appearing_immediately() {
        let mut world = WorldState::new(&SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        });
        assert_eq!(world.cells.wake_strength[0], 0.0);

        world.update(1.0 / 60.0);
        let first_frame = world.cells.wake_strength[0];
        assert!(first_frame > 0.0 && first_frame < 0.1);

        for _ in 0..90 {
            world.update(1.0 / 60.0);
        }
        assert!(world.cells.wake_strength[0] > first_frame);
    }

    #[test]
    fn rear_target_causes_a_turn_instead_of_immediate_reverse() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.heading[0] = 0.0;
        world.cells.vx[0] = 0.0;
        world.cells.vy[0] = 0.0;
        world.cells.speed[0] = 100.0;
        world.cells.viability[0] = world.cells.max_viability[0];
        world.cells.angle_offsets[0] = [0.0; SOFT_BODY_POINTS];
        world.cells.rebuild_soft_body_cache(0);

        world.drive_cell(0, -Vec2::X * 100.0, Vec2::ZERO, 1.0 / 60.0);

        let front = Vec2::from_angle(world.cells.heading[0]);
        let velocity = Vec2::new(world.cells.vx[0], world.cells.vy[0]);
        assert_ne!(world.cells.heading[0], 0.0);
        assert!(velocity.dot(front) >= 0.0);
        assert_eq!(world.cells.reverse_time[0], 0.0);
    }

    #[test]
    fn emergency_reverse_requires_a_sustained_stall() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.heading[0] = 0.0;
        world.cells.vx[0] = 0.0;
        world.cells.vy[0] = 0.0;
        world.cells.speed[0] = 100.0;
        world.cells.turn_speed[0] = 0.0;
        world.cells.viability[0] = world.cells.max_viability[0];
        world.cells.angle_offsets[0] = [0.0; SOFT_BODY_POINTS];
        world.cells.rebuild_soft_body_cache(0);

        for _ in 0..44 {
            world.drive_cell(0, -Vec2::X * 100.0, Vec2::ZERO, 1.0 / 60.0);
        }
        assert_eq!(world.cells.reverse_time[0], 0.0);

        for _ in 0..4 {
            world.drive_cell(0, -Vec2::X * 100.0, Vec2::ZERO, 1.0 / 60.0);
        }
        assert!(world.cells.reverse_time[0] > 0.0);
        assert!(world.cells.reverse_time[0] <= EMERGENCY_REVERSE_DURATION);
    }

    #[test]
    fn liquid_current_can_still_push_a_cell_sideways() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.heading[0] = 0.0;
        world.cells.vx[0] = 0.0;
        world.cells.vy[0] = 0.0;
        world.cells.viability[0] = world.cells.max_viability[0];

        for _ in 0..60 {
            world.drive_cell(0, Vec2::X * 100.0, Vec2::Y * 24.0, 1.0 / 60.0);
        }

        assert!(world.cells.vy[0] > 5.0);
    }

    #[test]
    fn velocity_lags_behind_heading_during_turns() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            obstacles: 0,
            food_growers: 0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.heading[0] = 0.0;
        world.cells.vx[0] = 80.0;
        world.cells.vy[0] = 0.0;
        world.cells.speed[0] = 80.0;
        world.cells.turn_speed[0] = TURN_GENE_MAX;
        world.cells.angle_offsets[0] = [0.0; SOFT_BODY_POINTS];
        world.cells.rebuild_soft_body_cache(0);

        world.drive_cell(0, Vec2::Y * 80.0, Vec2::ZERO, 1.0 / 60.0);

        let velocity_angle = world.cells.vy[0].atan2(world.cells.vx[0]);
        assert!(world.cells.heading[0] > 0.0);
        assert!(velocity_angle < world.cells.heading[0] * 0.5);
        assert!(world.cells.vx[0] > 75.0);
    }

    #[test]
    fn liquid_current_moves_food() {
        let config = SimConfig {
            cells: 0,
            food: 1,
            width: 1_000.0,
            height: 1_000.0,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.food.x[0] = 123.0;
        world.food.y[0] = -77.0;
        let before = Vec2::new(world.food.x[0], world.food.y[0]);

        world.update(1.0 / 60.0);

        let after = Vec2::new(world.food.x[0], world.food.y[0]);
        assert!(after.distance_squared(before) > 0.0001);
    }

    #[test]
    fn overlapping_cells_are_separated_without_position_teleport() {
        let config = SimConfig {
            cells: 2,
            food: 0,
            segmented_cells: false,
            ..default()
        };
        let mut world = WorldState::new(&config);
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.x[1] = 4.0;
        world.cells.y[1] = 0.0;
        set_test_cell_soft_radius(&mut world, 0, 8.0);
        set_test_cell_soft_radius(&mut world, 1, 8.0);
        world.cells.vx[0] = 10.0;
        world.cells.vx[1] = -10.0;

        world.solve_cell_collisions(1.0 / 60.0);

        assert_eq!(world.cells.x[0], 0.0);
        assert_eq!(world.cells.y[0], 0.0);
        assert_eq!(world.cells.x[1], 4.0);
        assert_eq!(world.cells.y[1], 0.0);
        assert!(world.cells.vx[0] < 10.0);
        assert!(world.cells.vx[1] > -10.0);
        assert!(world.cells.jelly_intensity[0] > 0.0);
        assert!(world.cells.jelly_intensity[1] > 0.0);
    }

    #[test]
    fn default_cell_shape_is_exact_circle() {
        let config = SimConfig {
            cells: 1,
            food: 0,
            ..default()
        };
        let world = WorldState::new(&config);

        for angle in [
            0.0,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            std::f32::consts::PI * 1.5,
        ] {
            assert_eq!(world.cells.shape_radius_at(0, angle), 1.0);
        }
    }
}
