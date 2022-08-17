use std::{ops::Deref, sync::Arc};

pub mod math;
mod touch;
pub use touch::*;

#[cfg_attr(target_os = "ios", path = "ios.rs")]
#[cfg_attr(target_os = "android", path = "android.rs")]
mod app_surface;
pub use app_surface::*;

pub mod fs;

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

impl Deref for AppSurface {
    type Target = SurfaceDeviceQueue;
    fn deref(&self) -> &Self::Target {
        &self.sdq
    }
}

pub trait SurfaceFrame {
    // After App view's size or orientation changed, need to resize surface.
    fn resize_surface(&mut self);
    fn pintch(&mut self, _touch: Touch, _scale: f32) {}
    fn touch(&mut self, _touch: Touch) {}
    fn enter_frame(&mut self) {}
    fn get_current_frame_view(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        unimplemented!()
    }
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
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("frame texture view"),
            ..Default::default()
        });
        (frame, view)
    }
}

impl SurfaceFrame for AppSurface {
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
    let base_dir = std::env::var("CARGO_MANIFEST_DIR");
    let trace_path = if let Ok(base_dir) = base_dir {
        Some(std::path::PathBuf::from(&base_dir).join("WGPU_TRACE_ERROR"))
    } else {
        None
    };
    let res = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: adapter.features(),
                limits: adapter.limits(),
            },
            if trace_path.is_some() {
                Some(trace_path.as_ref().unwrap().as_path())
            } else {
                None
            },
        )
        .await;
    match res {
        Err(err) => {
            panic!("request_device failed: {:?}", err);
        }
        Ok(tuple) => (adapter, tuple.0, tuple.1),
    }
}
