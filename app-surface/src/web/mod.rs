use crate::IASDQContext;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ops::Deref;

mod canvas;
pub use canvas::*;

mod offscreen_canvas;
pub use offscreen_canvas::*;

pub struct AppSurface {
    pub view: ViewObj,
    pub scale_factor: f32,
    pub ctx: IASDQContext,
}

impl AppSurface {
    pub async fn new(view: ViewObj) -> Self {
        let (scale_factor, logical_size) = match view {
            ViewObj::Canvas(ref canvas) => (canvas.scale_factor, canvas.logical_resolution()),
            ViewObj::Offscreen(ref offscreen) => {
                (offscreen.scale_factor, offscreen.logical_resolution())
            }
        };
        let physical_size = (
            (logical_size.0 as f32 * scale_factor) as u32,
            (logical_size.1 as f32 * scale_factor) as u32,
        );

        let backends = wgpu::Backends::BROWSER_WEBGPU;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let surface = unsafe {
            instance
                .create_surface_unsafe(match view {
                    ViewObj::Canvas(ref canvas) => wgpu::SurfaceTargetUnsafe::RawHandle {
                        raw_display_handle: canvas.display_handle().unwrap().into(),
                        raw_window_handle: canvas.window_handle().unwrap().into(),
                    },
                    ViewObj::Offscreen(ref offscreen) => wgpu::SurfaceTargetUnsafe::RawHandle {
                        raw_display_handle: offscreen.display_handle().unwrap().into(),
                        raw_window_handle: offscreen.window_handle().unwrap().into(),
                    },
                })
                .expect("Surface creation failed")
        };

        let ctx = crate::create_iasdq_context(instance, surface, physical_size).await;

        Self {
            view,
            scale_factor,
            ctx,
        }
    }

    pub fn get_view_size(&self) -> (u32, u32) {
        let (scale_factor, logical_size) = match self.view {
            ViewObj::Canvas(ref canvas) => (canvas.scale_factor, canvas.logical_resolution()),
            ViewObj::Offscreen(ref offscreen) => {
                (offscreen.scale_factor, offscreen.logical_resolution())
            }
        };
        (
            (logical_size.0 as f32 * scale_factor) as u32,
            (logical_size.1 as f32 * scale_factor) as u32,
        )
    }
}

/// 用 Canvas id 创建 AppSurface
///
/// element_id: 存在于当前页面中的 canvas 元素的 id
/// handle: 用于 WebGPU 的 raw handle number, 0 是保留的值, 不能使用
pub async fn app_surface_from_canvas(element_id: &str, handle: u32) -> AppSurface {
    let wrapper = CanvasWrapper::new(Canvas::new(element_id, handle));
    AppSurface::new(ViewObj::Canvas(wrapper)).await
}

// 封装 ViewObj 来同时支持 Canvas 与 Offscreen
#[derive(Debug)]
pub enum ViewObj {
    Canvas(CanvasWrapper),
    Offscreen(OffscreenCanvasWrapper),
}

impl ViewObj {
    pub fn from_canvas(canvas: Canvas) -> Self {
        ViewObj::Canvas(CanvasWrapper::new(canvas))
    }

    pub fn from_offscreen_canvas(canvas: OffscreenCanvas) -> Self {
        ViewObj::Offscreen(OffscreenCanvasWrapper::new(canvas))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SendSyncWrapper<T>(pub(crate) T);

unsafe impl<T> Send for SendSyncWrapper<T> {}
unsafe impl<T> Sync for SendSyncWrapper<T> {}

impl Deref for AppSurface {
    type Target = IASDQContext;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl crate::SurfaceFrame for AppSurface {
    fn view_size(&self) -> crate::ViewSize {
        let size = self.get_view_size();
        crate::ViewSize {
            width: size.0,
            height: size.1,
        }
    }

    fn resize_surface(&mut self) {
        let size = self.get_view_size();
        self.ctx.config.width = size.0;
        self.ctx.config.height = size.1;
        self.surface.configure(&self.device, &self.config);
    }

    fn resize_surface_by_size(&mut self, size: (u32, u32)) {
        self.ctx.config.width = size.0;
        self.ctx.config.height = size.1;
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
