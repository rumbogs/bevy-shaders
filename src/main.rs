mod custom_material;
use custom_material::*;

use bevy::{
    asset::LoadState,
    prelude::*,
    render::{
        mesh::MeshVertexAttribute,
        render_resource::{
            AddressMode, Extent3d, FilterMode, PrimitiveTopology, SamplerDescriptor,
            TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, VertexFormat,
        },
        texture::ImageSampler,
        view::NoFrustumCulling,
    },
    window::close_on_esc,
};

#[derive(Deref, DerefMut, Debug)]
pub struct TextureShaderResources(Option<Vec<Handle<Image>>>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    LoadAssets,
    Main,
}

const CUBE_POS: [Vec3; 10] = [
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(2.0, 5.0, -15.0),
    Vec3::new(-1.5, -2.2, -2.5),
    Vec3::new(-3.8, -2.0, -12.3),
    Vec3::new(2.4, -0.4, -3.5),
    Vec3::new(-1.7, 3.0, -7.5),
    Vec3::new(1.3, -2.0, -2.5),
    Vec3::new(1.5, 2.0, -2.5),
    Vec3::new(1.5, 0.2, -1.5),
    Vec3::new(-1.3, 1.0, -1.5),
];

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 800.,
            height: 600.,
            ..default()
        })
        //.insert_resource(ImageSettings {
        //default_sampler: SamplerDescriptor {
        //address_mode_u: AddressMode::Repeat,
        //address_mode_v: AddressMode::Repeat,
        //address_mode_w: AddressMode::Repeat,
        //..default()
        //},
        //})
        .insert_resource(TextureShaderResources(None))
        .add_plugins(DefaultPlugins)
        .add_plugin(CustomMaterialPlugin)
        .add_state(AppState::LoadAssets)
        .add_system_set(SystemSet::on_enter(AppState::LoadAssets).with_system(load_assets))
        .add_system_set(SystemSet::on_update(AppState::LoadAssets).with_system(assets_loaded))
        .add_system_set(SystemSet::on_enter(AppState::Main).with_system(setup))
        .add_system_set(SystemSet::on_update(AppState::Main).with_system(update_offset))
        .add_system_set(SystemSet::on_update(AppState::Main).with_system(update_model_mat))
        .add_system(close_on_esc)
        .run();
}

fn update_offset(mut query: Query<&mut OffsetUniform>, input: Res<Input<KeyCode>>) {
    if query.is_empty() {
        return;
    }
    let mut offset_uniform = query.get_single_mut().unwrap();
    if input.just_pressed(KeyCode::W) {
        **offset_uniform += 0.1;
    }
    if input.just_pressed(KeyCode::S) {
        **offset_uniform -= 0.1;
    }
}

fn update_model_mat(mut query: Query<&mut InstanceMaterialData>, time: Res<Time>) {
    if query.is_empty() {
        return;
    }
    let mut instance_material_data = query.get_single_mut().unwrap();
    for (i, instance_data) in (**instance_material_data).iter_mut().enumerate().step_by(3) {
        instance_data.model = Mat4::from_translation(CUBE_POS[i])
            * Mat4::from_rotation_y((time.seconds_since_startup() as f32) * 50.0_f32.to_radians())
            * Mat4::from_rotation_x(25.0_f32.to_radians());
    }
}

fn load_assets(
    mut texture_resources: ResMut<TextureShaderResources>,
    asset_server: Res<AssetServer>,
) {
    **texture_resources = Some(vec![
        asset_server.load("textures/wall.png"),
        asset_server.load("textures/awesomeface.png"),
    ]);
}

