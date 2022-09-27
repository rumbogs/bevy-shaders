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

@group(2) @binding(6)
var emission_tex: texture_2d<f32>;

@group(2) @binding(7)
var emission_tex_sampler: sampler;

@group(2) @binding(8)
var<uniform> view_pos: vec3<f32>;

struct DirLight {
    direction: vec3<f32>,
    ambient: vec4<f32>,
    diffuse: vec4<f32>,
    specular: vec4<f32>,
};

@group(2) @binding(9)
var<uniform> dir_light: DirLight;

struct PointLight {
    position: vec3<f32>,

    constant: f32,
    lin: f32,
    quadratic: f32,

    ambient: vec4<f32>,
    diffuse: vec4<f32>,
    specular: vec4<f32>,
};

@group(2) @binding(10)
var<uniform> point_l: array<PointLight, 4>;

// The props need to be kept in the same order as the binding
struct Spotlight {
    direction: vec3<f32>,
    position: vec3<f32>,
    cutoff: f32,
    outer_cutoff: f32,
    ambient: vec4<f32>,
    diffuse: vec4<f32>,
    specular: vec4<f32>,
    constant: f32,
    lin: f32,
    quadratic: f32,
};

@group(2) @binding(11)
var<uniform> spotlight: Spotlight;

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
    out.clip_position = projection_mat * view_mat * vec4<f32>(out.frag_pos, 1.0);
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

fn calc_dir_light(light: DirLight, normal: vec3<f32>, view_dir: vec3<f32>, shininess: f32, uv: vec2<f32>) -> vec4<f32> {
    let light_dir = normalize(-light.direction);
    // Diffuse
    let diff = max(dot(normal, light_dir), 0.0);
    // Specular
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), shininess);
    // Combined
    let ambient = light.ambient * textureSample(diff_tex, diff_tex_sampler, uv);
    let diffuse = light.diffuse * diff * textureSample(diff_tex, diff_tex_sampler, uv);

    let specular = light.specular * spec * textureSample(spec_tex, spec_tex_sampler, uv);
    return ambient + diffuse + specular;
}

fn calc_point_light(light: PointLight, normal: vec3<f32>, frag_pos: vec3<f32>, view_dir: vec3<f32>, shininess: f32, uv: vec2<f32>) -> vec4<f32> {
    let light_dir = normalize(light.position - frag_pos);
    // Diffuse
    let diff = max(dot(normal, light_dir), 0.0);
    // Specular
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), shininess);
    // Attenuation
    let dist = length(light.position - frag_pos);
    let attenuation = 1.0 / (light.constant + light.lin * dist + light.quadratic * (dist * dist));
    // Combined
    var ambient = light.ambient * textureSample(diff_tex, diff_tex_sampler, uv);
    var diffuse = light.diffuse * diff * textureSample(diff_tex, diff_tex_sampler, uv);
    var specular = light.specular * spec * textureSample(spec_tex, spec_tex_sampler, uv);
    ambient *= attenuation;
    diffuse *= attenuation;
    specular *= attenuation;
    return ambient + diffuse + specular;
}

fn calc_spot_light(light: Spotlight, normal: vec3<f32>, frag_pos: vec3<f32>, view_dir: vec3<f32>, shininess: f32, uv: vec2<f32>) -> vec4<f32> {
    let light_dir = normalize(light.position - frag_pos);
    // Diffuse
    let diff = max(dot(normal, light_dir), 0.0);
    // Specular
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), shininess);
    // Attenuation
    let dist = length(light.position - frag_pos);
    let attenuation = 1.0 / (light.constant + light.lin * dist + light.quadratic * (dist * dist));
    // Intensity
    let theta = dot(light_dir, normalize(-light.direction));
    let epsilon = light.cutoff - light.outer_cutoff;
    let intensity = clamp((theta - light.outer_cutoff) / epsilon, 0.0, 1.0);
    // Combined
    var ambient = light.ambient * textureSample(diff_tex, diff_tex_sampler, uv);
    var diffuse = light.diffuse * diff * textureSample(diff_tex, diff_tex_sampler, uv);
    var specular = light.specular * spec * textureSample(spec_tex, spec_tex_sampler, uv);

    ambient *= attenuation * intensity;
    diffuse *= attenuation * intensity;
    specular *= attenuation * intensity;

    return ambient + diffuse + specular;
}

/// Entry point for the fragment shader
@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    in: FragmentInput
) -> @location(0) vec4<f32> {
    // Images are 0,0 to 1,1 and they expect 0.0 to be at top of y axis instead of bottom as in shaders
    //let image_pos = vec2<f32>(in.position.x + 0.5, 1.0 - in.position.y + 0.5);

    // Properties
    let norm = normalize(in.normal.xyz);
    // We need the view pos here which is passed through an uniform value,
    // But we could have just transformed the vertex out values from the world coord to a view coord
    // That way we would get the view pos for free (i.e. multiply the "frag_pos" and "normal" by both the "model" and "view mat", I think)
    // The light direction is pointing from the frag pos to the light source so negate that
    let view_dir = normalize(view_pos - in.frag_pos);

    // Phase 1: Directional lighting
    var result = calc_dir_light(dir_light, norm, view_dir, in.shininess, in.uv);
    // Phase 2: Point lights
    for (var i = 0; i < 4; i++) {
        result += calc_point_light(point_l[i], norm, in.frag_pos, view_dir, in.shininess, in.uv);
    }
    // Phase 3: Spot light
    result += calc_spot_light(spotlight, norm, in.frag_pos, view_dir, in.shininess, in.uv);

    //let emission = textureSample(emission_tex, emission_tex_sampler, in.uv).xyz * 3.0;

    return vec4<f32>(result.xyz, 1.0);
}
