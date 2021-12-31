use std::env;
use std::ops::Deref;
use std::path::PathBuf;

mod wgpu_canvas;

mod ffi;
pub use ffi::*;

#[repr(C)]
#[derive(Debug)]
pub struct ViewSize {
    pub width: u32,
    pub height: u32,
}

mod ios_view;
pub use ios_view::{AppView, IOSViewObj};

pub struct AppViewWrapper(pub AppView);
// `*mut libc::c_void` cannot be sent between threads safely
unsafe impl Send for AppViewWrapper {}
unsafe impl Sync for AppViewWrapper {}

impl Deref for AppViewWrapper {
    type Target = AppView;

    fn deref(&self) -> &AppView {
        &self.0
    }
}

pub trait GPUContext {
    fn set_view_size(&mut self, _size: (f64, f64)) {}
    fn get_view_size(&self) -> ViewSize;
    fn resize_surface(&mut self);
    fn normalize_touch_point(&self, touch_point_x: f32, touch_point_y: f32) -> (f32, f32);
    fn get_current_frame_view(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView);
    fn create_current_frame_view(
        &self, device: &wgpu::Device, surface: &wgpu::Surface, config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                surface.configure(&device, &config);
                surface.get_current_texture().expect("Failed to acquire next swap chain texture!")
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // frame cannot be drop early
        (frame, view)
    }
}

#[cfg(target_arch = "wasm32")]
pub fn application_root_dir() -> String {
    let host = web_sys::window().unwrap().location().host().unwrap();
    "http://".to_string() + &host
}

#[cfg(not(target_arch = "wasm32"))]
pub fn application_root_dir() -> String {
    match env::var("PROFILE") {
        Ok(_) => String::from(env!("CARGO_MANIFEST_DIR")),
        Err(_) => {
            let mut path = env::current_exe().expect("Failed to find executable path.");
            while let Ok(target) = std::fs::read_link(path.clone()) {
                path = target;
            }
            if cfg!(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux"
            )) {
                path = path.join("../../../").canonicalize().unwrap();
            }
            String::from(
                path.parent()
                    .expect("Failed to get parent directory of the executable.")
                    .to_str()
                    .unwrap(),
            )
        }
    }
}

use lazy_static::*;
use objc::{
    rc::StrongPtr,
    runtime::{Class, Object},
    *,
};
use objc_foundation::{INSString, NSString};

lazy_static! {
    static ref BUNDLE_PATH: &'static str = get_bundle_url();
}

fn get_bundle_url() -> &'static str {
    let cls = class!(NSBundle);
    let path: &str = unsafe {
        // Allocate an instance
        let bundle: *mut Object = msg_send![cls, mainBundle];
        // let url: *mut Object = msg_send![*bundle, resourcePath];
        // 资源路径要用 resourcePath
        let path: &NSString = msg_send![bundle, resourcePath];
        path.as_str()
    };
    path
}

pub fn get_wgsl_path(name: &str) -> PathBuf {
    let base_dir = application_root_dir();
    let p = get_bundle_url().to_string() + "/wgsl_shader/" + name;
    PathBuf::from(&p)
}
