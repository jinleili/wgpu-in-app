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
    pub position: crate::math::Position,
    // The angle of the stylus: Apple Pencil
    pub stylus_angle: Option<StylusAngle<f32>>,
    pub pressure: f32,
    // The radius of the contact ellipse along the major axis, in logical pixels.
    pub major_radius: f32,
    // Time interval from the previous touch point
    pub interval: f32,
}

impl Touch {
    pub fn touch_start(position: crate::math::Position) -> Self {
        Self::new(position, TouchPhase::Started)
    }

    pub fn touch_move(position: crate::math::Position) -> Self {
        Self::new(position, TouchPhase::Moved)
    }

    pub fn touch_end(position: crate::math::Position) -> Self {
        Self::new(position, TouchPhase::Ended)
    }

    fn new(position: crate::math::Position, phase: TouchPhase) -> Self {
        Touch {
            position,
            phase,
            stylus_angle: None,
            pressure: 0.0,
            major_radius: 0.0,
            interval: 0.0,
        }
    }
}
