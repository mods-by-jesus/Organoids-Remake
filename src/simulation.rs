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
const SEARCH_RING: i32 = 2;
const STEER_GAIN: f32 = 9.5;
const DRAG: f32 = 0.985;
const WANDER_GAIN: f32 = 0.45;
pub const CELL_VIABILITY_MAX: f32 = 100.0;
pub const CELL_SPEED_DISPLAY_MAX: f32 = 120.0;
pub const CELL_TURN_DISPLAY_MAX: f32 = 5.2;
pub const CELL_MUTATION_DISPLAY_MAX: f32 = 100.0;
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
const VIABILITY_DECAY_BASE: f32 = 0.95;
const VIABILITY_DECAY_SPEED: f32 = 0.45;
const SOFT_BODY_ELASTICITY_SPEED: f32 = 8.0;
const SOFT_BODY_VISUAL_FOLLOW_SPEED: f32 = 12.0;
const SOFT_BODY_COMPRESSION_RESPONSE: f32 = 0.58;
const SOFT_BODY_BIOMASS_DRAIN_RATE: f32 = 0.012;
const SOFT_BODY_MIN_RADIUS_FACTOR: f32 = 0.25;
const SOFT_BODY_BASE_MIN_FACTOR: f32 = 0.35;
const SOFT_BODY_START_MIN_FACTOR: f32 = 0.45;
const SOFT_BODY_MAX_ANGLE_OFFSET: f32 = 0.261_799_4;
const SOFT_BODY_MUTATION_ANGLE_DELTA: f32 = 0.08;
const SOFT_BODY_SHAPE_DRAG: f32 = 0.12;
const SOFT_BODY_TURN_BONUS: f32 = 0.18;
const SOFT_BODY_COMPRESSION_IMPULSE: f32 = 2.8;
const SOFT_BODY_SOLID_PUSH_FACTOR: f32 = 0.62;
const SOFT_BODY_SOLID_PUSH_MAX: f32 = 5.5;
const SOFT_BODY_CELL_PUSH_MAX: f32 = 4.0;
const MUTATION_FACTOR_DELTA_SCALE: f32 = 100.0 / (0.3 - 0.005);
const FOOD_VIABILITY_GAIN: f32 = 18.0;
const FEEDER_FOOD_VIABILITY_GAIN: f32 = 14.0;
const MIN_VIABILITY_MOVE_FACTOR: f32 = 0.28;
const REVERSE_ALIGNMENT: f32 = -0.35;
const TURN_IN_PLACE_ANGLE: f32 = 1.35;
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
const COLLISION_RESTITUTION: f32 = 0.05;
const COLLISION_PUSH: f32 = 0.18;
const JELLY_DECAY: f32 = 2.8;
const JELLY_HIT_GAIN: f32 = 0.42;
const HUNGER_EPSILON: f32 = 0.25;
const DIVISION_CHILD_OFFSET: f32 = 2.35;
const MEAT_CHUNKS_MIN: usize = 3;
const MEAT_CHUNKS_MAX: usize = 6;
const MUTATION_CHANCE_MIN: f32 = 0.15;
const MUTATION_CHANCE_MAX: f32 = 0.85;
const MUTATION_STRENGTH_MIN: f32 = 0.02;
const MUTATION_STRENGTH_MAX: f32 = 0.20;
const MUTATION_POWER_MIN: f32 = 0.35;
const MUTATION_POWER_MAX: f32 = 1.0;
const SPEED_GENE_MIN: f32 = 30.0;
const SPEED_GENE_MAX: f32 = 130.0;
const TURN_GENE_MIN: f32 = 0.8;
const TURN_GENE_MAX: f32 = 6.0;
const MUTATION_GENE_MIN: f32 = 0.0;
const MUTATION_GENE_MAX: f32 = 100.0;
const DIVISION_THRESHOLD_MIN: f32 = 50.0;
const DIVISION_THRESHOLD_MAX: f32 = 95.0;

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
    pub seed: u64,
    pub vsync: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            cells: 10_000,
            food: 2_000,
            width: 24_000.0,
            height: 13_500.0,
            arena_shape: ArenaShape::Rectangle,
            obstacles: 26,
            food_growers: 4,
            seed: 0xC011_CE11,
            vsync: false,
        }
    }
}

