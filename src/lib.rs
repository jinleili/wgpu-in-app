mod examples;
mod wgpu_canvas;
pub use wgpu_canvas::WgpuCanvas;

#[cfg(target_os = "ios")]
#[path = "ios/ffi.rs"]
mod ffi;
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[path = "android/ffi.rs"]
mod ffi;
#[cfg(all(target_os = "android", target_os = "ios"))]
pub use ffi::*;

#[cfg(target_os = "ios")]
#[path = "ios/app_surface.rs"]
mod app_surface;
#[cfg(target_os = "android")]
#[path = "android/app_surface.rs"]
mod app_surface;
#[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
#[path = "app_surface.rs"]
mod app_surface;

pub use app_surface::AppSurface;

impl std::ops::Deref for AppSurface {
    type Target = wgpu::Queue;
    fn deref(&self) -> &Self::Target {
        &self.queue
    }
}

pub trait FrameContext {
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

impl FrameContext for AppSurface {
    fn resize_surface(&mut self) {
        let size = self.get_view_size();
        self.config.width = size.0;
        self.config.height = size.1;
        self.surface.configure(&self.device, &self.config);
    }

    fn get_current_frame_view(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        self.create_current_frame_view(&self.device, &self.surface, &self.config)
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

    // // These downlevel limits will allow the code to run on all possible hardware
    // let downlevel_limits = wgpu::Limits::downlevel_webgl2_defaults();
    // // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the surface.
    // let needed_limits = downlevel_limits.using_resolution(adapter.limits());
    let res = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: request_features,
                limits: adapter.limits(),
            },
            None,
        )
        .await;
    match res {
        Err(err) => {
            panic!("request_device failed: {:?}", err);
        }
        Ok(tuple) => tuple,
    }
}
