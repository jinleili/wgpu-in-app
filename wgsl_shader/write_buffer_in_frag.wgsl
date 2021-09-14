struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn main([[builtin(vertex_index)]] vertexIndex: u32) -> VertexOutput {
    let uv: vec2<f32> = vec2<f32>(f32((vertexIndex << 1u) & 2u), f32(vertexIndex & 2u));
    var out: VertexOutput;
    out.position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, (uv.y - 1.0) *  (-1.0));
    return out;
}

[[block]]
struct TestBuffer {
    data: [[stride(4)]] array<f32>;
};
[[group(0), binding(1)]] var<storage, read_write> buf: TestBuffer;

[[stage(fragment)]] 
fn main([[builtin(position)]] coord : vec4<f32>) -> [[location(0)]] vec4<f32> {
    let index = i32(coord.y) * 1000 + i32(coord.x);
    buf.data[index] = 1.0;
    
    return vec4<f32><1.0>;
}