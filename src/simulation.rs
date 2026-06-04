use bevy::prelude::*;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use std::time::Duration;

pub const FOOD_RADIUS: f32 = 3.4;
pub const FEEDER_FOOD_SURFACE_GAP: f32 = 2.2;
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
const VIABILITY_DECAY_BASE: f32 = 0.95;
const VIABILITY_DECAY_SPEED: f32 = 0.45;
const FOOD_VIABILITY_GAIN: f32 = 18.0;
const FEEDER_FOOD_VIABILITY_GAIN: f32 = 14.0;
const MIN_VIABILITY_MOVE_FACTOR: f32 = 0.28;
const REVERSE_ALIGNMENT: f32 = -0.35;
const TURN_IN_PLACE_ANGLE: f32 = 1.35;
const FOOD_CURRENT_SPEED: f32 = 42.0;
const CELL_CURRENT_SPEED: f32 = 24.0;
const OBSTACLE_CURRENT_SPEED: f32 = 22.0;
const GROWER_CURRENT_SPEED: f32 = 10.0;
const FOOD_PUSH_STRENGTH: f32 = 58.0;
const GROWER_FOOD_PUSH_STRENGTH: f32 = 42.0;
const FOOD_SOLID_SPAWN_MARGIN: f32 = 18.0;
const FLOOR_FOOD_RATIO: f32 = 0.25;
const EMPTY_WORLD_FEEDER_FOOD_PER_GROWER: usize = 120;
const CELL_AVOIDANCE_MARGIN: f32 = 92.0;
const CELL_AVOIDANCE_STRENGTH: f32 = 1.15;
const CELL_OBSTACLE_RESTITUTION: f32 = 0.35;
const COLLISION_RESTITUTION: f32 = 0.18;
const COLLISION_PUSH: f32 = 0.54;
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

