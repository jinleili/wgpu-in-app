use core_graphics_min::{CGFloat, CGRect};
use libc::c_void;
use objc::{runtime::Object, *};
use std::marker::Sync;
use std::sync::Arc;

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
    pub sdq: crate::SurfaceDeviceQueue,
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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
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
        let (adapter, device, queue) =
            pollster::block_on(crate::request_device(&instance, &surface));
        let caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // CAMatalLayer's pixel format default value is MTLPixelFormatBGRA8Unorm.
            // https://developer.apple.com/documentation/quartzcore/cametallayer/1478155-pixelformat?language=objc
            // format: wgpu::TextureFormat::Bgra8Unorm,
            // format: surface.get_supported_formats(&adapter)[0],
            format: caps.formats[0],
            width: physical.0,
            height: physical.1,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        AppSurface {
            view: obj.view,
            scale_factor,
            sdq: crate::SurfaceDeviceQueue {
                surface: Arc::new(surface),
                config,
                adapter: Arc::new(adapter),
                device: Arc::new(device),
                queue: Arc::new(queue),
            },
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
    #[cfg(target_os = "macos")]
    unsafe {
        let window: *mut Object = msg_send![obj, window];
        if !window.is_null() {
            _scale_factor = msg_send![window, backingScaleFactor];
        }
    };

    #[cfg(target_os = "ios")]
    {
        _scale_factor = unsafe { msg_send![obj, contentScaleFactor] };
    }

    _scale_factor as f32
}
