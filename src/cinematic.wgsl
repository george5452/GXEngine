// ==================================================
// GX CINEMATIC 2.0
// Unified Surface Lighting Architecture
// ==================================================

struct Object {
    position: vec4<f32>,
    scale: vec4<f32>,
    color: vec4<f32>,
    material: vec4<f32>,
    rotation: vec4<f32>,
    light_color: vec4<f32>,
    light_settings: vec4<f32>,
};

struct Scene {
    camera: vec4<f32>,
    forward: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
    settings: vec4<f32>,
    effects: vec4<f32>,
    extra: vec4<f32>,
    objects: array<Object, 64>,
};

@group(0) @binding(0)
var<uniform> scene: Scene;

const PI: f32 = 3.14159265;
const FOV: f32 = 0.46630766;
const NEAR: f32 = 0.03;
const FAR: f32 = 100000.0;

const SUN_DIRECTION: vec3<f32> =
    vec3<f32>(-0.557086, 0.742781, 0.371391);

const SUN_COLOR: vec3<f32> =
    vec3<f32>(1.0, 0.94, 0.83);

// ==================================================
// UNIVERSAL MATERIAL
// ==================================================

struct Surface {
    position: vec3<f32>,
    normal: vec3<f32>,
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    emission: f32,
    occlusion: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct CinematicOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32
) -> VertexOutput {

    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );

    var output: VertexOutput;

    output.uv = positions[index];

    output.position = vec4<f32>(
        output.uv,
        0.0,
        1.0
    );

    return output;
}

// ==================================================
// CAMERA
// ==================================================

fn camera_ray(uv: vec2<f32>) -> vec3<f32> {
    return normalize(
        scene.forward.xyz
        + scene.right.xyz
            * uv.x * scene.camera.w * FOV
        + scene.up.xyz * uv.y * FOV
    );
}

fn project_world(p: vec3<f32>) -> vec4<f32> {

    let relative = p - scene.camera.xyz;

    let x = dot(relative, scene.right.xyz);
    let y = dot(relative, scene.up.xyz);
    let z = dot(relative, scene.forward.xyz);

    return vec4<f32>(
        x / (FOV * scene.camera.w),
        y / FOV,
        z * FAR / (FAR - NEAR)
            - FAR * NEAR / (FAR - NEAR),
        z
    );
}

fn world_depth(p: vec3<f32>) -> f32 {

    let z = max(
        dot(
            p - scene.camera.xyz,
            scene.forward.xyz
        ),
        NEAR
    );

    return clamp(
        FAR / (FAR - NEAR)
        - FAR * NEAR / ((FAR - NEAR) * z),
        0.0,
        1.0
    );
}

// ==================================================
// MATH
// ==================================================

fn rotate(
    p: vec3<f32>,
    q: vec4<f32>
) -> vec3<f32> {

    return p + 2.0 * cross(
        q.xyz,
        cross(q.xyz, p) + q.w * p
    );
}

fn safe_normalize(v: vec3<f32>) -> vec3<f32> {

    let l = length(v);

    if l < 0.000001 {
        return vec3<f32>(0.0, 1.0, 0.0);
    }

    return v / l;
}

fn box_sdf(
    p: vec3<f32>,
    size: vec3<f32>
) -> f32 {

    let q = abs(p) - size;

    return length(max(q, vec3<f32>(0.0)))
        + min(max(q.x, max(q.y, q.z)), 0.0);
}

// ==================================================
// GEOMETRY
// ==================================================

fn object_sdf(
    p: vec3<f32>,
    o: Object
) -> f32 {

    let shape = i32(o.position.w);

    // Meshes use the triangle rasterizer.
    if shape == 4 || shape == 5 {
        return 1000000.0;
    }

    let scale = max(
        abs(o.scale.xyz),
        vec3<f32>(0.05)
    );

    let local = rotate(
        p - o.position.xyz,
        o.rotation
    ) / scale;

    let factor = min(
        scale.x,
        min(scale.y, scale.z)
    );

    if shape == 1 {
        return (
            length(local) - 0.5
        ) * factor;
    }

    if shape == 2 {

        let d = abs(vec2<f32>(
            length(local.xz),
            local.y
        )) - vec2<f32>(0.5);

        return (
            min(max(d.x, d.y), 0.0)
            + length(max(d, vec2<f32>(0.0)))
        ) * factor;
    }

    if shape == 3 {

        var q = local;

        q.y -= clamp(q.y, -0.3, 0.3);

        return (
            length(q) - 0.2
        ) * factor;
    }

    return box_sdf(
        local,
        vec3<f32>(0.5)
    ) * factor;
}

