struct VertexOutput {
    @location(0) uv: vec2f,
    @builtin(position) position: vec4f,
};

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VertexOutput {
    let uv: vec2f = vec2f(f32((vertexIndex << 1u) & 2u), f32(vertexIndex & 2u));
    var result: VertexOutput;
    result.position = vec4f(uv * 2.0 - 1.0, 0.0, 1.0);
    // invert uv.y
    result.uv = vec2f(uv.x, (uv.y - 1.0) *  (-1.0));
    return result;
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
    return textureSample(tex, tex_sampler, vertex.uv);
}