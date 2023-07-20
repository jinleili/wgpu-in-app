use crate::wgpu_canvas::WgpuCanvas;
use app_surface::{AppSurface, IOSViewObj};

#[no_mangle]
pub fn create_wgpu_canvas(ios_obj: IOSViewObj) -> *mut libc::c_void {
    println!(
        "create_wgpu_canvas, maximum frames: {}",
        ios_obj.maximum_frames
    );
    let obj = WgpuCanvas::new(AppSurface::new(ios_obj), 0_i32);
    // 使用 Box 对 Rust 对象进行装箱操作。
    // 我们无法将 Rust 对象直接传递给外部语言，通过装箱来传递此对象的胖指针
    let box_obj = Box::new(obj);
    // into_raw 返回指针的同时，将此对象的内存管理权转交给调用方
    Box::into_raw(box_obj) as *mut libc::c_void
}

// #[no_mangle]
// pub extern "C" fn enter_frame(obj: *mut libc::c_void) {
// // 将指针转换为其指代的实际 Rust 对象，同时也拿回此对象的内存管理权
// let mut obj: Box<WgpuCanvas> = unsafe { Box::from_raw(obj as *mut _) };
//     obj.enter_frame();
//     // 将 obj 对象的内存管理权重新转交给调用方
//     Box::into_raw(obj);
// }

#[no_mangle]
pub fn enter_frame(obj: *mut libc::c_void) {
    // 获取到指针指代的 Rust 对象的可变借用
    let obj = unsafe { &mut *(obj as *mut WgpuCanvas) };
    obj.enter_frame();
}

#[no_mangle]
pub fn change_example(obj: *mut libc::c_void, idx: i32) {
    let obj = unsafe { &mut *(obj as *mut WgpuCanvas) };
    obj.change_example(idx);
}
