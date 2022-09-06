use bevy::{
    core_pipeline::core_2d::Transparent2d,
    ecs::system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer,
            BufferDescriptor, BufferUsages, PipelineCache, SpecializedRenderPipelines,
        },
        renderer::{RenderDevice, RenderQueue},
        view::VisibleEntities,
        Extract, RenderApp, RenderStage,
    },
    sprite::{
        DrawMesh2d, Mesh2dHandle, Mesh2dPipelineKey, Mesh2dUniform, SetMesh2dBindGroup,
        SetMesh2dViewBindGroup,
    },
    utils::FloatOrd,
};
use bytemuck::{Pod, Zeroable};

use crate::{
    ColorUniform, CustomMesh2d, CustomMesh2dPipeline, CustomMesh2dPipelineKey, CustomShader,
    OffsetUniform, TextureShaderResource,
};

pub struct CustomMesh2dPlugin;

impl Plugin for CustomMesh2dPlugin {
    fn build(&self, app: &mut App) {
        let render_device = app.world.resource::<RenderDevice>();

        let buffers = [
            render_device.create_buffer(&BufferDescriptor {
                label: Some("Color Uniform"),
                size: 4 * (std::mem::size_of::<f32>() as u64),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            render_device.create_buffer(&BufferDescriptor {
                label: Some("Offset Uniform"),
                size: std::mem::size_of::<f32>() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        ];

        app.add_plugin(ExtractComponentPlugin::<ExtractedColorUniform>::default())
            .add_plugin(ExtractComponentPlugin::<ExtractedOffsetUniform>::default());

        // Register our custom draw function and pipeline, and add our render systems
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .add_render_command::<Transparent2d, DrawCustomMesh2d>()
            .insert_resource(UniformsMeta {
                buffers,
                bind_group: None,
            })
            .init_resource::<CustomMesh2dPipeline>()
            .init_resource::<SpecializedRenderPipelines<CustomMesh2dPipeline>>()
            .add_system_to_stage(RenderStage::Extract, extract_custom_mesh2d)
            .add_system_to_stage(RenderStage::Extract, extract_texture)
            .add_system_to_stage(RenderStage::Prepare, prepare_uniforms)
            .add_system_to_stage(RenderStage::Queue, queue_custom_mesh2d)
            .add_system_to_stage(RenderStage::Queue, queue_uniforms_bind_group);
    }
}

#[derive(Component, Default, Deref, Pod, Copy, Clone, Zeroable)]
#[repr(C)]
pub struct ExtractedColorUniform(Vec4);

#[derive(Component, Default, Deref, Pod, Copy, Clone, Zeroable)]
#[repr(C)]
pub struct ExtractedOffsetUniform(f32);

#[derive(Deref, DerefMut)]
pub struct ExtractedTexture(Handle<Image>);

impl ExtractComponent for ExtractedColorUniform {
    type Query = Read<ColorUniform>;

    type Filter = ();

    fn extract_component(color: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        ExtractedColorUniform(Vec4::from(**color))
    }
}

impl ExtractComponent for ExtractedOffsetUniform {
    type Query = Read<OffsetUniform>;

    type Filter = ();

    fn extract_component(offset: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        ExtractedOffsetUniform(**offset)
    }
}

pub struct UniformsMeta {
    pub buffers: [Buffer; 2],
    pub bind_group: Option<BindGroup>,
}

// Extract the [`CustomMesh2d`] marker component into the render app
pub fn extract_custom_mesh2d(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    // When extracting, you must use 'Extract' to mark the 'SystemParam's
    // which should be taken from the main world.
    query: Extract<Query<(Entity, &ComputedVisibility, &CustomShader), With<CustomMesh2d>>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, computed_visibility, shader) in query.iter() {
        if !computed_visibility.is_visible() {
            continue;
        }

        values.push((entity, (CustomMesh2d, CustomShader((**shader).clone()))));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

// Extract the [`TextureShaderResource`] into the render app
pub fn extract_texture(
    mut commands: Commands,
    // When extracting, you must use 'Extract' to mark the 'SystemParam's
    // which should be taken from the main world.
    texture: Extract<Res<TextureShaderResource>>,
) {
    match &texture.0 {
        Some(t) => {
            commands.insert_resource(ExtractedTexture(t.clone()));
        }
        None => {}
    };
}

// Write the extracted uniforms into the corresponding uniform buffer
fn prepare_uniforms(
    color_uniform_query: Query<&ExtractedColorUniform>,
    offset_uniform_query: Query<&ExtractedOffsetUniform>,
    uniforms_meta: ResMut<UniformsMeta>,
    render_queue: Res<RenderQueue>,
) {
    if color_uniform_query.is_empty() && offset_uniform_query.is_empty() {
        return;
    }
    let color_uniform = color_uniform_query.get_single().unwrap();
    let offset_uniform = offset_uniform_query.get_single().unwrap();
    render_queue.write_buffer(
        &uniforms_meta.buffers[0],
        0,
        bevy::core::cast_slice(&[**color_uniform]),
    );
    render_queue.write_buffer(
        &uniforms_meta.buffers[1],
        0,
        bevy::core::cast_slice(&[**offset_uniform]),
    );
}

// Create a bind group for the time uniform buffer
fn queue_uniforms_bind_group(
    render_device: Res<RenderDevice>,
    mut uniforms_meta: ResMut<UniformsMeta>,
    pipeline: Res<CustomMesh2dPipeline>,
    texture: Res<ExtractedTexture>,
    images: Res<RenderAssets<Image>>,
) {
    let image = images.get(&**texture).unwrap();
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: uniforms_meta.buffers[0].as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: uniforms_meta.buffers[1].as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(&image.texture_view),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::Sampler(&image.sampler),
            },
        ],
    });
    uniforms_meta.bind_group = Some(bind_group);
}

// Queue the 2d meshes marked with ['CustomMesh2d'] using our custom pipeline and draw function
pub fn queue_custom_mesh2d(
    transparent_draw_functions: Res<DrawFunctions<Transparent2d>>,
    custom_mesh2d_pipeline: Res<CustomMesh2dPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<CustomMesh2dPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    custom_mesh2d: Query<(&Mesh2dHandle, &Mesh2dUniform, &CustomShader), With<CustomMesh2d>>,
    mut views: Query<(&VisibleEntities, &mut RenderPhase<Transparent2d>)>,
) {
    if custom_mesh2d.is_empty() {
        return;
    }

    // Iterate each view (a camera is a view)
    for (visible_entities, mut transparent_phase) in &mut views {
        let draw_custom_mesh2d = transparent_draw_functions
            .read()
            .get_id::<DrawCustomMesh2d>()
            .unwrap();

        let mesh_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples);

        // Queue all entities visible to that view
        for visible_entity in &visible_entities.entities {
            if let Ok((mesh2d_handle, mesh2d_uniform, shader)) = custom_mesh2d.get(*visible_entity)
            {
                // Get our specialized pipeline
                let mut mesh2d_key = mesh_key;
                if let Some(mesh) = render_meshes.get(&mesh2d_handle.0) {
                    mesh2d_key |=
                        Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology);
                }

                let pipeline_id = pipelines.specialize(
                    &mut pipeline_cache,
                    &custom_mesh2d_pipeline,
                    // Hack to pass the shader to the pipeline, this allows for using the same
                    // pipeline with multiple shaders
                    CustomMesh2dPipelineKey {
                        original_key: mesh2d_key,
                        shader: (**shader).clone(),
                    },
                );

                let mesh_z = mesh2d_uniform.transform.w_axis.z;
                transparent_phase.add(Transparent2d {
                    // The 2d render items are sorted according to their z value before rendering
                    // in order to get correct transparency
                    sort_key: FloatOrd(mesh_z),
                    entity: *visible_entity,
                    pipeline: pipeline_id,
                    draw_function: draw_custom_mesh2d,
                    batch_range: None,
                });
            }
        }
    }
}

// This specifies how to render a custom mesh 2d
type DrawCustomMesh2d = (
    // Set the pipeline
    SetItemPipeline,
    // Set the view uniform as bind group 0
    SetMesh2dViewBindGroup<0>,
    // Set the mesh uniform as bind group 1
    SetMesh2dBindGroup<1>,
    // Set the uniforms as bind group 2
    SetUniformsBindGroup<2>,
    // Draw the mesh
    DrawMesh2d,
);

struct SetUniformsBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetUniformsBindGroup<I> {
    type Param = SRes<UniformsMeta>;

    fn render<'w>(
        _view: Entity,
        _item: Entity,
        uniforms: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let uniforms_bind_group = uniforms.into_inner().bind_group.as_ref().unwrap();
        pass.set_bind_group(I, uniforms_bind_group, &[]);

        RenderCommandResult::Success
    }
}
