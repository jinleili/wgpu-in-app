

struct MVPMatUniform {  
    mvp: mat4x4f,
};


struct TurningUniform {
    // 开始卷动的半径
    radius: f32,
    angle: f32,
    np: vec2f,
    n: vec2f,
    alpha: f32,
    instance_index: i32,
};

@group(0) @binding(0) var<uniform> mvp_mat: MVPMatUniform;
@group(0) @binding(1) var<uniform> params: TurningUniform;

let PI: f32 = 3.14159265358979;
let PI_2: f32 = 1.57079632675;

struct VertexOutput {
    @builtin(position) position: vec4f;
    @location(0) paper_uv: vec2f;
    @location(1) brush_uv: vec2f;
    @location(2) verCoord: vec3f;
    // 卷起的高度
    @location(3) roll_height: f32;
    @location(4) instance_index: u32;
};

@vertex
fn vs_main(
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3f,
    @location(1) paper_texcoord: vec2f,
    @location(2) tex_coord: vec2f,
) -> VertexOutput {

    var result: VertexOutput;
    result.brush_uv = tex_coord;
    result.paper_uv = paper_texcoord;
    result.instance_index = instance_index;

    // 从 np 位置到 position 的矢量
    let v: vec2f = position.xy - params.np;
    // v 在单位矢量 n 上的投影长度
    let l: f32 = dot(v, params.n);
    if (instance_index == 0u) {
        result.verCoord = position;
        result.roll_height = 0.0;
        // 将底下的 paper z-index 放低一些, 避免 z fighting
        result.position = mvp_mat.mvp * vec4f(position.xy, position.z - 0.00001, 1.0);
    } else {
        // 投影长度值为正，表示 position 是需要被卷起的点
        if (l > 0.0) {
            // 半圆周长
            let half_circle: f32 = PI * params.radius;
            var new_position: vec3f = position.xyz;

            // position 卷起后与之前的位置差
            var d = 0.0;

            // 切点到 half_circle 之间的顶点计算卷起
            if (l <= half_circle) {
                // 被卷起的弧度
                let degress = (l / half_circle) * PI - PI_2;
                d = l - cos(degress) * params.radius;
                // position 卷起后的高度
                new_position.z = (params.radius + sin(degress) * params.radius);
            } else {
                d = l + (l - half_circle);
                // half_circle 之外的顶点，z 轴是固定的圆的直径
                new_position.z = params.radius * 2.0;
            }
            // new_position.z *= -1;
            new_position.y -= sin(params.angle) * d;
            new_position.x -= cos(params.angle) * d;
            result.roll_height = new_position.z;
            result.verCoord = new_position;
            result.position = mvp_mat.mvp * vec4f(new_position, 1.0);
        } else {
            result.verCoord = position;
            result.roll_height = 0.0;
            result.position = mvp_mat.mvp * vec4f(position, 1.0);
        }
    }
    return result;
}

@group(0) @binding(2) var bg_texture: texture_2d<f32>;
@group(0) @binding(3) var front_texture: texture_2d<f32>;
@group(0) @binding(4) var tex_sampler: sampler;

let whiteWeight: f32 = 0.25;
let texWeight: f32 = 0.75;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
    // 使用内置的 gl_InstanceIndex 实例索引来区分绘制，第一个实例只绘制背景纸，有翻页效果的第二个实例需要带上笔墨效果
    // let tex = select(front_texture, bg_texture, vertex.instance_index == 0u);
        // var tex_color: vec4f = textureSample(bg_texture, tex_sampler, vertex.paper_uv);
    if (params.instance_index == 0) {
        return textureSample(bg_texture, tex_sampler, vertex.paper_uv);
    } else {
        var tex_color: vec4f = textureSample(front_texture, tex_sampler, vertex.paper_uv);
        var rgb_color: vec3f = vec3f(0.0);
        if (vertex.instance_index == 1u) {
            let diameter = params.radius * 2.0;
            if (vertex.roll_height > 0.0) {
                if (vertex.roll_height > params.radius) {
                    rgb_color = tex_color.rgb * texWeight + whiteWeight;
                    if (vertex.roll_height < diameter) {
                        //模拟卷起片段的背面阴影, 卷起得越高,阴影越小
                        rgb_color *= (1.0 - 0.15 * ((diameter - vertex.roll_height) / params.radius));
                    }
                } else {
                    //高效模拟卷起片段的内面阴影, 卷起得越高,阴影越大
                    rgb_color = tex_color.rgb * (1.0 - 0.2 * (vertex.roll_height / params.radius));
                }
            } 
        }
        return vec4f(rgb_color, 1.0);
    }
    
}