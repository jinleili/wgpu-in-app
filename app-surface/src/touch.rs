#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Position<T> {
    pub x: T,
    pub y: T,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StylusAngle<T> {
    pub azimuth: T,
    pub altitude: T,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub enum TouchPhase {
    Started,
    Moved, // Or pintch changed
    Ended,
    Cancelled,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Touch {
    pub phase: TouchPhase,
    pub position: Position<f32>,
    // The angle of the stylus: Apple Pencil
    pub stylus_angle: Option<StylusAngle<f32>>,
    pub pressure: f32,
    // The radius of the contact ellipse along the major axis, in logical pixels.
    pub major_radius: f32,
    // Time interval from the previous touch point
    pub interval: f32,
}
