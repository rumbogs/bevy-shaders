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
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};
use bytemuck::{Pod, Zeroable};

#[derive(Component, Deref, DerefMut, Debug)]
pub struct OffsetUniform(pub f32);

impl ExtractComponent for OffsetUniform {
    type Query = &'static OffsetUniform;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        OffsetUniform(**item)
    }
}

#[derive(Component, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InstanceData {
    pub model: Mat4,
}

#[derive(Component, Deref, DerefMut, Debug)]
pub struct InstanceMaterialData(pub Vec<InstanceData>);

impl ExtractComponent for InstanceMaterialData {
    type Query = &'static InstanceMaterialData;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        InstanceMaterialData(item.0.clone())
    }
}

#[derive(Component, Deref, Debug)]
pub struct BaseColorTexture(pub Handle<Image>);

impl ExtractComponent for BaseColorTexture {
    type Query = &'static BaseColorTexture;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        BaseColorTexture((**item).clone())
    }
}

#[derive(Component, Deref, Debug)]
pub struct MixColorTexture(pub Handle<Image>);

impl ExtractComponent for MixColorTexture {
    type Query = &'static MixColorTexture;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        MixColorTexture((**item).clone())
    }
}

#[derive(Component, Deref, Debug)]
pub struct ViewMat(pub Mat4);

impl ExtractComponent for ViewMat {
    type Query = &'static ViewMat;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        ViewMat(**item)
    }
}

#[derive(Component, Deref, Debug)]
pub struct ProjectionMat(pub Mat4);

impl ExtractComponent for ProjectionMat {
    type Query = &'static ProjectionMat;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        ProjectionMat(**item)
    }
}

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<InstanceMaterialData>::default())
            .add_plugin(ExtractComponentPlugin::<ViewMat>::default())
            .add_plugin(ExtractComponentPlugin::<ProjectionMat>::default())
            .add_plugin(ExtractComponentPlugin::<BaseColorTexture>::default())
            .add_plugin(ExtractComponentPlugin::<OffsetUniform>::default())
            .add_plugin(ExtractComponentPlugin::<MixColorTexture>::default());

        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom>()
            .init_resource::<CustomMaterialPipeline>()
            .init_resource::<SpecializedMeshPipelines<CustomMaterialPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_custom_material)
            .add_system_to_stage(RenderStage::Prepare, prepare_buffers);
    }
}

fn queue_custom_material(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CustomMaterialPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomMaterialPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    meshes: Res<RenderAssets<Mesh>>,
    material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<InstanceMaterialData>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .get_id::<DrawCustom>()
        .unwrap();

    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

    for (view, mut transparent_phase) in &mut views {
        let rangefinder = view.rangefinder3d();
        for (entity, mesh_uniform, mesh_handle) in &material_meshes {
            if let Some(mesh) = meshes.get(mesh_handle) {
                let key =
                    msaa_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                let pipeline = pipelines
                    .specialize(&mut pipeline_cache, &custom_pipeline, key, &mesh.layout)
                    .unwrap();
                transparent_phase.add(Transparent3d {
                    entity,
                    pipeline,
                    draw_function: draw_custom,
                    distance: rangefinder.distance(&mesh_uniform.transform),
                });
            }
        }
    }
}

#[derive(Component)]
pub struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}
#[derive(Component)]
pub struct UniformMeta {
    bind_group: BindGroup,
}

