use core_graphics::{base::CGFloat, geometry::CGRect};
use libc::c_void;
use objc::{runtime::Object, *};
use std::marker::Sync;
use std::sync::Arc;

#[repr(C)]
pub struct IOSViewObj {
    pub view: *mut Object,
    pub metal_layer: *mut c_void,
    pub maximum_frames: i32,
    pub callback_to_swift: extern "C" fn(arg: i32),
}

pub struct AppSurface {
    pub view: *mut Object,
    pub scale_factor: f32,
    pub sdq: crate::SurfaceDeviceQueue,
    pub maximum_frames: i32,
    pub callback_to_app: Option<extern "C" fn(arg: i32)>,
}

unsafe impl Sync for AppSurface {}

impl AppSurface {
    pub fn new(obj: IOSViewObj) -> Self {
        // hook up rust logging
        env_logger::init();

        let scale_factor = get_scale_factor(obj.view);
        let s: CGRect = unsafe { msg_send![obj.view, frame] };
        let physical = (
            (s.size.width as f32 * scale_factor) as u32,
            (s.size.height as f32 * scale_factor) as u32,
        );
        let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(|| wgpu::Backends::METAL);
        let instance = wgpu::Instance::new(backend);
        let surface = unsafe { instance.create_surface_from_core_animation_layer(obj.metal_layer) };
        let (_adapter, device, queue) =
            pollster::block_on(crate::request_device(&instance, backend, &surface));
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: physical.0,
            height: physical.1,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);
        AppSurface {
            view: obj.view,
            scale_factor,
            sdq: crate::SurfaceDeviceQueue {
                surface: surface,
                config,
                device: Arc::new(device),
                queue: Arc::new(queue),
            },
            callback_to_app: Some(obj.callback_to_swift),
            maximum_frames: obj.maximum_frames,
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
    let s: CGFloat = unsafe { msg_send![obj, contentScaleFactor] };
    s as f32
}
