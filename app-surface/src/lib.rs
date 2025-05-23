use core::ops::Deref;
use wgpu::{Instance, Surface};

mod touch;
pub use touch::*;

#[cfg_attr(
    any(target_os = "ios", all(feature = "mac_catalyst", target_os = "macos")),
    path = "ios.rs"
)]
#[cfg_attr(target_os = "android", path = "android.rs")]
#[cfg_attr(
    all(target_arch = "wasm32", feature = "web_rwh"),
    path = "web_rwh/mod.rs"
)]
#[cfg_attr(
    any(
        all(not(feature = "mac_catalyst"), target_os = "macos"),
        target_os = "windows",
        target_os = "linux",
    ),
    path = "app_surface_use_winit.rs"
)]
#[cfg_attr(
    all(target_arch = "wasm32", not(feature = "web_rwh")),
    path = "app_surface_use_winit.rs"
)]
mod app_surface;
pub use app_surface::*;

// #[cfg(all(target_arch = "wasm32", feature = "web_rwh"))]
// compile_error!("web_rwh feature is enabled for wasm32");

// #[cfg(all(target_arch = "wasm32", not(feature = "web_rwh")))]
// compile_error!("web_rwh feature is not enabled -");

#[repr(C)]
#[derive(Debug)]
pub struct ViewSize {
    pub width: u32,
    pub height: u32,
}

#[cfg(target_arch = "wasm32")]
use std::rc::Rc as SharedPtr;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc as SharedPtr;
/// wgpu v24 开始，Instance、Adapter、Device 和 Queue 都是可 `Clone` 的
#[derive(Clone)]
pub struct IASDQContext {
    pub instance: wgpu::Instance,
    pub surface: SharedPtr<wgpu::Surface<'static>>,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl IASDQContext {
    pub fn update_config_format(&mut self, format: wgpu::TextureFormat) {
        self.config.format = format;
        if cfg!(feature = "webgl") {
            // webgl 后端不支持 view_formats
        } else {
            self.config.view_formats = vec![format.add_srgb_suffix(), format.remove_srgb_suffix()];
        }
        self.surface.configure(&self.device, &self.config);
    }
}

impl Deref for AppSurface {
    type Target = IASDQContext;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

pub trait SurfaceFrame {
    fn view_size(&self) -> ViewSize;
    // After App view's size or orientation changed, need to resize surface.
    fn resize_surface(&mut self);
    fn resize_surface_by_size(&mut self, size: (u32, u32));
    fn pintch(&mut self, _touch: Touch, _scale: f32) {}
    fn touch(&mut self, _touch: Touch) {}
    fn normalize_touch_point(&self, _touch_point_x: f32, _touch_point_y: f32) -> (f32, f32) {
        unimplemented!()
    }
    fn enter_frame(&mut self) {}
    fn get_current_frame_view(
        &self,
        _view_format: Option<wgpu::TextureFormat>,
    ) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        unimplemented!()
    }
    fn create_current_frame_view(
        &self,
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        config: &wgpu::SurfaceConfiguration,
        view_format: Option<wgpu::TextureFormat>,
    ) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                surface.configure(device, config);
                surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture!")
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("frame texture view"),
            format: if view_format.is_none() {
                // frame buffer's view format prefer to use sRGB.
                Some(config.format.add_srgb_suffix())
            } else {
                view_format
            },
            ..Default::default()
        });
        (frame, view)
    }
}

impl SurfaceFrame for AppSurface {
    fn view_size(&self) -> ViewSize {
        let size = self.get_view_size();
        ViewSize {
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

async fn create_iasdq_context(
    instance: Instance,
    surface: Surface<'static>,
    physical_size: (u32, u32),
) -> IASDQContext {
    let (adapter, device, queue) = crate::request_device(&instance, &surface).await;

    let caps = surface.get_capabilities(&adapter);
    let prefered = caps.formats[0];

    let format = if cfg!(all(target_arch = "wasm32", not(feature = "webgl"))) {
        // Chrome WebGPU doesn't support sRGB:
        // unsupported swap chain format "xxxx8unorm-srgb"
        prefered.remove_srgb_suffix()
    } else {
        prefered
    };
    let view_formats = if cfg!(feature = "webgl") {
        // panicked at 'Error in Surface::configure: Validation Error
        // Caused by:
        // Downlevel flags DownlevelFlags(SURFACE_VIEW_FORMATS) are required but not supported on the device.
        vec![]
    } else if cfg!(target_os = "android") {
        // TODO:HarmonyOS 不支持 view_formats 格式
        // format 的值与 view_formats 的值一致时，configure 内部会自动忽略 view_formats 的值
        //
        // Android 不支持 view_formats:
        // Downlevel flags DownlevelFlags(SURFACE_VIEW_FORMATS) are required but not supported on the device.
        // This is not an invalid use of WebGPU: the underlying API or device does not support enough features
        // to be a fully compliant implementation. A subset of the features can still be used.
        // If you are running this program on native and not in a browser and wish to work around this issue,
        // call Adapter::downlevel_properties or Device::downlevel_properties to get a listing of the features the current platform supports.
        vec![format]
    } else if format.is_srgb() {
        vec![format, format.remove_srgb_suffix()]
    } else {
        vec![format.add_srgb_suffix(), format.remove_srgb_suffix()]
    };

    let mut config = surface
        .get_default_config(&adapter, physical_size.0, physical_size.1)
        .expect("Surface isn't supported by the adapter.");

    config.view_formats = view_formats;
    config.format = format;

    surface.configure(&device, &config);

    IASDQContext {
        instance,
        surface: SharedPtr::new(surface),
        config,
        adapter,
        device,
        queue,
    }
}

async fn request_device(
    instance: &Instance,
    surface: &Surface<'static>,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::from_env()
                .unwrap_or(wgpu::PowerPreference::HighPerformance),
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        })
        .await
        .expect("No suitable GPU adapters found on the system!");

    let adapter_info = adapter.get_info();
    println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    let base_dir = std::env::var("CARGO_MANIFEST_DIR");
    let _trace_path = if let Ok(base_dir) = base_dir {
        Some(std::path::PathBuf::from(&base_dir).join("WGPU_TRACE_ERROR"))
    } else {
        None
    };

    // remove raytracing features from acquired features under unix like os on nvidia discrete cards
    // this might be related to wgpu issue, need to keep tracing.
    let mut adp_features = adapter.features();
    #[cfg(target_family = "unix")]
    {
        if adapter_info.name.contains("NVIDIA") {
            adp_features.remove(wgpu::Features::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE);
            adp_features.remove(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
        }
    }
    // test features
    // let adp_features = wgpu::Features::from_bits(0b0011111111011100110111111111111111111111110111000000111111001111).unwrap();

    let res = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: adp_features,
                required_limits: adapter.limits(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            }
        )
        .await;

    match res {
        Err(err) => {
            panic!("request_device failed: {err:?}");
        }
        Ok(tuple) => (adapter, tuple.0, tuple.1),
    }
}
