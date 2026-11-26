struct Object {
    position: vec4<f32>,
    scale: vec4<f32>,
    color: vec4<f32>,
    material: vec4<f32>,
    rotation: vec4<f32>,
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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32
) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );

    let p = positions[index];

    var output: VertexOutput;
    output.position = vec4<f32>(p, 0.0, 1.0);
    output.uv = p;

    return output;
}

fn rotate(
    p: vec3<f32>,
    q: vec4<f32>
) -> vec3<f32> {
    return p + 2.0 * cross(
        q.xyz,
        cross(q.xyz, p) + q.w * p
    );
}

fn box_sdf(
    p: vec3<f32>,
    size: vec3<f32>
) -> f32 {
    let q = abs(p) - size;

    return length(max(q, vec3<f32>(0.0)))
        + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn object_sdf(
    p: vec3<f32>,
    o: Object
) -> f32 {
    let local = rotate(
        p - o.position.xyz,
        o.rotation
    ) / o.scale.xyz;

    let factor = min(
        o.scale.x,
        min(o.scale.y, o.scale.z)
    );

    let shape = i32(o.position.w);

    if shape == 1 {
        return (length(local) - 0.5) * factor;
    }

    if shape == 3 {
        var q = local;

        q.y -= clamp(q.y, -0.3, 0.3);

        return (length(q) - 0.2) * factor;
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

    return box_sdf(
        local,
        vec3<f32>(0.5)
    ) * factor;
}

fn map_scene(p: vec3<f32>) -> vec2<f32> {
    var result = vec2<f32>(p.y, -1.0);

    for (
        var i = 0u;
        i < u32(scene.settings.x);
        i = i + 1u
    ) {
        let d = object_sdf(p, scene.objects[i]);

        if d < result.x {
            result = vec2<f32>(d, f32(i));
        }
    }

    return result;
}

fn trace(
    origin: vec3<f32>,
    direction: vec3<f32>,
    limit: f32,
    steps: u32
) -> vec2<f32> {

    var ground_t = limit + 1.0;

    // Exact intersection with the ground plane.
    if abs(direction.y) > 0.000001 {
        let d = -origin.y / direction.y;

        if d > 0.001 && d <= limit {
            ground_t = d;
        }
    }

    let max_distance = min(limit, ground_t);

    var t = 0.002;

    for (var i = 0u; i < steps; i += 1u) {

        if t >= max_distance {
            break;
        }

        let p = origin + direction * t;

        var nearest = 1000000.0;
        var object_id = -2.0;

        // March objects independently of the ground.
        for (
            var j = 0u;
            j < u32(scene.settings.x);
            j += 1u
        ) {
            let d = abs(
                object_sdf(p, scene.objects[j])
            );

            if d < nearest {
                nearest = d;
                object_id = f32(j);
            }
        }

        if nearest < max(0.001, t * 0.00001) {
            return vec2<f32>(t, object_id);
        }

        t += max(nearest * 0.9, 0.001);
    }

    if ground_t <= limit {
        return vec2<f32>(ground_t, -1.0);
    }

    return vec2<f32>(-1.0, -2.0);
}

fn normal(p: vec3<f32>) -> vec3<f32> {
    let e = 0.002;

    return normalize(vec3<f32>(
        map_scene(p + vec3<f32>(e, 0.0, 0.0)).x
        - map_scene(p - vec3<f32>(e, 0.0, 0.0)).x,

        map_scene(p + vec3<f32>(0.0, e, 0.0)).x
        - map_scene(p - vec3<f32>(0.0, e, 0.0)).x,

        map_scene(p + vec3<f32>(0.0, 0.0, e)).x
        - map_scene(p - vec3<f32>(0.0, 0.0, e)).x
    ));
}

fn sky(rd: vec3<f32>) -> vec3<f32> {
    let t = clamp(rd.y, 0.0, 1.0);

    var color = mix(
        vec3<f32>(0.85, 0.73, 0.61),
        vec3<f32>(0.19, 0.38, 0.75),
        pow(t, 0.55)
    );

    let sun_dir = normalize(
        vec3<f32>(-0.6, 0.8, 0.4)
    );

    color += vec3<f32>(1.0, 0.8, 0.5)
        * pow(max(dot(rd, sun_dir), 0.0), 350.0)
        * 5.0;

    return color;
}

fn soft_shadow(
    ro: vec3<f32>,
    rd: vec3<f32>
) -> f32 {
    var t = 0.03;
    var visibility = 1.0;

    for (var i = 0u; i < 24u; i = i + 1u) {
        let h = map_scene(ro + rd * t).x;

        if h < 0.001 {
            return 0.0;
        }

        visibility = min(
            visibility,
            scene.effects.z * h / t
        );

        t += clamp(h, 0.02, 0.8);

        if t > 25.0 {
            break;
        }
    }

    return clamp(visibility, 0.0, 1.0);
}

fn ambient_occlusion(
    p: vec3<f32>,
    n: vec3<f32>
) -> f32 {
    var occlusion = 0.0;
    var weight = 1.0;

    for (var i = 1u; i <= 5u; i = i + 1u) {
        let distance = f32(i) * 0.14;
        let d = map_scene(p + n * distance).x;

        occlusion += max(distance - d, 0.0) * weight;
        weight *= 0.65;
    }

    return clamp(
        1.0 - occlusion * scene.effects.x * 2.0,
        0.0,
        1.0
    );
}

fn albedo_at(
    id: f32,
    p: vec3<f32>
) -> vec3<f32> {

    if id < 0.0 {

        let distance = length(
            p - scene.camera.xyz
        );

        let cell = abs(
            fract(p.xz + 0.5) - 0.5
        );

        let width = 0.015
            + distance * 0.0003;

        let line = 1.0 - smoothstep(
            width,
            width * 2.0,
            min(cell.x, cell.y)
        );

        let fade = exp(-distance * 0.025);

        return mix(
            vec3<f32>(0.42),
            vec3<f32>(0.24),
            line * fade
        );
    }

    return scene.objects[u32(id)].color.xyz;
}

// Analytical SDF single-bounce indirect lighting.
// Rays sample nearby surfaces and estimate bounced light.
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

    let tangent = normalize(cross(n, axis));
    let bitangent = cross(n, tangent);

    var result = vec3<f32>(0.0);

    for (var i = 0u; i < 4u; i = i + 1u) {
        let angle = f32(i) * 1.570796;

        let direction = normalize(
            n * 0.65
            + tangent * cos(angle) * 0.76
            + bitangent * sin(angle) * 0.76
        );

        let hit = trace(
            p + n * 0.035,
            direction,
            scene.extra.x,
            20u
        );

        if hit.x > 0.0 {
            let hp = p + n * 0.035
                + direction * hit.x;

            let hn = normal(hp);
            let color = albedo_at(hit.y, hp);

            let lighting = max(
                dot(hn, light),
                0.0
            );

            let visibility = soft_shadow(
                hp + hn * 0.02,
                light
            );

            var emission = 0.0;

            if hit.y >= 0.0 {
                emission =
                    scene.objects[u32(hit.y)].material.z;
            }

            result += color * (
                lighting * visibility
                    * scene.settings.z * 0.3
                + emission
                + 0.06
            ) / (1.0 + hit.x * hit.x * 0.2);
        } else {
            result += sky(direction) * 0.2;
        }
    }

    return result * 0.25 * scene.settings.w;
}

fn aces(x: vec3<f32>) -> vec3<f32> {
    return clamp(
        (x * (2.51 * x + 0.03))
        / (x * (2.43 * x + 0.59) + 0.14),
        vec3<f32>(0.0),
        vec3<f32>(1.0)
    );
}

@fragment
fn fs_main(
    input: VertexOutput
) -> @location(0) vec4<f32> {
    let ro = scene.camera.xyz;

    let fov = 0.46630766;

    let rd = normalize(
        scene.forward.xyz
        + scene.right.xyz
         * input.uv.x * scene.camera.w * fov
        + scene.up.xyz * input.uv.y * fov
    );

    let hit = trace(ro, rd, 100000.0, 96u);

    var color = sky(rd);

    if hit.x > 0.0 {
        let p = ro + rd * hit.x;
        let n = normal(p);

        let light = normalize(
            vec3<f32>(-0.6, 0.8, 0.4)
        );

        let albedo = albedo_at(hit.y, p);

        var metallic = 0.0;
        var roughness = 0.6;
        var emission = 0.0;

        if hit.y >= 0.0 {
            let material =
                scene.objects[u32(hit.y)].material;

            metallic = material.x;
            roughness = material.y;
            emission = material.z;
        }

        let diffuse = max(dot(n, light), 0.0);

        let shadow = soft_shadow(
            p + n * 0.025,
            light
        );

        let ao = ambient_occlusion(p, n);

        let gi = sdf_indirect(p, n, light);

        let half_vector = normalize(light - rd);

        let specular = pow(
            max(dot(n, half_vector), 0.0),
            mix(160.0, 4.0, roughness)
        );

        let f0 = mix(
            vec3<f32>(0.04),
            albedo,
            metallic
        );

        color = albedo * (
            diffuse * shadow * scene.settings.z
                * (1.0 - metallic)
            + ao * 0.28
            + gi
        );

        color += f0 * specular
            * shadow * scene.settings.z;

        let reflected = reflect(rd, n);

        let reflection_hit = trace(
            p + n * 0.035,
            reflected,
            20.0,
            24u
        );

        var reflection_color = sky(reflected);

        if reflection_hit.x > 0.0 {
            let rp = p + n * 0.035
                + reflected * reflection_hit.x;

            let rn = normal(rp);

            reflection_color =
                albedo_at(reflection_hit.y, rp)
                * (0.15
                + max(dot(rn, light), 0.0));
        }

        let fresnel = f0
            + (vec3<f32>(1.0) - f0)
            * pow(
                1.0 - max(dot(n, -rd), 0.0),
                5.0
            );

        color += reflection_color
            * fresnel
            * pow(1.0 - roughness, 3.0)
            * scene.effects.w
            * mix(0.25, 1.0, metallic)
            * ao;

        color += albedo * emission
            * (1.0 + scene.extra.y);

        let fog = 1.0 - exp(
            -hit.x * scene.effects.y
        );

        color = mix(
            color,
            sky(vec3<f32>(rd.x, 0.05, rd.z)),
            fog
        );
    }

    color = aces(color * scene.settings.y);

    return vec4<f32>(color, 1.0);
}