mod examples;
mod wgpu_canvas;
pub use wgpu_canvas::WgpuCanvas;

#[cfg(target_os = "ios")]
#[path = "ios_ffi.rs"]
mod ffi;
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[path = "android_ffi.rs"]
mod ffi;
#[cfg(all(target_os = "android", target_os = "ios"))]
pub use ffi::*;
