mod custom_mesh;
use custom_mesh::*;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute},
        render_resource::{PrimitiveTopology, VertexFormat},
    },
    window::close_on_esc,
};

#[derive(Component, Deref, Debug)]
pub struct CustomShader(Handle<Shader>);

#[derive(Component, Deref, DerefMut, Debug)]
pub struct ColorUniform(Color);

#[derive(Component, Deref, Debug)]
pub struct OffsetUniform(f32);

#[derive(Deref, DerefMut, Debug)]
pub struct TextureShaderResource(Option<Handle<Image>>);

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 800.,
            height: 600.,
            ..default()
        })
        .insert_resource(TextureShaderResource(None))
        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<CustomMesh>::default())
        .add_startup_system_to_stage(StartupStage::PreStartup, load_texture)
        .add_startup_system(setup)
        .add_system(close_on_esc)
        .add_system(update_custom_color)
        .run();
}

fn update_custom_color(mut query: Query<&mut CustomMesh>, time: Res<Time>) {
    if query.is_empty() {
        return;
    }
    let mut custom_mesh = query.get_single_mut().unwrap();
    custom_mesh.color.y = (time.seconds_since_startup() as f32).sin() / 2.0 + 0.5;
}

fn load_texture(
    mut texture_resource: ResMut<TextureShaderResource>,
    asset_server: Res<AssetServer>,
) {
    **texture_resource = Some(asset_server.load("textures/wall.png"));
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut custom_mesh_materials: ResMut<Assets<CustomMesh>>,
    texture: ResMut<TextureShaderResource>,
) {
    match &**texture {
        Some(t) => {
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

            // Set the position attribute
            let v_pos = vec![[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]];
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);

            // Set the color attribute
            let v_color: Vec<u32> = vec![Color::YELLOW.as_linear_rgba_u32(); 3];
            mesh.insert_attribute(
                MeshVertexAttribute::new("Vertex_Color", 1, VertexFormat::Uint32),
                v_color,
            );

            // Set vertex indices
            let indices = vec![0, 1, 2];
            mesh.set_indices(Some(Indices::U16(indices)));

            commands.spawn_bundle(MaterialMeshBundle::<CustomMesh> {
                mesh: meshes.add(mesh.clone()),
                material: custom_mesh_materials.add(CustomMesh {
                    color: Vec4::from(Color::BLACK),
                    offset: 0.1,
                    base_color_texture: Some(t.clone()),
                }),
                ..default()
            });
        }
        None => {}
    };

    commands.spawn_bundle(Camera3dBundle {
        projection: OrthographicProjection::default().into(),
        transform: Transform::from_xyz(0.0, 0.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
