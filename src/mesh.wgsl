// ==========================================
// GX MATERIAL V2
// Textured PBR Mesh Renderer
// ==========================================

@group(1) @binding(0)
var gx_base_textures: texture_2d_array<f32>;

@group(1) @binding(1)
var gx_texture_sampler: sampler;

@group(2) @binding(0)
var gx_pbr_textures: texture_2d_array<f32>;

@group(2) @binding(1)
var gx_pbr_sampler: sampler;

struct MeshInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) material: vec2<f32>,
    @location(4) uv: vec2<f32>,
    @location(5) texture_layer: f32,

    @location(6) pbr_uv: vec2<f32>,
    @location(7) pbr_layer: f32,
};

struct MeshOutput {
    @builtin(position) clip: vec4<f32>,

    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) material: vec2<f32>,
    @location(4) uv: vec2<f32>,

    @location(5) @interpolate(flat)
    texture_layer: i32,

    @location(6) pbr_uv: vec2<f32>,

    @location(7) @interpolate(flat)
    pbr_layer: i32,
};

@vertex
fn mesh_vertex(
    input: MeshInput
) -> MeshOutput {

    var output: MeshOutput;

    output.clip = project_world(
        input.position
    );

    output.position = input.position;
    output.normal = input.normal;
    output.color = input.color;
    output.material = input.material;

    output.uv = input.uv;
    output.texture_layer =
        i32(input.texture_layer);

    output.pbr_uv = input.pbr_uv;
    output.pbr_layer =
        i32(input.pbr_layer);

    return output;
}

@fragment
fn mesh_fragment(
    input: MeshOutput
) -> @location(0) vec4<f32> {

    let view = safe_normalize(
        scene.camera.xyz - input.position
    );

    var n = safe_normalize(input.normal);

    if dot(n, view) < 0.0 {
        n = -n;
    }

    // ==============================
    // BASE COLOR
    // ==============================

    let base_sample = textureSample(
        gx_base_textures,
        gx_texture_sampler,
        input.uv,
        max(input.texture_layer, 0)
    );

    var albedo = input.color;

    if input.texture_layer >= 0 {
        albedo *= base_sample.rgb;
    }

    // ==============================
    // METALLIC / ROUGHNESS
    // ==============================

    let pbr_sample = textureSample(
        gx_pbr_textures,
        gx_pbr_sampler,
        input.pbr_uv,
        max(input.pbr_layer, 0)
    );

    var metallic = input.material.x;
    var roughness = input.material.y;

    if input.pbr_layer >= 0 {

        // glTF specification:
        // Green = roughness
        // Blue = metallic

        roughness *= pbr_sample.g;
        metallic *= pbr_sample.b;
    }

    metallic = clamp(
        metallic,
        0.0,
        1.0
    );

    roughness = clamp(
        roughness,
        0.045,
        1.0
    );

    // ==============================
    // CINEMATIC LIGHTING
    // ==============================

    var surface: Surface;

    surface.position = input.position;
    surface.normal = n;
    surface.albedo = albedo;

    surface.metallic = metallic;
    surface.roughness = roughness;

    surface.emission = 0.0;
    surface.occlusion = 1.0;

    let color = shade_surface(
        surface,
        view
    );

    let fogged = apply_fog(
        color,
        surface.position
    );

    return cinematic_output(fogged);
}