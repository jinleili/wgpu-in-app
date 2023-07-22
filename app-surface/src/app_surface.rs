use std::sync::Arc;

pub struct AppSurface {
    pub view: winit::window::Window,
    pub scale_factor: f32,
    pub maximum_frames: i32,
    pub sdq: crate::SurfaceDeviceQueue,
    pub callback_to_app: Option<extern "C" fn(arg: i32)>,
    pub temporary_directory: &'static str,
    pub library_directory: &'static str,
}

impl AppSurface {
    pub async fn new(view: winit::window::Window) -> Self {
        let scale_factor = view.scale_factor();
        let default_backends = if cfg!(feature = "webgl") {
            wgpu::Backends::all()
        } else {
            wgpu::Backends::PRIMARY
        };
        let backends = wgpu::util::backend_bits_from_env().unwrap_or(default_backends);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let physical = view.inner_size();
        let surface = unsafe {
            let surface = instance.create_surface(&view);
            match surface {
                Ok(surface) => surface,
                Err(e) => {
                    panic!("Failed to create surface: {e:?}");
                }
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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: physical.width,
            height: physical.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats,
        };
        surface.configure(&device, &config);

        AppSurface {
            view,
            scale_factor: scale_factor as f32,
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
        let physical = self.view.inner_size();
        (physical.width, physical.height)
    }
}
