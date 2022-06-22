#[cfg(any(target_os = "android", target_os = "ios"))]
fn main() {}

#[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
fn main() {
    use app_surface::AppSurface;
    use std::time::{Duration, Instant};
    use wgpu_on_app::WgpuCanvas;
    use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};

    let events_loop = EventLoop::new();
    let size = winit::dpi::Size::Logical(winit::dpi::LogicalSize {
        width: 1200.0,
        height: 800.0,
    });
    let builder = winit::window::WindowBuilder::new()
        .with_inner_size(size)
        .with_max_inner_size(size)
        .with_title("wgpu on Desktop");
    let window = builder.build(&events_loop).unwrap();

    let mut canvas = WgpuCanvas::new(AppSurface::new(window), 0);

    let mut last_update_inst = Instant::now();
    let target_frametime = Duration::from_secs_f64(1.0 / 60.0);
    let spawner = Spawner::new();

    events_loop.run(move |event, _, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            Event::RedrawEventsCleared => {
                let time_since_last_frame = last_update_inst.elapsed();
                if time_since_last_frame >= target_frametime {
                    canvas.app_surface.view.request_redraw();
                    last_update_inst = Instant::now();
                } else {
                    *control_flow = ControlFlow::WaitUntil(
                        Instant::now() + target_frametime - time_since_last_frame,
                    );
                }

                spawner.run_until_stalled();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_size),
                ..
            } => {
                canvas.resize();
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(key),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => match key {
                    VirtualKeyCode::Key1 => canvas.change_example(1),
                    VirtualKeyCode::Key2 => canvas.change_example(2),
                    VirtualKeyCode::Key3 => canvas.change_example(3),
                    VirtualKeyCode::Key4 => canvas.change_example(4),
                    VirtualKeyCode::Key5 => canvas.change_example(5),

                    _ => canvas.change_example(0),
                },
                _ => {}
            },
            Event::RedrawRequested(_) => {
                canvas.enter_frame();
            }
            _ => (),
        }
    });
}

#[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
pub struct Spawner<'a> {
    executor: async_executor::LocalExecutor<'a>,
}

#[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
impl<'a> Spawner<'a> {
    fn new() -> Self {
        Self {
            executor: async_executor::LocalExecutor::new(),
        }
    }

    #[allow(dead_code)]
    pub fn spawn_local(&self, future: impl std::future::Future<Output = ()> + 'a) {
        self.executor.spawn(future).detach();
    }

    fn run_until_stalled(&self) {
        while self.executor.try_tick() {}
    }
}
