use crate::examples::*;
use crate::{AppView, GPUContext};

pub struct WgpuCanvas {
    pub app_view: AppView,
    example: Box<dyn Example>,
}

#[allow(dead_code)]
impl WgpuCanvas {
    pub fn new(app_view: AppView) -> Self {
        let example = Self::create_a_example(&app_view, 1);
        log::info!("example created");
        let instance = WgpuCanvas { app_view, example };
        if let Some(callback) = instance.app_view.callback_to_app {
            callback(0);
        }
        instance
    }

    pub fn enter_frame(&mut self) {
        self.example.enter_frame(&self.app_view);

        if let Some(_callback) = self.app_view.callback_to_app {
            // callback(1);
        }
    }

    pub fn change_example(&mut self, index: i32) {
        self.example = Self::create_a_example(&self.app_view, index);
    }

    fn create_a_example(app_view: &AppView, index: i32) -> Box<dyn Example> {
        if index == 1 {
            Box::new(MSAALine::new(app_view))
        } else if index == 2 {
            Box::new(Cube::new(app_view))
        } else if index == 3 {
            Box::new(Water::new(app_view))
        } else {
            Box::new(Boids::new(app_view))
        }
    }
}
