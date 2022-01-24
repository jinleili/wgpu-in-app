use core::ffi::c_void;
use jni::sys::jobject;
use jni::JNIEnv;
use raw_window_handle::{AndroidNdkHandle, HasRawWindowHandle, RawWindowHandle};

pub struct AppView {
    native_window: NativeWindow,
    pub scale_factor: f32,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub callback_to_app: Option<extern "C" fn(arg: i32)>,
}

impl AppView {
    pub fn new(env: *mut JNIEnv, surface: jobject) -> Self {
        let native_window = unsafe {
            NativeWindow::new(ndk_sys::ANativeWindow_fromSurface(env as *mut _, surface))
        };
        let instance = wgpu::Instance::new(wgpu::Backends::GL);
        let surface = unsafe { instance.create_surface(&native_window) };
        let (device, queue) = pollster::block_on(crate::request_device(&instance, &surface));

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            width: native_window.get_width(),
            height: native_window.get_height(),
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        Self {
            native_window,
            scale_factor: 1.0,
            device,
            queue,
            surface,
            config,
            callback_to_app: None,
        }
    }
}

impl crate::GPUContext for AppView {
    fn resize_surface(&mut self) {
        self.config.width = self.native_window.get_width();
        self.config.height = self.native_window.get_height();
        self.surface.configure(&self.device, &self.config);
    }

    fn get_current_frame_view(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        self.create_current_frame_view(&self.device, &self.surface, &self.config)
    }
}

struct NativeWindow {
    a_native_window: *mut ndk_sys::ANativeWindow,
}

impl NativeWindow {
    unsafe fn new(window: *mut ndk_sys::ANativeWindow) -> Self {
        ndk_sys::ANativeWindow_acquire(window);
        Self {
            a_native_window: window,
        }
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
        let mut handle = AndroidNdkHandle::empty();
        handle.a_native_window = self.a_native_window as *mut _ as *mut c_void;
        RawWindowHandle::AndroidNdk(handle)
    }
}
