struct VertexOutput {
    @location(0) tex_coord: vec2f,
    @builtin(position) position: vec4f,
};

struct Locals {
    transform: mat4x4f
};
@group(0)
@binding(0)
var<uniform> r_locals: Locals;

@vertex
fn vs_main(
    @location(0) position: vec4f,
    @location(1) tex_coord: vec2f,
) -> VertexOutput {
    var result: VertexOutput;
    result.tex_coord = tex_coord;
    result.position = r_locals.transform * position;
    return result;
}

@group(0)
@binding(1)
var r_color: texture_2d<u32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
    let tex = textureLoad(r_color, vec2<i32>(vertex.tex_coord * 256.0), 0);
    let v = f32(tex.x) / 255.0;
    return vec4f(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);
}

@fragment
fn fs_wire(vertex: VertexOutput) -> @location(0) vec4f {
    return vec4f(0.0, 0.5, 0.0, 0.5);
}
