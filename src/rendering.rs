use crate::simulation::{
    ArenaShape, CellTargetKind, FOOD_RADIUS, FoodKind, FrameStats, GRASS_FOOD_COLOR,
    LIQUID_CAUSTIC_STRENGTH, LIQUID_FLOW_SCALE, LIQUID_FLOW_SPEED, LIQUID_VIGNETTE_STRENGTH,
    MEAT_FOOD_COLOR, SimConfig, WorldState, cell_display_color,
};
use crate::{MainCamera, SelectedCell};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::core_pipeline::core_3d::Transparent3d;
use bevy::ecs::{query::QueryItem, system::SystemParamItem, system::lifetimeless::*};
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, VertexBufferLayout};
use bevy::pbr::{
    Material, MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup,
    SetMeshViewBindGroup, SetMeshViewBindingArrayBindGroup,
};
use bevy::prelude::*;
use bevy::render::{
    Render, RenderApp, RenderStartup, RenderSystems,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    mesh::{RenderMesh, RenderMeshBufferInfo, allocator::MeshAllocator},
    render_asset::RenderAssets,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
        RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
    },
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    sync_world::MainEntity,
    view::ExtractedView,
};
use bevy::window::PrimaryWindow;
use bevy::{reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::time::Instant;

const SHADER_ASSET_PATH: &str = "shaders/instanced_disc.wgsl";
const LIQUID_SHADER_ASSET_PATH: &str = "shaders/liquid_medium.wgsl";
const BRANCH_RENDER_SEGMENTS: usize = 6;
const BRANCH_SEGMENT_DEPTH_SPAN: f32 = 0.12;
const SELECTION_MIN_RADIUS_PX: f32 = 22.0;
const SELECTION_ARROW_RADIUS_PX: f32 = 18.0;
const SELECTION_ARROW_GAP_PX: f32 = 8.0;
const PERCEPTION_EDGE_WIDTH_PX: f32 = 1.5;
const TARGET_LINE_WIDTH_PX: f32 = 1.4;
const MAX_VISIBLE_WAKE_PATCHES: usize = 12_000;
const MIN_WAKE_SPEED: f32 = 9.0;
const MIN_WAKE_RADIUS_PX: f32 = 1.15;
const WAKE_PATCH_LIFETIME: f32 = 2.4;

#[derive(Component)]
pub struct ParticleLayer;

#[derive(Component)]
pub struct SimulationRenderEntity;

#[derive(Component, Deref, DerefMut)]
pub struct InstanceMaterialData(pub Vec<InstanceData>);

impl ExtractComponent for InstanceMaterialData {
    type QueryData = &'static InstanceMaterialData;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(InstanceMaterialData(item.0.clone()))
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InstanceData {
    pub pos_radius: [f32; 4],
    pub color: [f32; 4],
    pub nucleus: [f32; 4],
    pub motion: [f32; 4],
    pub shape: [f32; 4],
    pub soft_radii_a: [f32; 4],
    pub soft_radii_b: [f32; 4],
    pub section_radii_0: [f32; 4],
    pub section_radii_1: [f32; 4],
    pub section_radii_2: [f32; 4],
    pub section_radii_3: [f32; 4],
    pub section_meta: [f32; 4],
}

pub struct InstancedDiscPlugin;

#[derive(Resource, Default)]
pub(crate) struct SelectionVisualState {
    cell_id: Option<u64>,
    fade: f32,
    perception_radius: f32,
    target_position: Vec2,
    target_visible: bool,
    velocity_direction: Vec2,
}

#[derive(Clone, Copy)]
struct WakeEmitter {
    anchor: Vec2,
    last_seen_frame: u64,
}

#[derive(Clone, Copy)]
struct WakePatch {
    center: Vec2,
    direction: Vec2,
    half_length: f32,
    half_width: f32,
    strength: f32,
    phase: f32,
    age: f32,
}

#[derive(Resource, Default)]
pub(crate) struct CellWakeTrails {
    emitters: HashMap<(u64, u8), WakeEmitter>,
    patches: Vec<WakePatch>,
    frame: u64,
}

impl CellWakeTrails {
    fn begin_frame(&mut self, dt: f32) {
        self.frame = self.frame.wrapping_add(1);
        for patch in &mut self.patches {
            patch.age += dt;
        }
        self.patches.retain(|patch| patch.age < WAKE_PATCH_LIFETIME);
        let oldest_live_frame = self.frame.saturating_sub(180);
        self.emitters
            .retain(|_, emitter| emitter.last_seen_frame >= oldest_live_frame);
    }

    fn sample_cell(
        &mut self,
        cell_id: u64,
        section: u8,
        position: Vec2,
        half_width: f32,
        strength: f32,
        phase: f32,
    ) {
        let frame = self.frame;
        let emitter = self
            .emitters
            .entry((cell_id, section))
            .or_insert(WakeEmitter {
                anchor: position,
                last_seen_frame: frame,
            });
        emitter.last_seen_frame = frame;

        let displacement = position - emitter.anchor;
        let distance = displacement.length();
        let sample_distance = (half_width * 1.35).clamp(5.0, 42.0);
        if distance < sample_distance {
            return;
        }

        // Large discontinuities are teleports or recycled IDs, not water movement.
        if distance > sample_distance * 8.0 {
            emitter.anchor = position;
            return;
        }

        let direction = displacement / distance;
        let start = emitter.anchor;
        emitter.anchor = position;
        self.patches.push(WakePatch {
            center: start + displacement * 0.5,
            direction,
            half_length: distance * 0.5 + half_width * 0.45,
            half_width,
            strength,
            phase,
            age: 0.0,
        });

        if self.patches.len() > MAX_VISIBLE_WAKE_PATCHES {
            let overflow = self.patches.len() - MAX_VISIBLE_WAKE_PATCHES;
            self.patches.drain(..overflow);
        }
    }

    fn clear(&mut self) {
        self.emitters.clear();
        self.patches.clear();
        self.frame = 0;
    }
}

pub fn clear_cell_wake_trails(mut trails: ResMut<CellWakeTrails>) {
    trails.clear();
}

#[derive(Clone, Copy, Debug, ShaderType)]
struct LiquidMediumParams {
    deep_color: Vec4,
    caustic_color: Vec4,
    arena_size: Vec4,
    flow: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LiquidMediumMaterial {
    #[uniform(0)]
    params: LiquidMediumParams,
}

impl LiquidMediumMaterial {
    fn new(config: &SimConfig) -> Self {
        Self {
            params: LiquidMediumParams {
                deep_color: Vec4::new(0.035, 0.075, 0.095, 1.0),
                caustic_color: Vec4::new(0.18, 0.42, 0.48, 1.0),
                arena_size: Vec4::new(
                    config.width,
                    config.height,
                    config.arena_shape.shader_code(),
                    0.0,
                ),
                flow: Vec4::new(
                    LIQUID_FLOW_SCALE,
                    LIQUID_FLOW_SPEED,
                    LIQUID_CAUSTIC_STRENGTH,
                    LIQUID_VIGNETTE_STRENGTH,
                ),
            },
        }
    }
}

impl Material for LiquidMediumMaterial {
    fn fragment_shader() -> ShaderRef {
        LIQUID_SHADER_ASSET_PATH.into()
    }
}

impl Plugin for InstancedDiscPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionVisualState>()
            .init_resource::<CellWakeTrails>()
            .add_plugins(ExtractComponentPlugin::<InstanceMaterialData>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom>()
            .init_resource::<SpecializedMeshPipelines<CustomPipeline>>()
            .add_systems(RenderStartup, init_custom_pipeline)
            .add_systems(
                Render,
                (
                    queue_custom.in_set(RenderSystems::QueueMeshes),
                    prepare_instance_buffers.in_set(RenderSystems::PrepareResources),
                ),
            );
    }
}

pub fn spawn_simulation_layers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut liquid_materials: ResMut<Assets<LiquidMediumMaterial>>,
    config: Res<SimConfig>,
) {
    let disc_mesh = meshes.add(unit_quad_mesh());

    commands.spawn((
        Name::new("particles_instanced_layer"),
        Mesh3d(disc_mesh),
        InstanceMaterialData(Vec::new()),
        ParticleLayer,
        SimulationRenderEntity,
        NoFrustumCulling,
    ));

    spawn_arena(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut liquid_materials,
        &config,
    );
}

pub fn sync_instance_data(
    time: Res<Time>,
    world: Res<WorldState>,
    selected: Res<SelectedCell>,
    camera: Query<(&Projection, &Transform), With<MainCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut particles: Query<&mut InstanceMaterialData, With<ParticleLayer>>,
    mut stats: ResMut<FrameStats>,
    mut selection_visual: ResMut<SelectionVisualState>,
    mut wake_trails: ResMut<CellWakeTrails>,
) {
    let started = Instant::now();

    let mut particles = particles
        .single_mut()
        .expect("particle instanced layer exists");
    particles.clear();
    particles.reserve(
        world.cells.len() * 3
            + wake_trails.patches.len()
            + usize::from(selected.cell_id.is_some() || selection_visual.fade > 0.02) * 4
            + world.food.len() * 2
            + world.visual_particles.len()
            + world.obstacles.len()
            + world.food_growers.len()
            + world.food_growers.total_branches() * BRANCH_RENDER_SEGMENTS,
    );

    let selection_follow = 1.0 - (-9.0 * time.delta_secs()).exp();
    if selected.cell_id.is_some() {
        selection_visual.fade += (1.0 - selection_visual.fade) * selection_follow;
    } else {
        selection_visual.fade += (0.0 - selection_visual.fade) * selection_follow;
        selection_visual.target_visible = false;
        if selection_visual.fade < 0.015 {
            selection_visual.cell_id = None;
        }
    }

    let (view_center, view_half_size, world_units_per_pixel) =
        camera_view_metrics(&camera, &windows).unwrap_or((Vec2::ZERO, Vec2::splat(f32::MAX), 1.0));
    let wake_stride = wake_sampling_stride(world_units_per_pixel);
    wake_trails.begin_frame(time.delta_secs().min(0.1));

    let (branch_z, branch_step) = branch_render_depths(&world.food_growers.branch_layer);

    for i in 0..world.cells.len() {
        let heading = world.cells.heading[i];
        let move_dir_x = heading.cos();
        let move_dir_y = heading.sin();
        let jelly = world.cells.jelly_intensity[i];
        let soft_radii = world.cells.lysis_visual_radii(i, 0);
        let base_visual_radius = soft_radii
            .iter()
            .copied()
            .fold(world.cells.radius[i] * 0.25, f32::max)
            .max(0.1);
        let mitosis = world.cells.mitosis_progress[i].clamp(0.0, 1.0);
        let mitosis_split = mitosis * mitosis * (3.0 - 2.0 * mitosis);
        let visual_radius = base_visual_radius * (1.0 + mitosis_split * 1.20);
        let inv_visual_radius = visual_radius.recip();
        let cell_color = cell_display_color(
            world.cells.species[i],
            world.cells.viability_ratio(i),
            world.cells.aggressiveness[i],
            world.cells.lysis[i],
        );

        let velocity = Vec2::new(world.cells.vx[i], world.cells.vy[i]);
        let velocity_length = velocity.length();
        let cell_position = Vec2::new(world.cells.x[i], world.cells.y[i]);
        let visible_margin = visual_radius * 8.0 + 40.0;
        let is_visible = (cell_position.x - view_center.x).abs()
            <= view_half_size.x + visible_margin
            && (cell_position.y - view_center.y).abs() <= view_half_size.y + visible_margin;
        if i % wake_stride == 0
            && is_visible
            && world.cells.wake_strength[i] >= 0.015
            && visual_radius / world_units_per_pixel.max(0.001) >= MIN_WAKE_RADIUS_PX
        {
            let wake_strength = world.cells.wake_strength[i].clamp(0.0, 1.0);
            for section in 0..world.cells.section_count[i] {
                let section_velocity = world.cells.section_velocity(i, section);
                let section_speed = section_velocity.length();
                if section_speed < MIN_WAKE_SPEED {
                    continue;
                }
                let speed_ratio = (section_speed / world.cells.speed[i].max(1.0)).clamp(0.0, 1.4);
                let wake_width =
                    world.cells.section_wake_half_width(i, section) * (1.28 + speed_ratio * 0.20);
                wake_trails.sample_cell(
                    world.cells.id[i],
                    section,
                    world.cells.section_center(i, section),
                    wake_width,
                    wake_strength,
                    world.cells.shape_phase[i] + section as f32 * 1.731,
                );
            }
            for edge in 0..world.cells.section_count[i].saturating_sub(1) as usize {
                for sample_index in 0..2 {
                    let t = (sample_index + 1) as f32 / 3.0;
                    let (position, connection_velocity, base_width) =
                        world.cells.connection_wake_sample(i, edge, t);
                    let connection_speed = connection_velocity.length();
                    if connection_speed < MIN_WAKE_SPEED {
                        continue;
                    }
                    let speed_ratio =
                        (connection_speed / world.cells.speed[i].max(1.0)).clamp(0.0, 1.4);
                    let wake_width = base_width * (1.28 + speed_ratio * 0.20);
                    let emitter_slot = 4 + edge as u8 * 2 + sample_index as u8;
                    wake_trails.sample_cell(
                        world.cells.id[i],
                        emitter_slot,
                        position,
                        wake_width,
                        wake_strength,
                        world.cells.shape_phase[i] + emitter_slot as f32 * 1.731,
                    );
                }
            }
        }

        if !is_visible {
            continue;
        }

        if world.cells.section_count[i] >= 2 {
            particles.push(segmented_cell_instance(
                &world, i, 2.0, cell_color, 10.0, 1.0, 0.0,
            ));
        } else {
            particles.push(InstanceData {
                pos_radius: [world.cells.x[i], world.cells.y[i], 2.0, visual_radius],
                color: cell_color,
                nucleus: [
                    (move_dir_x * world.cells.radius[i] * 0.24
                        + world.cells.nucleus_offset_x[i] * 0.25)
                        * inv_visual_radius,
                    (move_dir_y * world.cells.radius[i] * 0.24
                        + world.cells.nucleus_offset_y[i] * 0.25)
                        * inv_visual_radius,
                    world.cells.nucleus_radius[i] * inv_visual_radius,
                    1.0,
                ],
                motion: [move_dir_x, move_dir_y, jelly, world.cells.jelly_phase[i]],
                shape: [
                    world.cells.shape_wave_a[i],
                    world.cells.shape_wave_b[i],
                    world.cells.shape_phase[i],
                    world.cells.shape_softness[i],
                ],
                soft_radii_a: [
                    soft_radii[0] * inv_visual_radius,
                    soft_radii[1] * inv_visual_radius,
                    soft_radii[2] * inv_visual_radius,
                    soft_radii[3] * inv_visual_radius,
                ],
                soft_radii_b: [
                    soft_radii[4] * inv_visual_radius,
                    soft_radii[5] * inv_visual_radius,
                    soft_radii[6] * inv_visual_radius,
                    soft_radii[7] * inv_visual_radius,
                ],
                section_radii_0: [0.0; 4],
                section_radii_1: [0.0; 4],
                section_radii_2: [0.0; 4],
                section_radii_3: [0.0; 4],
                section_meta: [0.0, 0.0, 0.0, mitosis],
            });
        }

        let active_selected = selected.cell_id == Some(world.cells.id[i]);
        let fading_selected = !active_selected
            && selection_visual.cell_id == Some(world.cells.id[i])
            && selection_visual.fade > 0.015;
        if active_selected || fading_selected {
            let world_units_per_pixel = selection_world_units_per_pixel(&camera, &windows);
            let desired_perception = world.cells.perception[i].max(1.0);
            if active_selected && selection_visual.cell_id != selected.cell_id {
                selection_visual.cell_id = selected.cell_id;
                selection_visual.perception_radius = desired_perception * 0.82;
                selection_visual.target_position = Vec2::new(world.cells.x[i], world.cells.y[i]);
                selection_visual.target_visible = false;
                selection_visual.velocity_direction = Vec2::new(move_dir_x, move_dir_y);
            }
            let overlay_follow = 1.0 - (-8.0 * time.delta_secs()).exp();
            if active_selected {
                selection_visual.perception_radius +=
                    (desired_perception - selection_visual.perception_radius) * overlay_follow;
            }
            let selection_alpha = selection_visual.fade.clamp(0.0, 1.0);
            let perception_radius = selection_visual.perception_radius.max(1.0);
            particles.push(InstanceData {
                pos_radius: [world.cells.x[i], world.cells.y[i], 4.05, perception_radius],
                color: [0.30, 0.82, 0.80, 0.72 * selection_alpha],
                nucleus: [0.0, 0.0, 0.0, 7.0],
                motion: [0.0; 4],
                shape: [
                    world_units_per_pixel * PERCEPTION_EDGE_WIDTH_PX / perception_radius,
                    0.0,
                    0.0,
                    0.0,
                ],
                soft_radii_a: [1.0; 4],
                soft_radii_b: [1.0; 4],
                section_radii_0: [0.0; 4],
                section_radii_1: [0.0; 4],
                section_radii_2: [0.0; 4],
                section_radii_3: [0.0; 4],
                section_meta: [0.0; 4],
            });

            if world.cells.section_count[i] >= 2 {
                particles.push(segmented_cell_instance(
                    &world,
                    i,
                    4.20,
                    [0.36, 0.88, 0.92, 0.92 * selection_alpha],
                    11.0,
                    1.16,
                    world_units_per_pixel * SELECTION_MIN_RADIUS_PX,
                ));
            }

            if active_selected && let Some(target) = world.cell_target(i) {
                let cell_position = Vec2::new(world.cells.x[i], world.cells.y[i]);
                if !selection_visual.target_visible {
                    selection_visual.target_position = cell_position;
                }
                selection_visual.target_visible = true;
                let current_target = selection_visual.target_position;
                selection_visual.target_position =
                    current_target + (target.position - current_target) * overlay_follow;
                let delta = selection_visual.target_position - cell_position;
                let distance = delta.length();
                let half_length = distance * 0.5;
                let line_width = world_units_per_pixel * TARGET_LINE_WIDTH_PX;
                let instance_radius =
                    (half_length + world_units_per_pixel * 10.0).max(world_units_per_pixel * 12.0);
                let center = cell_position + delta * 0.5;
                let target_code = match target.kind {
                    CellTargetKind::Food => {
                        if target.remembered {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    CellTargetKind::Cell => 2.0,
                };
                let target_color = if target.kind == CellTargetKind::Cell {
                    [1.0, 0.30, 0.26, 0.94]
                } else if target.remembered {
                    [1.0, 0.76, 0.38, 0.72]
                } else {
                    [0.54, 1.0, 0.58, 0.90]
                };
                particles.push(InstanceData {
                    pos_radius: [center.x, center.y, 4.12, instance_radius],
                    color: target_color,
                    nucleus: [0.0, 0.0, 0.0, 8.0],
                    motion: [0.0, 0.0, delta.y.atan2(delta.x), 0.0],
                    shape: [
                        half_length / instance_radius,
                        line_width / instance_radius,
                        target_code,
                        world_units_per_pixel / instance_radius,
                    ],
                    soft_radii_a: [1.0; 4],
                    soft_radii_b: [1.0; 4],
                    section_radii_0: [0.0; 4],
                    section_radii_1: [0.0; 4],
                    section_radii_2: [0.0; 4],
                    section_radii_3: [0.0; 4],
                    section_meta: [0.0, 0.0, 0.0, mitosis],
                });
            } else {
                selection_visual.target_visible = false;
            }

            let highlight_radius =
                (visual_radius * 1.18).max(world_units_per_pixel * SELECTION_MIN_RADIUS_PX);
            let inv_highlight_radius = highlight_radius.recip();
            if world.cells.section_count[i] == 1 {
                particles.push(InstanceData {
                    pos_radius: [world.cells.x[i], world.cells.y[i], 4.20, highlight_radius],
                    color: [0.36, 0.88, 0.92, 0.92 * selection_alpha],
                    nucleus: [0.0, 0.0, 0.0, 5.0],
                    motion: [move_dir_x, move_dir_y, jelly, world.cells.jelly_phase[i]],
                    shape: [
                        world.cells.shape_wave_a[i],
                        world.cells.shape_wave_b[i],
                        world.cells.shape_phase[i],
                        world.cells.shape_softness[i],
                    ],
                    soft_radii_a: [
                        soft_radii[0] * inv_highlight_radius,
                        soft_radii[1] * inv_highlight_radius,
                        soft_radii[2] * inv_highlight_radius,
                        soft_radii[3] * inv_highlight_radius,
                    ],
                    soft_radii_b: [
                        soft_radii[4] * inv_highlight_radius,
                        soft_radii[5] * inv_highlight_radius,
                        soft_radii[6] * inv_highlight_radius,
                        soft_radii[7] * inv_highlight_radius,
                    ],
                    section_radii_0: [0.0; 4],
                    section_radii_1: [0.0; 4],
                    section_radii_2: [0.0; 4],
                    section_radii_3: [0.0; 4],
                    section_meta: [0.0, 0.0, 0.0, mitosis],
                });
            }

            let velocity_direction = velocity
                .try_normalize()
                .unwrap_or(Vec2::new(move_dir_x, move_dir_y));
            selection_visual.velocity_direction = selection_visual
                .velocity_direction
                .lerp(velocity_direction, overlay_follow)
                .normalize_or_zero();
            let velocity_direction = if selection_visual.velocity_direction == Vec2::ZERO {
                velocity_direction
            } else {
                selection_visual.velocity_direction
            };
            let arrow_radius = world_units_per_pixel * SELECTION_ARROW_RADIUS_PX;
            let arrow_gap = world_units_per_pixel * SELECTION_ARROW_GAP_PX;
            let arrow_center = Vec2::new(world.cells.x[i], world.cells.y[i])
                + velocity_direction * (highlight_radius + arrow_gap + arrow_radius * 0.35);
            particles.push(InstanceData {
                pos_radius: [arrow_center.x, arrow_center.y, 4.25, arrow_radius],
                color: [0.66, 0.98, 0.94, 0.98 * selection_alpha],
                nucleus: [0.0, 0.0, 0.0, 6.0],
                motion: [
                    velocity_direction.x,
                    velocity_direction.y,
                    velocity_direction.y.atan2(velocity_direction.x),
                    0.0,
                ],
                shape: [
                    (velocity_length / world.cells.speed[i].max(1.0)).clamp(0.0, 1.5),
                    0.0,
                    0.0,
                    0.0,
                ],
                soft_radii_a: [1.0; 4],
                soft_radii_b: [1.0; 4],
                section_radii_0: [0.0; 4],
                section_radii_1: [0.0; 4],
                section_radii_2: [0.0; 4],
                section_radii_3: [0.0; 4],
                section_meta: [0.0; 4],
            });
        }
    }

    for patch in &wake_trails.patches {
        let visible_margin = patch.half_length + patch.half_width * 2.0;
        if (patch.center.x - view_center.x).abs() > view_half_size.x + visible_margin
            || (patch.center.y - view_center.y).abs() > view_half_size.y + visible_margin
        {
            continue;
        }
        let life = (1.0 - patch.age / WAKE_PATCH_LIFETIME).clamp(0.0, 1.0);
        let fade_in = (patch.age / 0.16).clamp(0.0, 1.0);
        let opacity = patch.strength * life * life * fade_in;
        let instance_radius = patch.half_length + patch.half_width * 1.35;
        particles.push(InstanceData {
            pos_radius: [patch.center.x, patch.center.y, 1.92, instance_radius],
            color: [0.66, 0.92, 1.0, (0.09 + patch.strength * 0.08) * opacity],
            nucleus: [0.0, 0.0, 0.0, 13.0],
            motion: [
                patch.direction.x,
                patch.direction.y,
                patch.direction.y.atan2(patch.direction.x),
                patch.phase,
            ],
            shape: [
                patch.half_length / instance_radius,
                patch.half_width / instance_radius,
                patch.strength,
                life,
            ],
            soft_radii_a: [1.0; 4],
            soft_radii_b: [1.0; 4],
            section_radii_0: [0.0; 4],
            section_radii_1: [0.0; 4],
            section_radii_2: [0.0; 4],
            section_radii_3: [0.0; 4],
            section_meta: [0.0; 4],
        });
    }

    for i in 0..world.food.len() {
        if !world.food.active[i] {
            continue;
        }

        let mut color = match world.food.kind[i] {
            FoodKind::Grass => GRASS_FOOD_COLOR,
            FoodKind::Meat => MEAT_FOOD_COLOR,
        };
        let lobes = match world.food.kind[i] {
            FoodKind::Grass => 5.0,
            FoodKind::Meat => 4.0,
        };
        let roughness = match world.food.kind[i] {
            FoodKind::Grass => 0.11,
            FoodKind::Meat => 0.16,
        };

        let z_layer = 3.0;
        let render_x = world.food.x[i];
        let render_y = world.food.y[i];
        let food_margin = FOOD_RADIUS * 2.0;
        if (render_x - view_center.x).abs() > view_half_size.x + food_margin
            || (render_y - view_center.y).abs() > view_half_size.y + food_margin
        {
            continue;
        }
        let mut z_layer_adjusted = z_layer;
        if world.food.feeder[i] >= 0 {
            let branch_index = world.food.anchor_branch[i];
            if branch_index >= 0 {
                let branch_index = branch_index as usize;
                z_layer_adjusted = branch_z[branch_index] + branch_step * 0.66;

                if !world.food_growers.branch_has_collision(branch_index) {
                    color[0] *= 0.45;
                    color[1] *= 0.45;
                    color[2] *= 0.45;
                    color[3] = 0.35;
                }

                // Sway is already baked into food physics coordinates in advect_food, no manual rotation needed.
            }
        }

        particles.push(InstanceData {
            pos_radius: [
                render_x,
                render_y,
                z_layer_adjusted,
                FOOD_RADIUS * world.food.growth[i].clamp(0.28, 1.0),
            ],
            color,
            nucleus: [0.0, 0.0, 0.0, world.food.kind[i].shader_kind()],
            motion: [0.0, 1.0, world.food.rotation[i], world.food.phase[i]],
            shape: [
                lobes,
                roughness,
                world.food.spin[i],
                world.food.shape[i].shader_shape(),
            ],
            soft_radii_a: [1.0; 4],
            soft_radii_b: [1.0; 4],
            section_radii_0: [0.0; 4],
            section_radii_1: [0.0; 4],
            section_radii_2: [0.0; 4],
            section_radii_3: [0.0; 4],
            section_meta: [0.0; 4],
        });
    }

    for i in 0..world.visual_particles.len() {
        let particle_x = world.visual_particles.x[i];
        let particle_y = world.visual_particles.y[i];
        let particle_margin = world.visual_particles.radius[i] * 2.0 + 4.0;
        if (particle_x - view_center.x).abs() > view_half_size.x + particle_margin
            || (particle_y - view_center.y).abs() > view_half_size.y + particle_margin
        {
            continue;
        }
        let life =
            (world.visual_particles.life[i] / world.visual_particles.lifetime[i]).clamp(0.0, 1.0);
        let mut color = world.visual_particles.color[i];
        color[3] *= life;
        let radius = world.visual_particles.radius[i] * (0.55 + life * 0.65);
        particles.push(InstanceData {
            pos_radius: [
                world.visual_particles.x[i],
                world.visual_particles.y[i],
                3.85,
                radius,
            ],
            color,
            nucleus: [0.0, 0.0, 0.0, 12.0],
            motion: [
                world.visual_particles.vx[i],
                world.visual_particles.vy[i],
                life,
                world.visual_particles.phase[i],
            ],
            shape: [life, world.visual_particles.style[i], 0.0, 0.0],
            soft_radii_a: [1.0; 4],
            soft_radii_b: [1.0; 4],
            section_radii_0: [0.0; 4],
            section_radii_1: [0.0; 4],
            section_radii_2: [0.0; 4],
            section_radii_3: [0.0; 4],
            section_meta: [0.0; 4],
        });
    }

    for i in 0..world.obstacles.len() {
        let radius = world.obstacles.radius[i];
        if (world.obstacles.x[i] - view_center.x).abs() > view_half_size.x + radius
            || (world.obstacles.y[i] - view_center.y).abs() > view_half_size.y + radius
        {
            continue;
        }
        particles.push(InstanceData {
            pos_radius: [world.obstacles.x[i], world.obstacles.y[i], 1.5, radius],
            color: [0.58, 0.72, 0.95, 0.22],
            nucleus: [0.0, 0.0, 0.0, 2.0],
            motion: [
                world.obstacles.vx[i],
                world.obstacles.vy[i],
                world.obstacles.rotation[i],
                world.obstacles.phase[i],
            ],
            shape: [
                world.obstacles.spokes[i],
                world.obstacles.rings[i],
                radius,
                0.0,
            ],
            soft_radii_a: [1.0; 4],
            soft_radii_b: [1.0; 4],
            section_radii_0: [0.0; 4],
            section_radii_1: [0.0; 4],
            section_radii_2: [0.0; 4],
            section_radii_3: [0.0; 4],
            section_meta: [0.0; 4],
        });
    }

    for i in 0..world.food_growers.len() {
        let radius = world.food_growers.radius[i];
        let extent = world.food_growers.extent_radius(i) + radius;
        if (world.food_growers.x[i] - view_center.x).abs() > view_half_size.x + extent
            || (world.food_growers.y[i] - view_center.y).abs() > view_half_size.y + extent
        {
            continue;
        }
        for branch_index in world.food_growers.branch_range(i) {
            let branch_base_z = branch_z[branch_index];
            let solid_branch = world.food_growers.branch_has_collision(branch_index);
            let hue_shift = world.food_growers.branch_hue_shift[branch_index];
            let light_shift = world.food_growers.branch_lightness_shift[branch_index];
            let sat_shift = world.food_growers.branch_saturation_shift[branch_index];
            let width_scale = world.food_growers.branch_width_scale[branch_index];
            let color = if solid_branch {
                [
                    (0.25 + hue_shift).clamp(0.08, 0.42),
                    (1.0 + light_shift).clamp(0.75, 1.0),
                    (0.42 + sat_shift).clamp(0.15, 0.65),
                    0.80,
                ]
            } else {
                [
                    (0.10 + hue_shift).clamp(0.01, 0.22),
                    (0.45 + light_shift).clamp(0.25, 0.65),
                    (0.20 + sat_shift).clamp(0.08, 0.38),
                    0.35,
                ]
            };

            for segment_index in 0..BRANCH_RENDER_SEGMENTS {
                let t0 = segment_index as f32 / BRANCH_RENDER_SEGMENTS as f32;
                let t1 = (segment_index + 1) as f32 / BRANCH_RENDER_SEGMENTS as f32;
                let tm = (t0 + t1) * 0.5;
                let a = world.food_growers.branch_center_at(branch_index, t0);
                let b = world.food_growers.branch_center_at(branch_index, t1);
                let segment = b - a;
                let segment_len = segment.length();
                if segment_len <= 0.001 {
                    continue;
                }

                let visual_width = world.food_growers.branch_width[branch_index] * width_scale;
                let half_length = (segment_len * 0.5).max(1.0);
                let instance_radius = half_length + visual_width * 2.0;
                let center = a + segment * 0.5;
                let angle = segment.y.atan2(segment.x);
                particles.push(InstanceData {
                    pos_radius: [
                        center.x,
                        center.y,
                        branch_segment_depth(branch_base_z, branch_step, segment_index),
                        instance_radius,
                    ],
                    color,
                    nucleus: [0.0, 0.0, 0.0, 4.0],
                    motion: [
                        0.0,
                        0.0,
                        angle,
                        world.food_growers.branch_phase[branch_index] + tm * 1.7,
                    ],
                    shape: [
                        (visual_width / instance_radius).clamp(0.02, 0.28),
                        0.0,
                        (half_length / instance_radius).clamp(0.1, 0.98),
                        0.0,
                    ],
                    soft_radii_a: [1.0; 4],
                    soft_radii_b: [1.0; 4],
                    section_radii_0: [0.0; 4],
                    section_radii_1: [0.0; 4],
                    section_radii_2: [0.0; 4],
                    section_radii_3: [0.0; 4],
                    section_meta: [0.0; 4],
                });
            }
        }

        particles.push(InstanceData {
            pos_radius: [
                world.food_growers.x[i],
                world.food_growers.y[i],
                1.75,
                radius * 1.08,
            ],
            color: [0.24, 1.0, 0.42, 0.78],
            nucleus: [0.0, 0.0, 0.0, 3.0],
            motion: [
                world.food_growers.vx[i],
                world.food_growers.vy[i],
                world.food_growers.rotation[i],
                world.food_growers.phase[i],
            ],
            shape: [
                world.food_growers.branch_count[i] as f32,
                1.0,
                world.food_growers.timer[i],
                1.08,
            ],
            soft_radii_a: [1.0; 4],
            soft_radii_b: [1.0; 4],
            section_radii_0: [0.0; 4],
            section_radii_1: [0.0; 4],
            section_radii_2: [0.0; 4],
            section_radii_3: [0.0; 4],
            section_meta: [0.0; 4],
        });
    }

    // Render grower branchlets persistently
    for branchlet_index in 0..world.food_growers.branchlet_branch_index.len() {
        if let Some((stem_start, stem_end)) = world.branchlet_stem_points(branchlet_index) {
            let branch_index = world.food_growers.branchlet_branch_index[branchlet_index];
            let stem = stem_end - stem_start;
            let stem_len = stem.length();

            if stem_len > 0.75 {
                let hue_shift = world.food_growers.branch_hue_shift[branch_index];
                let light_shift = world.food_growers.branch_lightness_shift[branch_index];
                let sat_shift = world.food_growers.branch_saturation_shift[branch_index];

                let half_length = (stem_len * 0.5).max(0.5);

                let mut growth = 0.55;
                if let Some(food_idx) = world.food_growers.branchlet_food_index[branchlet_index] {
                    if food_idx < world.food.len() && world.food.active[food_idx] {
                        growth = world.food.growth[food_idx];
                    }
                }

                let stem_width = (FOOD_RADIUS * 0.34 * growth.clamp(0.55, 1.0)).max(0.9);
                let instance_radius = half_length + stem_width * 2.0;
                let stem_center = stem_start + stem * 0.5;
                if (stem_center.x - view_center.x).abs() > view_half_size.x + instance_radius
                    || (stem_center.y - view_center.y).abs() > view_half_size.y + instance_radius
                {
                    continue;
                }
                let stem_angle = stem.y.atan2(stem.x);

                let is_solid = world.food_growers.branch_has_collision(branch_index);
                let branchlet_color = if is_solid {
                    [
                        (0.34 + hue_shift).clamp(0.1, 0.6),
                        (1.0 + light_shift).clamp(0.7, 1.0),
                        (0.36 + sat_shift).clamp(0.1, 0.6),
                        0.82,
                    ]
                } else {
                    [
                        (0.15 + hue_shift).clamp(0.05, 0.35),
                        (0.50 + light_shift).clamp(0.3, 0.7),
                        (0.18 + sat_shift).clamp(0.05, 0.4),
                        0.35,
                    ]
                };

                particles.push(InstanceData {
                    pos_radius: [
                        stem_center.x,
                        stem_center.y,
                        branch_z[branch_index] + branch_step * 0.33,
                        instance_radius,
                    ],
                    color: branchlet_color,
                    nucleus: [0.0, 0.0, 0.0, 4.0],
                    motion: [0.0, 0.0, stem_angle, 0.0],
                    shape: [
                        (stem_width / instance_radius).clamp(0.02, 0.22),
                        0.0,
                        (half_length / instance_radius).clamp(0.1, 0.98),
                        0.0,
                    ],
                    soft_radii_a: [1.0; 4],
                    soft_radii_b: [1.0; 4],
                    section_radii_0: [0.0; 4],
                    section_radii_1: [0.0; 4],
                    section_radii_2: [0.0; 4],
                    section_radii_3: [0.0; 4],
                    section_meta: [0.0; 4],
                });
            }
        }
    }

    sort_instances_back_to_front(&mut particles);
    stats.upload_time = started.elapsed();
}

fn segmented_cell_instance(
    world: &WorldState,
    index: usize,
    z: f32,
    color: [f32; 4],
    kind: f32,
    radius_scale: f32,
    min_lobe_radius: f32,
) -> InstanceData {
    let count = world.cells.section_count[index] as usize;
    let mut positions = [Vec2::ZERO; 4];
    positions[0] = Vec2::new(world.cells.x[index], world.cells.y[index]);
    positions[1] = Vec2::new(world.cells.tail_x[index], world.cells.tail_y[index]);
    for extra_index in 0..2 {
        let extra = world.cells.extra_sections[index][extra_index];
        positions[extra_index + 2] = Vec2::new(extra.x, extra.y);
    }
    let mut radii = [world.cells.core_radius[index]; 4];
    radii[0] = world
        .cells
        .lysis_visual_radii(index, 0)
        .iter()
        .copied()
        .fold(world.cells.core_radius[index], f32::max);
    radii[1] = world
        .cells
        .lysis_visual_radii(index, 1)
        .iter()
        .copied()
        .fold(world.cells.tail_core_radius[index], f32::max);
    for extra_index in 0..2 {
        let extra = world.cells.extra_sections[index][extra_index];
        radii[extra_index + 2] = world
            .cells
            .lysis_visual_radii(index, extra_index as u8 + 2)
            .iter()
            .copied()
            .fold(extra.core_radius, f32::max);
    }
    let center = positions[..count].iter().copied().sum::<Vec2>() / count as f32;
    for radius in &mut radii[..count] {
        *radius = (*radius * radius_scale).max(min_lobe_radius).max(0.1);
    }
    let base_instance_radius = positions[..count]
        .iter()
        .zip(&radii[..count])
        .map(|(position, radius)| position.distance(center) + radius)
        .fold(0.1, f32::max);
    let mitosis = world.cells.mitosis_progress[index].clamp(0.0, 1.0);
    let mitosis_split = mitosis * mitosis * (3.0 - 2.0 * mitosis);
    let instance_radius = base_instance_radius * (1.0 + mitosis_split * 1.20);
    let inv_radius = instance_radius.recip();
    let nucleus_world = positions[0]
        + Vec2::new(
            world.cells.heading[index].cos(),
            world.cells.heading[index].sin(),
        ) * world.cells.radius[index]
            * 0.24
        + Vec2::new(
            world.cells.nucleus_offset_x[index],
            world.cells.nucleus_offset_y[index],
        ) * 0.25;
    let nucleus_local = (nucleus_world - center) * inv_radius;
    let local = positions.map(|position| (position - center) * inv_radius);
    let section_radii = [
        pack_section_radii(world.cells.lysis_visual_radii(index, 0), inv_radius),
        pack_section_radii(world.cells.lysis_visual_radii(index, 1), inv_radius),
        pack_section_radii(world.cells.lysis_visual_radii(index, 2), inv_radius),
        pack_section_radii(world.cells.lysis_visual_radii(index, 3), inv_radius),
    ];

    InstanceData {
        pos_radius: [center.x, center.y, z, instance_radius],
        color,
        nucleus: [
            nucleus_local.x,
            nucleus_local.y,
            world.cells.nucleus_radius[index] * inv_radius,
            kind,
        ],
        motion: [
            count as f32,
            world.cells.section_parents[index][0] as f32,
            world.cells.section_parents[index][1] as f32,
            world.cells.section_parents[index][2] as f32,
        ],
        shape: std::array::from_fn(|section| {
            if section == 0 {
                world.cells.heading[index]
            } else {
                let parent = world.cells.section_parents[index][section - 1] as usize;
                let delta = positions[parent] - positions[section];
                delta.y.atan2(delta.x)
            }
        }),
        soft_radii_a: [local[0].x, local[0].y, local[1].x, local[1].y],
        soft_radii_b: [local[2].x, local[2].y, local[3].x, local[3].y],
        section_radii_0: section_radii[0],
        section_radii_1: section_radii[1],
        section_radii_2: section_radii[2],
        section_radii_3: section_radii[3],
        section_meta: [
            world.cells.edge_curve_offsets[index][0] * inv_radius,
            world.cells.edge_curve_offsets[index][1] * inv_radius,
            world.cells.edge_curve_offsets[index][2] * inv_radius,
            mitosis,
        ],
    }
}

fn pack_section_radii(radii: [f32; 8], inv_instance_radius: f32) -> [f32; 4] {
    std::array::from_fn(|pair| {
        let low = (radii[pair * 2] * inv_instance_radius).clamp(0.0, 1.0);
        let high = (radii[pair * 2 + 1] * inv_instance_radius).clamp(0.0, 1.0);
        let low_bits = (low * u16::MAX as f32).round() as u32;
        let high_bits = (high * u16::MAX as f32).round() as u32;
        f32::from_bits(low_bits | (high_bits << 16))
    })
}

#[cfg(test)]
fn section_render_profile(
    radii: &[f32; 8],
    angle_offsets: &[f32; 8],
    section_heading: f32,
    axis: Vec2,
    core_radius: f32,
) -> (f32, f32, f32) {
    let side_axis = Vec2::new(-axis.y, axis.x);
    let mut along = core_radius;
    let mut side = core_radius;
    let mut sum = 0.0;
    for ray in 0..8 {
        let angle = section_heading + ray as f32 * std::f32::consts::TAU / 8.0 + angle_offsets[ray];
        let direction = Vec2::new(angle.cos(), angle.sin());
        along = along.max(radii[ray] * direction.dot(axis).abs());
        side = side.max(radii[ray] * direction.dot(side_axis).abs());
        sum += radii[ray];
    }
    let mean = sum / 8.0;
    let variance = radii
        .iter()
        .map(|radius| (radius - mean) * (radius - mean))
        .sum::<f32>()
        / 8.0;
    let irregularity = (variance.sqrt() / mean.max(0.1)).clamp(0.025, 0.55);
    (along.max(0.1), side.max(0.1), irregularity)
}

fn selection_world_units_per_pixel(
    camera: &Query<(&Projection, &Transform), With<MainCamera>>,
    windows: &Query<&Window, With<PrimaryWindow>>,
) -> f32 {
    let Ok((Projection::Orthographic(projection), _)) = camera.single() else {
        return 1.0;
    };
    let Ok(window) = windows.single() else {
        return 1.0;
    };
    projection.area.height().abs() / window.height().max(1.0)
}

fn camera_view_metrics(
    camera: &Query<(&Projection, &Transform), With<MainCamera>>,
    windows: &Query<&Window, With<PrimaryWindow>>,
) -> Option<(Vec2, Vec2, f32)> {
    let (Projection::Orthographic(projection), transform) = camera.single().ok()? else {
        return None;
    };
    let window = windows.single().ok()?;
    let size = Vec2::new(
        projection.area.width().abs(),
        projection.area.height().abs(),
    );
    Some((
        transform.translation.truncate(),
        size * 0.5,
        size.y / window.height().max(1.0),
    ))
}

fn wake_sampling_stride(world_units_per_pixel: f32) -> usize {
    if world_units_per_pixel < 1.8 {
        1
    } else if world_units_per_pixel < 3.5 {
        2
    } else if world_units_per_pixel < 6.0 {
        4
    } else {
        8
    }
}

fn sort_instances_back_to_front(instances: &mut [InstanceData]) {
    instances.sort_unstable_by(|left, right| left.pos_radius[2].total_cmp(&right.pos_radius[2]));
}

fn branch_render_depths(layers: &[f32]) -> (Vec<f32>, f32) {
    let mut order: Vec<usize> = (0..layers.len()).collect();
    order.sort_unstable_by(|&left, &right| {
        layers[left]
            .total_cmp(&layers[right])
            .then_with(|| left.cmp(&right))
    });
    let step = 0.18 / (layers.len().max(1) + 1) as f32;
    let mut depths = vec![1.48; layers.len()];
    for (rank, branch_index) in order.into_iter().enumerate() {
        depths[branch_index] = 1.48 + step * (rank + 1) as f32;
    }
    (depths, step)
}

fn branch_segment_depth(branch_depth: f32, branch_step: f32, segment_index: usize) -> f32 {
    let reverse_index = BRANCH_RENDER_SEGMENTS
        .saturating_sub(1)
        .saturating_sub(segment_index);
    let progress = reverse_index as f32 / BRANCH_RENDER_SEGMENTS.max(1) as f32;
    branch_depth + branch_step * BRANCH_SEGMENT_DEPTH_SPAN * progress
}

fn unit_quad_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

#[cfg(test)]
mod tests {
    use super::{
        BRANCH_RENDER_SEGMENTS, CellWakeTrails, InstanceData, branch_render_depths,
        branch_segment_depth, section_render_profile, sort_instances_back_to_front,
        wake_sampling_stride,
    };
    use bevy::prelude::Vec2;

    fn instance_at_depth(depth: f32) -> InstanceData {
        InstanceData {
            pos_radius: [0.0, 0.0, depth, 1.0],
            color: [1.0; 4],
            nucleus: [0.0; 4],
            motion: [0.0; 4],
            shape: [0.0; 4],
            soft_radii_a: [1.0; 4],
            soft_radii_b: [1.0; 4],
            section_radii_0: [0.0; 4],
            section_radii_1: [0.0; 4],
            section_radii_2: [0.0; 4],
            section_radii_3: [0.0; 4],
            section_meta: [0.0; 4],
        }
    }

    #[test]
    fn segmented_profile_preserves_axial_ray_stretch() {
        let radii = [20.0, 8.0, 8.0, 8.0, 20.0, 8.0, 8.0, 8.0];
        let (along, side, irregularity) =
            section_render_profile(&radii, &[0.0; 8], 0.0, Vec2::X, 2.0);

        assert!(along / side > 2.0);
        assert!(irregularity > 0.25);
    }

    #[test]
    fn cell_wakes_are_progressively_decimated_when_zooming_out() {
        assert_eq!(wake_sampling_stride(1.0), 1);
        assert_eq!(wake_sampling_stride(2.0), 2);
        assert_eq!(wake_sampling_stride(4.0), 4);
        assert_eq!(wake_sampling_stride(8.0), 8);
    }

    #[test]
    fn emitted_wake_patches_stay_in_world_space() {
        let mut trails = CellWakeTrails::default();
        trails.begin_frame(0.0);
        trails.sample_cell(7, 0, Vec2::ZERO, 5.0, 1.0, 0.0);
        trails.sample_cell(7, 0, Vec2::new(10.0, 0.0), 5.0, 1.0, 0.0);
        let first_center = trails.patches[0].center;

        trails.begin_frame(0.1);
        trails.sample_cell(7, 0, Vec2::new(20.0, 7.0), 5.0, 1.0, 0.0);

        assert_eq!(trails.patches[0].center, first_center);
        assert_eq!(first_center, Vec2::new(5.0, 0.0));
    }

    #[test]
    fn cell_sections_keep_independent_wake_emitters() {
        let mut trails = CellWakeTrails::default();
        trails.begin_frame(0.0);
        trails.sample_cell(9, 0, Vec2::ZERO, 4.0, 1.0, 0.0);
        trails.sample_cell(9, 1, Vec2::new(0.0, 20.0), 4.0, 1.0, 1.0);
        trails.sample_cell(9, 0, Vec2::new(8.0, 0.0), 4.0, 1.0, 0.0);
        trails.sample_cell(9, 1, Vec2::new(8.0, 20.0), 4.0, 1.0, 1.0);

        assert_eq!(trails.patches.len(), 2);
        assert_eq!(trails.patches[0].center, Vec2::new(4.0, 0.0));
        assert_eq!(trails.patches[1].center, Vec2::new(4.0, 20.0));
    }

    #[test]
    fn connection_samples_keep_independent_wake_emitters() {
        let mut trails = CellWakeTrails::default();
        trails.begin_frame(0.0);
        for slot in [4, 5] {
            trails.sample_cell(11, slot, Vec2::new(0.0, slot as f32), 4.0, 1.0, 0.0);
            trails.sample_cell(11, slot, Vec2::new(8.0, slot as f32), 4.0, 1.0, 0.0);
        }

        assert_eq!(trails.patches.len(), 2);
        assert_eq!(trails.emitters.len(), 2);
    }

    #[test]
    fn branch_children_stay_below_the_next_higher_branch() {
        let layers = [0.80, 0.21, 0.79, 0.42];
        let (depths, step) = branch_render_depths(&layers);

        for lower in 0..layers.len() {
            for upper in 0..layers.len() {
                if layers[lower] < layers[upper] {
                    assert!(depths[lower] + step * 0.66 < depths[upper]);
                }
            }
        }
    }

    #[test]
    fn branch_segments_descend_from_grower_center_and_stay_below_branchlets() {
        let branch_depth = 1.5;
        let branch_step = 0.02;
        let mut previous = branch_segment_depth(branch_depth, branch_step, 0);

        for segment_index in 1..BRANCH_RENDER_SEGMENTS {
            let depth = branch_segment_depth(branch_depth, branch_step, segment_index);
            assert!(depth < previous);
            assert!(depth < branch_depth + branch_step * 0.33);
            previous = depth;
        }
    }

    #[test]
    fn instance_buffer_draws_lower_branchlets_before_higher_branches() {
        let mut instances = [
            instance_at_depth(1.62),
            instance_at_depth(1.51),
            instance_at_depth(1.56),
            instance_at_depth(1.53),
        ];

        sort_instances_back_to_front(&mut instances);

        assert_eq!(
            instances.map(|instance| instance.pos_radius[2]),
            [1.51, 1.53, 1.56, 1.62]
        );
    }
}

fn spawn_arena(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    liquid_materials: &mut Assets<LiquidMediumMaterial>,
    config: &SimConfig,
) {
    let background = liquid_materials.add(LiquidMediumMaterial::new(config));
    let wall = materials.add(StandardMaterial {
        base_color: Color::srgb(0.58, 0.78, 0.92),
        unlit: true,
        ..default()
    });

    let thickness = 80.0;
    let half_w = config.width * 0.5;
    let half_h = config.height * 0.5;

    match config.arena_shape {
        ArenaShape::Rectangle => {
            commands.spawn((
                Name::new("arena_background"),
                Mesh3d(meshes.add(Cuboid::new(config.width, config.height, 0.4))),
                MeshMaterial3d(background),
                Transform::from_xyz(0.0, 0.0, -0.4),
                SimulationRenderEntity,
            ));

            for (name, x, y, w, h) in [
                ("wall_top", 0.0, half_h, config.width + thickness, thickness),
                (
                    "wall_bottom",
                    0.0,
                    -half_h,
                    config.width + thickness,
                    thickness,
                ),
                (
                    "wall_left",
                    -half_w,
                    0.0,
                    thickness,
                    config.height + thickness,
                ),
                (
                    "wall_right",
                    half_w,
                    0.0,
                    thickness,
                    config.height + thickness,
                ),
            ] {
                commands.spawn((
                    Name::new(name),
                    Mesh3d(meshes.add(Cuboid::new(w, h, 1.0))),
                    MeshMaterial3d(wall.clone()),
                    Transform::from_xyz(x, y, 3.0),
                    SimulationRenderEntity,
                ));
            }
        }
        ArenaShape::Circle => {
            let radius = config.width.min(config.height) * 0.5;
            commands.spawn((
                Name::new("arena_background_circle"),
                Mesh3d(meshes.add(circle_mesh(radius, 128))),
                MeshMaterial3d(background),
                Transform::from_xyz(0.0, 0.0, -0.4),
                SimulationRenderEntity,
            ));
            commands.spawn((
                Name::new("wall_circle"),
                Mesh3d(meshes.add(ring_mesh(radius, thickness, 160))),
                MeshMaterial3d(wall),
                Transform::from_xyz(0.0, 0.0, 3.0),
                SimulationRenderEntity,
            ));
        }
    }
}

fn circle_mesh(radius: f32, segments: usize) -> Mesh {
    let segments = segments.max(16);
    let mut positions = Vec::with_capacity(segments + 1);
    let mut normals = Vec::with_capacity(segments + 1);
    let mut uvs = Vec::with_capacity(segments + 1);
    let mut indices = Vec::with_capacity(segments * 3);

    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 0.0, 1.0]);
    uvs.push([0.5, 0.5]);

    for index in 0..segments {
        let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
        let (s, c) = angle.sin_cos();
        positions.push([c * radius, s * radius, 0.0]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([c * 0.5 + 0.5, s * 0.5 + 0.5]);
    }

    for index in 0..segments {
        let next = if index + 1 == segments { 1 } else { index + 2 };
        indices.extend_from_slice(&[0, (index + 1) as u32, next as u32]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn ring_mesh(inner_radius: f32, thickness: f32, segments: usize) -> Mesh {
    let segments = segments.max(16);
    let outer_radius = inner_radius + thickness;
    let mut positions = Vec::with_capacity(segments * 2);
    let mut normals = Vec::with_capacity(segments * 2);
    let mut uvs = Vec::with_capacity(segments * 2);
    let mut indices = Vec::with_capacity(segments * 6);

    for index in 0..segments {
        let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
        let (s, c) = angle.sin_cos();
        positions.push([c * inner_radius, s * inner_radius, 0.0]);
        positions.push([c * outer_radius, s * outer_radius, 0.0]);
        normals.push([0.0, 0.0, 1.0]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([0.0, 0.0]);
        uvs.push([1.0, 1.0]);
    }

    for index in 0..segments {
        let inner_a = (index * 2) as u32;
        let outer_a = inner_a + 1;
        let next_index = if index + 1 == segments { 0 } else { index + 1 };
        let inner_b = (next_index * 2) as u32;
        let outer_b = inner_b + 1;
        indices.extend_from_slice(&[inner_a, outer_a, outer_b, inner_a, outer_b, inner_b]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[derive(Component)]
struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &InstanceMaterialData)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut previous_buffers: Local<Vec<(Entity, Buffer, u64)>>,
) {
    for (entity, instance_data) in &query {
        let required_size = std::mem::size_of_val(instance_data.as_slice()) as u64;
        let previous = previous_buffers
            .iter_mut()
            .find(|(buffer_entity, _, _)| *buffer_entity == entity);

        if let Some((_, buffer, capacity)) = previous
            && *capacity >= required_size
        {
            render_queue.write_buffer(buffer, 0, bytemuck::cast_slice(instance_data.as_slice()));
            commands.entity(entity).insert(InstanceBuffer {
                buffer: buffer.clone(),
                length: instance_data.len(),
            });
            continue;
        }

        let capacity = required_size.next_power_of_two().max(1024);
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("organoids instance data buffer"),
            size: capacity,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        render_queue.write_buffer(&buffer, 0, bytemuck::cast_slice(instance_data.as_slice()));

        if let Some((_, previous_buffer, previous_capacity)) = previous {
            *previous_buffer = buffer.clone();
            *previous_capacity = capacity;
        } else {
            previous_buffers.push((entity, buffer.clone(), capacity));
        }

        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instance_data.len(),
        });
    }
}

#[derive(Resource)]
struct CustomPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
}

fn init_custom_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh_pipeline: Res<MeshPipeline>,
) {
    commands.insert_resource(CustomPipeline {
        shader: asset_server.load(SHADER_ASSET_PATH),
        mesh_pipeline: mesh_pipeline.clone(),
    });
}

impl SpecializedMeshPipeline for CustomPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: 4,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 2,
                    shader_location: 5,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 3,
                    shader_location: 6,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 4,
                    shader_location: 7,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 5,
                    shader_location: 8,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 6,
                    shader_location: 9,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 7,
                    shader_location: 10,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 8,
                    shader_location: 11,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 9,
                    shader_location: 12,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 10,
                    shader_location: 13,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 11,
                    shader_location: 14,
                },
            ],
        });

        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader = self.shader.clone();
        }

        Ok(descriptor)
    }
}

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    DrawMeshInstanced,
);

struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMeshInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = Read<InstanceBuffer>;

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        instance_buffer: Option<&'w InstanceBuffer>,
        (meshes, render_mesh_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };

        let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };

        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Skip;
        };

        let Some(vertex_buffer_slice) =
            mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id)
        else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) =
                    mesh_allocator.mesh_index_slice(&mesh_instance.mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    0..instance_buffer.length as u32,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, 0..instance_buffer.length as u32);
            }
        }

        RenderCommandResult::Success
    }
}

fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CustomPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    material_meshes: Query<(Entity, &MainEntity), With<InstanceMaterialData>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(&ExtractedView, &Msaa)>,
) {
    let draw_custom = transparent_3d_draw_functions.read().id::<DrawCustom>();

    for (view, msaa) in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr)
            | MeshPipelineKey::BLEND_ALPHA;
        let rangefinder = view.rangefinder3d();

        for (entity, main_entity) in &material_meshes {
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*main_entity)
            else {
                continue;
            };
            let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let key =
                view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());
            let pipeline = pipelines
                .specialize(&pipeline_cache, &custom_pipeline, key, &mesh.layout)
                .expect("custom instancing pipeline specialization");

            transparent_phase.add(Transparent3d {
                entity: (entity, *main_entity),
                pipeline,
                draw_function: draw_custom,
                distance: rangefinder.distance(&mesh_instance.center),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}