fn map_objects(p: vec3<f32>) -> vec2<f32> {

    var result = vec2<f32>(
        1000000.0,
        -2.0
    );

    let count = min(
        u32(scene.settings.x),
        64u
    );

    for (
        var i = 0u;
        i < count;
        i += 1u
    ) {

        let d = object_sdf(
            p,
            scene.objects[i]
        );

        if d < result.x {
            result = vec2<f32>(d, f32(i));
        }
    }

    return result;
}

fn map_scene(p: vec3<f32>) -> vec2<f32> {

    let objects = map_objects(p);

    if p.y < objects.x {
        return vec2<f32>(p.y, -1.0);
    }

    return objects;
}

// ==================================================
// STABLE RAY TRACING
// ==================================================

fn trace(
    origin: vec3<f32>,
    direction: vec3<f32>,
    limit: f32,
    steps: u32
) -> vec2<f32> {

    var ground = limit + 1.0;

    if abs(direction.y) > 0.000001 {

        let t = -origin.y / direction.y;

        if t > 0.001 && t <= limit {
            ground = t;
        }
    }

    let end = min(ground, limit);

    var t = 0.001;

    for (
        var i = 0u;
        i < steps;
        i += 1u
    ) {

        if t >= end {
            break;
        }

        let hit = map_objects(
            origin + direction * t
        );

        let distance = abs(hit.x);

        let epsilon = clamp(
            t * 0.000025,
            0.0005,
            0.02
        );

        if hit.y >= 0.0
            && distance < epsilon
        {
            return vec2<f32>(t, hit.y);
        }

        t += max(distance * 0.85, 0.0005);
    }

    if ground <= limit {
        return vec2<f32>(ground, -1.0);
    }

    return vec2<f32>(-1.0, -2.0);
}

// Calculate the normal using ONLY the hit object.
// This prevents neighboring objects from corrupting
// the normal around contact points.

fn surface_normal(
    p: vec3<f32>,
    id: f32
) -> vec3<f32> {

    if id < 0.0 {
        return vec3<f32>(0.0, 1.0, 0.0);
    }

    let object = scene.objects[u32(id)];

    let e = 0.001;

    let x = vec3<f32>(e, 0.0, 0.0);
    let y = vec3<f32>(0.0, e, 0.0);
    let z = vec3<f32>(0.0, 0.0, e);

    return safe_normalize(vec3<f32>(
        object_sdf(p + x, object)
            - object_sdf(p - x, object),

        object_sdf(p + y, object)
            - object_sdf(p - y, object),

        object_sdf(p + z, object)
            - object_sdf(p - z, object)
    ));
}

fn normal(p: vec3<f32>) -> vec3<f32> {
    return surface_normal(p, map_scene(p).y);
}

// ==================================================
// SKY AND ENVIRONMENT
// ==================================================

fn sky(rd: vec3<f32>) -> vec3<f32> {

    let t = pow(
        clamp(rd.y, 0.0, 1.0),
        0.65
    );

    let horizon = vec3<f32>(
        0.65, 0.72, 0.82
    );

    let zenith = vec3<f32>(
        0.12, 0.30, 0.66
    );

    let upper = mix(
        horizon,
        zenith,
        t
    );

    let lower = vec3<f32>(
        0.18, 0.19, 0.21
    );

    var color = mix(
        lower,
        upper,
        smoothstep(-0.2, 0.05, rd.y)
    );

    let sun = max(
        dot(rd, SUN_DIRECTION),
        0.0
    );

    color += SUN_COLOR
        * pow(sun, 512.0) * 5.0;

    color += SUN_COLOR
        * pow(sun, 12.0) * 0.08;

    return color;
}

// ==================================================
// SINGLE CONTINUOUS SOFT SHADOW
// ==================================================

fn soft_shadow(
    ro: vec3<f32>,
    rd: vec3<f32>
) -> f32 {
    if gx_mesh_shadow(ro, rd) < 0.5 {
        return 0.0;
    }

    var t = 0.025;

    var visibility = 1.0;

    let softness = max(
        scene.effects.z,
        1.0
    );

    for (
        var i = 0u;
        i < 48u;
        i += 1u
    ) {

        let p = ro + rd * t;

        // Do not trace the ground when testing
        // shadows from an upward-facing sun.

        let h = map_objects(p).x;

        if h < 0.0008 {
            return 0.0;
        }

        visibility = min(
            visibility,
            softness * h / max(t, 0.001)
        );

        t += max(h * 0.8, 0.002);

        if t > 80.0 {
            break;
        }
    }

    let v = clamp(visibility, 0.0, 1.0);

    return v * v * (3.0 - 2.0 * v);
}

