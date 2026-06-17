use wgpu::{Instance, Surface};

mod touch;
pub use touch::*;

#[cfg(any(target_os = "ios", all(feature = "mac_catalyst", target_os = "macos")))]
#[path = "ios.rs"]
mod app_surface;

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod app_surface;

#[cfg(all(target_arch = "wasm32", feature = "web_rwh"))]
#[path = "web_rwh/mod.rs"]
mod app_surface;

#[cfg(all(
    feature = "winit",
    any(
        all(not(feature = "mac_catalyst"), target_os = "macos"),
        target_os = "windows",
        target_os = "linux",
        all(target_arch = "wasm32", not(feature = "web_rwh")),
    )
))]
#[path = "app_surface_use_winit.rs"]
mod app_surface;

#[cfg(any(
    all(
        not(feature = "winit"),
        any(
            all(not(feature = "mac_catalyst"), target_os = "macos"),
            target_os = "windows",
            target_os = "linux",
        ),
    ),
    all(
        target_arch = "wasm32",
        not(feature = "winit"),
        not(feature = "web_rwh")
    ),
))]
#[path = "unsupported.rs"]
mod app_surface;
#[cfg(not(any(
    all(
        not(feature = "winit"),
        any(
            all(not(feature = "mac_catalyst"), target_os = "macos"),
            target_os = "windows",
            target_os = "linux",
        ),
    ),
    all(
        target_arch = "wasm32",
        not(feature = "winit"),
        not(feature = "web_rwh")
    ),
)))]
pub use app_surface::*;

#[repr(C)]
#[derive(Debug)]
pub struct ViewSize {
    pub width: u32,
    pub height: u32,
}

pub(crate) fn normalize_view_size(size: (u32, u32)) -> (u32, u32) {
    (size.0.max(1), size.1.max(1))
}

pub(crate) fn normalize_scale_factor(scale_factor: f32) -> f32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

#[allow(dead_code)]
pub(crate) fn physical_size_from_logical_size(
    width: f32,
    height: f32,
    scale_factor: f32,
) -> (u32, u32) {
    let scale_factor = normalize_scale_factor(scale_factor);
    let width = if width.is_finite() && width > 0.0 {
        (width * scale_factor) as u32
    } else {
        0
    };
    let height = if height.is_finite() && height > 0.0 {
        (height * scale_factor) as u32
    } else {
        0
    };

    normalize_view_size((width, height))
}

#[allow(dead_code)]
pub(crate) fn logical_size_from_physical_size(
    physical_size: (u32, u32),
    scale_factor: f32,
) -> (f32, f32) {
    let physical_size = normalize_view_size(physical_size);
    let scale_factor = normalize_scale_factor(scale_factor);

    (
        physical_size.0 as f32 / scale_factor,
        physical_size.1 as f32 / scale_factor,
    )
}

pub(crate) fn normalize_touch_point(
    touch_point_x: f32,
    touch_point_y: f32,
    physical_size: (u32, u32),
    scale_factor: f32,
) -> (f32, f32) {
    let physical_size = normalize_view_size(physical_size);
    let scale_factor = normalize_scale_factor(scale_factor);

    (
        touch_point_x * scale_factor / physical_size.0 as f32,
        touch_point_y * scale_factor / physical_size.1 as f32,
    )
}

pub(crate) fn resize_surface_config(
    config: &mut wgpu::SurfaceConfiguration,
    size: (u32, u32),
) -> bool {
    let size = normalize_view_size(size);
    if config.width == size.0 && config.height == size.1 {
        return false;
    }

    config.width = size.0;
    config.height = size.1;
    true
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
        } else if format == format.remove_srgb_suffix() {
            self.config.view_formats = vec![format.add_srgb_suffix()];
        } else {
            self.config.view_formats = vec![format];
        }
        self.surface.configure(&self.device, &self.config);
    }
}

#[cfg(not(any(
    all(
        not(feature = "winit"),
        any(
            all(not(feature = "mac_catalyst"), target_os = "macos"),
            target_os = "windows",
            target_os = "linux",
        ),
    ),
    all(
        target_arch = "wasm32",
        not(feature = "winit"),
        not(feature = "web_rwh")
    ),
)))]
impl core::ops::Deref for AppSurface {
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
    ) -> Option<(wgpu::SurfaceTexture, wgpu::TextureView)> {
        unimplemented!()
    }
    fn create_current_frame_view(
        &self,
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        config: &wgpu::SurfaceConfiguration,
        view_format: Option<wgpu::TextureFormat>,
    ) -> Option<(wgpu::SurfaceTexture, wgpu::TextureView)> {
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost => {
                surface.configure(device, config);
                match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                    _ => panic!("Failed to acquire next swap chain texture!"),
                }
            }
            wgpu::CurrentSurfaceTexture::Occluded => return None,
            wgpu::CurrentSurfaceTexture::Validation => panic!("Validation error acquiring texture"),
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
        Some((frame, view))
    }
}

