use crate::{AppView, SurfaceView};

pub struct WgpuCanvas {
    pub app_view: AppView,
}

#[allow(dead_code)]
impl WgpuCanvas {
    pub fn new(app_view: AppView) -> Self {
        // let shader_path = crate::get_wgsl_path("write_buffer_in_frag.wgsl");
        let crash_texture = app_view.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: 100,
                height: 100,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            label: None,
        });

        let instance = WgpuCanvas { app_view };
        if let Some(callback) = instance.app_view.callback_to_app {
            callback(0);
        }
        instance
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
