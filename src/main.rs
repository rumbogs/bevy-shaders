mod custom_mesh;

use std::f32::consts::PI;

use custom_mesh::*;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute},
        render_phase::SetItemPipeline,
        render_resource::{
            BlendState, ColorTargetState, ColorWrites, Face, FragmentState, FrontFace,
            MultisampleState, PolygonMode, PrimitiveState, PrimitiveTopology,
            RenderPipelineDescriptor, SpecializedRenderPipeline, TextureFormat, VertexBufferLayout,
            VertexFormat, VertexState, VertexStepMode,
        },
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

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
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
        Mesh2dHandle(meshes.add(mesh)),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        ComputedVisibility::default(),
    ));
    commands.spawn_bundle(Camera2dBundle::default());
}

pub struct CustomMesh2dPipeline {
    shader: Handle<Shader>,
    mesh2d_pipeline: Mesh2dPipeline,
}

impl FromWorld for CustomMesh2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.load("shaders/custom_mesh_2d.wgsl");
        Self {
            mesh2d_pipeline: Mesh2dPipeline::from_world(world),
            shader,
        }
    }
}

// Implement the SpecializedPipeline to customize the default rendering from Mesh2dPipeline
impl SpecializedRenderPipeline for CustomMesh2dPipeline {
    type Key = Mesh2dPipelineKey;

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
            ]),
            vertex: VertexState {
                // Use our custom shader
                shader: self.shader.clone(),
                entry_point: "vertex".into(),
                shader_defs: Vec::new(),
                // Use our custom vertex buffer
                buffers: vec![vertex_layout],
            },
            primitive: PrimitiveState {
                front_face: FrontFace::Cw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Line,
                conservative: false,
                topology: key.primitive_topology(),
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                // Use our custom shader
                shader: self.shader.clone(),
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
    // Draw the mesh
    DrawMesh2d,
);
