struct VertexOutput {
    @location(0) color: vec4f,
    @builtin(position) position: vec4f,
};

@vertex
fn vs_main(
    @location(0) position: vec2f,
    @location(1) color: vec4f,
) -> VertexOutput {
    var result: VertexOutput;
    result.position = vec4f(position, 0.0, 1.0);
    result.color = color;
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
    return vertex.color;
}
