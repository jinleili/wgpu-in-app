use core::ffi::c_void;
use jni::sys::jobject;
use jni::JNIEnv;
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, HasRawDisplayHandle, HasRawWindowHandle,
    RawDisplayHandle, RawWindowHandle,
};
use std::sync::Arc;

mod vk_extension;

pub struct AppSurface {
    pub native_window: NativeWindow,
    pub scale_factor: f32,
    pub sdq: crate::SurfaceDeviceQueue,
    pub instance: wgpu::Instance,
    pub callback_to_app: Option<extern "C" fn(arg: i32)>,
}

impl AppSurface {
    pub fn new(env: *mut JNIEnv, surface: jobject) -> Self {
        let native_window = NativeWindow::new(env, surface);

        let backend = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(backend);
        if let Some(instance) = unsafe { instance.as_hal::<hal::api::Vulkan>() } {
            // VK_VERSION_1_1
            if instance.shared_instance().driver_api_version() < 4198400 {
                panic!("Vulkan 1.1 is required.")
            }
        }
        let surface = unsafe { instance.create_surface(&native_window).unwrap() };
        let (adapter, device, queue) =
            pollster::block_on(vk_extension::request_device(&instance, backend, &surface));

        // let backend = wgpu::Backends::GL;
        // let instance = wgpu::Instance::new(backend);
        // let surface = unsafe { instance.create_surface(&native_window).unwrap() };
        // let (adapter, device, queue) =
        //     pollster::block_on(crate::request_device(&instance, backend, &surface));

        let caps = surface.get_capabilities(&adapter);

        log::info!("adapter.limits(): {:?}", adapter.limits());
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            width: native_window.get_width(),
            height: native_window.get_height(),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
        };
        surface.configure(&device, &config);

        Self {
            native_window,
            scale_factor: 1.0,
            sdq: crate::SurfaceDeviceQueue {
                surface: surface,
                config,
                adapter,
                device: Arc::new(device),
                queue: Arc::new(queue),
            },
            instance,
            callback_to_app: None,
        }
    }

    pub fn get_view_size(&self) -> (u32, u32) {
        (
            self.native_window.get_width(),
            self.native_window.get_height(),
        )
    }
}

pub struct NativeWindow {
    a_native_window: *mut ndk_sys::ANativeWindow,
}

impl NativeWindow {
    fn new(env: *mut JNIEnv, surface: jobject) -> Self {
        let a_native_window = unsafe {
            // 获取与安卓端 surface 对象关联的 ANativeWindow，以便能通过 Rust 与之交互。
            // 此函数在返回 ANativeWindow 的同时会自动将其引用计数 +1，以防止该对象在安卓端被意外释放。
            ndk_sys::ANativeWindow_fromSurface(env as *mut _, surface as *mut _)
        };
        Self { a_native_window }
    }

    pub fn get_raw_window(&self) -> *mut ndk_sys::ANativeWindow {
        self.a_native_window
    }

    fn get_width(&self) -> u32 {
        unsafe { ndk_sys::ANativeWindow_getWidth(self.a_native_window) as u32 }
    }

    fn get_height(&self) -> u32 {
        unsafe { ndk_sys::ANativeWindow_getHeight(self.a_native_window) as u32 }
    }
}

impl Drop for NativeWindow {
    fn drop(&mut self) {
        unsafe {
            ndk_sys::ANativeWindow_release(self.a_native_window);
        }
    }
}

unsafe impl HasRawWindowHandle for NativeWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = AndroidNdkWindowHandle::empty();
        handle.a_native_window = self.a_native_window as *mut _ as *mut c_void;
        RawWindowHandle::AndroidNdk(handle)
    }
}

unsafe impl HasRawDisplayHandle for NativeWindow {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Android(AndroidDisplayHandle::empty())
    }
}
