use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
            BlendState, BufferBindingType, BufferSize, ColorTargetState, ColorWrites, Face,
            FragmentState, FrontFace, MultisampleState, PolygonMode, PrimitiveState,
            RenderPipelineDescriptor, ShaderStages, SpecializedRenderPipeline, TextureFormat,
            VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
    sprite::{Mesh2dPipeline, Mesh2dPipelineKey},
};

pub struct CustomMesh2dPipeline {
    pub mesh2d_pipeline: Mesh2dPipeline,
    pub color_bind_group_layout: BindGroupLayout,
}

impl FromWorld for CustomMesh2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let color_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Custom color uniform"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(16), // 4 * 4 (f32 size is 4)
                    },
                    count: None,
                }],
            });
        Self {
            mesh2d_pipeline: Mesh2dPipeline::from_world(world),
            color_bind_group_layout,
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct CustomMesh2dPipelineKey {
    pub original_key: Mesh2dPipelineKey,
    pub shader: Handle<Shader>,
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
                self.color_bind_group_layout.clone(),
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
