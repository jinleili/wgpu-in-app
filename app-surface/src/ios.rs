use core::marker::Sync;
use libc::c_void;
use objc2::{msg_send, runtime::AnyObject};
use objc2_core_foundation::{CGFloat, CGRect};

#[repr(C)]
pub struct IOSViewObj {
    // metal_layer 所在的 UIView 容器
    // UIView 有一系列方便的函数可供我们在 Rust 端来调用
    pub view: *mut AnyObject,
    // 指向 iOS 端 CAMetalLayer 的指针
    pub metal_layer: *mut c_void,
    // 不同的 iOS 设备支持不同的屏幕刷新率，有时我们的 GPU 程序需要用到这类信息
    pub maximum_frames: i32,
    // 外部函数接口，用于给 iOS 端传递状态码
    pub callback_to_swift: extern "C" fn(arg: i32),
}

pub struct AppSurface {
    pub view: *mut AnyObject,
    pub scale_factor: f32,
    pub ctx: crate::IASDQContext,
    pub maximum_frames: i32,
    pub callback_to_app: Option<extern "C" fn(arg: i32)>,
    pub temporary_directory: &'static str,
    pub library_directory: &'static str,
}

unsafe impl Sync for AppSurface {}

impl AppSurface {
    pub fn new(obj: IOSViewObj) -> Self {
        // 初始化 Rust 日志。
        _ = env_logger::try_init();

        let scale_factor = crate::normalize_scale_factor(get_scale_factor(obj.view));
        let s: CGRect = unsafe { msg_send![obj.view, frame] };
        let physical = crate::physical_size_from_logical_size(
            s.size.width as f32,
            s.size.height as f32,
            scale_factor,
        );
        let mut surface = futures_lite::future::block_on(unsafe {
            Self::try_new(
                obj.view,
                obj.metal_layer,
                physical,
                scale_factor,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
        })
        .expect("Surface initialization failed");
        surface.callback_to_app = Some(obj.callback_to_swift);
        surface.maximum_frames = obj.maximum_frames;
        surface
    }

    /// 按宿主提供的实际像素尺寸创建 surface，初始化错误返回调用方。
    ///
    /// `physical_size`、`scale_factor` 使用宿主视图的实际值；尺寸和 `usage`
    /// 须满足 wgpu 的 surface 配置要求，配置错误由 wgpu 验证。
    ///
    /// # Errors
    /// 无法创建 surface、获取适配器或创建设备时返回错误。
    ///
    /// # Safety
    /// 必须在 UIKit 主线程创建、轮询和使用；view/layer 必须分别指向有效的
    /// UIView/CAMetalLayer，且存活到返回的 AppSurface 及其 surface 引用全部销毁。
    pub async unsafe fn try_new(
        view: *mut AnyObject,
        metal_layer: *mut c_void,
        physical_size: (u32, u32),
        scale_factor: f32,
        usage: wgpu::TextureUsages,
    ) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(metal_layer))
        }
        .map_err(|error| format!("Metal surface creation failed: {error}"))?;

        let ctx = crate::create_iasdq_context(instance, surface, physical_size, usage).await?;

        Ok(AppSurface {
            view,
            scale_factor,
            ctx,
            callback_to_app: None,
            maximum_frames: 60,
            temporary_directory: "",
            library_directory: "",
        })
    }

    pub fn get_view_size(&self) -> (u32, u32) {
        let s: CGRect = unsafe { msg_send![self.view, frame] };
        crate::physical_size_from_logical_size(
            s.size.width as f32,
            s.size.height as f32,
            self.scale_factor,
        )
    }
}

fn get_scale_factor(obj: *mut AnyObject) -> f32 {
    let scale_factor: CGFloat = unsafe { msg_send![obj, contentScaleFactor] };
    scale_factor as f32
}
