use std::{ops::Deref, sync::Arc};
use wgpu::{Instance, Surface};

mod touch;
pub use touch::*;

#[cfg_attr(target_os = "ios", path = "ios.rs")]
#[cfg_attr(target_os = "android", path = "android.rs")]
#[cfg_attr(all(feature = "mac_catalyst", target_os = "macos"), path = "ios.rs")]
mod app_surface;
pub use app_surface::*;

#[repr(C)]
#[derive(Debug)]
pub struct ViewSize {
    pub width: u32,
    pub height: u32,
}

pub struct SurfaceDeviceQueue {
    pub surface: Arc<wgpu::Surface<'static>>,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter: Arc<wgpu::Adapter>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl SurfaceDeviceQueue {
    pub fn update_config_format(&mut self, format: wgpu::TextureFormat) {
        self.config.format = format;
        if cfg!(feature = "webgl") {
            // webgl 后端不支持 view_formats
        } else {
            self.config.view_formats = vec![format.add_srgb_suffix(), format.remove_srgb_suffix()];
        }
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
    fn view_size(&self) -> ViewSize;
    // After App view's size or orientation changed, need to resize surface.
    fn resize_surface(&mut self);
    #[cfg(target_arch = "wasm32")]
    fn resize_surface_by_size(&mut self, size: (u32, u32));
    fn pintch(&mut self, _touch: Touch, _scale: f32) {}
    fn touch(&mut self, _touch: Touch) {}
    fn normalize_touch_point(&self, _touch_point_x: f32, _touch_point_y: f32) -> (f32, f32) {
        unimplemented!()
    }
    fn enter_frame(&mut self) {}
    fn get_current_frame_view(
        &self,
        _view_format: Option<wgpu::TextureFormat>,
    ) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        unimplemented!()
    }
    fn create_current_frame_view(
        &self,
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        config: &wgpu::SurfaceConfiguration,
        view_format: Option<wgpu::TextureFormat>,
    ) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                surface.configure(device, config);
                surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture!")
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("frame texture view"),
            format: if view_format.is_none() {
                // frame buffer's view format prefer to use sRGB.
                Some(config.format.add_srgb_suffix())
            } else {
                view_format
            },
            ..Default::default()
        });
        (frame, view)
    }
}

impl SurfaceFrame for AppSurface {
    fn view_size(&self) -> ViewSize {
        let size = self.get_view_size();
        ViewSize {
            width: size.0,
            height: size.1,
        }
    }

    fn resize_surface(&mut self) {
        let size = self.get_view_size();
        self.sdq.config.width = size.0;
        self.sdq.config.height = size.1;
        self.surface.configure(&self.device, &self.config);
    }

    #[cfg(target_arch = "wasm32")]
    fn resize_surface_by_size(&mut self, size: (u32, u32)) {
        self.sdq.config.width = size.0;
        self.sdq.config.height = size.1;
        self.surface.configure(&self.device, &self.config);
    }

    fn normalize_touch_point(&self, touch_point_x: f32, touch_point_y: f32) -> (f32, f32) {
        let size = self.get_view_size();
        (
            touch_point_x * self.scale_factor / size.0 as f32,
            touch_point_y * self.scale_factor / size.1 as f32,
        )
    }

    fn get_current_frame_view(
        &self,
        view_format: Option<wgpu::TextureFormat>,
    ) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        self.create_current_frame_view(&self.device, &self.surface, &self.config, view_format)
    }
}

#[allow(unused)]
async fn request_device(
    instance: &Instance,
    surface: &Surface<'static>,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::util::power_preference_from_env()
                .unwrap_or(wgpu::PowerPreference::HighPerformance),
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        })
        .await
        .expect("No suitable GPU adapters found on the system!");
    let adapter_info = adapter.get_info();
    println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    let base_dir = std::env::var("CARGO_MANIFEST_DIR");
    let _trace_path = if let Ok(base_dir) = base_dir {
        Some(std::path::PathBuf::from(&base_dir).join("WGPU_TRACE_ERROR"))
    } else {
        None
    };
    let res = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: adapter.features(),
                required_limits: adapter.limits(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await;
    match res {
        Err(err) => {
            panic!("request_device failed: {err:?}");
        }
        Ok(tuple) => (adapter, tuple.0, tuple.1),
    }
}
