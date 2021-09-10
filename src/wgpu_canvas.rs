use crate::{AppView, SurfaceView};

pub struct WgpuCanvas {
    pub app_view: AppView,
}

#[allow(dead_code)]
impl WgpuCanvas {
    pub fn new(app_view: AppView) -> Self {
        // ...
        let texture = app_view.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: 40 * 32,
                height: 230 * 16,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING,
            label: None,
        });
        println!("tex: {:?}", texture);
        WgpuCanvas { app_view }
    }

    pub fn reset(&mut self) {}
}

impl SurfaceView for WgpuCanvas {
    fn resize(&mut self) {}

    fn enter_frame(&mut self) {
        if let Some(callback) = self.app_view.callback_to_app {
            callback(123);
        }
    }
}
