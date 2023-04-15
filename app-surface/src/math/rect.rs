use super::Position;
use crate::math::Size;

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub origin: Position,
    pub size: Size<f32>,
}

impl Rect {
    pub fn new(width: f32, height: f32, center_to: Position) -> Self {
        let x = center_to.x - width / 2.0;
        let y = center_to.y + height / 2.0;
        Rect {
            x,
            y,
            width,
            height,
            origin: Position::new(x, y),
            size: (width, height).into(),
        }
    }

    pub fn get_standard_new() -> Self {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            origin: Position::zero(),
            size: (1.0, 1.0).into(),
        }
    }

    pub fn from_origin_n_size(x: f32, y: f32, width: f32, height: f32) -> Self {
        Rect {
            x,
            y,
            width,
            height,
            origin: Position::new(x, y),
            size: (width, height).into(),
        }
    }

    pub fn zero() -> Self {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            origin: Position::zero(),
            size: (0.0, 0.0).into(),
        }
    }

    // 将像素坐标转换成NDC空间中的坐标
    // 这个空间可能不是当前可见视口，需要传入实际 reander target 的尺寸
    pub fn get_std_coord(&self, viewport_size: Size<f32>) -> Self {
        let half_w = viewport_size.width / 2.0;
        let half_h = viewport_size.height / 2.0;
        // 像素在NDC空间对应的值
        let x = (self.x - half_w) / half_w;
        let mut y = (self.y - half_h) / half_h;
        // 反转 y 坐标
        y *= -1.0;
        let width = self.width / half_w;
        let height = self.height / half_h;

        Rect {
            x,
            y,
            width,
            height,
            origin: Position::new(x, y),
            size: (width, height).into(),
        }
    }

    pub fn center_x(&self) -> f32 {
        self.x + self.width / 2.0
    }

    pub fn center_y(&self) -> f32 {
        self.y + self.height / 2.0
    }

    pub fn anchor(&self) -> Position {
        Position::new(self.center_x(), self.center_y())
    }

    // 中心点移动到坐标原点
    pub fn move_anchor_to_origin(&mut self) {
        self.x = -self.width / 2.0;
        self.y = self.height / 2.0;
        self.origin = Position::new(self.x, self.y);
    }

    // 一个正交投影坐标是否在区域内
    pub fn is_ortho_intersect(&self, ortho_point: Position) -> bool {
        let x_left = -self.center_x();
        let x_right = self.center_x();
        let y_top = self.center_y();
        let y_bottom = -self.center_y();

        ortho_point.x >= x_left
            && ortho_point.x <= x_right
            && ortho_point.y >= y_bottom
            && ortho_point.y <= y_top
    }
}