#[derive(Clone, Resource)]
pub struct SimConfig {
    pub cells: usize,
    pub food: usize,
    pub width: f32,
    pub height: f32,
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
    "Usage: organoids [--cells 10000] [--food 2000] [--width 24000] [--height 13500] [--obstacles 26] [--food-growers 4] [--seed 123] [--vsync]".to_string()
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

#[derive(Resource)]
pub struct WorldState {
    pub cells: CellStore,
    pub food: FoodStore,
    pub obstacles: ObstacleStore,
    pub food_growers: FoodGrowerStore,
    pub width: f32,
    pub height: f32,
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
        let cells = CellStore::new(config.cells, config.width, config.height, &mut rng);
        let food = FoodStore::new(floor_food_count, config.width, config.height, &mut rng);
        let obstacles = ObstacleStore::new(config.obstacles, config.width, config.height, &mut rng);
        let food_growers =
            FoodGrowerStore::new(food_grower_count, config.width, config.height, &mut rng);
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
            grid,
            cell_grid,
            rng,
            elapsed: 0.0,
            max_food: floor_food_count.saturating_add(feeder_food_capacity),
        };
        world.relocate_world_food_away_from_solids();
        world.seed_feeder_food(feeder_food_capacity);
        world.grid.rebuild(&world.food);
        world
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 1.0 / 20.0);
        self.elapsed += dt;
        self.remove_dead_cells();
        self.advect_obstacles(dt);
        self.advect_food_growers(dt);
        self.grow_food(dt);
        self.advect_food(dt);
        self.push_food_from_obstacles(dt);
        self.push_food_from_food_growers(dt);
        self.grid.rebuild(&self.food);
        self.decay_visuals(dt);

        for i in 0..self.cells.len() {
            let x = self.cells.x[i];
            let y = self.cells.y[i];
            let viability_ratio = self.cells.viability_ratio(i);
            let speed = self.cells.speed[i] * (MIN_VIABILITY_MOVE_FACTOR + viability_ratio * 0.72);
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
        let turn_step = self.cells.turn_speed[cell_index] * dt;
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
        let drive_speed = self.cells.speed[cell_index]
            * (MIN_VIABILITY_MOVE_FACTOR + self.cells.viability_ratio(cell_index) * 0.72);
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
        let half_w = self.width * 0.5;
        let half_h = self.height * 0.5;

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
            clamp_bounce_axis(
                &mut self.obstacles.x[i],
                &mut self.obstacles.vx[i],
                half_w,
                self.obstacles.radius[i],
            );
            clamp_bounce_axis(
                &mut self.obstacles.y[i],
                &mut self.obstacles.vy[i],
                half_h,
                self.obstacles.radius[i],
            );
        }
    }

    fn advect_food_growers(&mut self, dt: f32) {
        let half_w = self.width * 0.5;
        let half_h = self.height * 0.5;

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
            clamp_bounce_axis(
                &mut self.food_growers.x[i],
                &mut self.food_growers.vx[i],
                half_w,
                extent,
            );
            clamp_bounce_axis(
                &mut self.food_growers.y[i],
                &mut self.food_growers.vy[i],
                half_h,
                extent,
            );
        }

        self.food_growers.rebuild_branch_world_geometry();
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

        let branch_range = self.food_growers.branch_range(grower_index);
        if branch_range.is_empty() {
            return false;
        }

        let mut spawn = None;
        for _ in 0..12 {
            let branch_index = self.rng.random_range(branch_range.clone());
            let anchor_angle =
                self.food_growers.branch_angle[branch_index] + self.rng.random_range(-0.01..0.01);
            let angle = self.food_growers.rotation[grower_index] + anchor_angle;
            let branch_length = self.food_growers.branch_length[branch_index];
            let branch_t = self.rng.random_range(0.80..0.99);
            let distance = branch_length * branch_t;
            let branch_width = self
                .food_growers
                .branch_collision_width_at(branch_index, branch_t);
            let side = if self.rng.random_bool(0.5) { -1.0 } else { 1.0 };
            let lateral = side * (branch_width + FOOD_RADIUS + FEEDER_FOOD_SURFACE_GAP);
            let (s, c) = angle.sin_cos();
            let x = self.food_growers.x[grower_index] + c * distance - s * lateral;
            let y = self.food_growers.y[grower_index] + s * distance + c * lateral;
            let point = Vec2::new(x, y);

            if !self.point_overlaps_solid(point, FOOD_RADIUS) {
                spawn = Some((x, y, anchor_angle, distance, lateral));
                break;
            }
        }

        if let Some((x, y, anchor_angle, distance, lateral)) = spawn {
            self.food.push_feeder_at(
                grower_index as i32,
                x,
                y,
                anchor_angle,
                distance,
                lateral,
                self.width,
                self.height,
                &mut self.rng,
            );
            return true;
        }

        false
    }

    fn advect_food(&mut self, dt: f32) {
        let half_w = self.width * 0.5;
        let half_h = self.height * 0.5;

        for i in 0..self.food.len() {
            if !self.food.active[i] {
                continue;
            }

            if let Some(grower_index) = self.food.feeder_index(i) {
                if grower_index < self.food_growers.len() {
                    let angle =
                        self.food_growers.rotation[grower_index] + self.food.anchor_angle[i];
                    let (s, c) = angle.sin_cos();
                    self.food.x[i] = self.food_growers.x[grower_index]
                        + c * self.food.anchor_distance[i]
                        - s * self.food.anchor_lateral[i];
                    self.food.y[i] = self.food_growers.y[grower_index]
                        + s * self.food.anchor_distance[i]
                        + c * self.food.anchor_lateral[i];
                    self.food.growth[i] = (self.food.growth[i] + dt * 1.7).min(1.0);
                    self.food.rotation[i] += self.food.spin[i] * dt;
                } else {
                    self.food.deactivate(i);
                }
                continue;
            }

            let position = Vec2::new(self.food.x[i], self.food.y[i]);
            let current = liquid_current_at(position, self.elapsed) * FOOD_CURRENT_SPEED;
            self.food.x[i] += current.x * dt;
            self.food.y[i] += current.y * dt;
            self.food.rotation[i] += self.food.spin[i] * dt;

            wrap_axis(&mut self.food.x[i], half_w, FOOD_RADIUS);
            wrap_axis(&mut self.food.y[i], half_h, FOOD_RADIUS);
        }
    }

    fn push_food_from_obstacles(&mut self, dt: f32) {
        for obstacle_index in 0..self.obstacles.len() {
            let center = Vec2::new(
                self.obstacles.x[obstacle_index],
                self.obstacles.y[obstacle_index],
            );
            let radius = self.obstacles.radius[obstacle_index] + FOOD_RADIUS + 18.0;

            for food_index in 0..self.food.len() {
                if !self.food.active[food_index] || self.food.is_feeder_food(food_index) {
                    continue;
                }

                let delta = Vec2::new(self.food.x[food_index], self.food.y[food_index]) - center;
                let dist_sq = delta.length_squared();
                if dist_sq >= radius * radius {
                    continue;
                }

                let dir = if dist_sq > 0.0001 {
                    delta * dist_sq.sqrt().recip()
                } else {
                    Vec2::X
                };
                let push = (1.0 - dist_sq.sqrt() / radius).clamp(0.0, 1.0);
                self.food.x[food_index] += dir.x * push * FOOD_PUSH_STRENGTH * dt;
                self.food.y[food_index] += dir.y * push * FOOD_PUSH_STRENGTH * dt;
            }
        }
    }

    fn push_food_from_food_growers(&mut self, dt: f32) {
        for grower_index in 0..self.food_growers.len() {
            let center = Vec2::new(
                self.food_growers.x[grower_index],
                self.food_growers.y[grower_index],
            );
            let radius = self.food_growers.radius[grower_index] + FOOD_RADIUS + 20.0;

            for food_index in 0..self.food.len() {
                if !self.food.active[food_index] || self.food.is_feeder_food(food_index) {
                    continue;
                }

                let delta = Vec2::new(self.food.x[food_index], self.food.y[food_index]) - center;
                let dist_sq = delta.length_squared();
                if dist_sq >= radius * radius {
                    continue;
                }

                let dir = if dist_sq > 0.0001 {
                    delta * dist_sq.sqrt().recip()
                } else {
                    Vec2::X
                };
                let push = (1.0 - dist_sq.sqrt() / radius).clamp(0.0, 1.0);
                self.food.x[food_index] += dir.x * push * GROWER_FOOD_PUSH_STRENGTH * dt;
                self.food.y[food_index] += dir.y * push * GROWER_FOOD_PUSH_STRENGTH * dt;
            }
        }
    }

    fn resolve_cell_obstacles(&mut self, cell_index: usize) {
        for obstacle_index in 0..self.obstacles.len() {
            let dx = self.cells.x[cell_index] - self.obstacles.x[obstacle_index];
            let dy = self.cells.y[cell_index] - self.obstacles.y[obstacle_index];
            let min_dist = self.cells.collision_bound_radius(cell_index)
                + self.obstacles.radius[obstacle_index];
            let dist_sq = dx * dx + dy * dy;
            if dist_sq >= min_dist * min_dist {
                continue;
            }

            let (nx, ny, dist) = if dist_sq > 0.0001 {
                let dist = dist_sq.sqrt();
                (dx / dist, dy / dist, dist)
            } else {
                (1.0, 0.0, 0.001)
            };
            let push = min_dist - dist;
            self.cells.x[cell_index] += nx * push;
            self.cells.y[cell_index] += ny * push;

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
        let cell_radius = self.cells.collision_bound_radius(cell_index);

        for grower_index in 0..self.food_growers.len() {
            let grower_x = self.food_growers.x[grower_index];
            let grower_y = self.food_growers.y[grower_index];
            let dx = cell_x - grower_x;
            let dy = cell_y - grower_y;
            let extent = self.food_growers.extent_radius(grower_index) + cell_radius;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq >= extent * extent {
                continue;
            }

            let min_dist = cell_radius + self.food_growers.radius[grower_index];
            if dist_sq < min_dist * min_dist {
                let (nx, ny, dist) = if dist_sq > 0.0001 {
                    let dist = dist_sq.sqrt();
                    (dx / dist, dy / dist, dist)
                } else {
                    (1.0, 0.0, 0.001)
                };
                self.push_cell_from_grower_surface(cell_index, nx, ny, dist, min_dist);
                cell_x = self.cells.x[cell_index];
                cell_y = self.cells.y[cell_index];
            }

            for branch_index in self.food_growers.branch_range(grower_index) {
                let ax = self.food_growers.branch_start_x[branch_index];
                let ay = self.food_growers.branch_start_y[branch_index];
                let bx = self.food_growers.branch_end_x[branch_index];
                let by = self.food_growers.branch_end_y[branch_index];
                let sx = bx - ax;
                let sy = by - ay;
                let segment_len_sq = (sx * sx + sy * sy).max(0.0001);
                let t =
                    (((cell_x - ax) * sx + (cell_y - ay) * sy) / segment_len_sq).clamp(0.0, 1.0);
                let closest_x = ax + sx * t;
                let closest_y = ay + sy * t;
                let dx = cell_x - closest_x;
                let dy = cell_y - closest_y;
                let branch_width =
                    self.food_growers.branch_width[branch_index] * (1.28 + (0.58 - 1.28) * t);
                let min_dist = cell_radius + branch_width;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq >= min_dist * min_dist {
                    continue;
                }

                let (nx, ny, dist) = if dist_sq > 0.0001 {
                    let dist = dist_sq.sqrt();
                    (dx / dist, dy / dist, dist)
                } else {
                    let inv_len = segment_len_sq.sqrt().recip();
                    (-sy * inv_len, sx * inv_len, 0.001)
                };
                self.push_cell_from_grower_surface(cell_index, nx, ny, dist, min_dist);
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
    ) {
        let push = min_dist - dist;
        self.cells.x[cell_index] += nx * push;
        self.cells.y[cell_index] += ny * push;

        let into_grower = self.cells.vx[cell_index] * nx + self.cells.vy[cell_index] * ny;
        if into_grower < 0.0 {
            self.cells.vx[cell_index] -= into_grower * nx * (1.0 + CELL_OBSTACLE_RESTITUTION);
            self.cells.vy[cell_index] -= into_grower * ny * (1.0 + CELL_OBSTACLE_RESTITUTION);
            self.cells.jelly_intensity[cell_index] =
                (self.cells.jelly_intensity[cell_index] + 0.35).min(1.0);
            self.cells.jelly_dir_x[cell_index] = nx;
            self.cells.jelly_dir_y[cell_index] = ny;
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
        self.food.anchor_angle[food_index] = 0.0;
        self.food.anchor_distance[food_index] = 0.0;
        self.food.anchor_lateral[food_index] = 0.0;
    }

    fn safe_random_food_position(&mut self) -> Vec2 {
        let half_w = self.width * 0.5 - FOOD_RADIUS;
        let half_h = self.height * 0.5 - FOOD_RADIUS;

        for _ in 0..96 {
            let point = Vec2::new(
                self.rng.random_range(-half_w..half_w),
                self.rng.random_range(-half_h..half_h),
            );
            if !self.point_overlaps_solid(point, FOOD_RADIUS + FOOD_SOLID_SPAWN_MARGIN) {
                return point;
            }
        }

        Vec2::new(
            self.rng.random_range(-half_w..half_w),
            self.rng.random_range(-half_h..half_h),
        )
    }

    fn point_overlaps_solid(&self, point: Vec2, radius: f32) -> bool {
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
                let a = Vec2::new(
                    self.food_growers.branch_start_x[branch_index],
                    self.food_growers.branch_start_y[branch_index],
                );
                let b = Vec2::new(
                    self.food_growers.branch_end_x[branch_index],
                    self.food_growers.branch_end_y[branch_index],
                );
                let segment = b - a;
                let segment_len_sq = segment.length_squared().max(0.0001);
                let t = ((point - a).dot(segment) / segment_len_sq).clamp(0.0, 1.0);
                let closest = a + segment * t;
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
        let speed = self.cells.speed[cell_index];
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
                let a = Vec2::new(
                    self.food_growers.branch_start_x[branch_index],
                    self.food_growers.branch_start_y[branch_index],
                );
                let b = Vec2::new(
                    self.food_growers.branch_end_x[branch_index],
                    self.food_growers.branch_end_y[branch_index],
                );
                let segment = b - a;
                let segment_len_sq = segment.length_squared().max(0.0001);
                let t = ((position - a).dot(segment) / segment_len_sq).clamp(0.0, 1.0);
                let closest = a + segment * t;
                let influence = self.food_growers.branch_collision_width_at(branch_index, t)
                    + cell_radius
                    + CELL_AVOIDANCE_MARGIN * 0.72;
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
            let drain = (VIABILITY_DECAY_BASE + speed_cost * VIABILITY_DECAY_SPEED) * dt;
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
            self.food
                .push_meat_at(point.x, point.y, self.width, self.height, &mut self.rng);
        }
    }

    pub fn cell_index_by_id(&self, cell_id: u64) -> Option<usize> {
        self.cells.id.iter().position(|id| *id == cell_id)
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

        let (nx, ny, dist) = if dist_sq > 0.0001 {
            let dist = dist_sq.sqrt();
            (dx / dist, dy / dist, dist)
        } else {
            let angle = ((a as f32 * 12.9898 + b as f32 * 78.233).sin()) * std::f32::consts::TAU;
            let (ny, nx) = angle.sin_cos();
            (nx, ny, 0.001)
        };
        let angle = ny.atan2(nx);
        let radius_a = self.cells.radius[a] * self.cells.shape_radius_at(a, angle);
        let radius_b =
            self.cells.radius[b] * self.cells.shape_radius_at(b, angle + std::f32::consts::PI);
        let min_dist = radius_a + radius_b;

        if dist_sq >= min_dist * min_dist {
            return;
        }

        let overlap = min_dist - dist;
        let push = overlap * COLLISION_PUSH;
        self.cells.x[a] -= nx * push;
        self.cells.y[a] -= ny * push;
        self.cells.x[b] += nx * push;
        self.cells.y[b] += ny * push;

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
        let half_w = self.width * 0.5;
        let half_h = self.height * 0.5;
        let r = self.cells.collision_bound_radius(i);
        let mut bounced = false;

        if self.cells.x[i] < -half_w + r {
            self.cells.x[i] = -half_w + r;
            self.cells.vx[i] = self.cells.vx[i].abs();
            bounced = true;
        } else if self.cells.x[i] > half_w - r {
            self.cells.x[i] = half_w - r;
            self.cells.vx[i] = -self.cells.vx[i].abs();
            bounced = true;
        }

        if self.cells.y[i] < -half_h + r {
            self.cells.y[i] = -half_h + r;
            self.cells.vy[i] = self.cells.vy[i].abs();
            bounced = true;
        } else if self.cells.y[i] > half_h - r {
            self.cells.y[i] = half_h - r;
            self.cells.vy[i] = -self.cells.vy[i].abs();
            bounced = true;
        }

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

fn wrap_axis(value: &mut f32, half_extent: f32, margin: f32) {
    let min = -half_extent + margin;
    let max = half_extent - margin;

    if *value < min {
        *value = max;
    } else if *value > max {
        *value = min;
    }
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
    fn new(count: usize, arena_w: f32, arena_h: f32, rng: &mut SmallRng) -> Self {
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

            store
                .x
                .push(rng.random_range((-arena_w * 0.5 + radius)..(arena_w * 0.5 - radius)));
            store
                .y
                .push(rng.random_range((-arena_h * 0.5 + radius)..(arena_h * 0.5 - radius)));
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
        rng: &mut SmallRng,
    ) {
        let parent_heading = self.heading[parent_index];
        let side = if rng.random_bool(0.5) { -1.0 } else { 1.0 };
        let offset_angle = parent_heading + side * std::f32::consts::FRAC_PI_2;
        let (offset_s, offset_c) = offset_angle.sin_cos();
        let radius = self.radius[parent_index];
        let offset = radius * DIVISION_CHILD_OFFSET;
        let half_w = arena_w * 0.5 - radius;
        let half_h = arena_h * 0.5 - radius;
        let x = (self.x[parent_index] + offset_c * offset).clamp(-half_w, half_w);
        let y = (self.y[parent_index] + offset_s * offset).clamp(-half_h, half_h);
        let susceptibility = self.mutation_susceptibility[parent_index];

        self.id.push(self.next_id);
        self.next_id += 1;
        self.x.push(x);
        self.y.push(y);
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
        self.mutation_susceptibility.push(mutate_gene(
            self.mutation_susceptibility[parent_index],
            MUTATION_GENE_MIN,
            MUTATION_GENE_MAX,
            susceptibility,
            rng,
        ));
        self.division_threshold.push(mutate_gene(
            self.division_threshold[parent_index],
            DIVISION_THRESHOLD_MIN,
            DIVISION_THRESHOLD_MAX,
            susceptibility,
            rng,
        ));
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

    pub fn shape_radius_at(&self, index: usize, angle: f32) -> f32 {
        let wave_a = self.shape_wave_a[index];
        let wave_b = self.shape_wave_b[index];
        let phase = self.shape_phase[index];
        let radius =
            1.0 + wave_a * (angle * 3.0 + phase).sin() + wave_b * (angle * 5.0 - phase * 0.7).sin();

        radius.clamp(0.55, 1.0)
    }

    fn collision_bound_radius(&self, index: usize) -> f32 {
        self.radius[index] * (1.0 + self.shape_wave_a[index].abs() + self.shape_wave_b[index].abs())
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
    pub anchor_angle: Vec<f32>,
    pub anchor_distance: Vec<f32>,
    pub anchor_lateral: Vec<f32>,
}

impl FoodStore {
    fn new(count: usize, arena_w: f32, arena_h: f32, rng: &mut SmallRng) -> Self {
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
            store.push_random(arena_w, arena_h, kind, rng);
        }

        store
    }

    fn push_random(&mut self, arena_w: f32, arena_h: f32, kind: FoodKind, rng: &mut SmallRng) {
        self.x
            .push(rng.random_range((-arena_w * 0.5 + FOOD_RADIUS)..(arena_w * 0.5 - FOOD_RADIUS)));
        self.y
            .push(rng.random_range((-arena_h * 0.5 + FOOD_RADIUS)..(arena_h * 0.5 - FOOD_RADIUS)));
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
        self.anchor_angle.push(0.0);
        self.anchor_distance.push(0.0);
        self.anchor_lateral.push(0.0);
    }

    fn push_feeder_at(
        &mut self,
        grower_index: i32,
        x: f32,
        y: f32,
        anchor_angle: f32,
        anchor_distance: f32,
        anchor_lateral: f32,
        arena_w: f32,
        arena_h: f32,
        rng: &mut SmallRng,
    ) {
        let half_w = arena_w * 0.5 - FOOD_RADIUS;
        let half_h = arena_h * 0.5 - FOOD_RADIUS;
        let x = x.clamp(-half_w, half_w);
        let y = y.clamp(-half_h, half_h);

        if let Some(index) = self.inactive_slot() {
            self.x[index] = x;
            self.y[index] = y;
            self.kind[index] = FoodKind::Grass;
            self.shape[index] = FoodShape::random_feeder_food(rng);
            self.phase[index] = rng.random_range(0.0..std::f32::consts::TAU);
            self.rotation[index] = rng.random_range(0.0..std::f32::consts::TAU);
            self.spin[index] = random_food_spin(rng);
            self.growth[index] = 0.24;
            self.active[index] = true;
            self.feeder[index] = grower_index;
            self.anchor_angle[index] = anchor_angle;
            self.anchor_distance[index] = anchor_distance;
            self.anchor_lateral[index] = anchor_lateral;
            return;
        }

        self.x.push(x);
        self.y.push(y);
        self.kind.push(FoodKind::Grass);
        self.shape.push(FoodShape::random_feeder_food(rng));
        self.phase
            .push(rng.random_range(0.0..std::f32::consts::TAU));
        self.rotation
            .push(rng.random_range(0.0..std::f32::consts::TAU));
        self.spin.push(random_food_spin(rng));
        self.growth.push(0.24);
        self.active.push(true);
        self.feeder.push(grower_index);
        self.anchor_angle.push(anchor_angle);
        self.anchor_distance.push(anchor_distance);
        self.anchor_lateral.push(anchor_lateral);
    }

    fn push_meat_at(&mut self, x: f32, y: f32, arena_w: f32, arena_h: f32, rng: &mut SmallRng) {
        let half_w = arena_w * 0.5 - FOOD_RADIUS;
        let half_h = arena_h * 0.5 - FOOD_RADIUS;
        let x = x.clamp(-half_w, half_w);
        let y = y.clamp(-half_h, half_h);

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
    fn new(count: usize, arena_w: f32, arena_h: f32, rng: &mut SmallRng) -> Self {
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
            let half_w = arena_w * 0.5 - radius;
            let half_h = arena_h * 0.5 - radius;
            store.x.push(rng.random_range(-half_w..half_w));
            store.y.push(rng.random_range(-half_h..half_h));
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
    pub branch_phase: Vec<f32>,
    pub branch_world_angle: Vec<f32>,
    pub branch_start_x: Vec<f32>,
    pub branch_start_y: Vec<f32>,
    pub branch_end_x: Vec<f32>,
    pub branch_end_y: Vec<f32>,
    pub extent: Vec<f32>,
    pub timer: Vec<f32>,
    pub interval: Vec<f32>,
}

impl FoodGrowerStore {
    fn new(count: usize, arena_w: f32, arena_h: f32, rng: &mut SmallRng) -> Self {
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
            branch_phase: Vec::with_capacity(count.saturating_mul(10)),
            branch_world_angle: Vec::with_capacity(count.saturating_mul(10)),
            branch_start_x: Vec::with_capacity(count.saturating_mul(10)),
            branch_start_y: Vec::with_capacity(count.saturating_mul(10)),
            branch_end_x: Vec::with_capacity(count.saturating_mul(10)),
            branch_end_y: Vec::with_capacity(count.saturating_mul(10)),
            extent: Vec::with_capacity(count),
            timer: Vec::with_capacity(count),
            interval: Vec::with_capacity(count),
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
            let half_w = (arena_w * 0.5 - extent).max(1.0);
            let half_h = (arena_h * 0.5 - extent).max(1.0);
            if titanic {
                store.x.push(0.0_f32.clamp(-half_w, half_w));
                store.y.push(0.0_f32.clamp(-half_h, half_h));
                store.vx.push(rng.random_range(-0.45..0.45));
                store.vy.push(rng.random_range(-0.45..0.45));
            } else {
                store.x.push(rng.random_range(-half_w..half_w));
                store.y.push(rng.random_range(-half_h..half_h));
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
                let curve_range = if titanic { -0.55..0.55 } else { -0.42..0.42 };
                store.branch_curve.push(rng.random_range(curve_range));
                store
                    .branch_phase
                    .push(rng.random_range(0.0..std::f32::consts::TAU));
                store.branch_world_angle.push(0.0);
                store.branch_start_x.push(0.0);
                store.branch_start_y.push(0.0);
                store.branch_end_x.push(0.0);
                store.branch_end_y.push(0.0);
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

        store.rebuild_branch_world_geometry();
        store
    }

    pub fn len(&self) -> usize {
        self.x.len()
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
        self.branch_width[branch_index] * (1.28 + (0.58 - 1.28) * t)
    }

    pub fn total_branches(&self) -> usize {
        self.branch_angle.len()
    }

    pub fn rebuild_branch_world_geometry(&mut self) {
        for grower_index in 0..self.len() {
            let center_x = self.x[grower_index];
            let center_y = self.y[grower_index];
            let start_distance = self.radius[grower_index] * 0.56;
            let start = self.branch_start[grower_index];
            let end = start + self.branch_count[grower_index];

            for branch_index in start..end {
                let angle = self.rotation[grower_index] + self.branch_angle[branch_index];
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
            let r = world.cells.radius[i];
            assert!(world.cells.x[i] >= -world.width * 0.5 + r);
            assert!(world.cells.x[i] <= world.width * 0.5 - r);
            assert!(world.cells.y[i] >= -world.height * 0.5 + r);
            assert!(world.cells.y[i] <= world.height * 0.5 - r);
        }
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
            assert!(!world.point_overlaps_solid(point, FOOD_RADIUS));
        }
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
        world.cells.radius[0] = 8.0;

        world.resolve_cell_obstacles(0);

        let dist = Vec2::new(world.cells.x[0], world.cells.y[0]).length();
        assert!(dist >= 88.0);
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
        world.cells.radius[0] = 8.0;

        world.resolve_cell_food_growers(0);

        let dist = Vec2::new(world.cells.x[0], world.cells.y[0]).length();
        assert!(dist >= 88.0);
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
    fn mutation_susceptibility_controls_mutation_parameters() {
        assert!(mutation_chance(100.0) > mutation_chance(0.0));
        assert!(mutation_power(100.0) > mutation_power(0.0));
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
        world.cells.viability[0] = 0.0;

        world.remove_dead_cells();

        let selected_index = world
            .cell_index_by_id(selected_id)
            .expect("selected id remains");
        assert_eq!(world.cells.id[selected_index], selected_id);
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
        world.cells.x[0] = 0.0;
        world.cells.y[0] = 0.0;
        world.cells.viability[0] = 10.0;

        for i in 0..world.food.len() {
            world.food.active[i] = false;
        }
        world.food.active[0] = true;
        world.food.x[0] = 0.0;
        world.food.y[0] = 0.0;
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
        world.cells.radius[0] = 8.0;
        world.cells.radius[1] = 8.0;
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
