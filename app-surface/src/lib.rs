use std::{ops::Deref, sync::Arc};

mod touch;
pub use touch::*;

#[cfg_attr(target_os = "ios", path = "ios.rs")]
#[cfg_attr(target_os = "android", path = "android.rs")]
mod app_surface;

pub use app_surface::*;

pub struct SurfaceDeviceQueue {
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl SurfaceDeviceQueue {
    pub fn update_config_format(&mut self, format: wgpu::TextureFormat) {
        self.config.format = format;
        self.surface.configure(&self.device, &self.config);
    }
}

impl std::ops::Deref for AppSurface {
    type Target = SurfaceDeviceQueue;
    fn deref(&self) -> &Self::Target {
        &self.sdq
    }
}

pub trait Frame {
    // After App view's size or orientation changed, need to resize surface.
    fn resize_surface(&mut self);
    fn pintch(&mut self, _touch: Touch, _scale: f32) {}
    fn touch(&mut self, _touch: Touch) {}
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
        (frame, view)
    }
}

impl Frame for AppSurface {
    fn resize_surface(&mut self) {
        let size = self.get_view_size();
        self.sdq.config.width = size.0;
        self.sdq.config.height = size.1;
        self.surface.configure(&self.device, &self.config);
    }

    fn get_current_frame_view(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        self.create_current_frame_view(&self.device, &self.surface, &self.config)
    }
}

async fn request_device(
    instance: &wgpu::Instance,
    backend: wgpu::Backends,
    surface: &wgpu::Surface,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter =
        wgpu::util::initialize_adapter_from_env_or_default(instance, backend, Some(surface))
            .await
            .expect("No suitable GPU adapters found on the system!");
    let adapter_info = adapter.get_info();
    println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    // unsupported features on some android: POLYGON_MODE_LINE | VERTEX_WRITABLE_STORAGE
    let mut request_features = wgpu::Features::empty();
    for f in [
        wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
        wgpu::Features::POLYGON_MODE_LINE,
        wgpu::Features::VERTEX_WRITABLE_STORAGE,
        wgpu::Features::TEXTURE_COMPRESSION_ASTC_HDR,
    ] {
        if adapter.features().contains(f) {
            request_features |= f;
        }
    }

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
        Ok(tuple) => (adapter, tuple.0, tuple.1),
    }
}
