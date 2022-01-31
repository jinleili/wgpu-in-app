pub trait Example {
    fn resize(&mut self, _app_surface: &crate::AppSurface) {}
    fn enter_frame(&mut self, app_surface: &crate::AppSurface);
}

mod boids;
pub use boids::Boids;

mod msaa_line;
pub use msaa_line::MSAALine;

mod cube;
pub use cube::Cube;

mod point_gen;
mod water;
pub use water::Water;

mod shadow;
pub use shadow::Shadow;

// copy from wgpu's example
#[rustfmt::skip]
#[allow(unused)]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
