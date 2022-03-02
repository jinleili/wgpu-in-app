#[derive(Copy, Clone, Debug)]
pub struct EdgeInset {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

mod rect;
pub use rect::Rect;

mod size;
pub use size::Size;

mod position;
pub use position::Position;
