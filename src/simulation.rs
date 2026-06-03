use bevy::prelude::*;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use std::time::Duration;

pub const DEFAULT_CELLS: usize = 10_000;
pub const DEFAULT_FOOD: usize = 2_000;
pub const ARENA_WIDTH: f32 = 24_000.0;
pub const ARENA_HEIGHT: f32 = 13_500.0;
pub const FOOD_RADIUS: f32 = 3.4;
const GRID_CELL_SIZE: f32 = 240.0;
const CELL_GRID_SIZE: f32 = 96.0;
const SEARCH_RING: i32 = 2;
const STEER_GAIN: f32 = 9.5;
const DRAG: f32 = 0.985;
const WANDER_GAIN: f32 = 0.45;
const COLLISION_RESTITUTION: f32 = 0.18;
const COLLISION_PUSH: f32 = 0.54;
const JELLY_DECAY: f32 = 2.8;
const JELLY_HIT_GAIN: f32 = 0.42;

#[derive(Clone, Resource)]
pub struct SimConfig {
    pub cells: usize,
    pub food: usize,
    pub seed: u64,
    pub vsync: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            cells: DEFAULT_CELLS,
            food: DEFAULT_FOOD,
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
    "Usage: organoids [--cells 10000] [--food 2000] [--seed 123] [--vsync]".to_string()
}

#[derive(Resource)]
pub struct WorldState {
    pub cells: CellStore,
    pub food: FoodStore,
    grid: SpatialGrid,
    cell_grid: CellGrid,
    rng: SmallRng,
}

impl WorldState {
    pub fn new(config: &SimConfig) -> Self {
        let mut rng = SmallRng::seed_from_u64(config.seed);
        let cells = CellStore::new(config.cells, &mut rng);
        let food = FoodStore::new(config.food, &mut rng);
        let mut grid = SpatialGrid::new(ARENA_WIDTH, ARENA_HEIGHT, GRID_CELL_SIZE);
        grid.rebuild(&food);
        let mut cell_grid = CellGrid::new(ARENA_WIDTH, ARENA_HEIGHT, CELL_GRID_SIZE);
        cell_grid.rebuild(&cells);

        Self {
            cells,
            food,
            grid,
            cell_grid,
            rng,
        }
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 1.0 / 20.0);
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

            let steer = (STEER_GAIN * dt).clamp(0.0, 1.0);
            self.cells.vx[i] = (self.cells.vx[i] + (desired_x - self.cells.vx[i]) * steer) * DRAG;
            self.cells.vy[i] = (self.cells.vy[i] + (desired_y - self.cells.vy[i]) * steer) * DRAG;

            self.cells.x[i] += self.cells.vx[i] * dt;
            self.cells.y[i] += self.cells.vy[i] * dt;

            self.bounce_cell(i);

            if let Some((food_index, dist_sq)) = target_food {
                let eat_radius = self.cells.collision_bound_radius(i) + FOOD_RADIUS;
                if dist_sq <= eat_radius * eat_radius {
                    self.food.respawn(food_index, &mut self.rng);
                    self.cells.energy[i] = (self.cells.energy[i] + 1.0).min(255.0);
                }
            }
        }

        self.solve_cell_collisions();
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
        let half_w = ARENA_WIDTH * 0.5;
        let half_h = ARENA_HEIGHT * 0.5;
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
    fn new(count: usize, rng: &mut SmallRng) -> Self {
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

            store.x.push(
                rng.random_range((-ARENA_WIDTH * 0.5 + radius)..(ARENA_WIDTH * 0.5 - radius)),
            );
            store.y.push(
                rng.random_range((-ARENA_HEIGHT * 0.5 + radius)..(ARENA_HEIGHT * 0.5 - radius)),
            );
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
        let gx = ((x + ARENA_WIDTH * 0.5) / self.cell_size)
            .floor()
            .clamp(0.0, self.cols as f32 - 1.0) as usize;
        let gy = ((y + ARENA_HEIGHT * 0.5) / self.cell_size)
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
}

impl FoodStore {
    fn new(count: usize, rng: &mut SmallRng) -> Self {
        let mut store = Self {
            x: Vec::with_capacity(count),
            y: Vec::with_capacity(count),
        };

        for _ in 0..count {
            store.push_random(rng);
        }

        store
    }

    fn push_random(&mut self, rng: &mut SmallRng) {
        self.x.push(
            rng.random_range((-ARENA_WIDTH * 0.5 + FOOD_RADIUS)..(ARENA_WIDTH * 0.5 - FOOD_RADIUS)),
        );
        self.y.push(
            rng.random_range(
                (-ARENA_HEIGHT * 0.5 + FOOD_RADIUS)..(ARENA_HEIGHT * 0.5 - FOOD_RADIUS),
            ),
        );
    }

    fn respawn(&mut self, index: usize, rng: &mut SmallRng) {
        self.x[index] =
            rng.random_range((-ARENA_WIDTH * 0.5 + FOOD_RADIUS)..(ARENA_WIDTH * 0.5 - FOOD_RADIUS));
        self.y[index] = rng
            .random_range((-ARENA_HEIGHT * 0.5 + FOOD_RADIUS)..(ARENA_HEIGHT * 0.5 - FOOD_RADIUS));
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }
}

pub struct SpatialGrid {
    cols: usize,
    rows: usize,
    cell_size: f32,
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
            buckets,
        }
    }

    fn rebuild(&mut self, food: &FoodStore) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }

        for i in 0..food.len() {
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
        let gx = ((x + ARENA_WIDTH * 0.5) / self.cell_size)
            .floor()
            .clamp(0.0, self.cols as f32 - 1.0) as i32;
        let gy = ((y + ARENA_HEIGHT * 0.5) / self.cell_size)
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
            assert!(world.cells.x[i] >= -ARENA_WIDTH * 0.5 + r);
            assert!(world.cells.x[i] <= ARENA_WIDTH * 0.5 - r);
            assert!(world.cells.y[i] >= -ARENA_HEIGHT * 0.5 + r);
            assert!(world.cells.y[i] <= ARENA_HEIGHT * 0.5 - r);
        }
    }

    #[test]
    fn food_count_is_constant() {
        let config = SimConfig {
            cells: 500,
            food: 50,
            ..default()
        };
        let mut world = WorldState::new(&config);
        let initial_food = world.food.len();

        for _ in 0..120 {
            world.update(1.0 / 60.0);
        }

        assert_eq!(world.food.len(), initial_food);
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
