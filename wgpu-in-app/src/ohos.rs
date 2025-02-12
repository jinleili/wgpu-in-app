use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::thread;

use crate::WgpuCanvas;
use app_surface::AppSurface;
use napi_derive_ohos::napi;
use std::sync::Arc;
use std::time;
use winit::event_loop::EventLoopProxy;

use openharmony_ability::OpenHarmonyApp;
use openharmony_ability_derive::ability;
use winit::platform::ohos::EventLoopBuilderExtOpenHarmony;
use winit::platform::ohos::EventLoopExtOpenHarmony;
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

static RENDER_INDEX: AtomicI32 = AtomicI32::new(0);
static EVENT_NOTIFY: LazyLock<Mutex<Option<EventLoopProxy<()>>>> =
    LazyLock::new(|| Mutex::new(None));

#[napi]
pub fn change_render(index: i32) {
    RENDER_INDEX.store(index, Ordering::SeqCst);
    let guard = EVENT_NOTIFY.lock().unwrap();
    if let Some(proxy) = guard.as_ref() {
        proxy.send_event(()).unwrap();
    }
}

#[ability]
pub fn run(app: OpenHarmonyApp) -> Result<()> {
    crate::init_logger();

    let events_loop = EventLoop::builder()
        .with_openharmony_app(app)
        .build()
        .unwrap();

    let proxy = events_loop.create_proxy();

    let mut guard = EVENT_NOTIFY.lock().unwrap();
    *guard = Some(proxy);

    let app = WgpuApp::default();
    events_loop.spawn_app(app)
}

const WAIT_TIME: time::Duration = time::Duration::from_millis(16);
const POLL_SLEEP_TIME: time::Duration = time::Duration::from_millis(16);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[allow(dead_code)]
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

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        match RENDER_INDEX.load(Ordering::SeqCst) {
            1 => self.get_canvas().change_example(1),
            2 => self.get_canvas().change_example(2),
            3 => self.get_canvas().change_example(3),
            4 => self.get_canvas().change_example(4),
            _ => self.get_canvas().change_example(0),
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.canvas.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes().with_title("Wgpu on Desktop");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let app_view = pollster::block_on(AppSurface::new(window));

        self.canvas = Some(WgpuCanvas::new(app_view, 3));
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
