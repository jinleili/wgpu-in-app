use libc::c_void;
use std::marker::Sync;
use std::path::PathBuf;

use lazy_static::*;
use objc_foundation::{INSString, NSString};

use core_graphics::{base::CGFloat, geometry::CGRect};
use objc::{runtime::Object, *};

#[repr(C)]
pub struct IOSViewObj {
    pub view: *mut Object,
    pub metal_layer: *mut c_void,
    pub maximum_frames: i32,
    pub callback_to_swift: extern "C" fn(arg: i32),
}

pub struct AppView {
    pub view: *mut Object,
    pub scale_factor: f32,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub maximum_frames: i32,
    pub callback_to_app: Option<extern "C" fn(arg: i32)>,
}

unsafe impl Sync for AppView {}

impl AppView {
    pub fn new(obj: IOSViewObj) -> Self {
        // hook up rust logging
        env_logger::init();

        let scale_factor = get_scale_factor(obj.view);
        let s: CGRect = unsafe { msg_send![obj.view, frame] };
        let physical = crate::ViewSize {
            width: (s.size.width as f32 * scale_factor) as u32,
            height: (s.size.height as f32 * scale_factor) as u32,
        };

        let instance = wgpu::Instance::new(wgpu::Backends::METAL);
        let surface = unsafe { instance.create_surface_from_core_animation_layer(obj.metal_layer) };
        let (device, queue) = pollster::block_on(crate::request_device(&instance, &surface));

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: physical.width,
            height: physical.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        AppView {
            view: obj.view,
            scale_factor,
            device,
            queue,
            surface,
            config,
            callback_to_app: Some(obj.callback_to_swift),
            maximum_frames: obj.maximum_frames,
        }
    }

    fn get_view_size(&self) -> crate::ViewSize {
        let s: CGRect = unsafe { msg_send![self.view, frame] };
        crate::ViewSize {
            width: (s.size.width as f32 * self.scale_factor) as u32,
            height: (s.size.height as f32 * self.scale_factor) as u32,
        }
    }
}

impl crate::GPUContext for AppView {
    fn resize_surface(&mut self) {
        let size = self.get_view_size();
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn get_current_frame_view(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        self.create_current_frame_view(&self.device, &self.surface, &self.config)
    }
}

lazy_static! {
    static ref BUNDLE_PATH: &'static str = get_bundle_url();
}

fn get_scale_factor(obj: *mut Object) -> f32 {
    let s: CGFloat = unsafe { msg_send![obj, contentScaleFactor] };
    s as f32
}

fn get_bundle_url() -> &'static str {
    let cls = class!(NSBundle);
    let path: &str = unsafe {
        // Allocate an instance
        let bundle: *mut Object = msg_send![cls, mainBundle];
        let path: &NSString = msg_send![bundle, resourcePath];
        path.as_str()
    };
    path
}

pub fn get_wgsl_path(name: &str) -> PathBuf {
    let p = get_bundle_url().to_string() + "/wgsl_shader/" + name;
    PathBuf::from(&p)
}
