use log::error;
mod examples;
mod wgpu_canvas;

#[cfg(target_os = "ios")]
#[path = "ios/ffi.rs"]
mod ffi;
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[path = "android/ffi.rs"]
mod ffi;
#[cfg(all(target_os = "android", target_os = "ios"))]
pub use ffi::*;

#[cfg(not(target_os = "android"))]
#[path = "ios/app_view.rs"]
pub mod app_view;
#[cfg(target_os = "android")]
#[path = "android/app_view.rs"]
pub mod app_view;
use app_view::AppView;

#[repr(C)]
#[derive(Debug)]
pub struct ViewSize {
    pub width: u32,
    pub height: u32,
}

pub trait GPUContext {
    fn resize_surface(&mut self);
    fn get_current_frame_view(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView);
    fn create_current_frame_view(
        &self,
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                surface.configure(&device, &config);
                surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture!")
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // frame cannot be drop early
        (frame, view)
    }
}

async fn request_device(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface,
) -> (wgpu::Device, wgpu::Queue) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();
    let request_features = if cfg!(target_os = "android") {
        // unsupported features on some android: POLYGON_MODE_LINE | VERTEX_WRITABLE_STORAGE
        wgpu::Features::default()
    } else {
        wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
            | wgpu::Features::POLYGON_MODE_LINE
            | wgpu::Features::VERTEX_WRITABLE_STORAGE
    };

    // These downlevel limits will allow the code to run on all possible hardware
    let downlevel_limits = wgpu::Limits::downlevel_webgl2_defaults();
    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the surface.
    let needed_limits = downlevel_limits.using_resolution(adapter.limits());
    let res = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: request_features,
                limits: needed_limits,
            },
            None,
        )
        .await;
    match res {
        Err(err) => {
            error!("request_device failed: {:?}", err);
            panic!("request_device failed: {:?}", err);
        }
        Ok(tuple) => tuple,
    }
}
