use std::sync::Arc;
use winit::window::Window;

pub struct AppSurface {
    pub view: Option<Arc<Window>>,
    pub is_offscreen_canvas: bool,
    pub scale_factor: f32,
    pub maximum_frames: i32,
    pub sdq: crate::SurfaceDeviceQueue,
    pub callback_to_app: Option<extern "C" fn(arg: i32)>,
    pub temporary_directory: &'static str,
    pub library_directory: &'static str,
}

#[derive(Default)]
struct ViewSetting {
    view: Option<Window>,
    scale_factor: f32,
    physical_size: (u32, u32),
    #[cfg(target_arch = "wasm32")]
    offscreen_canvas: Option<web_sys::OffscreenCanvas>,
}

impl AppSurface {
    #[allow(clippy::needless_update)]
    pub async fn new(view: Window) -> Self {
        let scale_factor = view.scale_factor() as f32;
        let physical_size = view.inner_size();
        let view_setting = ViewSetting {
            scale_factor,
            physical_size: (physical_size.width, physical_size.height),
            view: Some(view),
            ..Default::default()
        };

        Self::create(view_setting).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn from_offscreen_canvas(
        offscreen_canvas: web_sys::OffscreenCanvas,
        scale_factor: f32,
        physical_size: (u32, u32),
    ) -> Self {
        let view_setting = ViewSetting {
            scale_factor,
            physical_size,
            offscreen_canvas: Some(offscreen_canvas),
            ..Default::default()
        };
        Self::create(view_setting).await
    }

    #[allow(unused_variables)]
    async fn create(view_setting: ViewSetting) -> Self {
        let view = Arc::new(view_setting.view.unwrap());
        #[cfg(not(target_arch = "wasm32"))]
        let is_offscreen_canvas = false;
        #[cfg(target_arch = "wasm32")]
        let is_offscreen_canvas = if view_setting.offscreen_canvas.is_some() {
            true
        } else {
            false
        };
        let scale_factor = view_setting.scale_factor;
        let default_backends = if cfg!(feature = "webgl") {
            wgpu::Backends::GL
        } else {
            wgpu::Backends::PRIMARY
        };
        let backends = wgpu::util::backend_bits_from_env().unwrap_or(default_backends);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let surface = if is_offscreen_canvas {
                    // let offscreen = canvas.transfer_control_to_offscreen().unwrap();
                    instance.create_surface(
                        wgpu::SurfaceTarget::OffscreenCanvas(view_setting.offscreen_canvas.unwrap())
                    )
                } else {
                    use winit::platform::web::WindowExtWebSys;
                    let canvas: web_sys::HtmlCanvasElement =
                        view.as_ref().canvas().unwrap();
                    instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))
                };
            } else {
                let surface = instance.create_surface(view.clone());
            }
        }
        let surface = match surface {
            Ok(surface) => surface,
            Err(e) => {
                panic!("Failed to create surface: {e:?}");
            }
        };

        let (adapter, device, queue) = crate::request_device(&instance, &surface).await;
        let caps = surface.get_capabilities(&adapter);

        let modes = caps.alpha_modes;
        let alpha_mode = if modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
            // wasm can only support this alpha mode
            wgpu::CompositeAlphaMode::PreMultiplied
        } else if modes.contains(&wgpu::CompositeAlphaMode::PostMultiplied) {
            // Metal alpha mode
            wgpu::CompositeAlphaMode::PostMultiplied
        } else if modes.contains(&wgpu::CompositeAlphaMode::Inherit) {
            // Vulkan on Android
            wgpu::CompositeAlphaMode::Inherit
        } else {
            modes[0]
        };
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
        } else {
            vec![format.add_srgb_suffix(), format.remove_srgb_suffix()]
        };

        let physical = view_setting.physical_size;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: physical.0,
            height: physical.1,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats,
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        AppSurface {
            view: Some(view),
            is_offscreen_canvas,
            scale_factor,
            maximum_frames: 60,
            sdq: crate::SurfaceDeviceQueue {
                surface,
                config,
                adapter,
                device: Arc::new(device),
                queue: Arc::new(queue),
            },
            callback_to_app: None,
            temporary_directory: "",
            library_directory: "",
        }
    }

    pub fn get_view_size(&self) -> (u32, u32) {
        if self.is_offscreen_canvas {
            panic!("Offscreen canvas cannot provide any DOM interfaces.");
        } else {
            let physical = self.view.as_ref().unwrap().inner_size();
            (physical.width, physical.height)
        }
    }
}
