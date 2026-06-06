#import bevy_pbr::{
    mesh_functions::{get_world_from_local, mesh_position_local_to_clip},
    mesh_view_bindings::globals,
}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) i_pos_radius: vec4<f32>,
    @location(4) i_color: vec4<f32>,
    @location(5) i_nucleus: vec4<f32>,
    @location(6) i_motion: vec4<f32>,
    @location(7) i_shape: vec4<f32>,
    @location(8) i_soft_radii_a: vec4<f32>,
    @location(9) i_soft_radii_b: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) nucleus: vec4<f32>,
    @location(3) motion: vec4<f32>,
    @location(4) shape: vec4<f32>,
    @location(5) soft_radii_a: vec4<f32>,
    @location(6) soft_radii_b: vec4<f32>,
};

const TAU: f32 = 6.28318530718;

fn rotate2d(v: vec2<f32>, angle: f32) -> vec2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return vec2<f32>(v.x * c - v.y * s, v.x * s + v.y * c);
}

fn safe_dir(v: vec2<f32>) -> vec2<f32> {
    let len_sq = dot(v, v);
    if (len_sq < 0.0001) {
        return vec2<f32>(1.0, 0.0);
    }
    return v * inverseSqrt(len_sq);
}

fn radial_shape_radius(angle: f32, shape: vec4<f32>) -> f32 {
    let radius =
        1.0
        + shape.x * sin(angle * 3.0 + shape.z)
        + shape.y * sin(angle * 5.0 - shape.z * 0.7);
    return clamp(radius, 0.55, 1.0);
}

fn soft_body_shape_radius(angle: f32, shape: vec4<f32>, soft_a: vec4<f32>, soft_b: vec4<f32>) -> f32 {
    let sector = fract((angle + TAU) / TAU) * 8.0;
    let i0 = u32(floor(sector)) % 8u;
    let i1 = (i0 + 1u) % 8u;
    let t = smoothstep(0.0, 1.0, fract(sector));
    let rays = array<f32, 8>(
        soft_a.x,
        soft_a.y,
        soft_a.z,
        soft_a.w,
        soft_b.x,
        soft_b.y,
        soft_b.z,
        soft_b.w
    );
    let soft_radius = mix(rays[i0], rays[i1], t);
    let wave =
        1.0
        + shape.x * 0.35 * sin(angle * 3.0 + shape.z)
        + shape.y * 0.35 * sin(angle * 5.0 - shape.z * 0.7);
    return clamp(soft_radius * wave, 0.25, 1.0);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let local = vertex.position.xy;
    let world_position = vec3<f32>(
        vertex.i_pos_radius.xy + local * vertex.i_pos_radius.w,
        vertex.i_pos_radius.z
    );

    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(
        get_world_from_local(0u),
        vec4<f32>(world_position, 1.0)
    );
    out.color = vertex.i_color;
    out.local_pos = local;
    out.nucleus = vertex.i_nucleus;
    out.motion = vertex.i_motion;
    out.shape = vertex.i_shape;
    out.soft_radii_a = vertex.i_soft_radii_a;
    out.soft_radii_b = vertex.i_soft_radii_b;
    return out;
}

fn ring_mask(dist: f32, outer: f32, inner: f32) -> f32 {
    return smoothstep(outer, outer - 0.035, dist) * (1.0 - smoothstep(inner, inner - 0.08, dist));
}