fn prepare_buffers(
    mut commands: Commands,
    query: Query<(
        Entity,
        &InstanceMaterialData,
        &OffsetUniform,
        &BaseColorTexture,
        &MixColorTexture,
        &ViewMat,
        &ProjectionMat,
    )>,
    render_device: Res<RenderDevice>,
    pipeline: Res<CustomMaterialPipeline>,
    images: Res<RenderAssets<Image>>,
) {
    for (entity, instance_data, offset, base_tex, mix_tex, view, proj) in &query {
        let instance_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(instance_data.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer: instance_buffer,
            length: instance_data.len(),
        });

        let base_tex_image = images.get(base_tex).unwrap();
        let mix_tex_image = images.get(mix_tex).unwrap();

        let view_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("view mat buffer"),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            contents: bytemuck::cast_slice(&[**view]),
        });
        let proj_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("proj mat buffer"),
            contents: bytemuck::cast_slice(&[**proj]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let offset_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("offset buffer"),
            contents: bytemuck::cast_slice(&[**offset]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("uniform bind group"),
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
                    resource: offset_buffer.as_entire_binding(),
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
                            min_binding_size: BufferSize::new(std::mem::size_of::<f32>() as u64),
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
            array_stride: (std::mem::size_of::<InstanceData>() as u64),
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

type DrawCustom = (
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

//impl AsBindGroup for CustomMesh {
//type Data = ();

//fn as_bind_group(
//&self,
//layout: &BindGroupLayout,
//render_device: &RenderDevice,
//images: &RenderAssets<Image>,
//_fallback_image: &FallbackImage,
//) -> Result<PreparedBindGroup<Self>, AsBindGroupError> {
//// Base color texture
//let base_color_texture = self
//.base_color_texture
//.as_ref()
//.ok_or(AsBindGroupError::RetryNextUpdate)?;
//let image = images
//.get(base_color_texture)
//.ok_or(AsBindGroupError::RetryNextUpdate)?;

//// Mix color texture
//let mix_color_texture = self
//.mix_color_texture
//.as_ref()
//.ok_or(AsBindGroupError::RetryNextUpdate)?;
//let image2 = images
//.get(mix_color_texture)
//.ok_or(AsBindGroupError::RetryNextUpdate)?;

//let mut color_buffer = UniformBuffer::new(Vec::new());
//color_buffer.write(&self.color).unwrap();
//let mut offset_buffer = UniformBuffer::new(Vec::new());
//offset_buffer.write(&self.offset).unwrap();
//let mut model_buffer = UniformBuffer::new(Vec::new());
//model_buffer.write(&self.model).unwrap();
//let mut view_buffer = UniformBuffer::new(Vec::new());
//view_buffer.write(&self.view).unwrap();
//let mut projection_buffer = UniformBuffer::new(Vec::new());
//projection_buffer.write(&self.projection).unwrap();

//let bindings = vec![
//OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
//&BufferInitDescriptor {
//label: None,
//usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
//contents: color_buffer.as_ref(),
//},
//)),
//OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
//&BufferInitDescriptor {
//label: None,
//usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
//contents: offset_buffer.as_ref(),
//},
//)),
//OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
//&BufferInitDescriptor {
//label: None,
//usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
//contents: model_buffer.as_ref(),
//},
//)),
//OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
//&BufferInitDescriptor {
//label: None,
//usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
//contents: view_buffer.as_ref(),
//},
//)),
//OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
//&BufferInitDescriptor {
//label: None,
//usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
//contents: projection_buffer.as_ref(),
//},
//)),
//OwnedBindingResource::TextureView(image.texture_view.clone()),
//OwnedBindingResource::Sampler(image.sampler.clone()),
//OwnedBindingResource::TextureView(image2.texture_view.clone()),
//OwnedBindingResource::Sampler(image2.sampler.clone()),
//];

//let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
//entries: &[
//BindGroupEntry {
//binding: 0,
//resource: bindings[0].get_binding(),
//},
//BindGroupEntry {
//binding: 1,
//resource: bindings[1].get_binding(),
//},
//BindGroupEntry {
//binding: 2,
//resource: bindings[2].get_binding(),
//},
//BindGroupEntry {
//binding: 3,
//resource: bindings[3].get_binding(),
//},
//BindGroupEntry {
//binding: 4,
//resource: bindings[4].get_binding(),
//},
//BindGroupEntry {
//binding: 5,
//resource: BindingResource::TextureView(&image.texture_view),
//},
//BindGroupEntry {
//binding: 6,
//resource: BindingResource::Sampler(&image.sampler),
//},
//BindGroupEntry {
//binding: 7,
//resource: BindingResource::TextureView(&image2.texture_view),
//},
//BindGroupEntry {
//binding: 8,
//resource: BindingResource::Sampler(&image2.sampler),
//},
//],
//label: Some("custom_mesh_bind_group"),
//layout,
//});

//Ok(PreparedBindGroup {
//bind_group,
//bindings,
//data: (),
//})
//}

//fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
//render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
//label: Some("Custom Mesh Bind Group Layout"),
//entries: &[
//BindGroupLayoutEntry {
//binding: 0,
//visibility: ShaderStages::VERTEX_FRAGMENT,
//ty: BindingType::Buffer {
//ty: BufferBindingType::Uniform,
//has_dynamic_offset: false,
//min_binding_size: BufferSize::new(std::mem::size_of::<Vec4>() as u64),
//},
//count: None,
//},
//BindGroupLayoutEntry {
//binding: 1,
//visibility: ShaderStages::VERTEX_FRAGMENT,
//ty: BindingType::Buffer {
//ty: BufferBindingType::Uniform,
//has_dynamic_offset: false,
//min_binding_size: BufferSize::new(std::mem::size_of::<f32>() as u64),
//},
//count: None,
//},
//BindGroupLayoutEntry {
//binding: 2,
//visibility: ShaderStages::VERTEX_FRAGMENT,
//ty: BindingType::Buffer {
//ty: BufferBindingType::Uniform,
//has_dynamic_offset: false,
//min_binding_size: BufferSize::new(std::mem::size_of::<Mat4>() as u64),
//},
//count: None,
//},
//BindGroupLayoutEntry {
//binding: 3,
//visibility: ShaderStages::VERTEX_FRAGMENT,
//ty: BindingType::Buffer {
//ty: BufferBindingType::Uniform,
//has_dynamic_offset: false,
//min_binding_size: BufferSize::new(std::mem::size_of::<Mat4>() as u64),
//},
//count: None,
//},
//BindGroupLayoutEntry {
//binding: 4,
//visibility: ShaderStages::VERTEX_FRAGMENT,
//ty: BindingType::Buffer {
//ty: BufferBindingType::Uniform,
//has_dynamic_offset: false,
//min_binding_size: BufferSize::new(std::mem::size_of::<Mat4>() as u64),
//},
//count: None,
//},
//BindGroupLayoutEntry {
//binding: 5,
//visibility: ShaderStages::VERTEX_FRAGMENT,
//ty: BindingType::Texture {
//multisampled: false,
//sample_type: TextureSampleType::Float { filterable: true },
//view_dimension: TextureViewDimension::D2,
//},
//count: None,
//},
//BindGroupLayoutEntry {
//binding: 6,
//visibility: ShaderStages::VERTEX_FRAGMENT,
//ty: BindingType::Sampler(SamplerBindingType::Filtering),
//count: None,
//},
//BindGroupLayoutEntry {
//binding: 7,
//visibility: ShaderStages::VERTEX_FRAGMENT,
//ty: BindingType::Texture {
//multisampled: false,
//sample_type: TextureSampleType::Float { filterable: true },
//view_dimension: TextureViewDimension::D2,
//},
//count: None,
//},
//BindGroupLayoutEntry {
//binding: 8,
//visibility: ShaderStages::VERTEX_FRAGMENT,
//ty: BindingType::Sampler(SamplerBindingType::Filtering),
//count: None,
//},
//],
//})
//}
//}

//impl Material for CustomMesh {
//fn vertex_shader() -> ShaderRef {
//"shaders/custom_mesh.wgsl".into()
//}

//fn fragment_shader() -> ShaderRef {
//"shaders/custom_mesh.wgsl".into()
//}

//fn specialize(
//_pipeline: &MaterialPipeline<Self>,
//descriptor: &mut RenderPipelineDescriptor,
//_layout: &MeshVertexBufferLayout,
//key: MaterialPipelineKey<Self>,
//) -> Result<(), SpecializedMeshPipelineError> {
//descriptor.label = Some("Custom Mesh pipeline descriptor".into());
//descriptor.primitive = PrimitiveState {
//front_face: FrontFace::Cw,
//cull_mode: None,
//unclipped_depth: false,
//polygon_mode: PolygonMode::Fill,
//conservative: false,
//topology: key.mesh_key.primitive_topology(),
//strip_index_format: None,
//};
//descriptor.depth_stencil = Some(DepthStencilState {
//format: TextureFormat::Depth32Float,
//depth_write_enabled: true,
//depth_compare: CompareFunction::Less,
//stencil: StencilState::default(),
//bias: DepthBiasState::default(),
//});

//Ok(())
//}
//}
