use std::sync::Arc;

#[cfg(target_os = "ios")]
#[path = "ios_surface.rs"]
pub mod app_surface;
#[cfg(target_os = "android")]
#[path = "android_surface.rs"]
mod app_surface;
#[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
#[path = "app_surface.rs"]
mod app_surface;

pub use app_surface::AppSurface;

pub struct SurfaceDeviceQueue {
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl std::ops::Deref for AppSurface {
    type Target = SurfaceDeviceQueue;
    fn deref(&self) -> &Self::Target {
        &self.sdq
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
    surface: &wgpu::Surface,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
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
