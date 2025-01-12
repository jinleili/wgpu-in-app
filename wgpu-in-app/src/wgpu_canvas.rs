use crate::examples::*;
use app_surface::{AppSurface, SurfaceFrame};

pub struct WgpuCanvas {
    pub app_surface: AppSurface,
    example: Box<dyn Example>,
}

#[allow(dead_code)]
impl WgpuCanvas {
    pub fn new(app_surface: AppSurface, idx: i32) -> Self {
        let example = Box::new(Empty::new(&app_surface));

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
        self.example.resize(&self.app_surface);
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
