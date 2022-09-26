// Import the standard 2d mesh uniforms and set their bind groups
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_view_bindings

@group(2) @binding(0)
var<uniform> view_mat: mat4x4<f32>;

@group(2) @binding(1)
var<uniform> projection_mat: mat4x4<f32>;

@group(2) @binding(2)
var diff_tex: texture_2d<f32>;

@group(2) @binding(3)
var diff_tex_sampler: sampler;

@group(2) @binding(4)
var spec_tex: texture_2d<f32>;

@group(2) @binding(5)
var spec_tex_sampler: sampler;

struct LightMaterial {
    position: vec3<f32>,
    ambient: vec4<f32>,
    diffuse: vec4<f32>,
    specular: vec4<f32>,
};

@group(2) @binding(6)
var<uniform> light: LightMaterial;

@group(2) @binding(7)
var<uniform> view_pos: vec3<f32>;

struct InstanceInput {
    @location(3) model_mat_0: vec4<f32>,
    @location(4) model_mat_1: vec4<f32>,
    @location(5) model_mat_2: vec4<f32>,
    @location(6) model_mat_3: vec4<f32>,
    @location(7) normal_mat_0: vec4<f32>,
    @location(8) normal_mat_1: vec4<f32>,
    @location(9) normal_mat_2: vec4<f32>,
    @location(10) normal_mat_3: vec4<f32>,
    @location(11) shininess: f32,
}

// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

// The structure of the vertex buffer is as specified in `specialize()`
struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    // The vertex shader must set the on-screen position of the vertex
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) frag_pos: vec3<f32>,
    @location(4) shininess: f32,
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
fn vertex(vertex: Vertex, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let model_mat = mat4x4<f32>(instance.model_mat_0, instance.model_mat_1, instance.model_mat_2, instance.model_mat_3);
    // We need to get rid of the translation information since normals can't be translated
    // Either remove the 4x4 row/column or add a w=0.0 value to the normal vector
    let normal_mat = mat3x3<f32>(instance.normal_mat_0.xyz, instance.normal_mat_1.xyz, instance.normal_mat_2.xyz);
    // Project the world position of the mesh into screen position
    // This needs to be done whenever we pass in viewport size coords (1280 x 720)
    //out.clip_position = mesh_position_local_to_clip(mesh.model, vec4<f32>(vertex.position, 1.0));
    // Otherwise, if we have normalized coords (-1, 1) we can just copy the position
    out.clip_position = projection_mat * view_mat * model_mat * vec4<f32>(vertex.position, 1.0);
    out.position = vec4<f32>(vertex.position, 1.0);
    // Unpack the `u32` from the vertex buffer into the `vec4<f32>` used by the fragment shader
    //out.normal = vec4<f32>((vec4<u32>(vertex.normal) >> vec4<u32>(0u, 8u, 16u, 24u)) & vec4<u32>(255u)) / 255.0;

    // The normal calculations from the fragment shader are done in world space and not model space so
    // whenever we do non-uniform scaling (stretching) the normals won't be perpendicular to the surface any more.
    // A fix for this is to multiply the normal value by a normal matrix.
    // Inversing a matrix is an expensive calculation so it should be done on the CPU and passed in as a buffer
    // similar to the model matrix.   
    out.normal = normal_mat * vertex.normal;
    out.uv = vertex.uv;
    out.frag_pos = vec4<f32>(model_mat * vec4<f32>(vertex.position, 1.0)).xyz;
    out.shininess = instance.shininess;
    return out;
}

// The input of the fragment shader must correspond to the output of the vertex shader for all `location`s
struct FragmentInput {
    @location(0) normal: vec3<f32>,
    @location(1) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) frag_pos: vec3<f32>,
    @location(4) shininess: f32,
};


/// Entry point for the fragment shader
@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    in: FragmentInput
) -> @location(0) vec4<f32> {
    // Images are 0,0 to 1,1 and they expect 0.0 to be at top of y axis instead of bottom as in shaders
    //let image_pos = vec2<f32>(in.position.x + 0.5, 1.0 - in.position.y + 0.5);
    let diffuse_strength: f32 = 1.0;
    let specular_strength: f32 = 0.5;

    // Ambient color
    let ambient = textureSample(diff_tex, diff_tex_sampler, in.uv).xyz * light.ambient.rgb;

    // Diffuse color
    let norm = normalize(in.normal.xyz);
    // Light direction is the diff between it's position and the current frag pos
    let light_dir = normalize(light.position - in.frag_pos);
    // The angle between the normal and the light direction represents the diffuse intensity
    // Don't return negative values here, they aren't a thing
    let diff = max(dot(norm, light_dir), 0.0);
    let diffuse = textureSample(diff_tex, diff_tex_sampler, in.uv).xyz * diff * light.diffuse.rgb;

    // Specular color
    // We need the view pos here which is passed through an uniform value,
    // But we could have just transformed the vertex out values from the world coord to a view coord
    // That way we would get the view pos for free (i.e. multiply the "frag_pos" and "normal" by both the "model" and "view mat", I think)
    let view_dir = normalize(view_pos - in.frag_pos);
    // The light direction is pointing from the frag pos to the light source so negate that
    let reflect_dir = reflect(-light_dir, norm);
    // 32 is the shininess value, the large it is the less it scatters the light (smaller highlight)
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), in.shininess);
    let specular = textureSample(spec_tex, spec_tex_sampler, in.uv).xyz * spec * light.specular.rgb;

    let result = ambient + diffuse + specular;
    return vec4<f32>(result, 1.0);
}
