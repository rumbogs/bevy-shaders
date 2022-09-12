mod custom_mesh;
use custom_mesh::*;

use bevy::{
    asset::LoadState,
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute},
        render_resource::{
            AddressMode, Extent3d, FilterMode, PrimitiveTopology, SamplerDescriptor,
            TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, VertexFormat,
        },
        texture::{ImageSampler, ImageSettings},
    },
    window::close_on_esc,
};

#[derive(Component, Deref, Debug)]
pub struct CustomMeshHandle(Handle<CustomMesh>);

#[derive(Component, Deref, DerefMut, Debug)]
pub struct ColorUniform(Color);

#[derive(Component, Deref, Debug)]
pub struct OffsetUniform(f32);

#[derive(Component)]
pub struct TopLeft;

#[derive(Component)]
pub struct BottomRight;

#[derive(Deref, DerefMut, Debug)]
pub struct TextureShaderResources(Option<Vec<Handle<Image>>>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    LoadAssets,
    Main,
}

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
        .add_plugin(MaterialPlugin::<CustomMesh>::default())
        .add_state(AppState::LoadAssets)
        .add_system_set(SystemSet::on_enter(AppState::LoadAssets).with_system(load_assets))
        .add_system_set(SystemSet::on_update(AppState::LoadAssets).with_system(assets_loaded))
        .add_system_set(SystemSet::on_enter(AppState::Main).with_system(setup))
        .add_system_set(SystemSet::on_update(AppState::Main).with_system(update_custom_color))
        .add_system_set(SystemSet::on_update(AppState::Main).with_system(update_offset))
        .add_system_set(SystemSet::on_update(AppState::Main).with_system(update_transform_matrix))
        .add_system_set(SystemSet::on_update(AppState::Main).with_system(update_transform_matrix2))
        .add_system(close_on_esc)
        .run();
}

fn update_custom_color(
    mut query: Query<&mut CustomMeshHandle, With<BottomRight>>,
    mut materials: ResMut<Assets<CustomMesh>>,
    time: Res<Time>,
) {
    if query.is_empty() {
        return;
    }
    let custom_mesh_handle = query.get_single_mut().unwrap();
    let material = materials.get_mut(&**custom_mesh_handle).unwrap();
    material.color.y = (time.seconds_since_startup() as f32).sin() / 2.0 + 0.5;
}

fn update_offset(
    mut query: Query<&mut CustomMeshHandle, With<BottomRight>>,
    mut materials: ResMut<Assets<CustomMesh>>,
    input: Res<Input<KeyCode>>,
) {
    if query.is_empty() {
        return;
    }
    let custom_mesh_handle = query.get_single_mut().unwrap();
    let material = materials.get_mut(&**custom_mesh_handle).unwrap();
    if input.just_pressed(KeyCode::W) {
        material.offset += 0.1;
    }
    if input.just_pressed(KeyCode::S) {
        material.offset -= 0.1;
    }
}

fn update_transform_matrix(
    mut query: Query<&mut CustomMeshHandle, With<BottomRight>>,
    mut materials: ResMut<Assets<CustomMesh>>,
    time: Res<Time>,
) {
    if query.is_empty() {
        return;
    }
    let custom_mesh_handle = query.get_single_mut().unwrap();
    let material = materials.get_mut(&**custom_mesh_handle).unwrap();
    let transform_matrix = Mat4::from_translation(Vec3::new(0.5, -0.5, 0.0))
        * Mat4::from_rotation_z(time.seconds_since_startup() as f32);
    material.transform_matrix = transform_matrix;
}

fn update_transform_matrix2(
    mut query: Query<&mut CustomMeshHandle, With<TopLeft>>,
    mut materials: ResMut<Assets<CustomMesh>>,
    time: Res<Time>,
) {
    if query.is_empty() {
        return;
    }
    let custom_mesh_handle = query.get_single_mut().unwrap();
    let material = materials.get_mut(&**custom_mesh_handle).unwrap();
    let transform_matrix = Mat4::from_translation(Vec3::new(-0.5, 0.5, 0.0))
        * Mat4::from_scale(Vec3::new(
            (time.seconds_since_startup() as f32).sin(),
            (time.seconds_since_startup() as f32).sin(),
            1.0,
        ));
    material.transform_matrix = transform_matrix;
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
    mut custom_mesh_materials: ResMut<Assets<CustomMesh>>,
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
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
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
                [0.5, 0.5, 0.0],
                [0.5, -0.5, 0.0],
                [-0.5, -0.5, 0.0],
                [-0.5, 0.5, 0.0],
            ];
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);

            let v_uv = vec![[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]];
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, v_uv);

            // Set the color attribute
            let v_color: Vec<u32> = vec![
                Color::RED.as_linear_rgba_u32(),
                Color::GREEN.as_linear_rgba_u32(),
                Color::BLUE.as_linear_rgba_u32(),
                Color::YELLOW.as_linear_rgba_u32(),
            ];
            mesh.insert_attribute(
                MeshVertexAttribute::new("Vertex_Color", 1, VertexFormat::Uint32),
                v_color,
            );

            // Set vertex indices
            let indices = vec![0, 1, 3, 1, 2, 3];
            mesh.set_indices(Some(Indices::U16(indices)));

            let transform_matrix =
                Mat4::from_rotation_z(90.0_f32.to_radians()) * Mat4::from_scale(Vec3::splat(0.5));
            let custom_mesh_handle = custom_mesh_materials.add(CustomMesh {
                color: Vec4::from(Color::BLACK),
                offset: 0.1,
                base_color_texture: Some(textures[0].clone()),
                mix_color_texture: Some(textures[1].clone()),
                transform_matrix,
            });
            let custom_mesh_handle2 = custom_mesh_materials.add(CustomMesh {
                color: Vec4::from(Color::BLACK),
                offset: 0.1,
                base_color_texture: Some(textures[0].clone()),
                mix_color_texture: Some(textures[1].clone()),
                transform_matrix,
            });

            commands
                .spawn_bundle(MaterialMeshBundle::<CustomMesh> {
                    mesh: meshes.add(mesh.clone()),
                    material: custom_mesh_handle.clone(),
                    ..default()
                })
                .insert(CustomMeshHandle(custom_mesh_handle))
                .insert(BottomRight);

            commands
                .spawn_bundle(MaterialMeshBundle::<CustomMesh> {
                    mesh: meshes.add(mesh.clone()),
                    material: custom_mesh_handle2.clone(),
                    ..default()
                })
                .insert(CustomMeshHandle(custom_mesh_handle2))
                .insert(TopLeft);
        }
        None => {}
    };

    commands.spawn_bundle(Camera3dBundle {
        projection: OrthographicProjection::default().into(),
        transform: Transform::from_xyz(0.0, 0.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
