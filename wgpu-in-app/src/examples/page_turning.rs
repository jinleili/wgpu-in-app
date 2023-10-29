use super::Example;
use app_surface::{AppSurface, SurfaceFrame};

use zerocopy::{AsBytes, FromBytes};

use std::cell::RefCell;
use std::cmp::PartialEq;
use std::f32::consts::FRAC_PI_2;
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, Copy, Debug, AsBytes, FromBytes)]
struct TurningUniform {
    radius: f32,
    angle: f32,
    np: [f32; 2],
    n: [f32; 2],
    alpha: f32,
    any: f32,
}

#[derive(PartialEq)]
enum TurningAnimationStatus {
    ScaleIn,
    Turning,
    ScaleOut,
}

pub struct PageTurning {
    paper: Rc<RefCell<BackPaper>>,
    page_rect: Rect,
    turning_buf: BufferObj,
    turning_uniform: TurningUniform,
    turning_node: ViewNode,
    depth_texture_view: wgpu::TextureView,
    is_animating: bool,
    roll_length: f32,
    start_pos: Position,
    target_pos: Position,
    gap_pos: Position,
    animate_index: u32,
    pub draw_count: u32,
}

fn init_turning_uniform() -> TurningUniform {
    TurningUniform {
        radius: 1.0 / 8.0,
        angle: 0.0,
        alpha: 1.0,
        np: [0.0, 0.0],
        n: [0.0, 0.0],
        any: 0.0,
    }
}

impl PageTurning {
    pub fn new(app_surface: &mut AppSurface) -> Self {
        let device = &app_surface.device;
        let queue = &app_surface.queue;
        let config = &app_surface.config;
        let scale_factor = app_surface.scale_factor;
        let encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let turning_uniform = init_turning_uniform();
        let turning_buf = BufferObj::create_uniform_buffer(device, &turning_uniform, None);

        // Create the vertex and index buffers
        let vpw = config.width as f32;
        let vph = config.height as f32;
        let (vertices, indices) = Plane::new_by_rect(
            page_rect,
            (vpw / (scale_factor * 2.0)) as u32,
            (vph / (scale_factor * 2.0)) as u32,
        )
        .generate_vertices_by_texcoord2(Rect::from_origin_n_size(0.0, 0.0, 1.0, 1.0), None);

        let bg_texture = &paper.borrow().texture;
        let sampler = &paper.borrow().sampler;
        let builder = ViewNodeBuilder::<PosTex2>::new(
            vec![(bg_texture, None), (tex, None)],
            &shaders.page_turning,
        )
        .with_uniform_buffers(vec![view_mvp_buf, &turning_buf])
        .with_vertices_and_indices((vertices, indices))
        .with_samplers(vec![sampler])
        .with_use_depth_stencil(true)
        .with_shader_stages(vec![
            wgpu::ShaderStages::VERTEX,
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            wgpu::ShaderStages::FRAGMENT,
            wgpu::ShaderStages::FRAGMENT,
            wgpu::ShaderStages::FRAGMENT,
        ]);
        let turning_node = builder.build(device);

        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let depth_texture_view = crate::depth_stencil::create_depth_texture_view(size, device);
        queue.submit(Some(encoder.finish()));

        let start_pos = Position::new(1.0, 0.0);
        //  NDC 坐标是 （-1， -1）在左下角
        let target_pos = Position::new(-5.8, 2.5);

        let maximum_frames = 60;
        let instance = PageTurning {
            paper: paper.clone(),
            page_rect,
            turning_buf,
            turning_uniform,
            turning_node,
            depth_texture_view,
            is_animating: false,
            start_pos,
            target_pos,
            gap_pos: target_pos.minus(&start_pos),
            roll_length: 0.0,
            animate_index: 0,
            draw_count: (maximum_frames * 1.0) as u32,
        };

        instance
    }

    pub fn reset_status(&mut self) {
        self.is_animating = false;
        self.turning_uniform.np = Position::zero().into();
        self.turning_uniform.n = Position::zero().into();
        self.animate_index = 0;
    }

    fn step_frame_data(&mut self, is_out: bool) -> bool {
        if self.animate_index > self.draw_count {
            return true;
        }
        let is_stopped = false;
        self.animate_index += 1;
        // 由慢到快的缓动效果
        let step = 1.0 - (FRAC_PI_2 * (self.animate_index as f32 / self.draw_count as f32)).cos();
        let step_pos = self.gap_pos.multiply_f(step);
        let (dx, dy) = if is_out {
            (-step_pos.x, -step_pos.y)
        } else {
            (step_pos.x, step_pos.y)
        };

        let distance = (dx * dx + dy * dy).sqrt();
        let half_circle = std::f32::consts::PI * self.turning_uniform.radius;
        let pi_2 = FRAC_PI_2;

        let a = -dy.atan2(dx);
        let sin_a = a.sin();
        let cos_a = a.cos();
        // 最大可卷起距离
        let mut max_roll = 0.0;
        if a < pi_2 && a > (-pi_2) {
            max_roll = (cos_a * (2.0 * 2.0)).abs();
        }

        // 实际的卷起距离
        self.roll_length = distance;
        if distance > half_circle {
            self.roll_length = (distance - half_circle) / 2.0 + half_circle;
        }
        if self.roll_length > max_roll {
            self.roll_length = max_roll;
        }
        self.turning_uniform.angle = a;
        self.turning_uniform.np = [
            self.page_rect.center_x() - (cos_a * self.roll_length).abs(),
            self.page_rect.center_y() * (if a > 0.0 { 1.0 } else { -1.0 })
                - sin_a * self.roll_length,
        ];
        self.turning_uniform.n = [cos_a, sin_a];

        is_stopped
    }
}

impl Example for PageTurning {
    fn enter_frame(&mut self, app_surface: &AppSurface) {
        let device = &app_surface.device;
        let queue = &app_surface.queue;
        // instance 0 只绘制 paper, instance 1 计算 turning
        let instance_count: u32 = 2;
        let is_turing_completed = self.step_frame_data(true);
        if is_turing_completed {
            self.is_animating = false;
        } else {
            self.is_animating = true;
        }
        queue.write_buffer(
            &self.turning_buf.buffer,
            0,
            bytemuck::cast_slice(&self.turning_uniform),
        );
        let (frame, view) = app_surface.get_current_frame_view();
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.paper.borrow().clear_color),
                    store: wgpu::StoreOp::Store,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
        });
        self.turning_node
            .draw_by_instance_count(&mut rpass, instance_count);
    }
}
