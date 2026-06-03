#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) i_pos_radius: vec4<f32>,
    @location(4) i_color: vec4<f32>,
    @location(5) i_nucleus: vec4<f32>,
    @location(6) i_motion: vec4<f32>,
    @location(7) i_shape: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) nucleus: vec4<f32>,
    @location(3) motion: vec4<f32>,
    @location(4) shape: vec4<f32>,
};

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
    return out;
}

fn ring_mask(dist: f32, outer: f32, inner: f32) -> f32 {
    return smoothstep(outer, outer - 0.035, dist) * (1.0 - smoothstep(inner, inner - 0.08, dist));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let local = in.local_pos;
    let dist = length(local);
    let kind = in.nucleus.w;

    if (kind < 0.5) {
        if (dist > 1.0) {
            discard;
        }

        let food_alpha = smoothstep(1.0, 0.5, dist) * in.color.a;
        let food_core = 0.55 + (1.0 - dist) * 0.55;
        return vec4<f32>(in.color.rgb * food_core, food_alpha);
    }

    let angle = atan2(local.y, local.x);
    let shape_radius = radial_shape_radius(angle, in.shape);
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
