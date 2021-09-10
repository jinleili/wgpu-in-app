use crate::wgpu_canvas::WgpuCanvas;
use crate::{AppView, IOSViewObj};

pub trait SurfaceView {
    fn resize(&mut self) {}
    fn enter_frame(&mut self);
}

#[no_mangle]
pub unsafe extern "C" fn create_wgpu_canvas(ios_obj: IOSViewObj) -> *mut libc::c_void {
    let rust_view = AppView::new(ios_obj);
    let obj = WgpuCanvas::new(rust_view);

    box_obj(obj)
}

#[no_mangle]
pub unsafe extern "C" fn enter_frame(obj: *mut libc::c_void) -> *mut libc::c_void {
    let mut obj: Box<Box<dyn SurfaceView>> = Box::from_raw(obj as *mut _);
    obj.enter_frame();

    Box::into_raw(obj) as *mut libc::c_void
}

fn box_obj(obj: impl SurfaceView) -> *mut libc::c_void {
    let boxed_trait: Box<dyn SurfaceView> = Box::new(obj);
    let boxed_boxed_trait = Box::new(boxed_trait);
    let heap_pointer = Box::into_raw(boxed_boxed_trait);
    // let boxed_boxed_trait = Box::new(v);
    // let heap_pointer = Box::into_raw(boxed_boxed_trait);
    heap_pointer as *mut libc::c_void
}
