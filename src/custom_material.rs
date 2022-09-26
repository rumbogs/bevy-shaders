use crate::{BaseColorTexture, CustomCamera, InstanceMaterialData, LightMaterial, MixColorTexture};
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
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages, CompareFunction,
            DepthBiasState, DepthStencilState, FrontFace, PipelineCache, PolygonMode,
            PrimitiveState, RenderPipelineDescriptor, SamplerBindingType, ShaderStages,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            StencilState, TextureFormat, TextureSampleType, TextureViewDimension, VertexAttribute,
            VertexBufferLayout, VertexFormat, VertexStepMode,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};
use bytemuck::{Pod, Zeroable};

#[derive(Component)]
pub struct CustomMaterial;

impl ExtractComponent for CustomMaterial {
    type Query = &'static CustomMaterial;
    type Filter = ();

    fn extract_component(_item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        CustomMaterial
    }
}

#[derive(Component, Deref, DerefMut, Debug)]
pub struct ColorUniform(pub [f32; 4]);

impl ExtractComponent for ColorUniform {
    type Query = &'static ColorUniform;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        ColorUniform(**item)
    }
}

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<CustomMaterial>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawCustomMaterial>()
            .init_resource::<CustomMaterialPipeline>()
            .init_resource::<SpecializedMeshPipelines<CustomMaterialPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_custom_material)
            .add_system_to_stage(RenderStage::Prepare, prepare_buffers);
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_custom_material(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_material_pipeline: Res<CustomMaterialPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomMaterialPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    meshes: Res<RenderAssets<Mesh>>,
    custom_material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<CustomMaterial>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom_material = transparent_3d_draw_functions
        .read()
        .get_id::<DrawCustomMaterial>()
        .unwrap();

    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

    for (view, mut transparent_phase) in &mut views {
        let rangefinder = view.rangefinder3d();
        for (entity, mesh_uniform, mesh_handle) in &custom_material_meshes {
            if let Some(mesh) = meshes.get(mesh_handle) {
                let key =
                    msaa_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                let pipeline = pipelines
                    .specialize(
                        &mut pipeline_cache,
                        &custom_material_pipeline,
                        key,
                        &mesh.layout,
                    )
                    .unwrap();
                transparent_phase.add(Transparent3d {
                    entity,
                    pipeline,
                    draw_function: draw_custom_material,
                    distance: rangefinder.distance(&mesh_uniform.transform),
                });
            }
        }
    }
}

#[derive(Component)]
pub struct InstanceBuffer {
    pub buffer: Buffer,
    pub length: usize,
}
#[derive(Component, Debug)]
pub struct UniformMeta {
    pub bind_group: BindGroup,
}
#[derive(Component, Debug, Pod, Zeroable, Copy, Clone)]
#[repr(C)]
struct RenderInstanceData {
    model: Mat4,
    normal: Mat4,
}

#[allow(clippy::too_many_arguments)]
fn prepare_buffers(
    mut commands: Commands,
    query: Query<(
        Entity,
        &InstanceMaterialData,
        &ColorUniform,
        &BaseColorTexture,
        &MixColorTexture,
    )>,
    lights_query: Query<(&ColorUniform, &InstanceMaterialData), With<LightMaterial>>,
    camera: Res<CustomCamera>,
    render_device: Res<RenderDevice>,
    pipeline: Res<CustomMaterialPipeline>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
) {
    for (entity, instance_data, color, base_tex, mix_tex) in &query {
        let instance_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(
                instance_data
                    .iter()
                    .map(|instance| {
                        let model = Mat4::from_translation(instance.position);
                        RenderInstanceData {
                            model,
                            normal: model.inverse().transpose(),
                        }
                    })
                    .collect::<Vec<RenderInstanceData>>()
                    .as_slice(),
            ),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer: instance_buffer,
            length: instance_data.len(),
        });

        // TODO: Figure out why the fallback image doesn't work
        let base_tex_image = images.get(base_tex).unwrap_or(&fallback_image);
        let mix_tex_image = images.get(mix_tex).unwrap_or(&fallback_image);

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
        let color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("color buffer"),
            contents: bytemuck::cast_slice(&[**color]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        // Default to WHITE if there is no light
        // We know there's only one light here so add only one binding
        // Ideally this could add a number of default black lights so that it could support
        // multiple lights in case they exist, but right now we know there's only 1
        let mut lights_color = Color::WHITE.as_rgba_f32();
        let mut lights_pos = Vec3::ZERO;
        if let Ok((color, instance_data)) = lights_query.get_single() {
            lights_color = **color;
            lights_pos = (**instance_data)[0].position;
        }
        let light_color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("light color buffer"),
            contents: bytemuck::cast_slice(&[lights_color]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let light_pos_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("light pos buffer"),
            contents: bytemuck::cast_slice(&[lights_pos]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let view_pos_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("view pos buffer"),
            contents: bytemuck::cast_slice(&[camera.position]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("custom material uniform bind group"),
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
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&base_tex_image.texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&base_tex_image.sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&mix_tex_image.texture_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(&mix_tex_image.sampler),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: color_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: light_color_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: light_pos_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: view_pos_buffer.as_entire_binding(),
                },
            ],
        });
        commands.entity(entity).insert(UniformMeta { bind_group });
    }
}

pub struct CustomMaterialPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for CustomMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        asset_server.watch_for_changes().unwrap();
        let shader = asset_server.load("shaders/custom_mesh.wgsl");

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
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                std::mem::size_of::<[f32; 4]>() as u64
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                std::mem::size_of::<[f32; 4]>() as u64
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                std::mem::size_of::<[f32; 3]>() as u64
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 9,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                std::mem::size_of::<[f32; 3]>() as u64
                            ),
                        },
                        count: None,
                    },
                ],
            });

        CustomMaterialPipeline {
            shader,
            mesh_pipeline: mesh_pipeline.clone(),
            bind_group_layout,
        }
    }
}

impl SpecializedMeshPipeline for CustomMaterialPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: (std::mem::size_of::<RenderInstanceData>() as u64),
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // shader locations 0-2 are taken up by Position, Normal and UV attributes
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

type DrawCustomMaterial = (
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