fn food_fragment(local: vec2<f32>, color: vec4<f32>, kind: f32, motion: vec4<f32>, shape: vec4<f32>) -> vec4<f32> {
    let time = globals.time + motion.w;
    let rotation = motion.z + globals.time * shape.z;
    let tumble = 0.82 + 0.18 * sin(time * 1.7 + shape.z * 0.41);
    let p = rotate2d(vec2<f32>(local.x, local.y / max(tumble, 0.2)), rotation);
    let dist = length(p);
    let angle = atan2(p.y, p.x);
    let lobes = shape.x;
    let roughness = shape.y;
    let shape_kind = shape.w;
    let pulse_speed = select(3.0, 4.4, kind < -0.5);
    let radius =
        0.78
        + sin(angle * lobes + time * pulse_speed) * roughness
        + sin(angle * (lobes + 2.0) - time * 1.7) * roughness * 0.35;
    var alpha = 1.0 - smoothstep(radius - 0.11, radius, dist);

    if (shape_kind > 1.5 && shape_kind < 2.5) {
        let box_dist = max(abs(p.x), abs(p.y));
        alpha = 1.0 - smoothstep(0.66, 0.78, box_dist);
    } else if (shape_kind > 2.5 && shape_kind < 3.5) {
        let tri_radius = 0.58 + 0.2 * cos(3.0 * (angle + 0.52));
        alpha = 1.0 - smoothstep(tri_radius - 0.1, tri_radius, dist);
    } else if (shape_kind > 3.5 && shape_kind < 4.5) {
        let diamond_dist = abs(p.x) + abs(p.y);
        alpha = 1.0 - smoothstep(0.86, 1.02, diamond_dist);
    } else if (shape_kind > 4.5 && shape_kind < 5.5) {
        let star_radius = 0.64 + 0.18 * cos(angle * 5.0 + time * 0.8);
        alpha = 1.0 - smoothstep(star_radius - 0.09, star_radius, dist);
    } else if (shape_kind > 5.5) {
        let pebble_radius = 0.72
            + 0.09 * sin(angle * 2.0 + motion.w)
            + 0.07 * sin(angle * 4.0 - motion.w * 0.7);
        alpha = 1.0 - smoothstep(pebble_radius - 0.12, pebble_radius, dist);
    }

    let normalized_dist = clamp(dist / max(radius, 0.1), 0.0, 1.0);
    let z = sqrt(max(0.0, 1.0 - normalized_dist * normalized_dist));
    let normal = normalize(vec3<f32>(p.x * (1.0 + (1.0 - tumble) * 0.7), p.y, z + 0.3));
    let light_dir = normalize(vec3<f32>(-0.34, 0.42, 0.84));
    let view_dir = vec3<f32>(0.0, 0.0, 1.0);
    let diffuse = max(dot(normal, light_dir), 0.0) * 0.58;
    let specular = pow(max(dot(normal, normalize(light_dir + view_dir)), 0.0), 16.0) * 0.36;
    let core = 1.0 - smoothstep(0.0, radius, dist);
    let rim = smoothstep(radius * 0.52, radius, dist) * alpha;
    let pulse = 0.88 + 0.12 * sin(time * pulse_speed + lobes);
    let food_color = color.rgb * pulse;

    var rgb = mix(food_color * 0.58, food_color * 1.55, core);
    rgb += food_color * diffuse;
    rgb += vec3<f32>(1.0, 0.96, 0.82) * specular;

    if (kind < -0.5) {
        let vein = smoothstep(0.05, 0.0, abs(sin(angle * 3.0 + time * 1.6))) * smoothstep(0.2, 0.75, dist);
        rgb += vec3<f32>(0.85, 0.05, 0.03) * vein * 0.18;
        rgb += vec3<f32>(1.0, 0.34, 0.28) * rim * 0.22;
    } else {
        let sprout = smoothstep(0.16, 0.0, abs(p.x)) * smoothstep(-0.28, 0.42, p.y) * smoothstep(0.76, 0.08, dist);
        rgb += vec3<f32>(0.36, 1.0, 0.38) * sprout * 0.16;
        rgb += vec3<f32>(0.72, 1.0, 0.52) * rim * 0.16;
    }

    return vec4<f32>(rgb, clamp(alpha * color.a, 0.0, 0.98));
}

