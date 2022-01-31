use crate::examples::*;
use crate::{AppSurface, FrameContext};

pub struct WgpuCanvas {
    pub app_surface: AppSurface,
    example: Box<dyn Example>,
}

#[allow(dead_code)]
impl WgpuCanvas {
    pub fn new(app_surface: AppSurface, idx: i32) -> Self {
        let example = Self::create_a_example(&app_surface, idx);
        log::info!("example created");
        let instance = WgpuCanvas {
            app_surface,
            example,
        };
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
        self.example = Self::create_a_example(&self.app_surface, index);
    }

    fn create_a_example(app_surface: &AppSurface, index: i32) -> Box<dyn Example> {
        if index == 0 {
            Box::new(Boids::new(app_surface))
        } else if index == 1 {
            Box::new(MSAALine::new(app_surface))
        } else if index == 2 {
            Box::new(Cube::new(app_surface))
        } else if index == 3 {
            Box::new(Water::new(app_surface))
        } else {
            Box::new(Shadow::new(app_surface))
        }
    }
}
