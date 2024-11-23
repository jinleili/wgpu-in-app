use crate::IASDQContext;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

mod canvas;
pub use canvas::*;

mod offscreen_canvas;
pub use offscreen_canvas::*;

pub struct AppSurface {
    pub view: ViewObj,
    pub scale_factor: f32,
    pub ctx: IASDQContext,
}

#[allow(dead_code)]
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

    /// 用 Canvas id 创建 AppSurface
    ///
    /// element_id: 存在于当前页面中的 canvas 元素的 id
    /// handle: 用于 WebGPU 的 raw handle number, 0 是保留的值, 不能使用
    pub async fn from_canvas(element_id: &str, handle: u32) -> Self {
        let wrapper = CanvasWrapper::new(Canvas::new(element_id, handle));
        Self::new(ViewObj::Canvas(wrapper)).await
    }

    /// 用 OffscreenCanvas 创建 AppSurface
    ///
    /// handle: 用于 WebGPU 的 raw handle number, 0 是保留的值, 不能使用
    pub async fn from_offscreen_canvas(
        offscreen_canvas: web_sys::OffscreenCanvas,
        scale_factor: f32,
        handle: u32,
    ) -> Self {
        let wrapper = OffscreenCanvasWrapper::new(OffscreenCanvas::new(
            offscreen_canvas,
            scale_factor,
            handle,
        ));
        Self::new(ViewObj::Offscreen(wrapper)).await
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

    pub fn get_view_logical_size(&self) -> (u32, u32) {
        let (_, logical_size) = match self.view {
            ViewObj::Canvas(ref canvas) => (canvas.scale_factor, canvas.logical_resolution()),
            ViewObj::Offscreen(ref offscreen) => {
                (offscreen.scale_factor, offscreen.logical_resolution())
            }
        };
        logical_size
    }

    pub fn update_device_pixel_ratio(&mut self, ratio: f32) {
        match self.view {
            ViewObj::Canvas(ref mut canvas) => canvas.scale_factor = ratio,
            ViewObj::Offscreen(ref mut offscreen) => offscreen.scale_factor = ratio,
        }
    }
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
