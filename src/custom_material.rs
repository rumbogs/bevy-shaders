use crate::{
    CustomCamera, DiffuseTexture, DirectionalLight, EmissionTexture, PointLightInstances,
    PointLightMaterial, SpecularTexture, Spotlight,
};
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
            encase::UniformBuffer, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages, CompareFunction,
            DepthBiasState, DepthStencilState, FrontFace, PipelineCache, PolygonMode,
            PrimitiveState, RenderPipelineDescriptor, SamplerBindingType, ShaderStages, ShaderType,
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

#[derive(Component, Debug, Clone, Copy)]
#[repr(C)]
pub struct MaterialInstance {
    pub position: Vec3,
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub rotation_z: f32,
    pub shininess: f32,
}

#[derive(Component, Deref, DerefMut, Debug)]
pub struct MaterialInstances(pub Vec<MaterialInstance>);

// The array size needs to be kept in sync with the custom_mesh.wgsl shader
const NR_POINT_LIGHTS: usize = 4;

impl ExtractComponent for MaterialInstances {
    type Query = &'static MaterialInstances;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        MaterialInstances(item.0.clone())
    }
}

#[derive(Component, Clone, Copy)]
#[repr(C)]
pub struct CustomMaterial;

impl ExtractComponent for CustomMaterial {
    type Query = &'static CustomMaterial;
    type Filter = ();

    fn extract_component(_item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        CustomMaterial
    }
}

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<CustomMaterial>::default())
            .add_plugin(ExtractComponentPlugin::<MaterialInstances>::default());
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
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct RenderMaterialInstance {
    // These need to be arrays, otherwise we couldn't derive Pod due to padding with shininess
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 4],
    shininess: f32,
}

#[derive(Debug, Copy, Clone, ShaderType)]
#[repr(C)]
struct DirectionalLightSettings {
    direction: Vec3,
    ambient: Vec4,
    diffuse: Vec4,
    specular: Vec4,
}

#[derive(Debug, Copy, Clone, ShaderType)]
#[repr(C)]
struct PointLightSettings {
    position: Vec3,
    constant: f32,
    linear: f32,
    quadratic: f32,
    ambient: Vec4,
    diffuse: Vec4,
    specular: Vec4,
}

#[derive(Debug, Copy, Clone, ShaderType)]
#[repr(C)]
struct SpotlightSettings {
    direction: Vec3,
    position: Vec3,
    cutoff: f32,
    outer_cutoff: f32,
    ambient: Vec4,
    diffuse: Vec4,
    specular: Vec4,
    constant: f32,
    linear: f32,
    quadratic: f32,
}

impl Default for PointLightSettings {
    fn default() -> Self {
        Self {
            position: Vec3::splat(0.0),
            constant: 1.0,
            linear: 0.1,
            quadratic: 0.01,
            ambient: Vec4::splat(0.0),
            diffuse: Vec4::splat(0.0),
            specular: Vec4::splat(0.0),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_buffers(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &MaterialInstances,
            &DiffuseTexture,
            &SpecularTexture,
            &EmissionTexture,
        ),
        With<CustomMaterial>,
    >,
    lights_query: Query<&PointLightInstances, With<PointLightMaterial>>,
    camera: Res<CustomCamera>,
    render_device: Res<RenderDevice>,
    pipeline: Res<CustomMaterialPipeline>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    dir_light: Res<DirectionalLight>,
    spot_light: Res<Spotlight>,
) {
    for (entity, instance_data, diff_tex, spec_tex, emission_tex) in &query {
        let render_instance_data = instance_data
            .iter()
            .map(|instance| {
                let model = Mat4::from_translation(instance.position)
                    * Mat4::from_rotation_y(instance.rotation_y)
                    * Mat4::from_rotation_x(instance.rotation_x);
                RenderMaterialInstance {
                    model: model.to_cols_array_2d(),
                    normal: model.inverse().transpose().to_cols_array_2d(),
                    shininess: instance.shininess,
                }
            })
            .collect::<Vec<RenderMaterialInstance>>();

        let instance_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(render_instance_data.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer: instance_buffer,
            length: instance_data.len(),
        });

        // TODO: Figure out why the fallback image doesn't work
        let diff_tex_image = images.get(diff_tex).unwrap_or(&fallback_image);
        let spec_tex_image = images.get(spec_tex).unwrap_or(&fallback_image);
        let emission_tex_image = images.get(emission_tex).unwrap_or(&fallback_image);

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

        let mut spot_light_mat_buf = UniformBuffer::new(Vec::new());
        spot_light_mat_buf
            .write(&SpotlightSettings {
                direction: camera.get_direction(),
                position: camera.position,
                cutoff: spot_light.cutoff.to_radians().cos(),
                outer_cutoff: spot_light.outer_cutoff.to_radians().cos(),
                ambient: spot_light.ambient,
                diffuse: spot_light.diffuse,
                specular: spot_light.specular,
                constant: spot_light.constant,
                linear: spot_light.linear,
                quadratic: spot_light.quadratic,
            })
            .unwrap();
        let spot_light_mat_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("spot light buffer"),
            contents: spot_light_mat_buf.as_ref(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let mut dir_light_mat_buf = UniformBuffer::new(Vec::new());
        dir_light_mat_buf
            .write(&DirectionalLightSettings {
                direction: dir_light.direction,
                ambient: dir_light.ambient,
                diffuse: dir_light.diffuse,
                specular: dir_light.specular,
            })
            .unwrap();
        let dir_light_mat_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("light color buffer"),
            contents: dir_light_mat_buf.as_ref(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let mut point_lights = [PointLightSettings::default(); NR_POINT_LIGHTS];
        if let Ok(light_instances) = lights_query.get_single() {
            for (i, instance) in light_instances.iter().enumerate() {
                if i < 4 {
                    point_lights[i] = PointLightSettings {
                        position: instance.position,
                        constant: instance.constant,
                        linear: instance.linear,
                        quadratic: instance.quadratic,
                        ambient: instance.ambient,
                        diffuse: instance.diffuse,
                        specular: instance.specular,
                    };
                }
            }
        }
        let mut point_lights_mat_buf = UniformBuffer::new(Vec::new());
        point_lights_mat_buf.write(&point_lights).unwrap();

        let point_light_mat_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("light color buffer"),
            contents: point_lights_mat_buf.as_ref(),
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
                    resource: BindingResource::TextureView(&diff_tex_image.texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&diff_tex_image.sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&spec_tex_image.texture_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(&spec_tex_image.sampler),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&emission_tex_image.texture_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::Sampler(&emission_tex_image.sampler),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: view_pos_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: dir_light_mat_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: point_light_mat_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: spot_light_mat_buffer.as_entire_binding(),
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
                        ty: BindingType::Texture {
                            multisampled: false,
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
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
                            min_binding_size: Some(DirectionalLightSettings::min_size()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                (std::mem::size_of::<PointLightSettings>() * NR_POINT_LIGHTS)
                                    as u64,
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 11,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(SpotlightSettings::min_size()),
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
            array_stride: (std::mem::size_of::<RenderMaterialInstance>() as u64),
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
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: VertexFormat::Float32x4.size() * 8,
                    shader_location: 11,
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
