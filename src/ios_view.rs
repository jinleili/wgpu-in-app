use libc::c_void;
use std::marker::Sync;
extern crate objc;
use self::objc::{runtime::Object, *};

extern crate core_graphics;
use self::core_graphics::{base::CGFloat, geometry::CGRect};

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
        let scale_factor = get_scale_factor(obj.view);
        let s: CGRect = unsafe { msg_send![obj.view, frame] };
        let physical = crate::ViewSize {
            width: (s.size.width as f32 * scale_factor) as u32,
            height: (s.size.height as f32 * scale_factor) as u32,
        };

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface_from_core_animation_layer(obj.metal_layer) };

        let (device, queue) = pollster::block_on(request_device(&instance, &surface));
        println!("device: {:?} \n queue: {:?}", device, queue);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
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

    pub fn resize_surface(&mut self) {
        let size = self.get_view_size();
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn get_view_size(&self) -> crate::ViewSize {
        let s: CGRect = unsafe { msg_send![self.view, frame] };
        crate::ViewSize {
            width: (s.size.width as f32 * self.scale_factor) as u32,
            height: (s.size.height as f32 * self.scale_factor) as u32,
        }
    }

    pub fn get_current_frame_view(&self) -> (wgpu::SurfaceFrame, wgpu::TextureView) {
        self.create_current_frame_view(&self.device, &self.surface, &self.config)
    }

    fn create_current_frame_view(
        &self,
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::SurfaceFrame, wgpu::TextureView) {
        let frame = match surface.get_current_frame() {
            Ok(frame) => frame,
            Err(_) => {
                surface.configure(&device, &config);
                surface
                    .get_current_frame()
                    .expect("Failed to acquire next surface texture!")
            }
        };
        let view = frame
            .output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        (frame, view)
    }
}

fn get_scale_factor(obj: *mut Object) -> f32 {
    let s: CGFloat = unsafe { msg_send![obj, contentScaleFactor] };
    s as f32
}

async fn request_device(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface,
) -> (wgpu::Device, wgpu::Queue) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(surface),
        })
        .await
        .unwrap();

    let adapter_features = adapter.features();

    let base_dir = crate::application_root_dir();
    let trace_path = std::path::PathBuf::from(&base_dir).join("WGPU_TRACE_IOS");
    let res = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: adapter_features,
                limits: wgpu::Limits::default(),
            },
            Some(&trace_path),
        )
        .await;
    match res {
        Err(err) => {
            panic!("request_device failed: {:?}", err);
        }
        Ok(tuple) => tuple,
    }
}
