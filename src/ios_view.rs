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

    pub fn get_current_frame_view(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        self.create_current_frame_view(&self.device, &self.surface, &self.config)
    }

    fn create_current_frame_view(
        &self,
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                surface.configure(&device, &config);
                surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture!")
            }
        };
        let view = frame
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
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let all_features = adapter.features();
    let request_features = wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
        | wgpu::Features::POLYGON_MODE_LINE
        | wgpu::Features::VERTEX_WRITABLE_STORAGE;

    let base_dir = crate::application_root_dir();
    let trace_path = std::path::PathBuf::from(&base_dir).join("WGPU_TRACE_IOS");
    // iOS device can not support BC compressed texture, A8(iPhone 6, mini 4) and above support ASTC, All support ETC2
    let optional_features =
        wgpu::Features::TEXTURE_COMPRESSION_ASTC_LDR | wgpu::Features::TEXTURE_COMPRESSION_ETC2;
    let res = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: (optional_features & all_features) | request_features,
                // features: all_features,
                limits: wgpu::Limits {
                    max_dynamic_storage_buffers_per_pipeline_layout: 4,
                    max_storage_buffers_per_shader_stage: 8,
                    max_storage_textures_per_shader_stage: 6,
                    max_push_constant_size: 16,
                    ..Default::default()
                },
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
