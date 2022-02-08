use crate::wgpu_canvas::WgpuCanvas;
use app_surface::{AppSurface, IOSViewObj};

#[no_mangle]
pub unsafe extern "C" fn create_wgpu_canvas(ios_obj: IOSViewObj) -> *mut libc::c_void {
    let obj = WgpuCanvas::new(AppSurface::new(ios_obj), 0_i32);
    let box_obj = Box::new(obj);
    let heap_pointer = Box::into_raw(box_obj);

    heap_pointer as *mut libc::c_void
}

#[no_mangle]
pub unsafe extern "C" fn enter_frame(obj: *mut libc::c_void) {
    let mut obj: Box<WgpuCanvas> = Box::from_raw(obj as *mut _);
    obj.enter_frame();
    Box::into_raw(obj);
}

#[no_mangle]
pub unsafe extern "C" fn change_example(obj: *mut libc::c_void, idx: i32) {
    let mut obj: Box<WgpuCanvas> = Box::from_raw(obj as *mut _);
    obj.change_example(idx);
    Box::into_raw(obj);
}
