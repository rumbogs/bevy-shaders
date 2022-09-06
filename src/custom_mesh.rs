use bevy::{
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        render_asset::RenderAssets,
        render_resource::{
            encase::UniformBuffer, AsBindGroup, AsBindGroupError, BindGroupDescriptor,
            BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingResource, BindingType, BufferBindingType, BufferInitDescriptor, BufferSize,
            BufferUsages, Face, FrontFace, OwnedBindingResource, PolygonMode, PreparedBindGroup,
            PrimitiveState, RenderPipelineDescriptor, SamplerBindingType, ShaderRef, ShaderStages,
            SpecializedMeshPipelineError, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
    },
};

#[derive(Component, Default, Debug, Clone, TypeUuid)]
#[uuid = "e443eac8-85db-4ade-b039-d3ad19f392f2"]
pub struct CustomMesh {
    pub color: Vec4,
    pub offset: f32,
    pub base_color_texture: Option<Handle<Image>>,
}

impl AsBindGroup for CustomMesh {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &RenderAssets<Image>,
        _fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self>, AsBindGroupError> {
        let base_color_texture = self
            .base_color_texture
            .as_ref()
            .ok_or(AsBindGroupError::RetryNextUpdate)?;
        let image = images
            .get(base_color_texture)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;

        let mut color_buffer = UniformBuffer::new(Vec::new());
        color_buffer.write(&self.color).unwrap();
        let mut offset_buffer = UniformBuffer::new(Vec::new());
        offset_buffer.write(&self.offset).unwrap();

        let bindings = vec![
            OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: None,
                    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                    contents: color_buffer.as_ref(),
                },
            )),
            OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: None,
                    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                    contents: offset_buffer.as_ref(),
                },
            )),
            OwnedBindingResource::TextureView(image.texture_view.clone()),
            OwnedBindingResource::Sampler(image.sampler.clone()),
        ];

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: bindings[0].get_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: bindings[1].get_binding(),
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
            label: Some("custom_mesh_2d_bind_group"),
            layout,
        });

        Ok(PreparedBindGroup {
            bind_group,
            bindings,
            data: (),
        })
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Custom Mesh2d Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(4 * (std::mem::size_of::<f32>() as u64)),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<f32>() as u64),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}

impl Material for CustomMesh {
    fn vertex_shader() -> ShaderRef {
        "shaders/custom_mesh.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/custom_mesh.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.label = Some("Custom Mesh pipeline descriptor".into());
        descriptor.primitive = PrimitiveState {
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
            topology: key.mesh_key.primitive_topology(),
            strip_index_format: None,
        };

        Ok(())
    }
}
