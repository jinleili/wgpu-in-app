use std::thread;

use crate::WgpuCanvas;
use app_surface::AppSurface;
use std::sync::Arc;
use std::time;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

pub fn run() -> Result<(), impl std::error::Error> {
    crate::init_logger();

    let events_loop = EventLoop::new().unwrap();
    let mut app = WgpuApp::default();
    events_loop.run_app(&mut app)
}

const WAIT_TIME: time::Duration = time::Duration::from_millis(16);
const POLL_SLEEP_TIME: time::Duration = time::Duration::from_millis(16);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Wait,
    WaitUntil,
    #[default]
    Poll,
}

#[derive(Default)]
struct WgpuApp {
    mode: Mode,
    wait_cancelled: bool,
    close_requested: bool,
    canvas: Option<WgpuCanvas>,
}

impl WgpuApp {
    fn get_canvas(&mut self) -> &mut WgpuCanvas {
        self.canvas.as_mut().unwrap()
    }
}

impl ApplicationHandler for WgpuApp {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        self.wait_cancelled = match cause {
            StartCause::WaitCancelled { .. } => self.mode == Mode::WaitUntil,
            _ => false,
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.canvas.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes().with_title("Wgpu on Desktop");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let app_view = pollster::block_on(AppSurface::new(window));

        self.canvas = Some(WgpuCanvas::new(app_view, 0));
        self.get_canvas().app_surface.request_redraw();
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    println!("Window minimized!");
                } else {
                    self.get_canvas().resize();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key {
                KeyCode::Digit1 => self.get_canvas().change_example(1),
                KeyCode::Digit2 => self.get_canvas().change_example(2),
                KeyCode::Digit3 => self.get_canvas().change_example(3),
                KeyCode::Digit4 => self.get_canvas().change_example(4),
                KeyCode::Digit5 => self.get_canvas().change_example(5),
                _ => self.get_canvas().change_example(0),
            },
            WindowEvent::RedrawRequested => {
                self.get_canvas().app_surface.pre_present_notify();

                self.get_canvas().enter_frame();

                self.get_canvas().app_surface.request_redraw();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.mode {
            Mode::Wait => event_loop.set_control_flow(ControlFlow::Wait),
            Mode::WaitUntil => {
                if !self.wait_cancelled {
                    event_loop
                        .set_control_flow(ControlFlow::WaitUntil(time::Instant::now() + WAIT_TIME));
                }
            }
            Mode::Poll => {
                thread::sleep(POLL_SLEEP_TIME);
                event_loop.set_control_flow(ControlFlow::Poll);
            }
        };

        if self.close_requested {
            event_loop.exit();
        }
    }
}
