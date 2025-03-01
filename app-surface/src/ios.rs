use core::marker::Sync;
use core_graphics_types::{base::CGFloat, geometry::CGRect};
use libc::c_void;
use objc::{runtime::Object, *};

#[repr(C)]
pub struct IOSViewObj {
    // metal_layer 所在的 UIView 容器
    // UIView 有一系列方便的函数可供我们在 Rust 端来调用
    pub view: *mut Object,
    // 指向 iOS 端 CAMetalLayer 的指针
    pub metal_layer: *mut c_void,
    // 不同的 iOS 设备支持不同的屏幕刷新率，有时我们的 GPU 程序需要用到这类信息
    pub maximum_frames: i32,
    // 外部函数接口，用于给 iOS 端传递状态码
    pub callback_to_swift: extern "C" fn(arg: i32),
}

pub struct AppSurface {
    pub view: *mut Object,
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
        // hook up rust logging
        _ = env_logger::try_init();

        let scale_factor = get_scale_factor(obj.view);
        let s: CGRect = unsafe { msg_send![obj.view, frame] };
        let physical = (
            (s.size.width as f32 * scale_factor) as u32,
            (s.size.height as f32 * scale_factor) as u32,
        );
        let backends = wgpu::Backends::METAL;
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let surface = unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                    obj.metal_layer,
                ))
                .expect("Surface creation failed")
        };

        let ctx = pollster::block_on(crate::create_iasdq_context(instance, surface, physical));

        AppSurface {
            view: obj.view,
            scale_factor,
            ctx,
            callback_to_app: Some(obj.callback_to_swift),
            maximum_frames: obj.maximum_frames,
            temporary_directory: "",
            library_directory: "",
        }
    }

    pub fn get_view_size(&self) -> (u32, u32) {
        let s: CGRect = unsafe { msg_send![self.view, frame] };
        (
            (s.size.width as f32 * self.scale_factor) as u32,
            (s.size.height as f32 * self.scale_factor) as u32,
        )
    }
}

fn get_scale_factor(obj: *mut Object) -> f32 {
    let mut _scale_factor: CGFloat = 1.0;

    _scale_factor = unsafe { msg_send![obj, contentScaleFactor] };

    _scale_factor as f32
}
