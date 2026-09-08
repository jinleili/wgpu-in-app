//! #10013 的真实 wgpu surface 验收，运行方法见 issue_10013.md。

#[cfg(target_os = "macos")]
mod test {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };
    use wgpu::{SurfaceColorSpace as Space, TextureFormat as Format};
    use winit::{
        application::ApplicationHandler,
        dpi::PhysicalSize,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        window::{Window, WindowId},
    };

    const SHADER: &str = r#"
        @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
            return vec4f(array(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3))[i], 0, 1);
        }
        fn color(y: f32) -> vec3f {
            let colors = array(vec3f(250,199,51), vec3f(219,61,77), vec3f(133,92,219), vec3f(13,18,31));
            return colors[min(u32(y / 100), 3u)] / 255;
        }
        @fragment fn encoded(@builtin(position) p: vec4f) -> @location(0) vec4f {
            return vec4f(color(p.y), 1);
        }
        @fragment fn linear(@builtin(position) p: vec4f) -> @location(0) vec4f {
            let c = color(p.y);
            return vec4f(select(pow((c + 0.055) / 1.055, vec3f(2.4)), c / 12.92, c <= vec3f(0.04045)), 1);
        }
    "#;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mode {
        Baseline,
        Auto,
        Srgb,
        SrgbFormat,
        Reconfigure,
        Resize,
    }
    impl Mode {
        fn parse(value: &str) -> Self {
            match value {
                "baseline" => Self::Baseline,
                "auto" => Self::Auto,
                "srgb" => Self::Srgb,
                "srgb-format" => Self::SrgbFormat,
                "reconfigure" => Self::Reconfigure,
                "resize" => Self::Resize,
                _ => panic!("unknown mode: {value}"),
            }
        }

        fn name(self) -> &'static str {
            match self {
                Self::Baseline => "baseline",
                Self::Auto => "auto",
                Self::Srgb => "srgb",
                Self::SrgbFormat => "srgb-format",
                Self::Reconfigure => "reconfigure",
                Self::Resize => "resize",
            }
        }

        fn render_target(self) -> (Format, &'static str) {
            match self {
                Self::SrgbFormat => (Format::Bgra8UnormSrgb, "linear"),
                _ => (Format::Bgra8Unorm, "encoded"),
            }
        }

        fn color_space(self, frames: u32) -> Space {
            match self {
                Self::Auto => Space::Auto,
                Self::Reconfigure => [Space::DisplayP3, Space::Srgb, Space::DisplayP3, Space::Auto]
                    [(frames / 60) as usize],
                _ => Space::Srgb,
            }
        }
    }

    struct Gpu {
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        pipeline: wgpu::RenderPipeline,
        config: wgpu::SurfaceConfiguration,
    }
    impl Gpu {
        fn configure(&mut self, space: Space, size: PhysicalSize<u32>, baseline: bool) {
            self.config.color_space = space;
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            let hal = unsafe { self.surface.as_hal::<wgpu::hal::api::Metal>() }.unwrap();
            let layer = hal.render_layer().lock();
            let actual = layer.colorspace();
            eprintln!(
                "CONFIGURE format={:?} space={space:?} actual={actual:?} size={size:?}",
                self.config.format
            );
            assert_eq!(
                actual.is_some(),
                !baseline,
                "surface must have an explicit color space"
            );
            assert!(
                !layer.wantsExtendedDynamicRangeContent(),
                "SDR must not enable EDR"
            );
            assert!(
                layer.framebufferOnly(),
                "same-format render-only path must stay framebuffer-only"
            );
        }
        fn draw(&self, window: &Window) -> bool {
            let frame = match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(f)
                | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                    return false;
                }
                s => panic!("acquire failed: {s:?}"),
            };
            let view = frame.texture.create_view(&Default::default());
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&self.pipeline);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit([encoder.finish()]);
            window.pre_present_notify();
            self.queue.present(frame);
            self.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .unwrap();
            true
        }
    }

    struct RunningState {
        window: Arc<Window>,
        gpu: Gpu,
    }

    struct App {
        mode: Mode,
        running: Option<RunningState>,
        frames: u32,
        configured: bool,
        resized: bool,
        deadline: Instant,
    }
    impl ApplicationHandler for App {
        fn resumed(&mut self, events: &ActiveEventLoop) {
            if self.running.is_some() {
                return;
            }
            let window = Arc::new(
                events
                    .create_window(
                        Window::default_attributes()
                            .with_title(format!("wgpu #10013 {}", self.mode.name()))
                            .with_inner_size(PhysicalSize::new(520, 400))
                            .with_position(winit::dpi::PhysicalPosition::new(100, 100)),
                    )
                    .unwrap(),
            );
            let (format, fragment_entry) = self.mode.render_target();
            let gpu = futures_lite::future::block_on(async {
                let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                    backends: wgpu::Backends::METAL,
                    ..wgpu::InstanceDescriptor::new_without_display_handle()
                });
                let surface = instance.create_surface(window.clone()).unwrap();
                let adapter = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        compatible_surface: Some(&surface),
                        ..Default::default()
                    })
                    .await
                    .unwrap();
                eprintln!("ADAPTER={:?}", adapter.get_info());
                let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("#10013 encoded color patches"),
                    source: wgpu::ShaderSource::Wgsl(SHADER.into()),
                });
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some(fragment_entry),
                        compilation_options: Default::default(),
                        targets: &[Some(format.into())],
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
                Gpu {
                    surface,
                    device,
                    queue,
                    pipeline,
                    config: wgpu::SurfaceConfiguration {
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        format,
                        color_space: Space::Srgb,
                        width: 520,
                        height: 400,
                        present_mode: wgpu::PresentMode::Fifo,
                        desired_maximum_frame_latency: 1,
                        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                        view_formats: vec![],
                    },
                }
            });
            self.running = Some(RunningState { window, gpu });
        }
        fn about_to_wait(&mut self, events: &ActiveEventLoop) {
            assert!(Instant::now() < self.deadline, "timed out");
            if let Some(state) = &self.running {
                state.window.request_redraw();
            }
            events.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(16),
            ));
        }
        fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
            let Some(RunningState { window, gpu }) = &mut self.running else {
                return;
            };
            match event {
                WindowEvent::CloseRequested => std::process::exit(1),
                WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                    self.configured = false
                }
                WindowEvent::RedrawRequested => {
                    if self.mode == Mode::Resize && self.frames == 60 && !self.resized {
                        let _ = window.request_inner_size(PhysicalSize::new(640, 400));
                        self.resized = true;
                        self.configured = false;
                        return;
                    }
                    if !self.configured {
                        gpu.configure(
                            self.mode.color_space(self.frames),
                            window.inner_size(),
                            self.mode == Mode::Baseline,
                        );
                        self.configured = true;
                        return;
                    }
                    if !gpu.draw(window) {
                        return;
                    }
                    self.frames += 1;
                    if self.frames.is_multiple_of(60) {
                        eprintln!("PRESENTED frames={}", self.frames);
                        if self.mode == Mode::Reconfigure {
                            self.configured = false;
                        }
                    }
                    if self.frames == 240 {
                        eprintln!("ACCEPTANCE_OK mode={}", self.mode.name());
                        events.exit();
                    }
                }
                _ => {}
            }
        }
    }
    pub fn main() {
        let mode = Mode::parse(&std::env::args().nth(1).unwrap_or_else(|| "srgb".into()));
        EventLoop::new()
            .unwrap()
            .run_app(&mut App {
                mode,
                running: None,
                frames: 0,
                configured: false,
                resized: false,
                deadline: Instant::now() + Duration::from_secs(30),
            })
            .unwrap();
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    test::main();
    #[cfg(not(target_os = "macos"))]
    panic!("requires macOS Metal");
}