// ==================================================
// AMBIENT OCCLUSION
// ==================================================

fn ambient_occlusion(
    p: vec3<f32>,
    n: vec3<f32>
) -> f32 {

    var occlusion = 0.0;
    var weight = 1.0;

    for (
        var i = 1u;
        i <= 5u;
        i += 1u
    ) {

        let distance = f32(i) * 0.09;

        let d = map_scene(
            p + n * distance
        ).x;

        occlusion += max(
            distance - d,
            0.0
        ) * weight;

        weight *= 0.55;
    }

    return clamp(
        1.0 - occlusion
            * scene.effects.x * 1.5,
        0.0,
        1.0
    );
}

// ==================================================
// MATERIAL RESOLUTION
// ==================================================

fn ground_color(p: vec3<f32>) -> vec3<f32> {

    let distance = length(
        p - scene.camera.xyz
    );

    let cell = abs(
        fract(p.xz + 0.5) - 0.5
    );

    let width = 0.012 + distance * 0.0004;

    let line = 1.0 - smoothstep(
        width,
        width * 2.0,
        min(cell.x, cell.y)
    );

    let fade = exp(-distance * 0.055);

    return mix(
        vec3<f32>(0.32),
        vec3<f32>(0.20),
        line * fade
    );
}

fn albedo_at(
    id: f32,
    p: vec3<f32>
) -> vec3<f32> {

    if id < 0.0 {
        return ground_color(p);
    }

    return scene.objects[u32(id)].color.xyz;
}

fn get_surface(
    p: vec3<f32>,
    id: f32
) -> Surface {

    var s: Surface;

    s.position = p;
    s.normal = surface_normal(p, id);
    s.albedo = albedo_at(id, p);

    s.metallic = 0.0;
    s.roughness = 0.8;
    s.emission = 0.0;
    s.occlusion = 1.0;

    if id >= 0.0 {

        let material =
            scene.objects[u32(id)].material;

        s.metallic = material.x;
        s.roughness = material.y;
        s.emission = material.z;
    }

    return s;
}

// ==================================================
// PHYSICALLY BASED BRDF
// ==================================================

fn fresnel_schlick(
    cosine: f32,
    f0: vec3<f32>
) -> vec3<f32> {

    return f0
        + (vec3<f32>(1.0) - f0)
        * pow(
            1.0 - clamp(cosine, 0.0, 1.0),
            5.0
        );
}

fn distribution_ggx(
    ndh: f32,
    roughness: f32
) -> f32 {

    let a = roughness * roughness;
    let a2 = a * a;

    let d = ndh * ndh * (a2 - 1.0)
        + 1.0;

    return a2 / max(
        PI * d * d,
        0.000001
    );
}

fn geometry_schlick(
    nd: f32,
    roughness: f32
) -> f32 {

    let r = roughness + 1.0;
    let k = r * r / 8.0;

    return nd / max(
        nd * (1.0 - k) + k,
        0.0001
    );
}

// ==================================================
// SDF GLOBAL ILLUMINATION
// Deterministic single-bounce sampling.
// ==================================================

fn sdf_indirect(
    p: vec3<f32>,
    n: vec3<f32>,
    light: vec3<f32>
) -> vec3<f32> {

    let axis = select(
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(1.0, 0.0, 0.0),
        abs(n.y) > 0.95
    );

    let tangent = safe_normalize(
        cross(axis, n)
    );

    let bitangent = cross(n, tangent);

    var result = vec3<f32>(0.0);

    let radius = max(scene.extra.x, 0.1);

    for (
        var i = 0u;
        i < 4u;
        i += 1u
    ) {

        let angle = f32(i) * 1.570796;

        let direction = safe_normalize(
            n * 0.78
            + tangent * cos(angle) * 0.62
            + bitangent * sin(angle) * 0.62
        );

        let origin = p + n * 0.025;

        let hit = trace(
            origin,
            direction,
            radius,
            24u
        );

        if hit.x > 0.0 {

            let hp = origin
                + direction * hit.x;

            let hn = surface_normal(hp, hit.y);

            let bounce = get_surface(hp, hit.y);

            let ndl = max(
                dot(hn, light),
                0.0
            );

            let visibility = soft_shadow(
                hp + hn * 0.02,
                light
            );

            let energy =
                ndl * visibility
                * scene.settings.z / PI
                ;

            let attenuation =
                1.0 / (1.0 + hit.x * hit.x * 0.3);

            result += bounce.albedo
                * energy * attenuation;

        } else {

            result += sky(direction) * 0.18;
        }
    }

    return result * 0.25
        * max(scene.settings.w, 0.0);
}