fn assets_loaded(
    mut state: ResMut<State<AppState>>,
    texture_resources: Res<TextureShaderResources>,
    asset_server: Res<AssetServer>,
) {
    match &**texture_resources {
        Some(textures) => {
            if textures
                .iter()
                .all(|t| asset_server.get_load_state(t.clone_weak()) == LoadState::Loaded)
            {
                state.set(AppState::Main).unwrap();
            }
        }
        None => {}
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    textures: ResMut<TextureShaderResources>,
) {
    match &**textures {
        Some(textures) => {
            // Base color texture
            let mut image = images.get_mut(&textures[0]).unwrap();
            image.texture_descriptor = TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: 512,
                    height: 512,
                    ..default()
                },
                // TODO: figure out why this doesn't work for > 1
                mip_level_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
            };
            image.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::Repeat,
                mag_filter: FilterMode::Nearest,
                ..default()
            });

            // Mix color texture
            let mut image2 = images.get_mut(&textures[1]).unwrap();
            image2.texture_descriptor = TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: 512,
                    height: 512,
                    ..default()
                },
                // TODO: figure out why this doesn't work for > 1
                mip_level_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
            };
            image2.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::Repeat,
                mag_filter: FilterMode::Nearest,
                ..default()
            });

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

            // Set the position attribute
            let v_pos = vec![
                // Face 1
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, -0.5, -0.5],
                // Face 2
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, -0.5, 0.5],
                // Face 3
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                // Face 4
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                // Face 5
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, -0.5, -0.5],
                // Face 6
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ];

            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);

            let v_uv = vec![
                // Face 1
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.0],
                // Face 2
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.0],
                // Face 3
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.0],
                [1.0, 0.0],
                // Face 4
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.0],
                [1.0, 0.0],
                // Face 5
                [0.0, 1.0],
                [1.0, 1.0],
                [1.0, 0.0],
                [1.0, 0.0],
                [0.0, 0.0],
                [0.0, 1.0],
                // Face 6
                [0.0, 1.0],
                [1.0, 1.0],
                [1.0, 0.0],
                [1.0, 0.0],
                [0.0, 0.0],
                [0.0, 1.0],
            ];
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, v_uv);

            // Set the color attribute
            let v_color: Vec<u32> = vec![Color::RED.as_linear_rgba_u32(); 36];
            mesh.insert_attribute(
                MeshVertexAttribute::new("Vertex_Color", 1, VertexFormat::Uint32),
                v_color,
            );

            // Set vertex indices
            //let indices = vec![0, 1, 3, 1, 2, 3];
            //mesh.set_indices(Some(Indices::U16(indices)));

            let view = Mat4::from_translation(Vec3::new(0.0, 0.0, -3.0));
            let projection = Mat4::perspective_rh(45.0_f32.to_radians(), 800.0 / 600.0, 0.1, 100.0);

            commands
                .spawn()
                .insert_bundle((
                    meshes.add(mesh.clone()),
                    InstanceMaterialData(
                        (1..=CUBE_POS.len())
                            .map(|i| InstanceData {
                                model: Mat4::from_translation(CUBE_POS[i - 1])
                                    * Mat4::from_rotation_x((20.0_f32 * i as f32).to_radians()),
                            })
                            .collect(),
                    ),
                    OffsetUniform(0.1),
                    BaseColorTexture(textures[0].clone()),
                    MixColorTexture(textures[1].clone()),
                    ViewMat(view),
                    ProjectionMat(projection),
                    // NOTE: Frustum culling is done based on the Aabb of the Mesh and the GlobalTransform.
                    // As the cube is at the origin, if its Aabb moves outside the view frustum, all the
                    // instanced cubes will be culled.
                    // The InstanceMaterialData contains the 'GlobalTransform' information for this custom
                    // instancing, and that is not taken into account with the built-in frustum culling.
                    // We must disable the built-in frustum culling by adding the `NoFrustumCulling` marker
                    // component to avoid incorrect culling.
                    NoFrustumCulling,
                ))
                .insert_bundle(SpatialBundle::default());
        }
        None => {}
    };

    // This is just used so that we can see something, there's basically another camera created
    // using model, view, projection matrices
    commands.spawn_bundle(Camera3dBundle {
        camera_3d: Camera3d {
            // This is 0.0 by default because 0.0 is the far plane due to bevy's use of reverse-z projections.
            // This goes hand in hand with the DepthStencilState depth_compare
            // If it's Less the load op needs to be 1.0
            // If it's Greater the load op needs to be 0.0
            // TODO: figure out why???
            depth_load_op: bevy::core_pipeline::core_3d::Camera3dDepthLoadOp::Clear(1.0),
            ..default()
        },
        ..default()
    });
}