impl SimConfig {
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
                "--seed" => {
                    config.seed = parse_next(&mut args, "--seed")?;
                }
                "--vsync" => {
                    config.vsync = true;
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
    "Usage: organoids [--cells 10000] [--food 2000] [--width 24000] [--height 13500] [--shape rectangle|circle] [--obstacles 26] [--food-growers 4] [--seed 123] [--vsync]".to_string()
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
    pub width: f32,
    pub height: f32,
    pub arena_shape: ArenaShape,
    grid: SpatialGrid,
    cell_grid: CellGrid,
    rng: SmallRng,
    elapsed: f32,
    max_food: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodKind {
    Grass,
    Meat,
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

    fn random(rng: &mut SmallRng) -> Self {
        if rng.random_bool(0.5) {
            FoodKind::Grass
        } else {
            FoodKind::Meat
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
        let cells = CellStore::new(
            config.cells,
            config.width,
            config.height,
            config.arena_shape,
            &mut rng,
        );
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

        let mut world = Self {
            cells,
            food,
            obstacles,
            food_growers,
            width: config.width,
            height: config.height,
            arena_shape: config.arena_shape,
            grid,
            cell_grid,
            rng,
            elapsed: 0.0,
            max_food: floor_food_count.saturating_add(feeder_food_capacity),
        };
        world.relocate_world_food_away_from_solids();
        world.seed_feeder_food(feeder_food_capacity);
        world.grid.rebuild(&world.food);

        // DEBUG: Log world creation stats
        let solid_branches: usize = (0..world.food_growers.branch_total())
            .filter(|&b| world.food_growers.branch_has_collision(b))
            .count();
        eprintln!(
            "[DEBUG] World created: {} growers, {} total branches ({} solid), {} food (floor: {}, feeder cap: {})",
            world.food_growers.len(),
            world.food_growers.branch_total(),
            solid_branches,
            world.food.len(),
            floor_food_count,
            feeder_food_capacity,
        );
        for g in 0..world.food_growers.len() {
            eprintln!(
                "[DEBUG]   Grower {}: radius={:.1}, branches={}, extent={:.1}",
                g,
                world.food_growers.radius[g],
                world.food_growers.branch_count[g],
                world.food_growers.extent_radius(g),
            );
        }

        world
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 1.0 / 20.0);
        self.elapsed += dt;
        self.remove_dead_cells();
        self.advect_obstacles(dt);
        self.advect_food_growers(dt);
        self.resolve_obstacle_food_growers();
        self.grow_food(dt);
        self.advect_food(dt);
        self.push_food_from_obstacles(dt);
        self.push_food_from_food_growers(dt);
        self.clamp_free_food_to_arena();
        self.grid.rebuild(&self.food);
        self.decay_visuals(dt);
        self.cells.relax_soft_body(dt);

        for i in 0..self.cells.len() {
            let x = self.cells.x[i];
            let y = self.cells.y[i];
            let speed = self.effective_cell_speed(i);
            let hungry = self.cells.viability[i] < self.cells.max_viability[i] - HUNGER_EPSILON;

            let mut target_food = None;

            let (desired_x, desired_y) = if hungry
                && let Some((food_index, dx, dy, dist_sq)) =
                    self.grid.nearest_food(x, y, &self.food, SEARCH_RING)
            {
                let inv_len = dist_sq.sqrt().recip();
                target_food = Some((food_index, dist_sq));
                (dx * inv_len * speed, dy * inv_len * speed)
            } else {
                let phase = ((i as f32 * 12.9898 + x * 0.017 + y * 0.011).sin()) * WANDER_GAIN;
                let (s, c) = phase.sin_cos();
                (
                    (self.cells.vx[i] * c - self.cells.vy[i] * s).clamp(-speed, speed),
                    (self.cells.vx[i] * s + self.cells.vy[i] * c).clamp(-speed, speed),
                )
            };

            let desired_velocity = (Vec2::new(desired_x, desired_y)
                + self.cell_avoidance_velocity(i, Vec2::new(desired_x, desired_y)))
            .clamp_length_max(speed * 1.2);
            let current = liquid_current_at(Vec2::new(x, y), self.elapsed) * CELL_CURRENT_SPEED;
            self.drive_cell(i, desired_velocity, current, dt);

            self.cells.x[i] += self.cells.vx[i] * dt;
            self.cells.y[i] += self.cells.vy[i] * dt;

            self.bounce_cell(i);
            self.resolve_cell_obstacles(i);
            self.resolve_cell_food_growers(i);

            if let Some((food_index, dist_sq)) = target_food {
                let eat_radius = self.cells.collision_bound_radius(i) + FOOD_RADIUS;
                if dist_sq <= eat_radius * eat_radius
                    && self.cells.viability[i] < self.cells.max_viability[i] - HUNGER_EPSILON
                {
                    if self.food.is_feeder_food(food_index) {
                        self.clear_branchlet_food_association(food_index);
                        self.food.deactivate(food_index);
                        self.cells.add_viability(i, FEEDER_FOOD_VIABILITY_GAIN);
                    } else {
                        self.respawn_world_food(food_index);
                        self.cells.add_viability(i, FOOD_VIABILITY_GAIN);
                    }
                }
            }
        }

        self.solve_cell_collisions();
        self.decay_viability(dt);
        self.process_cell_lifecycle();
    }

    fn effective_cell_speed(&self, cell_index: usize) -> f32 {
        self.cells.speed[cell_index]
            * (MIN_VIABILITY_MOVE_FACTOR + self.cells.viability_ratio(cell_index) * 0.72)
            * self.cells.shape_drag_factor(cell_index)
    }

    fn drive_cell(&mut self, cell_index: usize, desired_velocity: Vec2, current: Vec2, dt: f32) {
        let current_heading = self.cells.heading[cell_index];
        let forward = Vec2::new(current_heading.cos(), current_heading.sin());
        let desired_dir = desired_velocity.try_normalize().unwrap_or(forward);
        let desired_angle = desired_dir.y.atan2(desired_dir.x);
        let alignment = forward.dot(desired_dir);
        let reversing = alignment < REVERSE_ALIGNMENT;
        let target_heading = if reversing {
            wrap_angle(desired_angle + std::f32::consts::PI)
        } else {
            desired_angle
        };
        let turn_delta = angle_delta(target_heading, current_heading);
        let turn_step = self.cells.turn_speed[cell_index]
            * self.cells.turn_agility_factor(cell_index, turn_delta)
            * dt;
        let new_heading = wrap_angle(current_heading + turn_delta.clamp(-turn_step, turn_step));
        self.cells.heading[cell_index] = new_heading;

        let front = Vec2::new(new_heading.cos(), new_heading.sin());
        let abs_turn = turn_delta.abs();
        let throttle = if reversing {
            -0.55
        } else if abs_turn > TURN_IN_PLACE_ANGLE {
            0.18
        } else {
            1.0 - (abs_turn / TURN_IN_PLACE_ANGLE).clamp(0.0, 1.0) * 0.45
        };
        let drive_speed = self.effective_cell_speed(cell_index);
        let target_velocity = front * drive_speed * throttle + current;
        let steer = (STEER_GAIN * dt).clamp(0.0, 1.0);
        self.cells.vx[cell_index] = (self.cells.vx[cell_index]
            + (target_velocity.x - self.cells.vx[cell_index]) * steer)
            * DRAG;
        self.cells.vy[cell_index] = (self.cells.vy[cell_index]
            + (target_velocity.y - self.cells.vy[cell_index]) * steer)
            * DRAG;
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
                || (self.food.len() >= self.max_food && !self.food.has_inactive_slot())
            {
                continue;
            }

            self.food_growers.timer[i] = self.food_growers.interval[i];
            let branch_range = self.food_growers.branch_range(i);
            if branch_range.is_empty() {
                continue;
            }

            if self.try_spawn_feeder_food(i) {
                continue;
            }
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
        if self.food.len() >= self.max_food && !self.food.has_inactive_slot() {
            return false;
        }

        let mut inactive_branchlet_indices = Vec::new();
        for idx in 0..self.food_growers.branchlet_grower_index.len() {
            if self.food_growers.branchlet_grower_index[idx] == grower_index {
                let mut is_active = false;
                if let Some(food_idx) = self.food_growers.branchlet_food_index[idx] {
                    if food_idx < self.food.len() && self.food.active[food_idx] {
                        is_active = true;
                    }
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
                    self.food.growth[i] = (self.food.growth[i] + dt * 1.7).min(1.0);
                    self.food.rotation[i] += self.food.spin[i] * dt;
                } else {
                    self.clear_branchlet_food_association(i);
                    self.food.deactivate(i);
                }
                continue;
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

    fn resolve_cell_obstacles(&mut self, cell_index: usize) {
        for obstacle_index in 0..self.obstacles.len() {
            let dx = self.cells.x[cell_index] - self.obstacles.x[obstacle_index];
            let dy = self.cells.y[cell_index] - self.obstacles.y[obstacle_index];
            let dist_sq = dx * dx + dy * dy;
            let obstacle_radius = self.obstacles.radius[obstacle_index];
            let broad_min_dist = self.cells.collision_bound_radius(cell_index) + obstacle_radius;
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
            let push =
                ((min_dist - dist) * SOFT_BODY_SOLID_PUSH_FACTOR).min(SOFT_BODY_SOLID_PUSH_MAX);
            self.cells.x[cell_index] += nx * push;
            self.cells.y[cell_index] += ny * push;
            if compression > 0.0 {
                self.cells.vx[cell_index] += nx * compression * SOFT_BODY_COMPRESSION_IMPULSE;
                self.cells.vy[cell_index] += ny * compression * SOFT_BODY_COMPRESSION_IMPULSE;
            }

            let into_obstacle = self.cells.vx[cell_index] * nx + self.cells.vy[cell_index] * ny;
            if into_obstacle < 0.0 {
                self.cells.vx[cell_index] -= into_obstacle * nx * (1.0 + CELL_OBSTACLE_RESTITUTION);
                self.cells.vy[cell_index] -= into_obstacle * ny * (1.0 + CELL_OBSTACLE_RESTITUTION);
                self.cells.jelly_intensity[cell_index] =
                    (self.cells.jelly_intensity[cell_index] + 0.35).min(1.0);
                self.cells.jelly_dir_x[cell_index] = nx;
                self.cells.jelly_dir_y[cell_index] = ny;
            }
        }
    }

    fn resolve_cell_food_growers(&mut self, cell_index: usize) {
        let mut cell_x = self.cells.x[cell_index];
        let mut cell_y = self.cells.y[cell_index];
        let broad_cell_radius = self.cells.collision_bound_radius(cell_index);

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

    fn respawn_world_food(&mut self, food_index: usize) {
        let safe = self.safe_random_food_position();
        self.food.x[food_index] = safe.x;
        self.food.y[food_index] = safe.y;
        self.food.kind[food_index] = FoodKind::random(&mut self.rng);
        self.food.shape[food_index] = FoodShape::random(&mut self.rng);
        self.food.phase[food_index] = self.rng.random_range(0.0..std::f32::consts::TAU);
        self.food.rotation[food_index] = self.rng.random_range(0.0..std::f32::consts::TAU);
        self.food.spin[food_index] = random_food_spin(&mut self.rng);
        self.food.growth[food_index] = 1.0;
        self.food.active[food_index] = true;
        self.food.feeder[food_index] = -1;
        self.food.anchor_branch[food_index] = -1;
        self.food.anchor_angle[food_index] = 0.0;
        self.food.anchor_distance[food_index] = 0.0;
        self.food.anchor_lateral[food_index] = 0.0;
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

        for obstacle_index in 0..self.obstacles.len() {
            let center = Vec2::new(
                self.obstacles.x[obstacle_index],
                self.obstacles.y[obstacle_index],
            );
            let min_dist = self.obstacles.radius[obstacle_index] + radius;
            if point.distance_squared(center) < min_dist * min_dist {
                return true;
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
                if branch_index == ignore_branch {
                    continue;
                }
                if !self.food_growers.branch_has_collision(branch_index) {
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

    fn cell_avoidance_velocity(&self, cell_index: usize, desired_velocity: Vec2) -> Vec2 {
        let position = Vec2::new(self.cells.x[cell_index], self.cells.y[cell_index]);
        let cell_radius = self.cells.collision_bound_radius(cell_index);
        let speed = self.effective_cell_speed(cell_index);
        let desired_dir = if desired_velocity.length_squared() > 0.0001 {
            desired_velocity.normalize()
        } else {
            Vec2::new(self.cells.vx[cell_index], self.cells.vy[cell_index])
                .try_normalize()
                .unwrap_or(Vec2::X)
        };
        let mut avoidance = Vec2::ZERO;

        for obstacle_index in 0..self.obstacles.len() {
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
                    + CELL_AVOIDANCE_MARGIN * 1.0;
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
            self.cells.jelly_phase[i] += dt * (8.0 + self.cells.jelly_intensity[i] * 8.0);
            self.cells.jelly_intensity[i] *= decay;
        }
    }

    fn decay_viability(&mut self, dt: f32) {
        for i in 0..self.cells.len() {
            let speed_cost = (self.cells.speed[i] / CELL_SPEED_DISPLAY_MAX).clamp(0.0, 1.5);
            let biomass_cost = self.cells.biomass_sum(i) * SOFT_BODY_BIOMASS_DRAIN_RATE;
            let drain =
                (VIABILITY_DECAY_BASE + speed_cost * VIABILITY_DECAY_SPEED + biomass_cost) * dt;
            self.cells.viability[i] = (self.cells.viability[i] - drain).max(0.0);
        }
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
    }

    fn process_cell_lifecycle(&mut self) {
        self.remove_dead_cells();

        let initial_len = self.cells.len();
        for i in 0..initial_len {
            if self.cells.viability[i] <= 0.0 {
                continue;
            }

            let threshold = self.cells.max_viability[i] * self.cells.division_threshold[i] / 100.0;
            if self.cells.viability[i] >= threshold {
                self.divide_cell(i);
            }
        }
    }

    fn divide_cell(&mut self, parent_index: usize) {
        let split_viability = (self.cells.viability[parent_index] * 0.5)
            .clamp(0.0, self.cells.max_viability[parent_index]);
        self.cells.viability[parent_index] = split_viability;
        self.cells.push_child_from(
            parent_index,
            split_viability,
            self.width,
            self.height,
            self.arena_shape,
            &mut self.rng,
        );
    }

    fn spawn_meat_from_cell(&mut self, cell_index: usize) {
        let chunk_count = self.rng.random_range(MEAT_CHUNKS_MIN..=MEAT_CHUNKS_MAX);
        let origin = Vec2::new(self.cells.x[cell_index], self.cells.y[cell_index]);
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
                &mut self.rng,
            );
        }
    }

    pub fn cell_index_by_id(&self, cell_id: u64) -> Option<usize> {
        self.cells.id.iter().position(|id| *id == cell_id)
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

    fn solve_cell_collisions(&mut self) {
        self.cell_grid.rebuild(&self.cells);

        for bucket_index in 0..self.cell_grid.buckets.len() {
            if self.cell_grid.buckets[bucket_index].is_empty() {
                continue;
            }

            let bucket = self.cell_grid.buckets[bucket_index].clone();
            for a in 0..bucket.len() {
                for b in (a + 1)..bucket.len() {
                    self.resolve_cell_pair(bucket[a], bucket[b]);
                }
            }

            let (gx, gy) = self.cell_grid.coords_from_index(bucket_index);
            for (ox, oy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                let nx = gx + ox;
                let ny = gy + oy;

                if nx < 0
                    || ny < 0
                    || nx >= self.cell_grid.cols as i32
                    || ny >= self.cell_grid.rows as i32
                {
                    continue;
                }

                let other_index = ny as usize * self.cell_grid.cols + nx as usize;
                if self.cell_grid.buckets[other_index].is_empty() {
                    continue;
                }

                let other = self.cell_grid.buckets[other_index].clone();

                for &a in &bucket {
                    for &b in &other {
                        self.resolve_cell_pair(a, b);
                    }
                }
            }
        }

        for i in 0..self.cells.len() {
            self.bounce_cell(i);
        }
    }

    fn resolve_cell_pair(&mut self, a: usize, b: usize) {
        let dx = self.cells.x[b] - self.cells.x[a];
        let dy = self.cells.y[b] - self.cells.y[a];
        let dist_sq = dx * dx + dy * dy;
        let broad_min_dist =
            self.cells.collision_bound_radius(a) + self.cells.collision_bound_radius(b);
        if dist_sq >= broad_min_dist * broad_min_dist {
            return;
        }

        let (nx, ny, dist) = if dist_sq > 0.0001 {
            let dist = dist_sq.sqrt();
            (dx / dist, dy / dist, dist)
        } else {
            let angle = ((a as f32 * 12.9898 + b as f32 * 78.233).sin()) * std::f32::consts::TAU;
            let (ny, nx) = angle.sin_cos();
            (nx, ny, 0.001)
        };
        let normal = Vec2::new(nx, ny);
        let ray_a = self.cells.soft_ray_index_for_direction(a, normal);
        let ray_b = self.cells.soft_ray_index_for_direction(b, -normal);
        let radius_a = self.cells.current_radii[a][ray_a];
        let radius_b = self.cells.current_radii[b][ray_b];
        let min_dist = radius_a + radius_b;

        if dist_sq >= min_dist * min_dist {
            return;
        }

        let overlap = min_dist - dist;
        let compression_a = self.cells.compress_ray(a, ray_a, dist - radius_b);
        let compression_b = self.cells.compress_ray(b, ray_b, dist - radius_a);
        let push = (overlap * COLLISION_PUSH).min(SOFT_BODY_CELL_PUSH_MAX);
        self.cells.x[a] -= nx * push;
        self.cells.y[a] -= ny * push;
        self.cells.x[b] += nx * push;
        self.cells.y[b] += ny * push;
        if compression_a > 0.0 {
            self.cells.vx[a] -= nx * compression_a * SOFT_BODY_COMPRESSION_IMPULSE;
            self.cells.vy[a] -= ny * compression_a * SOFT_BODY_COMPRESSION_IMPULSE;
        }
        if compression_b > 0.0 {
            self.cells.vx[b] += nx * compression_b * SOFT_BODY_COMPRESSION_IMPULSE;
            self.cells.vy[b] += ny * compression_b * SOFT_BODY_COMPRESSION_IMPULSE;
        }

        let rvx = self.cells.vx[b] - self.cells.vx[a];
        let rvy = self.cells.vy[b] - self.cells.vy[a];
        let rel_normal_speed = rvx * nx + rvy * ny;

        if rel_normal_speed < 0.0 {
            let impulse = -(1.0 + COLLISION_RESTITUTION) * rel_normal_speed * 0.5;
            self.cells.vx[a] -= impulse * nx;
            self.cells.vy[a] -= impulse * ny;
            self.cells.vx[b] += impulse * nx;
            self.cells.vy[b] += impulse * ny;
        }

        let wobble = ((overlap / min_dist).clamp(0.0, 1.0) + rel_normal_speed.abs() / 160.0)
            .min(1.0)
            * JELLY_HIT_GAIN;
        self.cells.jelly_intensity[a] = (self.cells.jelly_intensity[a] + wobble).min(1.0);
        self.cells.jelly_intensity[b] = (self.cells.jelly_intensity[b] + wobble).min(1.0);
        self.cells.jelly_dir_x[a] = -nx;
        self.cells.jelly_dir_y[a] = -ny;
        self.cells.jelly_dir_x[b] = nx;
        self.cells.jelly_dir_y[b] = ny;
    }

    fn bounce_cell(&mut self, i: usize) {
        let r = self.cells.collision_bound_radius(i);
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

fn random_soft_body_radii(radius: f32, rng: &mut SmallRng) -> [f32; SOFT_BODY_POINTS] {
    let mut radii = [radius; SOFT_BODY_POINTS];
    for ray in &mut radii {
        *ray = (radius * rng.random_range(0.56..1.0))
            .clamp(radius * SOFT_BODY_START_MIN_FACTOR, radius);
    }
    radii
}

fn random_soft_body_angle_offsets(rng: &mut SmallRng) -> [f32; SOFT_BODY_POINTS] {
    let mut offsets = [0.0; SOFT_BODY_POINTS];
    for offset in &mut offsets {
        *offset = rng
            .random_range(-0.025_f32..0.025_f32)
            .clamp(-SOFT_BODY_MAX_ANGLE_OFFSET, SOFT_BODY_MAX_ANGLE_OFFSET);
    }
    offsets
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
    let mut base = parent_base;
    let mut current = parent_current;
    let mut offsets = parent_offsets;

    for ray_index in 0..SOFT_BODY_POINTS {
        if rng.random_bool(mutation_chance(susceptibility) as f64) {
            base[ray_index] += rng.random_range(-1.0..1.0) * size;
            offsets[ray_index] +=
                rng.random_range(-SOFT_BODY_MUTATION_ANGLE_DELTA..SOFT_BODY_MUTATION_ANGLE_DELTA);
        }

        base[ray_index] = base[ray_index].clamp(size * SOFT_BODY_BASE_MIN_FACTOR, size);
        current[ray_index] =
            current[ray_index].clamp(size * SOFT_BODY_MIN_RADIUS_FACTOR, base[ray_index]);
        offsets[ray_index] =
            offsets[ray_index].clamp(-SOFT_BODY_MAX_ANGLE_OFFSET, SOFT_BODY_MAX_ANGLE_OFFSET);
    }

    (base, current, offsets)
}

pub struct CellStore {
    pub id: Vec<u64>,
    next_id: u64,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub heading: Vec<f32>,
    pub radius: Vec<f32>,
    pub speed: Vec<f32>,
    pub turn_speed: Vec<f32>,
    pub species: Vec<u8>,
    pub viability: Vec<f32>,
    pub max_viability: Vec<f32>,
    pub mutation_susceptibility: Vec<f32>,
    pub division_threshold: Vec<f32>,
    pub base_radii: Vec<[f32; SOFT_BODY_POINTS]>,
    pub current_radii: Vec<[f32; SOFT_BODY_POINTS]>,
    pub visual_radii: Vec<[f32; SOFT_BODY_POINTS]>,
    pub angle_offsets: Vec<[f32; SOFT_BODY_POINTS]>,
    collision_radius: Vec<f32>,
    biomass: Vec<f32>,
    asymmetry_x: Vec<f32>,
    asymmetry_y: Vec<f32>,
    shape_drag: Vec<f32>,
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
}

impl CellStore {
    fn new(
        count: usize,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        rng: &mut SmallRng,
    ) -> Self {
        let mut store = Self {
            id: Vec::with_capacity(count),
            next_id: 0,
            x: Vec::with_capacity(count),
            y: Vec::with_capacity(count),
            vx: Vec::with_capacity(count),
            vy: Vec::with_capacity(count),
            heading: Vec::with_capacity(count),
            radius: Vec::with_capacity(count),
            speed: Vec::with_capacity(count),
            turn_speed: Vec::with_capacity(count),
            species: Vec::with_capacity(count),
            viability: Vec::with_capacity(count),
            max_viability: Vec::with_capacity(count),
            mutation_susceptibility: Vec::with_capacity(count),
            division_threshold: Vec::with_capacity(count),
            base_radii: Vec::with_capacity(count),
            current_radii: Vec::with_capacity(count),
            visual_radii: Vec::with_capacity(count),
            angle_offsets: Vec::with_capacity(count),
            collision_radius: Vec::with_capacity(count),
            biomass: Vec::with_capacity(count),
            asymmetry_x: Vec::with_capacity(count),
            asymmetry_y: Vec::with_capacity(count),
            shape_drag: Vec::with_capacity(count),
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
        };

        for _ in 0..count {
            let radius = rng.random_range(4.0..8.5);
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let speed = rng.random_range(48.0..92.0);
            let turn_speed = rng.random_range(1.75..4.65);
            let (s, c) = angle.sin_cos();
            let nucleus_angle = rng.random_range(0.0..std::f32::consts::TAU);
            let nucleus_distance = rng.random_range(0.0..0.36) * radius;
            let (nucleus_s, nucleus_c) = nucleus_angle.sin_cos();

            let position = random_point_in_arena(arena_w, arena_h, arena_shape, radius, rng);
            store.x.push(position.x);
            store.y.push(position.y);
            store.vx.push(c * speed);
            store.vy.push(s * speed);
            store.heading.push(angle);
            store.radius.push(radius);
            store.speed.push(speed);
            store.turn_speed.push(turn_speed);
            store.species.push(rng.random_range(0..6));
            store.max_viability.push(CELL_VIABILITY_MAX);
            store
                .viability
                .push(rng.random_range((CELL_VIABILITY_MAX * 0.45)..(CELL_VIABILITY_MAX * 0.70)));
            store
                .mutation_susceptibility
                .push(rng.random_range(35.0..65.0));
            store.division_threshold.push(rng.random_range(72.0..88.0));
            let base_radii = random_soft_body_radii(radius, rng);
            store.base_radii.push(base_radii);
            store.current_radii.push(base_radii);
            store.visual_radii.push(base_radii);
            store
                .angle_offsets
                .push(random_soft_body_angle_offsets(rng));
            store.collision_radius.push(radius);
            store.biomass.push(0.0);
            store.asymmetry_x.push(0.0);
            store.asymmetry_y.push(0.0);
            store.shape_drag.push(1.0);
            store.ray_dir_x.push([0.0; SOFT_BODY_POINTS]);
            store.ray_dir_y.push([0.0; SOFT_BODY_POINTS]);
            store.rebuild_soft_body_cache(store.len() - 1);
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

    fn push_child_from(
        &mut self,
        parent_index: usize,
        viability: f32,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        rng: &mut SmallRng,
    ) {
        let parent_heading = self.heading[parent_index];
        let side = if rng.random_bool(0.5) { -1.0 } else { 1.0 };
        let offset_angle = parent_heading + side * std::f32::consts::FRAC_PI_2;
        let (offset_s, offset_c) = offset_angle.sin_cos();
        let radius = self.radius[parent_index];
        let offset = radius * DIVISION_CHILD_OFFSET;
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
        self.ray_dir_x.push([0.0; SOFT_BODY_POINTS]);
        self.ray_dir_y.push([0.0; SOFT_BODY_POINTS]);
        self.rebuild_soft_body_cache(self.len() - 1);
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
    }

    fn swap_remove(&mut self, index: usize) {
        self.id.swap_remove(index);
        self.x.swap_remove(index);
        self.y.swap_remove(index);
        self.vx.swap_remove(index);
        self.vy.swap_remove(index);
        self.heading.swap_remove(index);
        self.radius.swap_remove(index);
        self.speed.swap_remove(index);
        self.turn_speed.swap_remove(index);
        self.species.swap_remove(index);
        self.viability.swap_remove(index);
        self.max_viability.swap_remove(index);
        self.mutation_susceptibility.swap_remove(index);
        self.division_threshold.swap_remove(index);
        self.base_radii.swap_remove(index);
        self.current_radii.swap_remove(index);
        self.visual_radii.swap_remove(index);
        self.angle_offsets.swap_remove(index);
        self.collision_radius.swap_remove(index);
        self.biomass.swap_remove(index);
        self.asymmetry_x.swap_remove(index);
        self.asymmetry_y.swap_remove(index);
        self.shape_drag.swap_remove(index);
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
        self.collision_radius[index]
    }

    pub fn max_base_radius(&self, index: usize) -> f32 {
        self.base_radii[index]
            .iter()
            .copied()
            .fold(self.radius[index] * SOFT_BODY_BASE_MIN_FACTOR, f32::max)
    }

    fn biomass_sum(&self, index: usize) -> f32 {
        self.biomass[index]
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
        let min_radius = (base * SOFT_BODY_MIN_RADIUS_FACTOR).max(0.1);
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
            let mut max_current = self.radius[index] * SOFT_BODY_MIN_RADIUS_FACTOR;
            let mut biomass = 0.0;
            for ray_index in 0..SOFT_BODY_POINTS {
                let base = self.base_radii[index][ray_index].min(self.radius[index]);
                self.base_radii[index][ray_index] = base;
                let current = self.current_radii[index][ray_index].min(base);
                let relaxed = current + (base - current) * elasticity;
                self.current_radii[index][ray_index] = relaxed;
                let visual = self.visual_radii[index][ray_index].min(self.radius[index]);
                self.visual_radii[index][ray_index] = visual + (relaxed - visual) * visual_follow;
                max_current = max_current.max(relaxed);
                biomass += base;
            }
            self.collision_radius[index] = max_current;
            self.biomass[index] = biomass;
        }
    }

    fn rebuild_soft_body_cache(&mut self, index: usize) {
        self.refresh_current_radius_cache(index);
        self.visual_radii[index] = self.current_radii[index];

        let mut biomass = 0.0;
        let mut vector = Vec2::ZERO;
        for ray_index in 0..SOFT_BODY_POINTS {
            biomass += self.base_radii[index][ray_index];
            let offset = self.angle_offsets[index][ray_index];
            let angle = SOFT_BODY_BASE_ANGLES[ray_index];
            let (ray_s, ray_c) = (angle + offset).sin_cos();
            self.ray_dir_x[index][ray_index] = ray_c;
            self.ray_dir_y[index][ray_index] = ray_s;
            let (s, c) = angle.sin_cos();
            vector += Vec2::new(c, s) * offset;
        }
        let normalized = vector / (SOFT_BODY_POINTS as f32 * SOFT_BODY_MAX_ANGLE_OFFSET);
        self.biomass[index] = biomass;
        self.asymmetry_x[index] = normalized.x;
        self.asymmetry_y[index] = normalized.y;
        let asymmetry = normalized.length().clamp(0.0, 1.0);
        self.shape_drag[index] = (1.0 - asymmetry * SOFT_BODY_SHAPE_DRAG).clamp(0.82, 1.0);
    }

    fn refresh_current_radius_cache(&mut self, index: usize) {
        self.collision_radius[index] = self.current_radii[index]
            .iter()
            .copied()
            .fold(self.radius[index] * SOFT_BODY_MIN_RADIUS_FACTOR, f32::max);
    }

    fn asymmetry_vector(&self, index: usize) -> Vec2 {
        Vec2::new(self.asymmetry_x[index], self.asymmetry_y[index])
    }

    fn shape_drag_factor(&self, index: usize) -> f32 {
        self.shape_drag[index]
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
        }
    }

    fn rebuild(&mut self, cells: &CellStore) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }

        for i in 0..cells.len() {
            let bucket = self.bucket_index(cells.x[i], cells.y[i]);
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

    fn coords_from_index(&self, index: usize) -> (i32, i32) {
        ((index % self.cols) as i32, (index / self.cols) as i32)
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
    pub active: Vec<bool>,
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
            active: Vec::with_capacity(count),
            feeder: Vec::with_capacity(count),
            anchor_branch: Vec::with_capacity(count),
            anchor_angle: Vec::with_capacity(count),
            anchor_distance: Vec::with_capacity(count),
            anchor_lateral: Vec::with_capacity(count),
        };

        for index in 0..count {
            let kind = if index % 2 == 0 {
                FoodKind::Grass
            } else {
                FoodKind::Meat
            };
            store.push_random(arena_w, arena_h, arena_shape, kind, rng);
        }

        store
    }

    fn push_random(
        &mut self,
        arena_w: f32,
        arena_h: f32,
        arena_shape: ArenaShape,
        kind: FoodKind,
        rng: &mut SmallRng,
    ) {
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
        self.active.push(true);
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

        if let Some(index) = self.inactive_slot() {
            self.x[index] = x;
            self.y[index] = y;
            self.kind[index] = FoodKind::Grass;
            self.shape[index] = FoodShape::random_feeder_food(rng);
            self.phase[index] = rng.random_range(0.0..std::f32::consts::TAU);
            self.rotation[index] = anchor_angle;
            self.spin[index] = 0.0;
            self.growth[index] = 0.24;
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
        self.active.push(true);
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
        rng: &mut SmallRng,
    ) {
        let point =
            clamp_point_to_arena(Vec2::new(x, y), arena_w, arena_h, arena_shape, FOOD_RADIUS);
        let x = point.x;
        let y = point.y;

        if let Some(index) = self.inactive_slot() {
            self.x[index] = x;
            self.y[index] = y;
            self.kind[index] = FoodKind::Meat;
            self.shape[index] = FoodShape::random(rng);
            self.phase[index] = rng.random_range(0.0..std::f32::consts::TAU);
            self.rotation[index] = rng.random_range(0.0..std::f32::consts::TAU);
            self.spin[index] = random_food_spin(rng);
            self.growth[index] = 1.0;
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
        self.active.push(true);
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

    fn inactive_slot(&self) -> Option<usize> {
        self.active.iter().position(|active| !*active)
    }

    fn has_inactive_slot(&self) -> bool {
        self.inactive_slot().is_some()
    }

    fn is_feeder_food(&self, index: usize) -> bool {
        self.feeder[index] >= 0
    }

    fn feeder_index(&self, index: usize) -> Option<usize> {
        (self.feeder[index] >= 0).then_some(self.feeder[index] as usize)
    }

    fn deactivate(&mut self, index: usize) {
        self.active[index] = false;
        self.growth[index] = 0.0;
        self.feeder[index] = -1;
        self.anchor_branch[index] = -1;
        self.anchor_angle[index] = 0.0;
        self.anchor_distance[index] = 0.0;
        self.anchor_lateral[index] = 0.0;
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

        let arena_scale = arena_w.min(arena_h).max(1_000.0);
        let titanic_radius = (arena_scale * 0.028).clamp(190.0, 760.0);

        for grower_index in 0..count {
            let titanic = grower_index == 0;
            let giant = !titanic && count > 6 && rng.random_bool(0.06);
            let radius = if titanic {
                titanic_radius
            } else if giant {
                rng.random_range(110.0..150.0)
            } else {
                rng.random_range(50.0..86.0)
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
                let position = random_point_in_arena(arena_w, arena_h, arena_shape, extent, rng);
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

    fn nearest_food(
        &self,
        x: f32,
        y: f32,
        food: &FoodStore,
        ring: i32,
    ) -> Option<(usize, f32, f32, f32)> {
        let (cx, cy) = self.grid_coords(x, y);
        let mut best = None;
        let mut best_dist_sq = f32::MAX;

        for oy in -ring..=ring {
            for ox in -ring..=ring {
                let gx = cx + ox;
                let gy = cy + oy;

                if gx < 0 || gy < 0 || gx >= self.cols as i32 || gy >= self.rows as i32 {
                    continue;
                }

                let bucket = (gy as usize * self.cols) + gx as usize;
                for &food_index in &self.buckets[bucket] {
                    let dx = food.x[food_index] - x;
                    let dy = food.y[food_index] - y;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq < best_dist_sq {
                        best_dist_sq = dist_sq;
                        best = Some((food_index, dx, dy, dist_sq.max(0.0001)));
                    }
                }
            }
        }

        best
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

pub fn species_color(_species: u8, _viability_ratio: f32) -> [f32; 4] {
    [0.74, 0.88, 1.0, 0.95]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_test_cell_soft_radius(world: &mut WorldState, index: usize, radius: f32) {
        world.cells.radius[index] = radius;
        world.cells.base_radii[index] = [radius; SOFT_BODY_POINTS];
        world.cells.current_radii[index] = [radius; SOFT_BODY_POINTS];
        world.cells.visual_radii[index] = [radius; SOFT_BODY_POINTS];
        world.cells.angle_offsets[index] = [0.0; SOFT_BODY_POINTS];
        world.cells.rebuild_soft_body_cache(index);
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
            let r = world.cells.collision_bound_radius(i);
            assert!(world.cells.x[i] >= -world.width * 0.5 + r);
            assert!(world.cells.x[i] <= world.width * 0.5 - r);
            assert!(world.cells.y[i] >= -world.height * 0.5 + r);
            assert!(world.cells.y[i] <= world.height * 0.5 - r);
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
            let point = Vec2::new(world.cells.x[i], world.cells.y[i]);
            assert!(point_inside_arena(
                point,
                world.width,
                world.height,
                world.arena_shape,
                world.cells.collision_bound_radius(i),
            ));
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
            food_growers: 1,
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
        let max_food = config.food + world.food_growers.len() * 80;

        for _ in 0..120 {
            world.update(1.0 / 60.0);
        }

        assert!(world.food.len() >= initial_food);
        assert!(world.food.len() <= max_food);
    }

    #[test]
    fn food_spawns_as_grass_and_meat() {
        let config = SimConfig {
            cells: 0,
            food: 12,
            ..default()
        };
        let world = WorldState::new(&config);

        assert!(world.food.kind.iter().any(|kind| *kind == FoodKind::Grass));
        assert!(world.food.kind.iter().any(|kind| *kind == FoodKind::Meat));
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
                assert!((size * SOFT_BODY_START_MIN_FACTOR..=size).contains(&base));
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

        world.resolve_cell_obstacles(0);

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

        world.solve_cell_collisions();

        let ray_a = world.cells.soft_ray_index_for_direction(0, Vec2::X);
        let ray_b = world.cells.soft_ray_index_for_direction(1, -Vec2::X);
        assert!(world.cells.current_radii[0][ray_a] < 8.0);
        assert!(world.cells.current_radii[1][ray_b] < 8.0);
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
        world.cells.rebuild_soft_body_cache(0);
        world.cells.rebuild_soft_body_cache(1);

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
            food_growers: 1,
            width: 2_000.0,
            height: 2_000.0,
            ..default()
        });
        let large = WorldState::new(&SimConfig {
            cells: 0,
            food: 0,
            obstacles: 0,
            food_growers: 1,
            width: 30_000.0,
            height: 20_000.0,
            ..default()
        });

        assert!(large.food_growers.radius[0] > small.food_growers.radius[0]);
        assert!(large.food_growers.extent_radius(0) > small.food_growers.extent_radius(0));
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

        let before = Vec2::new(world.cells.x[0], world.cells.y[0]).length();
        world.resolve_cell_obstacles(0);

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
        world.resolve_cell_food_growers(0);

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

        world.resolve_cell_food_growers(0);
        assert!((world.cells.y[0] - 1.0).abs() < 0.001);

        world.food_growers.branch_solid[branch] = true;
        world.resolve_cell_food_growers(0);
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
        world.cells.viability[0] = 0.0;

        world.remove_dead_cells();

        assert_eq!(world.cells.len(), 0);
        assert!(world.food.len() >= initial_food + MEAT_CHUNKS_MIN);
        assert!(
            world
                .food
                .kind
                .iter()
                .skip(initial_food)
                .any(|kind| *kind == FoodKind::Meat)
        );
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

        world.process_cell_lifecycle();

        assert_eq!(world.cells.len(), 2);
        assert!((world.cells.viability[0] - 40.0).abs() < 0.001);
        assert!((world.cells.viability[1] - 40.0).abs() < 0.001);
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
        world.cells.mutation_susceptibility[0] = MUTATION_GENE_MAX;

        world.process_cell_lifecycle();
        let child = 1;

        assert!((SPEED_GENE_MIN..=SPEED_GENE_MAX).contains(&world.cells.speed[child]));
        assert!((TURN_GENE_MIN..=TURN_GENE_MAX).contains(&world.cells.turn_speed[child]));
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

        world.process_cell_lifecycle();
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
        assert_eq!(world.cells.shape_drag_factor(0), 1.0);

        world.cells.angle_offsets[0][2] = SOFT_BODY_MAX_ANGLE_OFFSET;
        world.cells.rebuild_soft_body_cache(0);
        assert!(world.cells.shape_drag_factor(0) < 1.0);
        assert!(world.cells.turn_agility_factor(0, 0.5) > 1.0);
        assert_eq!(world.cells.turn_agility_factor(0, -0.5), 1.0);
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

        world.update(1.0 / 60.0);

        assert!(world.cells.viability[0] > 10.0);
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
    fn overlapping_cells_are_separated() {
        let config = SimConfig {
            cells: 2,
            food: 0,
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

        world.solve_cell_collisions();

        let dx = world.cells.x[1] - world.cells.x[0];
        let dy = world.cells.y[1] - world.cells.y[0];
        let dist = (dx * dx + dy * dy).sqrt();
        assert!(dist > 4.0);
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
