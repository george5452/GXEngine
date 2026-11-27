@group(0) @binding(1)
var<storage, read> gx_bvh: array<vec4<f32>>;

fn gx_box_hit(
    origin: vec3<f32>,
    inverse: vec3<f32>,
    lo: vec3<f32>,
    hi: vec3<f32>,
    limit: f32
) -> bool {

    let a = (lo - origin) * inverse;
    let b = (hi - origin) * inverse;

    let mn = min(a, b);
    let mx = max(a, b);

    let entry = max(
        max(mn.x, mn.y),
        max(mn.z, 0.0)
    );

    let exit = min(
        min(mx.x, mx.y),
        mx.z
    );

    return exit >= entry && entry < limit;
}

fn gx_triangle_hit(
    origin: vec3<f32>,
    direction: vec3<f32>,
    index: u32
) -> f32 {

    let a = gx_bvh[index].xyz;
    let b = gx_bvh[index + 1u].xyz;
    let c = gx_bvh[index + 2u].xyz;

    let e1 = b - a;
    let e2 = c - a;

    let h = cross(direction, e2);

    let determinant = dot(e1, h);

    if abs(determinant) < 0.0000001 {
        return -1.0;
    }

    let inv = 1.0 / determinant;

    let s = origin - a;

    let u = inv * dot(s, h);

    if u < 0.0 || u > 1.0 {
        return -1.0;
    }

    let q = cross(s, e1);

    let v = inv * dot(direction, q);

    if v < 0.0 || u + v > 1.0 {
        return -1.0;
    }

    return inv * dot(e2, q);
}

fn gx_mesh_trace(
    origin: vec3<f32>,
    direction: vec3<f32>,
    limit: f32,
    any_hit: bool
) -> f32 {

    if gx_bvh[0].x < 1.0 {
        return -1.0;
    }

    let triangle_offset = u32(gx_bvh[0].y);

    let tiny = select(
        vec3<f32>(-0.000001),
        vec3<f32>(0.000001),
        direction >= vec3<f32>(0.0)
    );

    let safe_direction = select(
        direction,
        tiny,
        abs(direction) < vec3<f32>(0.000001)
    );

    let inverse =
        vec3<f32>(1.0) / safe_direction;

    var stack: array<u32, 32>;

    stack[0] = 0u;

    var count = 1u;
    var closest = limit;

    loop {
        if count == 0u {
            break;
        }

        count -= 1u;

        let node = stack[count];

        let base = 1u + node * 3u;

        let lo = gx_bvh[base].xyz;
        let hi = gx_bvh[base + 1u].xyz;
        let node_info = gx_bvh[base + 2u];

        if !gx_box_hit(
            origin,
            inverse,
            lo,
            hi,
            closest
        ) {
            continue;
        }

        if node_info.z > 0.5 {

            let first = u32(node_info.x);
            let total = u32(node_info.y);

            for (
                var i = 0u;
                i < total;
                i += 1u
            ) {

                let index =
                    triangle_offset
                    + (first + i) * 3u;

                let hit = gx_triangle_hit(
                    origin,
                    direction,
                    index
                );

                if hit > 0.002
                    && hit < closest
                {
                    if any_hit {
                        return hit;
                    }

                    closest = hit;
                }
            }

        } else {

            if count + 2u <= 32u {
                stack[count] = u32(node_info.x);

                stack[count + 1u] =
                    u32(node_info.y);

                count += 2u;
            }
        }
    }

    if closest < limit {
        return closest;
    }

    return -1.0;
}

fn gx_mesh_shadow(
    origin: vec3<f32>,
    direction: vec3<f32>
) -> f32 {

    let hit = gx_mesh_trace(
        origin,
        direction,
        100.0,
        true
    );

    if hit > 0.0 {
        return 0.0;
    }

    return 1.0;
}

fn gx_mesh_ao(
    p: vec3<f32>,
    n: vec3<f32>
) -> f32 {

    let axis = select(
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(1.0, 0.0, 0.0),
        abs(n.y) > 0.95
    );

    let tangent = normalize(cross(axis, n));
    let bitangent = cross(n, tangent);

    var occlusion = 0.0;

    for (
        var i = 0u;
        i < 4u;
        i += 1u
    ) {

        let angle = f32(i) * 1.570796;

        let direction = normalize(
            n * 0.75
            + tangent * cos(angle) * 0.65
            + bitangent * sin(angle) * 0.65
        );

        let hit = gx_mesh_trace(
            p + n * 0.015,
            direction,
            0.6,
            false
        );

        if hit > 0.0 {
            occlusion +=
                1.0 - clamp(hit / 0.6, 0.0, 1.0);
        }
    }

    return clamp(
        1.0 - occlusion * 0.25 * scene.effects.x,
        0.0,
        1.0
    );
}