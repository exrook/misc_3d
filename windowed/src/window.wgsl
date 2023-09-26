@group(0) @binding(0)
var t: texture_2d<f32>;

var<push_constant> dd: DispatchData;

struct DispatchData {
    mode: i32,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32((in_vertex_index & 2u) >> 1u)) * 2.0 - 1.0;
    let y = f32(i32(in_vertex_index & 1u)) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) coord_in: vec4<f32>) -> @location(0) vec4<f32> {
    let tex_coord = vec2<u32>(coord_in.xy) % textureDimensions(t);
    let color = textureLoad(t, tex_coord, 0);
    return vec4<f32>(color);
}
