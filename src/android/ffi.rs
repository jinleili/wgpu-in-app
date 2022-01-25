use crate::{wgpu_canvas::WgpuCanvas, AppView};
use android_logger::Config;
use jni::objects::JClass;
use jni::sys::{jint, jlong, jobject};
use jni::JNIEnv;
use jni_fn::jni_fn;
use log::{info, Level};

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub unsafe fn createWgpuCanvas(env: *mut JNIEnv, _: JClass, surface: jobject, idx: jint) -> jlong {
    android_logger::init_once(Config::default().with_min_level(Level::Debug));
    let canvas = WgpuCanvas::new(AppView::new(env as *mut _, surface), idx as i32);
    info!("WgpuCanvas created!");
    Box::into_raw(Box::new(canvas)) as jlong
}

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub unsafe fn enterFrame(_env: *mut JNIEnv, _: JClass, obj: jlong) {
    let obj = &mut *(obj as *mut WgpuCanvas);
    obj.enter_frame();
}

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub unsafe fn changeExample(_env: *mut JNIEnv, _: JClass, obj: jlong, idx: jint) {
    let obj = &mut *(obj as *mut WgpuCanvas);
    obj.change_example(idx as i32);
}

#[no_mangle]
#[jni_fn("name.jinleili.wgpu.RustBridge")]
pub unsafe fn dropWgpuCanvas(_env: *mut JNIEnv, _: JClass, obj: jlong) {
    let _obj: Box<WgpuCanvas> = Box::from_raw(obj as *mut _);
}
