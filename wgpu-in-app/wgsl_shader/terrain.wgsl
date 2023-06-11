struct Uniforms {
    projection_view: mat4x4f,
    clipping_plane: vec4f,
};

@group(0)
@binding(0)
var<uniform> uniforms: Uniforms;

const light = vec3f(150.0, 70.0, 0.0);
const light_colour = vec3f(1.0, 0.98, 0.82);
const ambient = 0.2;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) colour: vec4f,
    // Comment this out if using user-clipping planes:
    @location(1) clip_dist: f32,
};

@vertex
fn vs_main(
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) colour: vec4f,
) -> VertexOutput {
    var result: VertexOutput;
    result.position = uniforms.projection_view * vec4f(position, 1.0);

    // https://www.desmos.com/calculator/nqgyaf8uvo
    let normalized_light_direction = normalize(position - light);
    let brightness_diffuse = clamp(dot(normalized_light_direction, normal), 0.2, 1.0);

    result.colour = vec4f(max((brightness_diffuse + ambient) * light_colour * colour.rgb, vec3f(0.0, 0.0, 0.0)), colour.a);
    result.clip_dist = dot(vec4f(position, 1.0), uniforms.clipping_plane);
    return result;
}

@fragment
fn fs_main(
    vertex: VertexOutput,
) -> @location(0) vec4f {
    // Comment this out if using user-clipping planes:
    if(vertex.clip_dist < 0.0) {
        discard;
    }

    return vec4f(vertex.colour.xyz, 1.0);
}
