// ==================================================
// GX MESH RENDERER 2.0
// Uses GX Cinematic shared surface lighting.
// ==================================================

struct MeshInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) material: vec2<f32>,
};

struct MeshOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) material: vec2<f32>,
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

    return output;
}

@fragment
fn mesh_fragment(
    input: MeshOutput,
    @builtin(front_facing) front: bool
) -> @location(0) vec4<f32> {

    let view = safe_normalize(
        scene.camera.xyz - input.position
    );

    var n = safe_normalize(input.normal);

    if dot(n, view) < 0.0 {
        n = -n;
    }

    var surface: Surface;

    surface.position = input.position;
    surface.normal = n;
    surface.albedo = input.color;
    surface.metallic = input.material.x;
    surface.roughness = input.material.y;
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