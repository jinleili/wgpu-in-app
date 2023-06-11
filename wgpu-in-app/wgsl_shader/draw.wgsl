@vertex
fn main_vs(
    @location(0) particle_pos: vec2f,
    @location(1) particle_vel: vec2f,
    @location(2) position: vec2f,
) -> @builtin(position) vec4f {
    let angle = -atan2(particle_vel.x, particle_vel.y);
    let pos = vec2f(
        position.x * cos(angle) - position.y * sin(angle),
        position.x * sin(angle) + position.y * cos(angle)
    );
    return vec4f(pos + particle_pos, 0.0, 1.0);
}

@fragment
fn main_fs() -> @location(0) vec4f {
    return vec4f(1.0, 1.0, 1.0, 1.0);
}
