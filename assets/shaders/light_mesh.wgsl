// Import the standard 3d mesh uniforms and set their bind groups
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_view_bindings

@group(2) @binding(0)
var<uniform> view_mat: mat4x4<f32>;

@group(2) @binding(1)
var<uniform> projection_mat: mat4x4<f32>;

struct InstanceInput {
    @location(3) model_mat_0: vec4<f32>,
    @location(4) model_mat_1: vec4<f32>,
    @location(5) model_mat_2: vec4<f32>,
    @location(6) model_mat_3: vec4<f32>,
    @location(7) ambient: vec4<f32>,
    @location(8) diffuse: vec4<f32>,
    @location(9) specular: vec4<f32>,
}

// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

// The structure of the vertex buffer is as specified in `specialize()`
struct Vertex {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    // The vertex shader must set the on-screen position of the vertex
    @builtin(position) clip_position: vec4<f32>,
    // We pass the vertex color to the fragment shader in location 0
    @location(1) position: vec4<f32>,
    @location(2) ambient: vec4<f32>,
    @location(3) diffuse: vec4<f32>,
    @location(4) specular: vec4<f32>,
};

/// Entry point for the vertex shader
@vertex
fn vertex(vertex: Vertex, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let model_mat = mat4x4<f32>(instance.model_mat_0, instance.model_mat_1, instance.model_mat_2, instance.model_mat_3);
    // Project the world position of the mesh into screen position
    // This needs to be done whenever we pass in viewport size coords (1280 x 720)
    //out.clip_position = mesh_position_local_to_clip(mesh.model, vec4<f32>(vertex.position, 1.0));
    // Otherwise, if we have normalized coords (-1, 1) we can just copy the position
    out.clip_position = projection_mat * view_mat * model_mat * vec4<f32>(vertex.position, 1.0);
    out.position = vec4<f32>(vertex.position, 1.0);
    out.ambient = instance.ambient;
    out.diffuse = instance.diffuse;
    out.specular = instance.specular;
    return out;
}

// The input of the fragment shader must correspond to the output of the vertex shader for all `location`s
struct FragmentInput {
    @location(1) position: vec4<f32>,
    @location(2) ambient: vec4<f32>,
    @location(3) diffuse: vec4<f32>,
    @location(4) specular: vec4<f32>,
};

/// Entry point for the fragment shader
@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    in: FragmentInput
) -> @location(0) vec4<f32> {
    return in.ambient + in.diffuse + in.specular;
}
