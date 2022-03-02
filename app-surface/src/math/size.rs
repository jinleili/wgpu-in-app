use super::{EdgeInset, Rect};

#[derive(Copy, Clone, Debug)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl Size<f32> {
    // 计算等比填充的 rect 坐标
    // margin： 填充区的边界
    // fill_size： 待缩放填充图形的原始尺寸
    pub fn aspect_fit(&self, margin: EdgeInset, fill_size: Size<f32>) -> Rect {
        // 待填充区
        let valid_w = self.width - margin.left - margin.right;
        let valid_h = self.height - margin.top - margin.bottom;
        let self_ratio = valid_w / valid_h;
        let external_size_ratio = fill_size.width / fill_size.height;
        let ratio = if external_size_ratio > self_ratio {
            // 按宽顶边计算缩放比
            valid_w / fill_size.width
        } else {
            valid_h / fill_size.height
        };
        // 缩放后的 size
        let new_fill_size: Size<f32> = (fill_size.width * ratio, fill_size.height * ratio).into();
        Rect::from_origin_n_size(
            margin.left + (valid_w - new_fill_size.width) / 2.0,
            margin.top + (valid_h - new_fill_size.height) / 2.0,
            new_fill_size.width,
            new_fill_size.height,
        )
    }
}

impl From<[u32; 2]> for Size<u32> {
    fn from(vs: [u32; 2]) -> Self {
        Size { width: vs[0], height: vs[1] }
    }
}

impl From<Size<u32>> for [u32; 2] {
    fn from(s: Size<u32>) -> Self {
        [s.width, s.height]
    }
}

impl From<(u32, u32)> for Size<u32> {
    fn from(data: (u32, u32)) -> Self {
        Size { width: data.0, height: data.1 }
    }
}

impl From<Size<u32>> for (u32, u32) {
    fn from(s: Size<u32>) -> Self {
        (s.width, s.height)
    }
}
impl From<wgpu::Extent3d> for Size<f32> {
    fn from(data: wgpu::Extent3d) -> Self {
        Size { width: data.width as f32, height: data.height as f32 }
    }
}
impl From<wgpu::Extent3d> for Size<u32> {
    fn from(data: wgpu::Extent3d) -> Self {
        Size { width: data.width, height: data.height }
    }
}

impl From<[f32; 2]> for Size<f32> {
    fn from(vs: [f32; 2]) -> Self {
        Size { width: vs[0], height: vs[1] }
    }
}

impl From<Size<f32>> for [f32; 2] {
    fn from(s: Size<f32>) -> Self {
        [s.width, s.height]
    }
}

impl From<(f32, f32)> for Size<f32> {
    fn from(data: (f32, f32)) -> Self {
        Size { width: data.0, height: data.1 }
    }
}

impl From<Size<f32>> for (f32, f32) {
    fn from(s: Size<f32>) -> Self {
        (s.width, s.height)
    }
}

impl From<super::Position> for Size<f32> {
    fn from(data: super::Position) -> Self {
        Size { width: data.x, height: data.y }
    }
}

impl From<&wgpu::SurfaceConfiguration> for Size<f32> {
    fn from(data: &wgpu::SurfaceConfiguration) -> Self {
        Size { width: data.width as f32, height: data.height as f32 }
    }
}

impl From<&wgpu::SurfaceConfiguration> for Size<u32> {
    fn from(data: &wgpu::SurfaceConfiguration) -> Self {
        Size { width: data.width, height: data.height }
    }
}