fn obstacle_fragment(local: vec2<f32>, color: vec4<f32>, motion: vec4<f32>, shape: vec4<f32>) -> vec4<f32> {
    let p = rotate2d(local, motion.z);
    let dist = length(p);
    let angle = atan2(p.y, p.x);
    let disc = 1.0 - smoothstep(0.92, 1.02, dist);

    let spokes = max(shape.x, 3.0);
    let rings = max(shape.y, 1.0);
    let spoke_window = smoothstep(0.30, 0.38, dist) * (1.0 - smoothstep(0.78, 0.96, dist));
    let spoke_mask =
        (1.0 - smoothstep(0.02, 0.055, abs(sin(angle * spokes * 0.5)))) * spoke_window * disc;
    let ring_wave = abs(fract(dist * rings + 0.12) - 0.5);
    let ring_mask = (1.0 - smoothstep(0.02, 0.08, ring_wave))
        * smoothstep(0.36, 0.48, dist)
        * (1.0 - smoothstep(0.82, 0.94, dist))
        * disc;
    let rim = smoothstep(0.84, 0.92, dist) * (1.0 - smoothstep(0.96, 1.02, dist));
    let core = 1.0 - smoothstep(0.0, 0.36, dist);

    var rgb = color.rgb * (0.45 + core * 0.22);
    rgb += vec3<f32>(0.88, 0.94, 1.0) * spoke_mask * 0.18;
    rgb += vec3<f32>(0.78, 0.90, 1.0) * ring_mask * 0.12;
    rgb += vec3<f32>(0.86, 0.94, 1.0) * rim * 0.26;
    return vec4<f32>(rgb, clamp(disc * color.a + rim * 0.08 + spoke_mask * 0.04, 0.0, 0.42));
}

fn feeder_core_fragment(local: vec2<f32>, color: vec4<f32>, motion: vec4<f32>, shape: vec4<f32>) -> vec4<f32> {
    let p = local * max(shape.w, 1.0);
    let dist = length(p);
    let angle = atan2(p.y, p.x);
    let time = globals.time + motion.w;
    let edge =
        0.96
        + 0.045 * sin(angle * 5.0 + time * 1.1)
        + 0.025 * sin(angle * 9.0 - time * 0.7);
    let body = 1.0 - smoothstep(edge * 0.86, edge, dist);
    let membrane = smoothstep(edge * 0.70, edge * 0.90, dist)
        * (1.0 - smoothstep(edge * 0.98, edge * 1.05, dist));
    let center = 1.0 - smoothstep(0.0, 0.42, dist);
    let vein = (1.0 - smoothstep(0.025, 0.085, abs(sin(angle * max(shape.x, 4.0) * 0.5 + time * 0.25))))
        * smoothstep(0.18, 0.52, dist)
        * (1.0 - smoothstep(0.54, 0.86, dist));
    let alpha = clamp(body * 0.54 + membrane * 0.24 + vein * 0.08, 0.0, color.a);

    var rgb = color.rgb * (0.42 + body * 0.34 + center * 0.18);
    rgb += vec3<f32>(0.74, 1.0, 0.52) * vein * 0.18;
    rgb += vec3<f32>(0.70, 1.0, 0.64) * membrane * 0.32;
    rgb += vec3<f32>(0.12, 0.42, 0.18) * (1.0 - center) * body * 0.08;
    return vec4<f32>(rgb, alpha);
}

