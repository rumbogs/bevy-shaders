mod custom_mesh;

use custom_mesh::*;

use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute},
        render_phase::{
            EntityRenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
            BlendState, BufferBindingType, BufferSize, ColorTargetState, ColorWrites, Face,
            FragmentState, FrontFace, MultisampleState, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPipelineDescriptor, ShaderStages, SpecializedRenderPipeline,
            TextureFormat, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
    sprite::{
        DrawMesh2d, Mesh2dHandle, Mesh2dPipeline, Mesh2dPipelineKey, SetMesh2dBindGroup,
        SetMesh2dViewBindGroup,
    },
    window::close_on_esc,
};

#[derive(Component, Default)]
pub struct CustomMesh2d;

#[derive(Component, Deref, Debug)]
pub struct CustomShader(Handle<Shader>);

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 800.,
            height: 600.,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(CustomMesh2dPlugin)
        .add_startup_system(setup)
        .add_system(close_on_esc)
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, asset_server: Res<AssetServer>) {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

    // Set the position attribute
    let v_pos = vec![
        [0.5, 0.5, 0.0],   // top right
        [0.5, -0.5, 0.0],  // bottom right
        [-0.5, -0.5, 0.0], // bottom left
        [-0.5, 0.5, 0.0],  // top left
    ];
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);

    // Set the color attribute
    let v_color: Vec<u32> = vec![Color::YELLOW.as_linear_rgba_u32(); 4];
    mesh.insert_attribute(
        MeshVertexAttribute::new("Vertex_Color", 1, VertexFormat::Uint32),
        v_color,
    );

    // Set vertex indices
    let indices = vec![
        0, 1, 3, // first triangle
        1, 2, 3, // second triangle
    ];
    mesh.set_indices(Some(Indices::U16(indices)));

    commands.spawn_bundle((
        CustomMesh2d::default(),
        CustomShader(asset_server.load("shaders/custom_mesh_2d.wgsl")),
        Mesh2dHandle(meshes.add(mesh.clone())),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        ComputedVisibility::default(),
    ));
    //commands.spawn_bundle((
    //CustomMesh2d::default(),
    //CustomShader(asset_server.load("shaders/custom_mesh_2d_2.wgsl")),
    //Mesh2dHandle(meshes.add(mesh)),
    //Transform::default(),
    //GlobalTransform::default(),
    //Visibility::default(),
    //ComputedVisibility::default(),
    //));
    commands.spawn_bundle(Camera2dBundle::default());
}

pub struct CustomMesh2dPipeline {
    mesh2d_pipeline: Mesh2dPipeline,
    time_bind_group_layout: BindGroupLayout,
}

impl FromWorld for CustomMesh2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let time_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Custom time uniform"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<f32>() as u64),
                    },
                    count: None,
                }],
            });
        Self {
            mesh2d_pipeline: Mesh2dPipeline::from_world(world),
            time_bind_group_layout,
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct CustomMesh2dPipelineKey {
    original_key: Mesh2dPipelineKey,
    shader: Handle<Shader>,
}

// Implement the SpecializedPipeline to customize the default rendering from Mesh2dPipeline
impl SpecializedRenderPipeline for CustomMesh2dPipeline {
    type Key = CustomMesh2dPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // Customize how we store the meshes vertex attributes in the vertex buffer
        // Our meshes only have position and color
        let formats = vec![
            // Position
            VertexFormat::Float32x3,
            // Color
            VertexFormat::Uint32,
        ];

        let vertex_layout =
            VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, formats);

        RenderPipelineDescriptor {
            label: Some("custom_mesh2d_pipeline".into()),
            // Use the two standard uniforms for 2d meshes
            layout: Some(vec![
                // Bind group 0 is the view uniform
                self.mesh2d_pipeline.view_layout.clone(),
                // Bind group 1 is the mesh uniform
                self.mesh2d_pipeline.mesh_layout.clone(),
                self.time_bind_group_layout.clone(),
            ]),
            vertex: VertexState {
                // Use our custom shader
                shader: key.shader.clone(),
                entry_point: "vertex".into(),
                shader_defs: Vec::new(),
                // Use our custom vertex buffer
                buffers: vec![vertex_layout],
            },
            primitive: PrimitiveState {
                front_face: FrontFace::Cw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.original_key.primitive_topology(),
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.original_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                // Use our custom shader
                shader: key.shader,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
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
    // Set the time uniform as bind group 2
    SetTimeBindGroup<2>,
    // Draw the mesh
    DrawMesh2d,
);

struct SetTimeBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetTimeBindGroup<I> {
    type Param = SRes<TimeMeta>;

    fn render<'w>(
        _view: Entity,
        _item: Entity,
        time_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let time_bind_group = time_meta.into_inner().bind_group.as_ref().unwrap();
        pass.set_bind_group(I, time_bind_group, &[]);

        RenderCommandResult::Success
    }
}
