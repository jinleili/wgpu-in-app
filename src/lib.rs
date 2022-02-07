mod examples;
mod wgpu_canvas;
pub use wgpu_canvas::WgpuCanvas;

#[cfg_attr(target_os = "ios", path = "ios.rs")]
#[cfg_attr(target_os = "android", path = "android.rs", allow(non_snake_case))]
mod ffi;

#[cfg(all(target_os = "android", target_os = "ios"))]
pub use ffi::*;
