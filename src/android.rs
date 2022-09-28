use crate::wgpu_canvas::WgpuCanvas;
use android_logger::Config;
use app_surface::AppSurface;
use jni::objects::JClass;
use jni::sys::{jint, jlong, jobject};
use jni::JNIEnv;
use jni_fn::jni_fn;
use log::{info, Level};

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub fn createWgpuCanvas(env: *mut JNIEnv, _: JClass, surface: jobject, idx: jint) -> jlong {
    android_logger::init_once(Config::default().with_min_level(Level::Trace));
    let canvas = WgpuCanvas::new(AppSurface::new(env as *mut _, surface), idx as i32);
    info!("WgpuCanvas created!");
    Box::into_raw(Box::new(canvas)) as jlong
}

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub fn enterFrame(_env: *mut JNIEnv, _: JClass, obj: jlong) {
    let obj = unsafe { &mut *(obj as *mut WgpuCanvas) };
    obj.enter_frame();
    // 将 obj 对象的内存管理权重新转交给调用方
    Box::into_raw(Box::new(obj));
}

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub fn changeExample(_env: *mut JNIEnv, _: JClass, obj: jlong, idx: jint) {
    // 获取到指针指代的 Rust 对象的可变借用
    let obj = unsafe { &mut *(obj as *mut WgpuCanvas) };
    obj.change_example(idx as i32);
}

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub fn dropWgpuCanvas(_env: *mut JNIEnv, _: JClass, obj: jlong) {
    let _obj: Box<WgpuCanvas> = unsafe { Box::from_raw(obj as *mut _) };
}