#[cfg(not(any(
    all(
        not(feature = "winit"),
        any(
            all(not(feature = "mac_catalyst"), target_os = "macos"),
            target_os = "windows",
            target_os = "linux",
        ),
    ),
    all(
        target_arch = "wasm32",
        not(feature = "winit"),
        not(feature = "web_rwh")
    ),
)))]
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
        if resize_surface_config(&mut self.ctx.config, size) {
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn resize_surface_by_size(&mut self, size: (u32, u32)) {
        if resize_surface_config(&mut self.ctx.config, size) {
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn normalize_touch_point(&self, touch_point_x: f32, touch_point_y: f32) -> (f32, f32) {
        normalize_touch_point(
            touch_point_x,
            touch_point_y,
            self.get_view_size(),
            self.scale_factor,
        )
    }

    fn get_current_frame_view(
        &self,
        view_format: Option<wgpu::TextureFormat>,
    ) -> Option<(wgpu::SurfaceTexture, wgpu::TextureView)> {
        self.create_current_frame_view(&self.device, &self.surface, &self.config, view_format)
    }
}

async fn create_iasdq_context(
    instance: Instance,
    surface: Surface<'static>,
    physical_size: (u32, u32),
) -> IASDQContext {
    let physical_size = normalize_view_size(physical_size);
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

    cfg_select! {
        target_arch = "wasm32" => {
            let adp_features = adapter.features() & wgpu::Features::all_webgpu_mask();
        }
        _ => {
            let mut adp_features = adapter.features();
            adp_features.remove(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS);
            // remove raytracing features from acquired features under unix like os on nvidia discrete cards
            // this might be related to wgpu issue, need to keep tracing.
            #[cfg(target_family = "unix")]
            if adapter_info.name.contains("NVIDIA") {
                adp_features.remove(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
            }
        }
    }

    let res = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: adp_features,
            required_limits: adapter.limits(),
            experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await;

    match res {
        Err(err) => {
            panic!("request_device failed: {err:?}");
        }
        Ok(tuple) => (adapter, tuple.0, tuple.1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_view_size_clamps_zero_axes() {
        assert_eq!(normalize_view_size((0, 720)), (1, 720));
        assert_eq!(normalize_view_size((1280, 0)), (1280, 1));
        assert_eq!(normalize_view_size((0, 0)), (1, 1));
    }

    #[test]
    fn normalize_scale_factor_falls_back_to_one_for_invalid_values() {
        assert_eq!(normalize_scale_factor(0.0), 1.0);
        assert_eq!(normalize_scale_factor(-2.0), 1.0);
        assert_eq!(normalize_scale_factor(f32::NAN), 1.0);
        assert_eq!(normalize_scale_factor(f32::INFINITY), 1.0);
        assert_eq!(normalize_scale_factor(2.0), 2.0);
    }

    #[test]
    fn physical_size_from_logical_size_normalizes_scale_and_size() {
        assert_eq!(
            physical_size_from_logical_size(100.0, 50.0, 2.0),
            (200, 100)
        );
        assert_eq!(physical_size_from_logical_size(0.0, 0.0, 2.0), (1, 1));
        assert_eq!(physical_size_from_logical_size(12.0, 8.0, 0.0), (12, 8));
    }

    #[test]
    fn logical_size_from_physical_size_normalizes_scale_and_size() {
        assert_eq!(
            logical_size_from_physical_size((200, 100), 2.0),
            (100.0, 50.0)
        );
        assert_eq!(logical_size_from_physical_size((0, 0), 0.0), (1.0, 1.0));
    }

    #[test]
    fn normalize_touch_point_uses_normalized_size_and_scale() {
        assert_eq!(
            normalize_touch_point(50.0, 25.0, (200, 100), 2.0),
            (0.5, 0.5)
        );
        assert_eq!(normalize_touch_point(1.0, 1.0, (0, 0), 0.0), (1.0, 1.0));
    }

    #[test]
    fn resize_surface_config_updates_only_when_size_changes() {
        let mut config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: 640,
            height: 480,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        assert!(!resize_surface_config(&mut config, (640, 480)));
        assert!(resize_surface_config(&mut config, (0, 720)));
        assert_eq!((config.width, config.height), (1, 720));
    }
}
