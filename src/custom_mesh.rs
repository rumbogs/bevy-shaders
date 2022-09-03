use bevy::{
    core_pipeline::core_2d::Transparent2d,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase},
        render_resource::{PipelineCache, SpecializedRenderPipelines},
        view::VisibleEntities,
        Extract, RenderApp, RenderStage,
    },
    sprite::{Mesh2dHandle, Mesh2dPipelineKey, Mesh2dUniform},
    utils::FloatOrd,
};

use crate::{CustomMesh2d, CustomMesh2dPipeline, DrawCustomMesh2d};

pub struct CustomMesh2dPlugin;

impl Plugin for CustomMesh2dPlugin {
    fn build(&self, app: &mut App) {
        // Register our custom draw function and pipeline, and add our render systems
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .add_render_command::<Transparent2d, DrawCustomMesh2d>()
            .init_resource::<CustomMesh2dPipeline>()
            .init_resource::<SpecializedRenderPipelines<CustomMesh2dPipeline>>()
            .add_system_to_stage(RenderStage::Extract, extract_custom_mesh2d)
            .add_system_to_stage(RenderStage::Queue, queue_custom_mesh2d);
    }
}

// Extract the [`CustomMesh2d`] marker component into the render app
pub fn extract_custom_mesh2d(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    // When extracting, you must use 'Extract' to mark the 'SystemParam's
    // which should be taken from the main world.
    query: Extract<Query<(Entity, &ComputedVisibility), With<CustomMesh2d>>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, computed_visibility) in query.iter() {
        if !computed_visibility.is_visible() {
            continue;
        }

        values.push((entity, (CustomMesh2d,)));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

// Queue the 2d meshes marked with ['CustomMesh2d'] using our custom pipeline and draw function
pub fn queue_custom_mesh2d(
    transparent_draw_functions: Res<DrawFunctions<Transparent2d>>,
    custom_mesh2d_pipeline: Res<CustomMesh2dPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<CustomMesh2dPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    custom_mesh2d: Query<(&Mesh2dHandle, &Mesh2dUniform), With<CustomMesh2d>>,
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
            if let Ok((mesh2d_handle, mesh2d_uniform)) = custom_mesh2d.get(*visible_entity) {
                // Get our specialized pipeline
                let mut mesh2d_key = mesh_key;
                if let Some(mesh) = render_meshes.get(&mesh2d_handle.0) {
                    mesh2d_key |=
                        Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology);
                }

                let pipeline_id =
                    pipelines.specialize(&mut pipeline_cache, &custom_mesh2d_pipeline, mesh2d_key);

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
