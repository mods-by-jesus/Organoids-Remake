use crate::simulation::{
    FOOD_RADIUS, FoodKind, FrameStats, GRASS_FOOD_COLOR, LIQUID_CAUSTIC_STRENGTH,
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

#[derive(Component)]
pub struct ParticleLayer;

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
                arena_size: Vec4::new(config.width, config.height, 0.0, 0.0),
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
        world.cells.len() + world.food.len() + world.obstacles.len() + world.food_growers.len(),
    );

    for i in 0..world.cells.len() {
        let speed =
            (world.cells.vx[i] * world.cells.vx[i] + world.cells.vy[i] * world.cells.vy[i]).sqrt();
        let inv_speed = if speed > 0.001 { speed.recip() } else { 0.0 };
        let move_dir_x = world.cells.vx[i] * inv_speed;
        let move_dir_y = world.cells.vy[i] * inv_speed;
        let jelly = world.cells.jelly_intensity[i];
        let dir_x = if jelly > 0.01 {
            world.cells.jelly_dir_x[i]
        } else {
            move_dir_x
        };
        let dir_y = if jelly > 0.01 {
            world.cells.jelly_dir_y[i]
        } else {
            move_dir_y
        };

        particles.push(InstanceData {
            pos_radius: [
                world.cells.x[i],
                world.cells.y[i],
                2.0,
                world.cells.radius[i],
            ],
            color: species_color(world.cells.species[i], world.cells.energy[i]),
            nucleus: [
                world.cells.nucleus_offset_x[i] / world.cells.radius[i],
                world.cells.nucleus_offset_y[i] / world.cells.radius[i],
                world.cells.nucleus_radius[i] / world.cells.radius[i],
                1.0,
            ],
            motion: [dir_x, dir_y, jelly, world.cells.jelly_phase[i]],
            shape: [
                world.cells.shape_wave_a[i],
                world.cells.shape_wave_b[i],
                world.cells.shape_phase[i],
                world.cells.shape_softness[i],
            ],
        });
    }

    for i in 0..world.food.len() {
        let color = match world.food.kind[i] {
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

        particles.push(InstanceData {
            pos_radius: [world.food.x[i], world.food.y[i], 3.0, FOOD_RADIUS],
            color,
            nucleus: [0.0, 0.0, 0.0, world.food.kind[i].shader_kind()],
            motion: [0.0, 1.0, 0.0, world.food.phase[i]],
            shape: [
                lobes,
                roughness,
                world.food.phase[i],
                world.food.shape[i].shader_shape(),
            ],
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
        });
    }

    for i in 0..world.food_growers.len() {
        let radius = world.food_growers.radius[i];
        particles.push(InstanceData {
            pos_radius: [
                world.food_growers.x[i],
                world.food_growers.y[i],
                1.8,
                radius * 1.85,
            ],
            color: [0.24, 1.0, 0.42, 0.24],
            nucleus: [0.0, 0.0, 0.0, 3.0],
            motion: [
                world.food_growers.vx[i],
                world.food_growers.vy[i],
                world.food_growers.rotation[i],
                world.food_growers.phase[i],
            ],
            shape: [
                world.food_growers.branches[i],
                world.food_growers.radius[i],
                world.food_growers.timer[i],
                0.0,
            ],
        });
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

    commands.spawn((
        Name::new("arena_background"),
        Mesh3d(meshes.add(Cuboid::new(config.width, config.height, 0.4))),
        MeshMaterial3d(background),
        Transform::from_xyz(0.0, 0.0, -0.4),
    ));

    let thickness = 80.0;
    let half_w = config.width * 0.5;
    let half_h = config.height * 0.5;

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
        ));
    }
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
