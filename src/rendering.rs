use crate::simulation::{
    ArenaShape, FOOD_RADIUS, FoodKind, FrameStats, GRASS_FOOD_COLOR, LIQUID_CAUSTIC_STRENGTH,
    LIQUID_FLOW_SCALE, LIQUID_FLOW_SPEED, LIQUID_VIGNETTE_STRENGTH, MEAT_FOOD_COLOR, SimConfig,
    WorldState, species_color,
};
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
use bevy::{reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef};
use bytemuck::{Pod, Zeroable};
use std::time::Instant;

const SHADER_ASSET_PATH: &str = "shaders/instanced_disc.wgsl";
const LIQUID_SHADER_ASSET_PATH: &str = "shaders/liquid_medium.wgsl";
const BRANCH_RENDER_SEGMENTS: usize = 6;

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
}

pub struct InstancedDiscPlugin;

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
        app.add_plugins(ExtractComponentPlugin::<InstanceMaterialData>::default());
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
    world: Res<WorldState>,
    mut particles: Query<&mut InstanceMaterialData, With<ParticleLayer>>,
    mut stats: ResMut<FrameStats>,
) {
    let started = Instant::now();

    let mut particles = particles
        .single_mut()
        .expect("particle instanced layer exists");
    particles.clear();
    particles.reserve(
        world.cells.len()
            + world.food.len() * 2
            + world.obstacles.len()
            + world.food_growers.len()
            + world.food_growers.total_branches() * BRANCH_RENDER_SEGMENTS,
    );

    for i in 0..world.cells.len() {
        let heading = world.cells.heading[i];
        let move_dir_x = heading.cos();
        let move_dir_y = heading.sin();
        let jelly = world.cells.jelly_intensity[i];
        let visual_radius = world.cells.visual_radii[i]
            .iter()
            .copied()
            .fold(world.cells.radius[i] * 0.25, f32::max)
            .max(0.1);
        let inv_visual_radius = visual_radius.recip();
        let soft_radii = world.cells.visual_radii[i];

        particles.push(InstanceData {
            pos_radius: [world.cells.x[i], world.cells.y[i], 2.0, visual_radius],
            color: species_color(world.cells.species[i], world.cells.viability_ratio(i)),
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
        let mut z_layer_adjusted = z_layer;
        if world.food.feeder[i] >= 0 {
            let branch_index = world.food.anchor_branch[i];
            if branch_index >= 0 {
                let branch_index = branch_index as usize;
                let layer = world.food_growers.branch_layer[branch_index];
                z_layer_adjusted = 1.48 + layer * 0.20 + 0.05;

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
        });
    }

    for i in 0..world.obstacles.len() {
        let radius = world.obstacles.radius[i];
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
        });
    }

    for i in 0..world.food_growers.len() {
        let radius = world.food_growers.radius[i];
        for branch_index in world.food_growers.branch_range(i) {
            let layer = world.food_growers.branch_layer[branch_index];
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
                let inv_segment_z = (BRANCH_RENDER_SEGMENTS - 1 - segment_index) as f32;

                particles.push(InstanceData {
                    pos_radius: [
                        center.x,
                        center.y,
                        1.48 + layer * 0.20 + inv_segment_z * 0.002,
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

                let layer = world.food_growers.branch_layer[branch_index];
                particles.push(InstanceData {
                    pos_radius: [
                        stem_center.x,
                        stem_center.y,
                        1.48 + layer * 0.20 + 0.04,
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
                });
            }
        }
    }

    stats.upload_time = started.elapsed();
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
            | MeshPipelineKey::from_hdr(view.hdr);
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
