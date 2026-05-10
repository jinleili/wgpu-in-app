use crate::IASDQContext;

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
        let scale_factor = view.scale_factor();
        let physical_size = view.physical_resolution();
        let backends = wgpu::Backends::BROWSER_WEBGPU;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let target = unsafe {
            match view {
                ViewObj::Canvas(ref canvas) => {
                    wgpu::SurfaceTargetUnsafe::from_display_and_window(canvas, canvas)
                }
                ViewObj::Offscreen(ref offscreen) => {
                    wgpu::SurfaceTargetUnsafe::from_display_and_window(offscreen, offscreen)
                }
            }
            .expect("Surface raw handle creation failed")
        };
        let surface = unsafe {
            instance
                .create_surface_unsafe(target)
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
        self.view.physical_resolution()
    }

    pub fn get_view_logical_size(&self) -> (f32, f32) {
        crate::logical_size_from_physical_size(
            self.view.physical_resolution(),
            self.view.scale_factor(),
        )
    }

    pub fn update_device_pixel_ratio(&mut self, ratio: f32) {
        match self.view {
            ViewObj::Canvas(ref mut canvas) => {
                canvas.scale_factor = crate::normalize_scale_factor(ratio);
            }
            ViewObj::Offscreen(ref mut offscreen) => {
                offscreen.scale_factor = crate::normalize_scale_factor(ratio);
            }
        }
        self.scale_factor = crate::normalize_scale_factor(ratio);
        if crate::resize_surface_config(&mut self.ctx.config, self.view.physical_resolution()) {
            self.ctx
                .surface
                .configure(&self.ctx.device, &self.ctx.config);
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

    fn scale_factor(&self) -> f32 {
        let scale_factor = match self {
            ViewObj::Canvas(canvas) => canvas.scale_factor,
            ViewObj::Offscreen(offscreen) => offscreen.scale_factor,
        };
        crate::normalize_scale_factor(scale_factor)
    }

    fn physical_resolution(&self) -> (u32, u32) {
        let physical_size = match self {
            ViewObj::Canvas(canvas) => canvas.physical_resolution(),
            ViewObj::Offscreen(offscreen) => offscreen.physical_resolution(),
        };
        crate::normalize_view_size(physical_size)
    }
}

#[derive(Clone, Debug)]
pub struct SendSyncWrapper<T>(pub T);

unsafe impl<T> Send for SendSyncWrapper<T> {}
unsafe impl<T> Sync for SendSyncWrapper<T> {}