fn feeder_branch_fragment(local: vec2<f32>, color: vec4<f32>, motion: vec4<f32>, shape: vec4<f32>) -> vec4<f32> {
    let p = rotate2d(local, -motion.z);
    let half_len = clamp(shape.z, 0.12, 0.98);
    let base_width = clamp(shape.x, 0.018, 0.26);
    let t = clamp(p.x / max(half_len * 2.0, 0.001) + 0.5, 0.0, 1.0);
    let curve = shape.y * base_width * sin(t * 3.14159265) * (0.95 + 0.15 * sin(globals.time * 0.7 + motion.w));
    let y = p.y - curve;
    let cap_x = max(abs(p.x) - half_len, 0.0);
    let width_profile =
        base_width
        * mix(1.28, 0.58, t)
        * (1.0 + 0.10 * sin(t * TAU * 2.0 + motion.w));
    let branch_dist = length(vec2<f32>(cap_x, y));
    let alpha = 1.0 - smoothstep(width_profile, width_profile + 0.045, branch_dist);
    let vein = (1.0 - smoothstep(width_profile * 0.16, width_profile * 0.42, abs(y)))
        * smoothstep(0.04, 0.18, t)
        * (1.0 - smoothstep(0.86, 1.0, t));
    let tip = (1.0 - smoothstep(0.82, 0.98, abs(t - 0.94))) * alpha;

    var rgb = color.rgb * (0.46 + alpha * 0.24);
    rgb += vec3<f32>(0.78, 1.0, 0.54) * vein * 0.32;
    rgb += vec3<f32>(0.86, 1.0, 0.58) * tip * 0.24;
    rgb += vec3<f32>(0.10, 0.36, 0.16) * (1.0 - t) * alpha * 0.10;
    return vec4<f32>(rgb, clamp(alpha * color.a, 0.0, color.a));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let local = in.local_pos;
    let dist = length(local);
    let kind = in.nucleus.w;

    if (kind < 0.5) {
        let food = food_fragment(local, in.color, kind, in.motion, in.shape);
        if (food.a < 0.01) {
            discard;
        }
        return food;
    }

    if (kind > 1.5 && kind < 2.5) {
        let obstacle = obstacle_fragment(local, in.color, in.motion, in.shape);
        if (obstacle.a < 0.01) {
            discard;
        }
        return obstacle;
    }

    if (kind > 2.5 && kind < 3.5) {
        let feeder_core = feeder_core_fragment(local, in.color, in.motion, in.shape);
        if (feeder_core.a < 0.01) {
            discard;
        }
        return feeder_core;
    }

    if (kind > 3.5 && kind < 4.5) {
        let feeder_branch = feeder_branch_fragment(local, in.color, in.motion, in.shape);
        if (feeder_branch.a < 0.01) {
            discard;
        }
        return feeder_branch;
    }

    let angle = atan2(local.y, local.x);
    let heading_angle = atan2(in.motion.y, in.motion.x);
    let shape_radius = soft_body_shape_radius(
        angle - heading_angle,
        in.shape,
        in.soft_radii_a,
        in.soft_radii_b
    );
    let effective_dist = dist / shape_radius;

    if (effective_dist > 1.0) {
        discard;
    }

    let body = smoothstep(1.0, 0.78, effective_dist);
    let membrane = ring_mask(effective_dist, 1.0, 0.83);
    let inner_glow = smoothstep(0.92, 0.12, effective_dist);
    let jelly_wave = sin(angle * 5.0 + in.motion.w * 3.2) * in.motion.z;
    let cytoplasm_noise =
        0.5 + 0.5 * sin(local.x * 13.0 + local.y * 17.0 + in.motion.w * 1.7 + jelly_wave * 3.0);
    let cytoplasm = in.color.rgb * (0.68 + inner_glow * 0.34 + cytoplasm_noise * 0.08);

    let nucleus_shift = safe_dir(in.motion.xy) * sin(in.motion.w * 2.4) * in.motion.z * 0.055;
    let nucleus_dist = length(local - in.nucleus.xy - nucleus_shift) / max(in.nucleus.z, 0.04);
    let nucleus_mask = smoothstep(1.0, 0.72, nucleus_dist);
    let nucleus_rim = ring_mask(nucleus_dist, 1.0, 0.64);
    let nucleus_color = vec3<f32>(0.55, 0.78, 1.0) * (0.8 + 0.22 * sin(in.motion.w + nucleus_dist * 5.0));

    let highlight_pos = local - vec2<f32>(-0.28, 0.34);
    let highlight = smoothstep(0.52, 0.0, length(highlight_pos)) * 0.22;
    let collision_flash = in.motion.z * ring_mask(effective_dist, 0.78, 0.36) * 0.45;

    var rgb = cytoplasm;
    rgb += vec3<f32>(0.72, 0.95, 1.0) * membrane * (0.72 + in.motion.z * 0.2);
    rgb += vec3<f32>(0.9, 1.0, 1.0) * highlight;
    rgb = mix(rgb, nucleus_color, nucleus_mask * 0.72);
    rgb += vec3<f32>(0.9, 1.0, 1.0) * nucleus_rim * 0.38;
    rgb += vec3<f32>(0.8, 0.95, 1.0) * collision_flash;

    let alpha = body * (0.24 + inner_glow * 0.18) + membrane * 0.5 + nucleus_mask * 0.24;
    return vec4<f32>(rgb, clamp(alpha * in.color.a, 0.0, 0.96));
}
