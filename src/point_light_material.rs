use crate::{CustomCamera, InstanceBuffer, UniformMeta};
use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingType, BufferBindingType, BufferInitDescriptor, BufferSize,
            BufferUsages, CompareFunction, DepthBiasState, DepthStencilState, FrontFace,
            PipelineCache, PolygonMode, PrimitiveState, RenderPipelineDescriptor, ShaderStages,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            StencilState, TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat,
            VertexStepMode,
        },
        renderer::RenderDevice,
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};
use bytemuck::{Pod, Zeroable};

#[derive(Component, Debug, Clone, Copy)]
pub struct PointLightInstance {
    pub position: Vec3,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
    pub ambient: Vec4,
    pub diffuse: Vec4,
    pub specular: Vec4,
}

#[derive(Component, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct RenderPointLightInstance {
    position: Mat4,
    ambient: Vec4,
    diffuse: Vec4,
    specular: Vec4,
}

#[derive(Component, Deref, DerefMut, Debug)]
pub struct PointLightInstances(pub Vec<PointLightInstance>);

impl ExtractComponent for PointLightInstances {
    type Query = &'static PointLightInstances;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        PointLightInstances(item.0.clone())
    }
}

#[derive(Component)]
pub struct PointLightMaterial;

impl ExtractComponent for PointLightMaterial {
    type Query = &'static PointLightMaterial;
    type Filter = ();

    fn extract_component(_item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        PointLightMaterial
    }
}

pub struct PointLightMaterialPlugin;

impl Plugin for PointLightMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<PointLightMaterial>::default())
            .add_plugin(ExtractComponentPlugin::<PointLightInstances>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawLightMaterial>()
            .init_resource::<PointLightMaterialPipeline>()
            .init_resource::<SpecializedMeshPipelines<PointLightMaterialPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_point_light_material)
            .add_system_to_stage(RenderStage::Prepare, prepare_point_light_material_buffers);
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_point_light_material(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    light_material_pipeline: Res<PointLightMaterialPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<PointLightMaterialPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    meshes: Res<RenderAssets<Mesh>>,
    light_material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<PointLightMaterial>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_light_material = transparent_3d_draw_functions
        .read()
        .get_id::<DrawLightMaterial>()
        .unwrap();

    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

    for (view, mut transparent_phase) in &mut views {
        let rangefinder = view.rangefinder3d();
        for (entity, mesh_uniform, mesh_handle) in &light_material_meshes {
            if let Some(mesh) = meshes.get(mesh_handle) {
                let key =
                    msaa_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                let pipeline = pipelines
                    .specialize(
                        &mut pipeline_cache,
                        &light_material_pipeline,
                        key,
                        &mesh.layout,
                    )
                    .unwrap();
                transparent_phase.add(Transparent3d {
                    entity,
                    pipeline,
                    draw_function: draw_light_material,
                    distance: rangefinder.distance(&mesh_uniform.transform),
                });
            }
        }
    }
}

pub fn prepare_point_light_material_buffers(
    mut commands: Commands,
    query: Query<(Entity, &PointLightInstances), With<PointLightMaterial>>,
    camera: Res<CustomCamera>,
    render_device: Res<RenderDevice>,
    pipeline: Res<PointLightMaterialPipeline>,
) {
    for (entity, light_instances) in &query {
        let instance_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(
                light_instances
                    .iter()
                    .map(|instance| RenderPointLightInstance {
                        position: Mat4::from_translation(instance.position),
                        ambient: instance.ambient,
                        diffuse: instance.diffuse,
                        specular: instance.specular,
                    })
                    .collect::<Vec<RenderPointLightInstance>>()
                    .as_slice(),
            ),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer: instance_buffer,
            length: light_instances.len(),
        });

        let view_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("view mat buffer"),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            contents: bytemuck::cast_slice(&[camera.get_view()]),
        });
        let proj_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("proj mat buffer"),
            contents: bytemuck::cast_slice(&[camera.get_proj()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("light uniform bind group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: proj_buffer.as_entire_binding(),
                },
            ],
        });
        commands.entity(entity).insert(UniformMeta { bind_group });
    }
}

pub struct PointLightMaterialPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for PointLightMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        asset_server.watch_for_changes().unwrap();
        let shader = asset_server.load("shaders/point_light_mesh.wgsl");

        let mesh_pipeline = world.resource::<MeshPipeline>();

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Uniforms"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(std::mem::size_of::<Mat4>() as u64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(std::mem::size_of::<Mat4>() as u64),
                        },
                        count: None,
                    },
                ],
            });

        PointLightMaterialPipeline {
            shader,
            mesh_pipeline: mesh_pipeline.clone(),
            bind_group_layout,
        }
    }
}

impl SpecializedMeshPipeline for PointLightMaterialPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: (std::mem::size_of::<RenderPointLightInstance>() as u64),
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // shader locations 0-2 is taken up by Position, Normal, UV
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
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
            self.bind_group_layout.clone(),
        ]);
        descriptor.label = Some("Custom Mesh pipeline descriptor".into());
        descriptor.primitive = PrimitiveState {
            front_face: FrontFace::Cw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
            topology: key.primitive_topology(),
            strip_index_format: None,
        };
        descriptor.depth_stencil = Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });

        Ok(descriptor)
    }
}

type DrawLightMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetUniformsBindGroup<2>,
    DrawMeshInstanced,
);

pub struct DrawMeshInstanced;

impl EntityRenderCommand for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<Handle<Mesh>>>,
        SQuery<Read<InstanceBuffer>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, mesh_query, instance_buffer_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_handle = mesh_query.get(item).unwrap();
        if let Ok(instance_buffer) = instance_buffer_query.get_inner(item) {
            let gpu_mesh = match meshes.into_inner().get(mesh_handle) {
                Some(gpu_mesh) => gpu_mesh,
                None => return RenderCommandResult::Failure,
            };

            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
                }
                GpuBufferInfo::NonIndexed { vertex_count } => {
                    pass.draw(0..*vertex_count, 0..instance_buffer.length as u32);
                }
            }
        }

        RenderCommandResult::Success
    }
}

struct SetUniformsBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetUniformsBindGroup<I> {
    type Param = SQuery<Read<UniformMeta>>;

    fn render<'w>(
        _view: Entity,
        item: Entity,
        uniforms_query: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Ok(uniforms_meta) = uniforms_query.get_inner(item) {
            pass.set_bind_group(I, &uniforms_meta.bind_group, &[]);
        };

        RenderCommandResult::Success
    }
}
