mod camera;
mod custom_material;
mod light_material;

use camera::*;
use custom_material::*;
use light_material::*;

use bevy::{
    asset::LoadState,
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            AddressMode, Extent3d, FilterMode, PrimitiveTopology, SamplerDescriptor,
            TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        texture::ImageSampler,
        view::NoFrustumCulling,
        MainWorld, RenderApp, RenderStage,
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

#[derive(Component, Deref, Debug)]
struct DiffuseTexture(pub Handle<Image>);

impl ExtractComponent for DiffuseTexture {
    type Query = &'static DiffuseTexture;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        DiffuseTexture((**item).clone())
    }
}

#[derive(Component, Deref, Debug)]
struct SpecularTexture(pub Handle<Image>);

impl ExtractComponent for SpecularTexture {
    type Query = &'static SpecularTexture;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        SpecularTexture((**item).clone())
    }
}

const CUBE: [[f32; 3]; 36] = [
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

const CUBE_UV: [[f32; 2]; 36] = [
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

const CUBE_NORMALS: [[f32; 3]; 36] = [
    // Face 1
    [0.0, 0.0, -1.0],
    [0.0, 0.0, -1.0],
    [0.0, 0.0, -1.0],
    [0.0, 0.0, -1.0],
    [0.0, 0.0, -1.0],
    [0.0, 0.0, -1.0],
    // Face 2
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 1.0],
    // Face 3
    [-1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    // Face 4
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    // Face 5
    [0.0, -1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, -1.0, 0.0],
    // Face 6
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
];

fn main() {
    let mut app = App::new();

    app.insert_resource(WindowDescriptor {
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
    .add_plugin(ExtractComponentPlugin::<DiffuseTexture>::default())
    .add_plugin(ExtractComponentPlugin::<SpecularTexture>::default())
    .add_plugin(LightMaterialPlugin)
    .add_plugin(CustomMaterialPlugin)
    .add_plugin(CameraPlugin)
    .add_state(AppState::LoadAssets)
    .add_system_set(SystemSet::on_enter(AppState::LoadAssets).with_system(load_assets))
    .add_system_set(SystemSet::on_update(AppState::LoadAssets).with_system(assets_loaded))
    .add_system_set(SystemSet::on_enter(AppState::Main).with_system(setup))
    .add_system_set(SystemSet::on_update(AppState::Main).with_system(move_light))
    .add_system(close_on_esc);

    app.sub_app_mut(RenderApp)
        .init_resource::<CustomCamera>()
        .add_system_to_stage(RenderStage::Extract, extract_custom_camera);

    app.run();
}

fn load_assets(
    mut texture_resources: ResMut<TextureShaderResources>,
    asset_server: Res<AssetServer>,
) {
    **texture_resources = Some(vec![
        asset_server.load("textures/container2.png"),
        asset_server.load("textures/container2_specular.png"),
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

fn extract_custom_camera(mut commands: Commands, world: Res<MainWorld>) {
    if let Some(camera) = world.get_resource::<CustomCamera>() {
        commands.insert_resource(camera.clone());
    }
}

fn move_light(mut query: Query<&mut LightInstances, With<LightMaterial>>, time: Res<Time>) {
    for mut light_instances in &mut query {
        let time_val = time.seconds_since_startup() as f32;
        light_instances[0].position.x = time_val.sin();

        //let light_col = Vec4::new(
        //(time_val * 2.0).sin(),
        //(time_val * 0.7).sin(),
        //(time_val * 1.3).sin(),
        //1.0,
        //);
        //let diffuse_col = light_col * Vec4::splat(0.5);
        //let ambient_col = diffuse_col * Vec4::splat(0.2);
        //light_instances[0].diffuse = diffuse_col;
        //light_instances[0].ambient = ambient_col;
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    textures: ResMut<TextureShaderResources>,
    mut windows: ResMut<Windows>,
) {
    let window = windows.get_primary_mut().unwrap();
    window.set_cursor_lock_mode(true);
    window.set_cursor_visibility(false);

    // This is just used so that we can see something through bevy, there's another camera created
    // using model, view, projection matrices
    commands.spawn_bundle(Camera3dBundle {
        camera_3d: Camera3d {
            // This is 0.0 by default because 0.0 is the far plane due to bevy's use of reverse-z projections.
            // This goes hand in hand with the DepthStencilState depth_compare
            // If it's Less the load op needs to be 1.0
            // If it's Greater the load op needs to be 0.0
            // TODO: figure out why???
            depth_load_op: bevy::core_pipeline::core_3d::Camera3dDepthLoadOp::Clear(1.0),
            clear_color: ClearColorConfig::Custom(Color::BLACK),
        },
        ..default()
    });

    commands.insert_resource(CustomCamera {
        position: Vec3::new(0.0, 0.0, 3.0),
        yaw: (-90.0_f32).to_radians(),
        pitch: 0.0_f32.to_radians(),
        up: Vec3::Y,
        fov: 45.0,
        aspect_ratio: 800.0 / 600.0,
        near: 0.1,
        far: 100.0,
    });

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, CUBE.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, CUBE_NORMALS.to_vec());
    // Set the color attribute this is needed otherwise there's a missing Vertex Normal attribute
    // error
    //let v_color: Vec<u32> = vec![Color::WHITE.as_linear_rgba_u32(); 36];
    //mesh.insert_attribute(
    //MeshVertexAttribute::new("Vertex_Color", 1, VertexFormat::Uint32),
    //v_color,
    //);

    commands
        .spawn()
        .insert_bundle((
            meshes.add(mesh),
            LightInstances(vec![LightInstance {
                position: Vec3::new(0.0, 1.0, 2.0),
                ambient: Vec3::splat(0.1).extend(1.0),
                diffuse: Vec3::splat(1.0).extend(1.0),
                specular: Vec3::splat(0.5).extend(1.0),
            }]),
            LightMaterial,
            NoFrustumCulling,
        ))
        .insert_bundle(SpatialBundle::default());

    match &**textures {
        Some(textures) => {
            // Base color texture
            let mut image = images.get_mut(&textures[0]).unwrap();
            image.texture_descriptor = TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: 500,
                    height: 500,
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
                    width: 500,
                    height: 500,
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
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, CUBE.to_vec());
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, CUBE_UV.to_vec());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, CUBE_NORMALS.to_vec());

            // Set vertex indices
            //let indices = vec![0, 1, 3, 1, 2, 3];
            //mesh.set_indices(Some(Indices::U16(indices)));

            commands
                .spawn()
                .insert_bundle((
                    meshes.add(mesh),
                    MaterialInstances(vec![MaterialInstance {
                        position: Vec3::new(0.0, 0.0, 0.0),
                        specular: Color::rgba(0.501_960_7, 0.501_960_7, 0.501_960_7, 1.0).into(),
                        shininess: 25.0,
                    }]),
                    DiffuseTexture(textures[0].clone()),
                    SpecularTexture(textures[1].clone()),
                    CustomMaterial,
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
}