// ==================================================
// REFLECTIONS
// ==================================================

fn reflection_radiance(
    p: vec3<f32>,
    n: vec3<f32>,
    direction: vec3<f32>,
    roughness: f32
) -> vec3<f32> {

    let origin = p + n * 0.025;

    let hit = trace(
        origin,
        direction,
        60.0,
        40u
    );

    var radiance = sky(direction);

    if hit.x > 0.0 {

        let rp = origin
            + direction * hit.x;

        let surface = get_surface(
            rp,
            hit.y
        );

        let diffuse = max(
            dot(surface.normal, SUN_DIRECTION),
            0.0
        );

        let shadow = soft_shadow(
            rp + surface.normal * 0.02,
            SUN_DIRECTION
        );

        radiance = surface.albedo * (
            0.12
            + diffuse * shadow
                * scene.settings.z / PI
            + surface.emission
        );
    }

    return mix(
        radiance,
        sky(n) * 0.5,
        roughness * roughness
    );
}

// ==================================================
// UNIVERSAL LIGHTING
// Called by BOTH geometry pipelines.
// ==================================================
fn artificial_lighting(
    surface: Surface,
    view: vec3<f32>
) -> vec3<f32> {

    let p = surface.position;
    let n = safe_normalize(surface.normal);

    let metallic = clamp(
        surface.metallic,
        0.0,
        1.0
    );

    let roughness = clamp(
        surface.roughness,
        0.045,
        1.0
    );

    let f0 = mix(
        vec3<f32>(0.04),
        surface.albedo,
        metallic
    );

    var result = vec3<f32>(0.0);

    for (
        var i = 0u;
        i < min(u32(scene.settings.x), 64u);
        i += 1u
    ) {
        let object = scene.objects[i];

        if object.light_settings.x < 0.5 {
            continue;
        }

        let panel =
            object.light_settings.y > 0.5;

        var light_position = object.position.xyz;

        let q = vec4<f32>(
            -object.rotation.xyz,
            object.rotation.w
        );

        let downward = rotate(
            vec3<f32>(0.0, -1.0, 0.0),
            q
        );

        if panel {
            light_position += downward
                * (object.scale.y * 0.5 + 0.06);
        }

        let difference = light_position - p;
        let distance = length(difference);

        let range = max(
            object.light_settings.z,
            0.1
        );

        if distance >= range
            || distance < 0.001
        {
            continue;
        }

        let l = difference / distance;

        let ndl = max(dot(n, l), 0.0);

        if ndl <= 0.0 {
            continue;
        }

        var directional = 1.0;

        if panel {
            directional = max(
                dot(downward, -l),
                0.0
            );
        }

        if directional <= 0.0 {
            continue;
        }

        let ratio = distance / range;

        let fade = pow(
            max(1.0 - pow(ratio, 4.0), 0.0),
            2.0
        );

        let attenuation =
            fade / (1.0 + distance * distance);

        // Check imported mesh geometry.
        if gx_mesh_trace(
            p + n * 0.025,
            l,
            max(distance - 0.06, 0.001),
            true
        ) > 0.0 {
            continue;
        }

        // Check primitives, excluding the light itself.
        var shadow = 1.0;
        var t = 0.03;

        for (
            var step = 0u;
            step < 40u;
            step += 1u
        ) {
            if t >= distance - 0.06 {
                break;
            }

            var nearest = 1000000.0;

            let sample_position =
                p + n * 0.025 + l * t;

            for (
                var j = 0u;
                j < min(u32(scene.settings.x), 64u);
                j += 1u
            ) {
                if j == i {
                    continue;
                }

                nearest = min(
                    nearest,
                    object_sdf(
                        sample_position,
                        scene.objects[j]
                    )
                );
            }

            if nearest < 0.001 {
                shadow = 0.0;
                break;
            }

            t += max(nearest * 0.85, 0.002);
        }

        let h = safe_normalize(view + l);

        let ndv = max(dot(n, view), 0.001);
        let ndh = max(dot(n, h), 0.0);

        let fresnel = fresnel_schlick(
            max(dot(h, view), 0.0),
            f0
        );

        let distribution = distribution_ggx(
            ndh,
            roughness
        );

        let geometry =
            geometry_schlick(ndv, roughness)
            * geometry_schlick(ndl, roughness);

        let specular =
            distribution * geometry * fresnel
            / max(4.0 * ndv * ndl, 0.001);

        let diffuse =
            (vec3<f32>(1.0) - fresnel)
            * (1.0 - metallic)
            * surface.albedo / PI;

        let radiance =
            object.light_color.xyz
            * object.light_color.w
            * attenuation
            * directional;

        result +=
            (diffuse + specular)
            * radiance
            * ndl
            * shadow;
    }

    return result;
}

