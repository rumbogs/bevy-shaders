// Import the standard 2d mesh uniforms and set their bind groups
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_view_bindings

@group(1) @binding(0)
var<uniform> ourColor: vec4<f32>;

@group(1) @binding(1)
var<uniform> offset: f32;

@group(1) @binding(2)
var<uniform> model_mat: mat4x4<f32>;

@group(1) @binding(3)
var<uniform> view_mat: mat4x4<f32>;

@group(1) @binding(4)
var<uniform> projection_mat: mat4x4<f32>;

@group(1) @binding(5)
var texture: texture_2d<f32>;

@group(1) @binding(6)
var texture_sampler: sampler;

@group(1) @binding(7)
var texture2: texture_2d<f32>;

@group(1) @binding(8)
var texture_sampler2: sampler;

// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

// The structure of the vertex buffer is as specified in `specialize()`
struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) color: u32,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    // The vertex shader must set the on-screen position of the vertex
    @builtin(position) clip_position: vec4<f32>,
    // We pass the vertex color to the fragment shader in location 0
    @location(0) color: vec4<f32>,
    @location(1) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

fn rotate_coords(coords: vec3<f32>, degrees: f32) -> vec3<f32> {
    var PI: f32 = 3.14159;
    let rad = degrees * PI / 180.0;
    let x = coords.x * cos(rad) - coords.y * sin(rad);
    let y = coords.y * cos(rad) - coords.x * sin(rad);
    return vec3<f32>(x, y, coords.z);
} 

/// Entry point for the vertex shader
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // Project the world position of the mesh into screen position
    // This needs to be done whenever we pass in viewport size coords (1280 x 720)
    //out.clip_position = mesh_position_local_to_clip(mesh.model, vec4<f32>(vertex.position, 1.0));
    // Otherwise, if we have normalized coords (-1, 1) we can just copy the position
    out.clip_position = projection_mat * view_mat * model_mat * vec4<f32>(vertex.position, 1.0);
    out.position = vec4<f32>(vertex.position, 1.0);
    // Unpack the `u32` from the vertex buffer into the `vec4<f32>` used by the fragment shader
    out.color = vec4<f32>((vec4<u32>(vertex.color) >> vec4<u32>(0u, 8u, 16u, 24u)) & vec4<u32>(255u)) / 255.0;
    out.uv = vertex.uv;
    return out;
}

// The input of the fragment shader must correspond to the output of the vertex shader for all `location`s
struct FragmentInput {
    // The color is interpolated between vertices by default
    @location(0) color: vec4<f32>,
    @location(1) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

/// Entry point for the fragment shader
@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    in: FragmentInput
) -> @location(0) vec4<f32> {
    // Images are 0,0 to 1,1 and they expect 0.0 to be at top of y axis instead of bottom as in shaders
    //let image_pos = vec2<f32>(in.position.x + 0.5, 1.0 - in.position.y + 0.5);
    return mix(textureSample(texture, texture_sampler, in.uv), textureSample(texture2, texture_sampler2, vec2<f32>(in.uv.x, 1.0 - in.uv.y)), offset);
}
