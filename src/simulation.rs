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
const FOOD_CURRENT_SPEED: f32 = 42.0;
const CELL_CURRENT_SPEED: f32 = 24.0;
const OBSTACLE_CURRENT_SPEED: f32 = 22.0;
const GROWER_CURRENT_SPEED: f32 = 10.0;
const FOOD_PUSH_STRENGTH: f32 = 58.0;
const GROWER_FOOD_PUSH_STRENGTH: f32 = 42.0;
const FOOD_SOLID_SPAWN_MARGIN: f32 = 18.0;
const CELL_AVOIDANCE_MARGIN: f32 = 92.0;
const CELL_AVOIDANCE_STRENGTH: f32 = 1.15;
const CELL_OBSTACLE_RESTITUTION: f32 = 0.35;
const COLLISION_RESTITUTION: f32 = 0.18;
const COLLISION_PUSH: f32 = 0.54;
const JELLY_DECAY: f32 = 2.8;
const JELLY_HIT_GAIN: f32 = 0.42;

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
        let cells = CellStore::new(config.cells, config.width, config.height, &mut rng);
        let food = FoodStore::new(config.food, config.width, config.height, &mut rng);
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
            max_food: config
                .food
                .saturating_add(food_grower_count.saturating_mul(80))
                .max(config.food),
        };
        world.relocate_world_food_away_from_solids();
        world.grid.rebuild(&world.food);
        world
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 1.0 / 20.0);
        self.elapsed += dt;
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
            let speed = self.cells.speed[i];

            let mut target_food = None;

            let (desired_x, desired_y) = if let Some((food_index, dx, dy, dist_sq)) =
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
            let steer = (STEER_GAIN * dt).clamp(0.0, 1.0);
            self.cells.vx[i] = (self.cells.vx[i]
                + (desired_velocity.x + current.x - self.cells.vx[i]) * steer)
                * DRAG;
            self.cells.vy[i] = (self.cells.vy[i]
                + (desired_velocity.y + current.y - self.cells.vy[i]) * steer)
                * DRAG;

            self.cells.x[i] += self.cells.vx[i] * dt;
            self.cells.y[i] += self.cells.vy[i] * dt;

            self.bounce_cell(i);
            self.resolve_cell_obstacles(i);
            self.resolve_cell_food_growers(i);

            if let Some((food_index, dist_sq)) = target_food {
                let eat_radius = self.cells.collision_bound_radius(i) + FOOD_RADIUS;
                if dist_sq <= eat_radius * eat_radius {
                    if self.food.is_feeder_food(food_index) {
                        self.food.deactivate(food_index);
                    } else {
                        self.respawn_world_food(food_index);
                    }
                    self.cells.energy[i] = (self.cells.energy[i] + 1.0).min(255.0);
                }
            }
        }

        self.solve_cell_collisions();
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

            let mut spawn = None;
            for _ in 0..12 {
                let branch_index = self.rng.random_range(branch_range.clone());
                let anchor_angle = self.food_growers.branch_angle[branch_index]
                    + self.rng.random_range(-0.01..0.01);
                let angle = self.food_growers.rotation[i] + anchor_angle;
                let branch_length = self.food_growers.branch_length[branch_index];
                let branch_t = self.rng.random_range(0.80..0.99);
                let distance = branch_length * branch_t;
                let branch_width = self
                    .food_growers
                    .branch_collision_width_at(branch_index, branch_t);
                let side = if self.rng.random_bool(0.5) { -1.0 } else { 1.0 };
                let lateral = side * (branch_width + FOOD_RADIUS + FEEDER_FOOD_SURFACE_GAP);
                let (s, c) = angle.sin_cos();
                let x = self.food_growers.x[i] + c * distance - s * lateral;
                let y = self.food_growers.y[i] + s * distance + c * lateral;
                let point = Vec2::new(x, y);

                if !self.point_overlaps_solid(point, FOOD_RADIUS) {
                    spawn = Some((x, y, anchor_angle, distance, lateral));
                    break;
                }
            }

            if let Some((x, y, anchor_angle, distance, lateral)) = spawn {
                self.food.push_feeder_at(
                    i as i32,
                    x,
                    y,
                    anchor_angle,
                    distance,
                    lateral,
                    self.width,
                    self.height,
                    &mut self.rng,
                );
            }
        }
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
        for grower_index in 0..self.food_growers.len() {
            let grower_x = self.food_growers.x[grower_index];
            let grower_y = self.food_growers.y[grower_index];
            let dx = self.cells.x[cell_index] - grower_x;
            let dy = self.cells.y[cell_index] - grower_y;
            let min_dist = self.cells.collision_bound_radius(cell_index)
                + self.food_growers.radius[grower_index];
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < min_dist * min_dist {
                let (nx, ny, dist) = if dist_sq > 0.0001 {
                    let dist = dist_sq.sqrt();
                    (dx / dist, dy / dist, dist)
                } else {
                    (1.0, 0.0, 0.001)
                };
                self.push_cell_from_grower_surface(cell_index, nx, ny, dist, min_dist);
            }

            for branch_index in self.food_growers.branch_range(grower_index) {
                let angle = self.food_growers.rotation[grower_index]
                    + self.food_growers.branch_angle[branch_index];
                let (s, c) = angle.sin_cos();
                let start_distance = self.food_growers.radius[grower_index] * 0.56;
                let end_distance = self.food_growers.branch_length[branch_index];
                let ax = grower_x + c * start_distance;
                let ay = grower_y + s * start_distance;
                let bx = grower_x + c * end_distance;
                let by = grower_y + s * end_distance;
                let sx = bx - ax;
                let sy = by - ay;
                let segment_len_sq = (sx * sx + sy * sy).max(0.0001);
                let t = (((self.cells.x[cell_index] - ax) * sx
                    + (self.cells.y[cell_index] - ay) * sy)
                    / segment_len_sq)
                    .clamp(0.0, 1.0);
                let closest_x = ax + sx * t;
                let closest_y = ay + sy * t;
                let dx = self.cells.x[cell_index] - closest_x;
                let dy = self.cells.y[cell_index] - closest_y;
                let branch_width =
                    self.food_growers.branch_width[branch_index] * (1.28 + (0.58 - 1.28) * t);
                let min_dist = self.cells.collision_bound_radius(cell_index) + branch_width;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq >= min_dist * min_dist {
                    continue;
                }

                let (nx, ny, dist) = if dist_sq > 0.0001 {
                    let dist = dist_sq.sqrt();
                    (dx / dist, dy / dist, dist)
                } else {
                    (-s, c, 0.001)
                };
                self.push_cell_from_grower_surface(cell_index, nx, ny, dist, min_dist);
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
            if point.distance_squared(center) < min_dist * min_dist {
                return true;
            }

            for branch_index in self.food_growers.branch_range(grower_index) {
                let angle = self.food_growers.rotation[grower_index]
                    + self.food_growers.branch_angle[branch_index];
                let (s, c) = angle.sin_cos();
                let start_distance = self.food_growers.radius[grower_index] * 0.56;
                let end_distance = self.food_growers.branch_length[branch_index];
                let a = center + Vec2::new(c, s) * start_distance;
                let b = center + Vec2::new(c, s) * end_distance;
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
            let influence =
                self.food_growers.radius[grower_index] + cell_radius + CELL_AVOIDANCE_MARGIN;
            add_avoidance(
                &mut avoidance,
                position - center,
                influence,
                desired_dir,
                speed,
            );

            for branch_index in self.food_growers.branch_range(grower_index) {
                let angle = self.food_growers.rotation[grower_index]
                    + self.food_growers.branch_angle[branch_index];
                let (s, c) = angle.sin_cos();
                let start_distance = self.food_growers.radius[grower_index] * 0.56;
                let end_distance = self.food_growers.branch_length[branch_index];
                let a = center + Vec2::new(c, s) * start_distance;
                let b = center + Vec2::new(c, s) * end_distance;
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

        if self.cells.x[i] < -half_w + r {
            self.cells.x[i] = -half_w + r;
            self.cells.vx[i] = self.cells.vx[i].abs();
        } else if self.cells.x[i] > half_w - r {
            self.cells.x[i] = half_w - r;
            self.cells.vx[i] = -self.cells.vx[i].abs();
        }

        if self.cells.y[i] < -half_h + r {
            self.cells.y[i] = -half_h + r;
            self.cells.vy[i] = self.cells.vy[i].abs();
        } else if self.cells.y[i] > half_h - r {
            self.cells.y[i] = half_h - r;
            self.cells.vy[i] = -self.cells.vy[i].abs();
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

pub struct CellStore {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub radius: Vec<f32>,
    pub speed: Vec<f32>,
    pub species: Vec<u8>,
    pub energy: Vec<f32>,
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
            x: Vec::with_capacity(count),
            y: Vec::with_capacity(count),
            vx: Vec::with_capacity(count),
            vy: Vec::with_capacity(count),
            radius: Vec::with_capacity(count),
            speed: Vec::with_capacity(count),
            species: Vec::with_capacity(count),
            energy: Vec::with_capacity(count),
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
            store.radius.push(radius);
            store.speed.push(speed);
            store.species.push(rng.random_range(0..6));
            store.energy.push(0.0);
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
        }

        store
    }

    pub fn len(&self) -> usize {
        self.x.len()
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

            for branch_index in 0..branch_count {
                let branch_step = std::f32::consts::TAU / branch_count as f32;
                let angle = branch_step * branch_index as f32 + rng.random_range(-0.16..0.16);
                store.branch_angle.push(angle);
                let length_variance = if titanic { 0.86..1.08 } else { 0.88..1.02 };
                store
                    .branch_length
                    .push(extent * rng.random_range(length_variance));
                let width_range = if titanic { 0.052..0.086 } else { 0.065..0.11 };
                store
                    .branch_width
                    .push(radius * rng.random_range(width_range));
                let curve_range = if titanic { -0.55..0.55 } else { -0.42..0.42 };
                store.branch_curve.push(rng.random_range(curve_range));
                store
                    .branch_phase
                    .push(rng.random_range(0.0..std::f32::consts::TAU));
            }
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

        store
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn extent_radius(&self, index: usize) -> f32 {
        self.branch_range(index)
            .map(|branch| self.branch_length[branch] + self.branch_width[branch])
            .fold(self.radius[index], f32::max)
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

pub fn species_color(_species: u8, _energy: f32) -> [f32; 4] {
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
