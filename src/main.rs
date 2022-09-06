mod custom_mesh;
mod custom_mesh_pipeline;

use custom_mesh::*;
use custom_mesh_pipeline::*;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute},
        render_resource::{PrimitiveTopology, VertexFormat},
    },
    sprite::Mesh2dHandle,
    window::close_on_esc,
};

#[derive(Component, Default, Debug)]
pub struct CustomMesh2d;

#[derive(Component, Deref, Debug)]
pub struct CustomShader(Handle<Shader>);

#[derive(Component, Deref, DerefMut, Debug)]
pub struct ColorUniform(Color);

#[derive(Component, Deref, Debug)]
pub struct OffsetUniform(f32);

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 800.,
            height: 600.,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(close_on_esc)
        .add_system(update_custom_color)
        .add_plugin(CustomMesh2dPlugin)
        .run();
}

fn update_custom_color(mut query: Query<&mut ColorUniform, With<CustomShader>>, time: Res<Time>) {
    let mut color_uniform = query.get_single_mut().unwrap();
    (**color_uniform).set_g((time.seconds_since_startup() as f32).sin() / 2.0 + 0.5);
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, asset_server: Res<AssetServer>) {
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

    commands.spawn_bundle((
        CustomMesh2d::default(),
        CustomShader(asset_server.load("shaders/custom_mesh_2d.wgsl")),
        Mesh2dHandle(meshes.add(mesh.clone())),
        ColorUniform(Color::BLACK),
        OffsetUniform(0.1),
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
