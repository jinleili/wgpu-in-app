use crate::examples::*;
use app_surface::{AppSurface, SurfaceFrame};
#[cfg(target_os = "macos")]
use objc::{
    class, msg_send,
    runtime::{Object, BOOL, YES},
    sel, sel_impl,
};
use wgpu::rwh::{HasWindowHandle, RawWindowHandle};

pub struct WgpuCanvas {
    pub app_surface: AppSurface,
    example: Box<dyn Example>,
}

#[allow(dead_code)]
impl WgpuCanvas {
    pub fn new(app_surface: AppSurface, idx: i32) -> Self {
        let mut app_surface = app_surface;
        #[cfg(target_os = "macos")]
        {
            if let Some(v) = app_surface.view.as_mut() {
                match v.window_handle().unwrap().as_raw() {
                    RawWindowHandle::AppKit(handle) => {
                        let view = handle.ns_view.as_ptr() as *mut Object;
                        let class = class!(CAMetalLayer);
                        unsafe {
                            let metal_layer: *mut Object = msg_send![view, layer];
                            let is_metal_layer: BOOL = msg_send![metal_layer, isKindOfClass: class];
                            if is_metal_layer == YES {
                                let () = msg_send![metal_layer, setPresentsWithTransaction: YES];
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
        let example = Box::new(Empty::new(&app_surface));
        log::info!("example created");

        let mut instance = WgpuCanvas {
            app_surface,
            example,
        };
        instance.change_example(idx);

        if let Some(callback) = instance.app_surface.callback_to_app {
            callback(0);
        }
        instance
    }

    pub fn enter_frame(&mut self) {
        self.example.enter_frame(&self.app_surface);

        if let Some(_callback) = self.app_surface.callback_to_app {
            // callback(1);
        }
    }

    pub fn resize(&mut self) {
        self.app_surface.resize_surface();
    }

    pub fn change_example(&mut self, index: i32) {
        self.example = Self::create_a_example(&mut self.app_surface, index);
    }

    fn create_a_example(app_surface: &mut AppSurface, index: i32) -> Box<dyn Example> {
        if index == 0 {
            Box::new(Boids::new(app_surface))
        } else if index == 1 {
            Box::new(MSAALine::new(app_surface))
        } else if index == 2 {
            Box::new(Cube::new(app_surface))
        } else if index == 3 {
            Box::new(Water::new(app_surface))
        } else if index == 4 {
            Box::new(Shadow::new(app_surface))
        } else {
            Box::new(HDRImageView::new(app_surface))
        }
    }
}
