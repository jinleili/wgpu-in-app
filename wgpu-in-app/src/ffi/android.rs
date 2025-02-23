use crate::wgpu_canvas::WgpuCanvas;
use app_surface::AppSurface;
use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::{jint, jlong, jobject};
use jni_fn::jni_fn;
use log::info;

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub fn createWgpuCanvas(env: *mut JNIEnv, _: JClass, surface: jobject, idx: jint) -> jlong {
    crate::init_logger();

    let canvas = WgpuCanvas::new(AppSurface::new(env as *mut _, surface), idx);
    info!("WgpuCanvas created!");

    Box::into_raw(Box::new(canvas)) as jlong
}

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub fn enterFrame(_env: *mut JNIEnv, _: JClass, obj: jlong) {
    let obj = unsafe { &mut *(obj as *mut WgpuCanvas) };
    obj.enter_frame();
}

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub fn changeExample(_env: *mut JNIEnv, _: JClass, obj: jlong, idx: jint) {
    // 获取到指针指代的 Rust 对象的可变借用
    let obj = unsafe { &mut *(obj as *mut WgpuCanvas) };
    obj.change_example(idx);
}

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub fn dropWgpuCanvas(_env: *mut JNIEnv, _: JClass, obj: jlong) {
    let _obj: Box<WgpuCanvas> = unsafe { Box::from_raw(obj as *mut _) };
}