fn shade_surface(
    surface: Surface,
    view: vec3<f32>
) -> vec3<f32> {

    let n = safe_normalize(surface.normal);
    let v = safe_normalize(view);
    let l = SUN_DIRECTION;

    let h = safe_normalize(v + l);

    let p = surface.position;

    let metallic = clamp(
        surface.metallic,
        0.0,
        1.0
    );

    let roughness = clamp(
        surface.roughness,
        0.045,
        1.0
    );

    let albedo = max(
        surface.albedo,
        vec3<f32>(0.0)
    );

    let ndl = max(dot(n, l), 0.0);
    let ndv = max(dot(n, v), 0.001);
    let ndh = max(dot(n, h), 0.0);

    let f0 = mix(
        vec3<f32>(0.04),
        albedo,
        metallic
    );

    let fresnel = fresnel_schlick(
        max(dot(h, v), 0.0),
        f0
    );

    let distribution = distribution_ggx(
        ndh,
        roughness
    );

    let geometry = geometry_schlick(
        ndv,
        roughness
    ) * geometry_schlick(
        ndl,
        roughness
    );

    let specular =
        distribution * geometry * fresnel
        / max(4.0 * ndv * ndl, 0.001);

    let kd = (
        vec3<f32>(1.0) - fresnel
    ) * (1.0 - metallic);

    let diffuse = kd * albedo / PI;

    let shadow = soft_shadow(
        p + n * 0.025,
        l
    );

    let sun = SUN_COLOR
        * scene.settings.z;

    let direct = (diffuse + specular)
        * sun * ndl * shadow;

    let ao = ambient_occlusion(p, n)
    * gx_mesh_ao(p, n)
    * surface.occlusion;

    let gi = sdf_indirect(p, n, l);

    let ambient = sky(n) * 0.23;

    let indirect = albedo
        * (1.0 - metallic)
        * (ambient + gi)
        * ao;

    var reflected = vec3<f32>(0.0);

    if scene.effects.w > 0.0
        && roughness < 0.98
    {

        let direction = reflect(-v, n);

        let radiance = reflection_radiance(
            p,
            n,
            direction,
            roughness
        );

        reflected = radiance
            * fresnel_schlick(ndv, f0)
            * (1.0 - roughness * 0.75)
            * scene.effects.w
            * ao;
    }

    let emission = albedo
        * max(surface.emission, 0.0)
        * (1.0 + scene.extra.y);

    return max(
        direct
        + indirect
        + reflected
        + emission
        + artificial_lighting(surface, v),
        vec3<f32>(0.0)
    );
}

// ==================================================
// SHARED POST PROCESSING
// ==================================================

fn aces(x: vec3<f32>) -> vec3<f32> {

    return clamp(
        (x * (2.51 * x + 0.03))
        / (x * (2.43 * x + 0.59) + 0.14),
        vec3<f32>(0.0),
        vec3<f32>(1.0)
    );
}

fn apply_fog(
    color: vec3<f32>,
    p: vec3<f32>
) -> vec3<f32> {

    let relative = p - scene.camera.xyz;

    let distance = length(relative);

    let direction = safe_normalize(relative);

    let density = max(
        scene.effects.y,
        0.0
    );

    let fog = 1.0 - exp(
        -distance * density
    );

    let fog_color = sky(
        safe_normalize(vec3<f32>(
            direction.x,
            0.08,
            direction.z
        ))
    );

    return mix(
        color,
        fog_color,
        clamp(fog, 0.0, 1.0)
    );
}

fn cinematic_output(
    color: vec3<f32>
) -> vec4<f32> {

    return vec4<f32>(
        aces(
            color * max(scene.settings.y, 0.0)
        ),
        1.0
    );
}

// ==================================================
// SDF RENDER PASS
// ==================================================

@fragment
fn fs_main(
    input: VertexOutput
) -> CinematicOutput {

    let ro = scene.camera.xyz;
    let rd = camera_ray(input.uv);

    let hit = trace(
        ro,
        rd,
        FAR,
        128u
    );

    var color = sky(rd);

    var output: CinematicOutput;

    output.depth = 1.0;

    if hit.x > 0.0 {

        let p = ro + rd * hit.x;

        let surface = get_surface(
            p,
            hit.y
        );

        color = shade_surface(
            surface,
            -rd
        );

        color = apply_fog(color, p);

        output.depth = world_depth(p);
    }

    output.color = cinematic_output(color);

    return output;
}