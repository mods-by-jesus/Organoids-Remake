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
    @location(10) i_section_radii_0: vec4<f32>,
    @location(11) i_section_radii_1: vec4<f32>,
    @location(12) i_section_radii_2: vec4<f32>,
    @location(13) i_section_radii_3: vec4<f32>,
    @location(14) i_section_meta: vec4<f32>,
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
    @location(7) section_radii_0: vec4<f32>,
    @location(8) section_radii_1: vec4<f32>,
    @location(9) section_radii_2: vec4<f32>,
    @location(10) section_radii_3: vec4<f32>,
    @location(11) section_meta: vec4<f32>,
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

fn smooth_min_distance(a: f32, b: f32, smoothing: f32) -> f32 {
    let k = max(smoothing, 0.0001);
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

fn organic_neck_distance(
    local: vec2<f32>,
    axis: vec2<f32>,
    normal: vec2<f32>,
    separation: f32,
    width: f32,
    phase: f32
) -> f32 {
    let half_length = separation + width * 1.42;
    let along = dot(local, axis);
    let normalized = clamp(along / max(half_length, 0.01), -1.0, 1.0);
    let envelope = max(0.0, 1.0 - normalized * normalized);
    let curve = (
        sin(normalized * 3.14159265 + phase) * (0.018 + width * 0.58)
        + sin(normalized * TAU - globals.time * 0.72 + phase * 1.7) * width * 0.18
    ) * envelope;
    let width_profile = width
        * (0.84 + 0.16 * sin(normalized * 3.14159265 * 3.0 + phase + globals.time * 0.34));
    let perpendicular = dot(local, normal) - curve;
    return max(
        abs(perpendicular) / max(width_profile, 0.008),
        abs(along) / max(half_length, 0.01)
    );
}

fn mitosis_contour_distance(
    local: vec2<f32>,
    heading_angle: f32,
    shape: vec4<f32>,
    soft_a: vec4<f32>,
    soft_b: vec4<f32>,
    progress: f32
) -> f32 {
    let original_angle = atan2(local.y, local.x) - heading_angle;
    let original_radius = soft_body_shape_radius(original_angle, shape, soft_a, soft_b);
    let original = length(local) / max(original_radius, 0.04);
    let split = smoothstep(0.10, 0.96, progress);
    if (split <= 0.0001) {
        return original;
    }

    let forward = vec2<f32>(cos(heading_angle), sin(heading_angle));
    let axis = vec2<f32>(-forward.y, forward.x);
    let normal = vec2<f32>(-axis.y, axis.x);
    let separation = split * 0.49;
    let lobe_scale = mix(1.0, 0.90, split);
    let q0 = (local - axis * separation) / lobe_scale;
    let q1 = (local + axis * separation) / lobe_scale;
    let angle0 = atan2(q0.y, q0.x) - heading_angle;
    let angle1 = atan2(q1.y, q1.x) - heading_angle;
    let lobe0 = length(q0) / max(soft_body_shape_radius(angle0, shape, soft_a, soft_b), 0.04);
    let lobe1 = length(q1) / max(soft_body_shape_radius(angle1, shape, soft_a, soft_b), 0.04);

    let pinch = smoothstep(0.38, 1.0, progress);
    let neck_width = mix(0.25, 0.026, pinch);
    let neck = organic_neck_distance(local, axis, normal, separation, neck_width, shape.z);
    let lobes = smooth_min_distance(lobe0, lobe1, 0.065);
    let joined = smooth_min_distance(lobes, neck, mix(0.11, 0.035, pinch));
    return mix(original, joined, split);
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
    out.section_radii_0 = vertex.i_section_radii_0;
    out.section_radii_1 = vertex.i_section_radii_1;
    out.section_radii_2 = vertex.i_section_radii_2;
    out.section_radii_3 = vertex.i_section_radii_3;
    out.section_meta = vertex.i_section_meta;
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

fn selection_fragment(
    local: vec2<f32>,
    color: vec4<f32>,
    motion: vec4<f32>,
    shape: vec4<f32>,
    soft_a: vec4<f32>,
    soft_b: vec4<f32>,
    mitosis: f32
) -> vec4<f32> {
    let heading_angle = atan2(motion.y, motion.x);
    let contour_distance = mitosis_contour_distance(
        local, heading_angle, shape, soft_a, soft_b, mitosis
    );
    if (contour_distance > 1.0) {
        discard;
    }

    let pulse = 0.5 + 0.5 * sin(globals.time * 2.1);
    let edge = smoothstep(0.76, 1.0, contour_distance);
    let scan = 0.5 + 0.5 * sin((local.x + local.y) * 18.0 - globals.time * 1.8);
    let fill_alpha = mix(0.20, 0.27, pulse) + scan * 0.035;
    let alpha = mix(fill_alpha, 0.78, edge);
    let rgb = mix(color.rgb * 0.70, vec3<f32>(0.82, 1.0, 0.98), edge * 0.72);
    return vec4<f32>(rgb, alpha * color.a);
}

fn velocity_arrow_fragment(local: vec2<f32>, color: vec4<f32>, motion: vec4<f32>, shape: vec4<f32>) -> vec4<f32> {
    let p = rotate2d(local, -motion.z);
    let speed_factor = clamp(shape.x, 0.0, 1.0);
    let shaft_half_width = mix(0.075, 0.11, speed_factor);
    let shaft = (1.0 - smoothstep(shaft_half_width, shaft_half_width + 0.035, abs(p.y)))
        * smoothstep(-0.86, -0.76, p.x)
        * (1.0 - smoothstep(0.28, 0.38, p.x));

    let head_x = p.x - 0.28;
    let head_width = max(0.0, 0.62 - head_x) * 0.72;
    let head = smoothstep(0.0, 0.07, head_x)
        * (1.0 - smoothstep(0.60, 0.68, head_x))
        * (1.0 - smoothstep(head_width, head_width + 0.045, abs(p.y)));

    let mask = max(shaft, head);
    if (mask < 0.01) {
        discard;
    }

    let pulse = 0.88 + 0.12 * sin(globals.time * 2.1);
    let rgb = mix(color.rgb * 0.72, vec3<f32>(0.90, 1.0, 0.98), head * 0.62) * pulse;
    return vec4<f32>(rgb, mask * color.a);
}

fn food_particle_fragment(
    local: vec2<f32>,
    color: vec4<f32>,
    motion: vec4<f32>,
    shape: vec4<f32>
) -> vec4<f32> {
    let life = clamp(motion.z, 0.0, 1.0);
    let mitosis_style = step(0.5, shape.y) * (1.0 - step(1.5, shape.y));
    let lysis_style = step(1.5, shape.y);
    let velocity_angle = atan2(motion.y, motion.x);
    let p = rotate2d(local, -velocity_angle);
    let speed = clamp(length(motion.xy) / 110.0, 0.0, 1.0);
    let stretched = length(vec2<f32>(
        p.x * mix(mix(mix(0.72, 0.48, speed), 0.72, mitosis_style), 0.38, lysis_style),
        p.y * mix(mix(1.18, 1.02, mitosis_style), 1.34, lysis_style)
    ));
    let wobble_strength = mix(mix(0.08, 0.14, mitosis_style), 0.20, lysis_style);
    let wobble_frequency = mix(mix(5.0, 7.0, mitosis_style), 4.0, lysis_style);
    let wobble = 1.0 + wobble_strength
        * sin(atan2(p.y, p.x) * wobble_frequency + motion.w + globals.time * 8.0);
    let body = 1.0 - smoothstep(0.48, 1.0, stretched * wobble);
    let hot_core = 1.0 - smoothstep(0.0, 0.38, length(local));
    let sparkle = pow(max(hot_core, 0.0), 1.8) * (0.72 + 0.28 * sin(globals.time * 13.0 + motion.w));
    let halo = ring_mask(length(local), 0.86, 0.54) * mitosis_style * life;
    let alpha = max(body * life * life, halo * 0.36);
    if (alpha < 0.01) {
        discard;
    }
    let hot_color = mix(vec3<f32>(1.0), vec3<f32>(1.0, 0.72, 0.82), lysis_style);
    let rgb = mix(color.rgb * mix(0.78, 0.62, lysis_style), hot_color, sparkle * 0.72 + halo * 0.24);
    return vec4<f32>(rgb, alpha * color.a);
}

fn cell_wake_fragment(
    local: vec2<f32>,
    color: vec4<f32>,
    motion: vec4<f32>,
    shape: vec4<f32>
) -> vec4<f32> {
    let p = rotate2d(local, -motion.z);
    let half_length = clamp(shape.x, 0.05, 0.94);
    let base_width = clamp(shape.y, 0.018, 0.52);
    let speed = clamp(shape.z, 0.0, 1.4);
    let strength = clamp(shape.w, 0.0, 1.0);
    let t = clamp((p.x + half_length) / max(half_length * 2.0, 0.001), 0.0, 1.0);
    let envelope = sin(t * 3.14159265);
    let sway = (
        sin(t * 8.0 - globals.time * (1.35 + speed * 0.35) + motion.w) * 0.045
        + sin(t * 13.0 - globals.time * 0.72 + motion.w * 1.7) * 0.015
    ) * base_width * envelope;
    let width = base_width * mix(0.74, 1.0, pow(t, 0.72))
        * (0.97 + 0.03 * sin(t * 9.0 + globals.time * 0.62));
    let cap_x = max(abs(p.x) - half_length, 0.0);
    let wake_distance = length(vec2<f32>(cap_x, p.y - sway));
    let lateral = wake_distance / max(width, 0.001);
    let signed_lateral = (p.y - sway) / max(width, 0.001);
    let outer_mask = 1.0 - smoothstep(0.96, 1.18, lateral);
    let interior = 1.0 - smoothstep(0.18, 0.94, lateral);
    let edge_ring = smoothstep(0.56, 0.82, lateral)
        * (1.0 - smoothstep(0.98, 1.16, lateral));

    let foam_noise =
        0.52 * sin(t * 47.0 - globals.time * 1.22 + motion.w)
        + 0.31 * sin(t * 19.0 + lateral * 8.0 + globals.time * 0.54)
        + 0.17 * sin(t * 83.0 - lateral * 13.0 + motion.w * 2.1);
    let broken_foam = smoothstep(-0.30, 0.42, foam_noise);
    let edge_foam = edge_ring * mix(0.12, 0.58, broken_foam);

    let patch_field =
        sin(t * 34.0 + motion.w * 1.3 - globals.time * 0.31)
        * sin(signed_lateral * 5.7 - motion.w * 0.8 + globals.time * 0.17)
        + 0.42 * sin(t * 19.0 - signed_lateral * 8.3 + motion.w * 2.1);
    let pale_patches = interior * smoothstep(0.52, 1.08, patch_field) * 0.10;

    let tail_fade = smoothstep(0.0, 0.22, t);
    let alpha = (
        interior * 0.030
        + edge_foam * 0.48
        + pale_patches
    ) * outer_mask * tail_fade * (0.54 + speed * 0.08);
    if (alpha < 0.008) {
        discard;
    }
    let foam = max(edge_foam, pale_patches * 0.42);
    let rgb = mix(color.rgb * 0.52, vec3<f32>(0.90, 0.98, 1.0), foam * 0.88);
    return vec4<f32>(rgb, alpha * color.a);
}

fn perception_fragment(local: vec2<f32>, color: vec4<f32>, shape: vec4<f32>) -> vec4<f32> {
    let dist = length(local);
    if (dist > 1.0) {
        discard;
    }

    let edge_width = clamp(shape.x, 0.001, 0.08);
    let angle = atan2(local.y, local.x);
    let scan_time = globals.time * 1.35;
    let dash = smoothstep(-0.25, 0.25, sin(angle * 52.0 - scan_time));
    let edge = smoothstep(1.0 - edge_width * 2.5, 1.0 - edge_width, dist)
        * (1.0 - smoothstep(1.0 - edge_width, 1.0, dist));
    let ring_phase = abs(fract(dist * 4.0 - globals.time * 0.16 + 0.5) - 0.5);
    let range_rings = (1.0 - smoothstep(edge_width * 0.8, edge_width * 2.2, ring_phase))
        * smoothstep(0.08, 0.16, dist);
    let inner_grid = (0.5 + 0.5 * sin((local.x + local.y) * 28.0 + scan_time * 0.45)) * 0.012;
    let breathe = 0.90 + 0.10 * sin(globals.time * 1.7);
    let alpha = (0.028 + inner_grid + range_rings * 0.10 + edge * mix(0.32, 0.88, dash)) * breathe;
    let rgb = mix(
        color.rgb * 0.62,
        vec3<f32>(0.72, 1.0, 0.96),
        max(edge, range_rings * 0.45)
    );
    return vec4<f32>(rgb, alpha * color.a);
}

fn target_vector_fragment(local: vec2<f32>, color: vec4<f32>, motion: vec4<f32>, shape: vec4<f32>) -> vec4<f32> {
    let p = rotate2d(local, -motion.z);
    let half_length = clamp(shape.x, 0.0, 0.98);
    let width = clamp(shape.y, 0.001, 0.08);
    let world_pixel = max(shape.w, 0.0001);
    let cap_x = max(abs(p.x) - half_length, 0.0);
    let line_dist = length(vec2<f32>(cap_x, p.y));
    let dash_phase = (p.x + half_length) / max(world_pixel * 9.0, 0.001);
    let dash = smoothstep(
        -0.20,
        0.20,
        sin((dash_phase - globals.time * 1.8) * 3.14159265)
    );
    let flow = smoothstep(
        0.62,
        1.0,
        sin((dash_phase - globals.time * 3.2) * 3.14159265)
    );
    let pulse_center = fract(globals.time * 0.42);
    let flow_t = clamp((p.x + half_length) / max(half_length * 2.0, 0.001), 0.0, 1.0);
    let pulse = exp(-pow((flow_t - pulse_center) * 7.0, 2.0));
    let line = (1.0 - smoothstep(width, width * 1.8, line_dist))
        * (0.14 + dash * 0.38 + flow * 0.34 + pulse * 0.58);

    let target_delta = p - vec2<f32>(half_length, 0.0);
    let target_dist = length(target_delta);
    let reticle_pulse = 1.0 + 0.20 * sin(globals.time * 3.0);
    let reticle_radius = world_pixel * 6.5 * reticle_pulse;
    let reticle_width = world_pixel * 1.6;
    let reticle = smoothstep(reticle_radius - reticle_width, reticle_radius, target_dist)
        * (1.0 - smoothstep(reticle_radius, reticle_radius + reticle_width, target_dist));
    let cross_x = (1.0 - smoothstep(reticle_width, reticle_width * 1.8, abs(target_delta.x)))
        * (1.0 - smoothstep(reticle_radius * 1.35, reticle_radius * 1.65, abs(target_delta.y)));
    let cross_y = (1.0 - smoothstep(reticle_width, reticle_width * 1.8, abs(target_delta.y)))
        * (1.0 - smoothstep(reticle_radius * 1.35, reticle_radius * 1.65, abs(target_delta.x)));
    let orbit = smoothstep(
        0.86,
        1.0,
        sin(atan2(target_delta.y, target_delta.x) * 3.0 - globals.time * 3.6)
    ) * reticle;
    let marker = max(reticle + orbit * 0.65, max(cross_x, cross_y));
    let alpha = max(line, marker);
    if (alpha < 0.01) {
        discard;
    }

    let rgb = mix(color.rgb * 0.68, vec3<f32>(0.88, 1.0, 0.72), marker * 0.78);
    return vec4<f32>(rgb, alpha * color.a);
}

fn cell_bridge_fragment(local: vec2<f32>, color: vec4<f32>, motion: vec4<f32>, shape: vec4<f32>) -> vec4<f32> {
    let p = rotate2d(local, -motion.z);
    let half_length = clamp(shape.z, 0.05, 0.98);
    let width = clamp(shape.x, 0.015, 0.32);
    let cap_x = max(abs(p.x) - half_length, 0.0);
    let capsule_dist = length(vec2<f32>(cap_x, p.y));
    let body = 1.0 - smoothstep(width * 0.82, width, capsule_dist);
    if (body < 0.01) {
        discard;
    }
    let center = 1.0 - smoothstep(0.0, width * 0.42, abs(p.y));
    let membrane = smoothstep(width * 0.62, width * 0.94, capsule_dist);
    let rgb = mix(color.rgb, vec3<f32>(1.0), membrane * 0.54 + center * 0.08);
    let alpha = mix(0.38, 0.90, membrane) * body;
    return vec4<f32>(rgb, alpha * color.a);
}

fn packed_shape_radius(packed: vec4<f32>, angle: f32) -> f32 {
    let sector = fract((angle + TAU) / TAU) * 8.0;
    let ray_0 = u32(floor(sector)) % 8u;
    let ray_1 = (ray_0 + 1u) % 8u;
    let packed_values = array<f32, 4>(packed.x, packed.y, packed.z, packed.w);
    let pair_0 = unpack2x16unorm(bitcast<u32>(packed_values[ray_0 / 2u]));
    let pair_1 = unpack2x16unorm(bitcast<u32>(packed_values[ray_1 / 2u]));
    let radius_0 = select(pair_0.x, pair_0.y, ray_0 % 2u == 1u);
    let radius_1 = select(pair_1.x, pair_1.y, ray_1 % 2u == 1u);
    return mix(radius_0, radius_1, smoothstep(0.0, 1.0, fract(sector)));
}

fn quadratic_point(start: vec2<f32>, control: vec2<f32>, end: vec2<f32>, t: f32) -> vec2<f32> {
    return mix(mix(start, control, t), mix(control, end, t), t);
}

fn segmented_cell_distance(
    local: vec2<f32>,
    motion: vec4<f32>,
    shape: vec4<f32>,
    endpoints: vec4<f32>,
    profile: vec4<f32>,
    packed_0: vec4<f32>,
    packed_1: vec4<f32>,
    packed_2: vec4<f32>,
    packed_3: vec4<f32>,
    section_meta: vec4<f32>
) -> f32 {
    let positions = array<vec2<f32>, 4>(
        endpoints.xy,
        endpoints.zw,
        profile.xy,
        profile.zw
    );
    let headings = array<f32, 4>(shape.x, shape.y, shape.z, shape.w);
    let packed_radii = array<vec4<f32>, 4>(packed_0, packed_1, packed_2, packed_3);
    let parents = array<u32, 3>(u32(motion.y), u32(motion.z), u32(motion.w));
    let count = u32(clamp(round(motion.x), 2.0, 4.0));
    var nearest = 1000.0;
    for (var section = 0u; section < 4u; section = section + 1u) {
        if (section >= count) {
            break;
        }
        let node_delta = local - positions[section];
        let node_angle = atan2(node_delta.y, node_delta.x) - headings[section];
        let node_radius = packed_shape_radius(packed_radii[section], node_angle);
        nearest = min(nearest, length(node_delta) / max(node_radius, 0.008));
        if (section == 0u) {
            continue;
        }
        let p0 = positions[parents[section - 1u]];
        let p1 = positions[section];
        let edge_axis = p1 - p0;
        let edge_side = safe_dir(vec2<f32>(-edge_axis.y, edge_axis.x));
        let curves = array<f32, 3>(section_meta.x, section_meta.y, section_meta.z);
        let control = (p0 + p1) * 0.5 + edge_side * curves[section - 1u];
        for (var curve_segment = 0u; curve_segment < 6u; curve_segment = curve_segment + 1u) {
            let t0 = f32(curve_segment) / 6.0;
            let t1 = f32(curve_segment + 1u) / 6.0;
            let q0 = quadratic_point(p0, control, p1, t0);
            let q1 = quadratic_point(p0, control, p1, t1);
            let segment_axis = q1 - q0;
            let along = clamp(
                dot(local - q0, segment_axis) / max(dot(segment_axis, segment_axis), 0.000001),
                0.0,
                1.0
            );
            let edge_t = mix(t0, t1, along);
            let closest = mix(q0, q1, along);
            let contact_angle = atan2((local - closest).y, (local - closest).x);
            let parent = parents[section - 1u];
            let parent_radius = packed_shape_radius(
                packed_radii[parent], contact_angle - headings[parent]
            );
            let child_radius = packed_shape_radius(
                packed_radii[section], contact_angle - headings[section]
            );
            let end_blend = (edge_t * 2.0 - 1.0) * (edge_t * 2.0 - 1.0);
            let radius = mix(parent_radius, child_radius, edge_t) * (0.78 + end_blend * 0.22);
            nearest = min(nearest, length(local - closest) / max(radius, 0.008));
        }
    }
    return nearest;
}

fn segmented_cell_fragment(
    local: vec2<f32>,
    color: vec4<f32>,
    nucleus: vec4<f32>,
    motion: vec4<f32>,
    shape: vec4<f32>,
    endpoints: vec4<f32>,
    profile: vec4<f32>,
    packed_0: vec4<f32>,
    packed_1: vec4<f32>,
    packed_2: vec4<f32>,
    packed_3: vec4<f32>,
    section_meta: vec4<f32>,
    selection: bool
) -> vec4<f32> {
    let original_contour = segmented_cell_distance(
        local, motion, shape, endpoints, profile,
        packed_0, packed_1, packed_2, packed_3, section_meta
    );
    let mitosis = clamp(section_meta.w, 0.0, 1.0);
    let split = smoothstep(0.10, 0.96, mitosis);
    let head_forward = vec2<f32>(cos(shape.x), sin(shape.x));
    let axis = vec2<f32>(-head_forward.y, head_forward.x);
    let normal = vec2<f32>(-axis.y, axis.x);
    let separation = split * 0.49;
    let lobe_scale = mix(1.0, 0.90, split);
    let lobe_0 = segmented_cell_distance(
        (local - axis * separation) / lobe_scale, motion, shape, endpoints, profile,
        packed_0, packed_1, packed_2, packed_3, section_meta
    );
    let lobe_1 = segmented_cell_distance(
        (local + axis * separation) / lobe_scale, motion, shape, endpoints, profile,
        packed_0, packed_1, packed_2, packed_3, section_meta
    );
    let pinch = smoothstep(0.38, 1.0, mitosis);
    let neck_width = mix(0.25, 0.026, pinch);
    let neck = organic_neck_distance(
        local, axis, normal, separation, neck_width, section_meta.x * 9.0 + shape.w
    );
    let lobes = smooth_min_distance(lobe_0, lobe_1, 0.065);
    let joined = smooth_min_distance(lobes, neck, mix(0.11, 0.035, pinch));
    let contour_distance = mix(original_contour, joined, split);
    if (contour_distance > 1.0) {
        discard;
    }

    if (selection) {
        let pulse = 0.5 + 0.5 * sin(globals.time * 2.1);
        let edge = smoothstep(0.76, 1.0, contour_distance);
        let scan = 0.5 + 0.5 * sin((local.x + local.y) * 18.0 - globals.time * 1.8);
        let fill_alpha = mix(0.20, 0.27, pulse) + scan * 0.035;
        let alpha = mix(fill_alpha, 0.78, edge);
        let rgb = mix(color.rgb * 0.70, vec3<f32>(0.82, 1.0, 0.98), edge * 0.72);
        return vec4<f32>(rgb, alpha * color.a);
    }

    let membrane_gradient = smoothstep(0.72, 1.0, contour_distance);
    let membrane_edge = smoothstep(0.94, 1.0, contour_distance);
    let nucleus_separation = axis * separation * 0.76;
    let nucleus_scale = mix(1.0, 0.76, split);
    let nucleus_dist_0 = length(local - nucleus.xy - nucleus_separation) / max(nucleus.z * nucleus_scale, 0.04);
    let nucleus_dist_1 = length(local - nucleus.xy + nucleus_separation) / max(nucleus.z * nucleus_scale, 0.04);
    let nucleus_mask_0 = smoothstep(1.0, 0.82, nucleus_dist_0);
    let nucleus_mask_1 = smoothstep(1.0, 0.82, nucleus_dist_1) * split;
    let nucleus_mask = max(nucleus_mask_0, nucleus_mask_1);
    let nucleus_rim = max(
        ring_mask(nucleus_dist_0, 1.0, 0.72),
        ring_mask(nucleus_dist_1, 1.0, 0.72) * split
    );
    let body_color = color.rgb;
    let membrane_color = mix(body_color, vec3<f32>(1.0), 0.68);
    let nucleus_color = mix(body_color, vec3<f32>(1.0), 0.48);
    var rgb = mix(body_color, membrane_color, membrane_gradient);
    rgb = mix(rgb, vec3<f32>(1.0), membrane_edge * 0.16);
    rgb = mix(rgb, nucleus_color, nucleus_mask);
    rgb = mix(rgb, vec3<f32>(1.0), nucleus_rim * 0.24);
    var alpha = mix(0.38, 0.84, membrane_gradient);
    alpha = mix(alpha, 0.98, membrane_edge);
    alpha = mix(alpha, 0.90, nucleus_mask);
    return vec4<f32>(rgb, alpha * color.a);
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
        return vec4<f32>(food.rgb, 1.0);
    }

    if (kind > 1.5 && kind < 2.5) {
        let obstacle = obstacle_fragment(local, in.color, in.motion, in.shape);
        if (obstacle.a < 0.01) {
            discard;
        }
        return vec4<f32>(obstacle.rgb, 1.0);
    }

    if (kind > 2.5 && kind < 3.5) {
        let feeder_core = feeder_core_fragment(local, in.color, in.motion, in.shape);
        if (feeder_core.a < 0.01) {
            discard;
        }
        return vec4<f32>(feeder_core.rgb, 1.0);
    }

    if (kind > 3.5 && kind < 4.5) {
        let feeder_branch = feeder_branch_fragment(local, in.color, in.motion, in.shape);
        if (feeder_branch.a < 0.01) {
            discard;
        }
        return vec4<f32>(feeder_branch.rgb, 1.0);
    }

    if (kind > 4.5 && kind < 5.5) {
        let selection = selection_fragment(
            local,
            in.color,
            in.motion,
            in.shape,
            in.soft_radii_a,
            in.soft_radii_b,
            in.section_meta.w
        );
        if (selection.a < 0.01) {
            discard;
        }
        return selection;
    }

    if (kind > 5.5 && kind < 6.5) {
        return velocity_arrow_fragment(local, in.color, in.motion, in.shape);
    }

    if (kind > 6.5 && kind < 7.5) {
        return perception_fragment(local, in.color, in.shape);
    }

    if (kind > 7.5 && kind < 8.5) {
        return target_vector_fragment(local, in.color, in.motion, in.shape);
    }

    if (kind > 8.5 && kind < 9.5) {
        return cell_bridge_fragment(local, in.color, in.motion, in.shape);
    }

    if (kind > 9.5 && kind < 10.5) {
        return segmented_cell_fragment(
            local,
            in.color,
            in.nucleus,
            in.motion,
            in.shape,
            in.soft_radii_a,
            in.soft_radii_b,
            in.section_radii_0,
            in.section_radii_1,
            in.section_radii_2,
            in.section_radii_3,
            in.section_meta,
            false
        );
    }

    if (kind > 10.5 && kind < 11.5) {
        return segmented_cell_fragment(
            local,
            in.color,
            in.nucleus,
            in.motion,
            in.shape,
            in.soft_radii_a,
            in.soft_radii_b,
            in.section_radii_0,
            in.section_radii_1,
            in.section_radii_2,
            in.section_radii_3,
            in.section_meta,
            true
        );
    }

    if (kind > 11.5 && kind < 12.5) {
        return food_particle_fragment(local, in.color, in.motion, in.shape);
    }

    if (kind > 12.5 && kind < 13.5) {
        return cell_wake_fragment(local, in.color, in.motion, in.shape);
    }

    let heading_angle = atan2(in.motion.y, in.motion.x);
    let effective_dist = mitosis_contour_distance(
        local,
        heading_angle,
        in.shape,
        in.soft_radii_a,
        in.soft_radii_b,
        in.section_meta.w
    );

    if (effective_dist > 1.0) {
        discard;
    }

    let membrane_gradient = smoothstep(0.72, 1.0, effective_dist);
    let membrane_edge = smoothstep(0.94, 1.0, effective_dist);

    let forward = safe_dir(in.motion.xy);
    let sideways = vec2<f32>(-forward.y, forward.x);
    let orbit_phase = in.shape.z;
    let nucleus_orbit = vec2<f32>(
        sin(globals.time * 0.38 + orbit_phase),
        sin(globals.time * 0.29 + orbit_phase * 1.71)
    ) * 0.010;
    let impact = clamp(in.motion.z / 0.35, 0.0, 1.0);
    let collision_direction =
        forward * cos(in.motion.w)
        + sideways * sin(in.motion.w);
    let collision_wobble = collision_direction * impact * 0.052;
    let nucleus_shift = nucleus_orbit + collision_wobble;
    let mitosis = clamp(in.section_meta.w, 0.0, 1.0);
    let split = smoothstep(0.10, 0.96, mitosis);
    let mitosis_axis = sideways;
    let nucleus_separation = mitosis_axis * split * 0.42;
    let nucleus_scale = mix(1.0, 0.76, split);
    let nucleus_center = in.nucleus.xy + nucleus_shift;
    let nucleus_dist_0 = length(local - nucleus_center - nucleus_separation)
        / max(in.nucleus.z * nucleus_scale, 0.04);
    let nucleus_dist_1 = length(local - nucleus_center + nucleus_separation)
        / max(in.nucleus.z * nucleus_scale, 0.04);
    let nucleus_mask = max(
        smoothstep(1.0, 0.82, nucleus_dist_0),
        smoothstep(1.0, 0.82, nucleus_dist_1) * split
    );
    let nucleus_rim = max(
        ring_mask(nucleus_dist_0, 1.0, 0.72),
        ring_mask(nucleus_dist_1, 1.0, 0.72) * split
    );

    let body_color = in.color.rgb;
    let membrane_color = mix(body_color, vec3<f32>(1.0), 0.68);
    let nucleus_color = mix(body_color, vec3<f32>(1.0), 0.48);
    var rgb = mix(body_color, membrane_color, membrane_gradient);
    rgb = mix(rgb, vec3<f32>(1.0), membrane_edge * 0.16);
    rgb = mix(rgb, nucleus_color, nucleus_mask);
    rgb = mix(rgb, vec3<f32>(1.0), nucleus_rim * 0.24);

    var alpha = mix(0.38, 0.84, membrane_gradient);
    alpha = mix(alpha, 0.98, membrane_edge);
    alpha = mix(alpha, 0.90, nucleus_mask);
    return vec4<f32>(rgb, alpha * in.color.a);
}
