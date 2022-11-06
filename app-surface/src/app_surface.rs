use std::sync::Arc;

pub struct AppSurface {
    pub view: winit::window::Window,
    pub scale_factor: f32,
    pub sdq: crate::SurfaceDeviceQueue,
    pub callback_to_app: Option<extern "C" fn(arg: i32)>,
    pub library_directory: &'static str,
}

impl AppSurface {
    pub async fn new(view: winit::window::Window) -> Self {
        let scale_factor = view.scale_factor();
        let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(|| wgpu::Backends::all());
        let instance = wgpu::Instance::new(backend);
        let (physical, surface) = unsafe { (view.inner_size(), instance.create_surface(&view)) };
        let (adapter, device, queue) = crate::request_device(&instance, backend, &surface).await;

        let modes = surface.get_supported_alpha_modes(&adapter);
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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // format: wgpu::TextureFormat::Bgra8UnormSrgb,
            format: surface.get_supported_formats(&adapter)[0],
            width: physical.width as u32,
            height: physical.height as u32,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
        };
        surface.configure(&device, &config);

        AppSurface {
            view,
            scale_factor: scale_factor as f32,
            sdq: crate::SurfaceDeviceQueue {
                surface: surface,
                config,
                adapter,
                device: Arc::new(device),
                queue: Arc::new(queue),
            },
            callback_to_app: None,
            library_directory: "",
        }
    }

    pub fn get_view_size(&self) -> (u32, u32) {
        let physical = self.view.inner_size();
        (physical.width as u32, physical.height as u32)
    }
}
