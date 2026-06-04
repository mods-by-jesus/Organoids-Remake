#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::globals,
}

struct LiquidMediumParams {
    deep_color: vec4<f32>,
    caustic_color: vec4<f32>,
    arena_size: vec4<f32>,
    flow: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: LiquidMediumParams;

fn rotate2d(angle: f32) -> mat2x2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat2x2<f32>(
        vec2<f32>(c, s),
        vec2<f32>(-s, c)
    );
}

fn soft_wave(p: vec2<f32>, time: f32) -> f32 {
    let a = sin(p.x * 1.7 + sin(p.y * 0.9 + time * 0.7) + time);
    let b = sin((p.x + p.y) * 1.15 - time * 0.63);
    let offset = vec2<f32>(sin(time * 0.2), cos(time * 0.17));
    let c = cos(length(p + offset) * 2.2 - time * 0.5);
    return (a + b + c) * 0.3333;
}

fn smooth_fluid(p: vec2<f32>, time: f32) -> f32 {
    var value = 0.0;
    var amplitude = 0.55;
    var frequency = 1.0;

    for (var i = 0; i < 5; i = i + 1) {
        let fi = f32(i);
        let q = rotate2d(fi * 0.73) * p * frequency;
        value += soft_wave(q, time * (0.45 + fi * 0.11)) * amplitude;
        frequency *= 1.72;
        amplitude *= 0.52;
    }

    return value * 0.5 + 0.5;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let world_pos = in.world_position.xy;
    let flow_scale = material.flow.x;
    let flow_speed = material.flow.y;
    let caustic_strength = material.flow.z;
    let vignette_strength = material.flow.w;
    let time = globals.time * flow_speed * 8.0;

    let p = world_pos / flow_scale;
    let drift = vec2<f32>(sin(time * 0.23), cos(time * 0.19)) * 0.35;
    var warped = p + drift;
    warped += vec2<f32>(
        soft_wave(p * 1.4, time * 0.4),
        soft_wave(rotate2d(1.57) * p * 1.2, time * 0.36)
    ) * 0.22;

    let slow_clouds = smooth_fluid(warped, time);
    let fine_caustics = smooth_fluid(warped * 3.1 + vec2<f32>(time * 0.18, -time * 0.12), time * 1.4);
    let strands = pow(smoothstep(0.58, 0.92, fine_caustics), 2.4);
    let depth = smoothstep(0.1, 0.9, slow_clouds);

    let half_size = max(material.arena_size.xy * 0.5, vec2<f32>(1.0, 1.0));
    let arena_uv = clamp(world_pos / half_size, vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0));
    let vignette = smoothstep(1.15, 0.15, length(arena_uv));

    var color = mix(material.deep_color.rgb * 0.7, material.deep_color.rgb, depth);
    color += material.caustic_color.rgb * strands * caustic_strength * vignette;
    color *= mix(1.0 - vignette_strength, 1.0, vignette);

    return vec4<f32>(color, 1.0);
}
