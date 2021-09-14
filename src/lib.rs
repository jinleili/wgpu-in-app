use std::env;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;

mod wgpu_canvas;

mod ffi;
pub use ffi::*;

#[repr(C)]
#[derive(Debug)]
pub struct ViewSize {
    pub width: u32,
    pub height: u32,
}

mod ios_view;
pub use ios_view::{AppView, IOSViewObj};
pub struct AppViewWrapper(pub AppView);
// `*mut libc::c_void` cannot be sent between threads safely
unsafe impl Send for AppViewWrapper {}
unsafe impl Sync for AppViewWrapper {}

impl Deref for AppViewWrapper {
    type Target = AppView;

    fn deref(&self) -> &AppView {
        &self.0
    }
}

#[cfg(target_arch = "wasm32")]
pub fn application_root_dir() -> String {
    let host = web_sys::window().unwrap().location().host().unwrap();
    "http://".to_string() + &host
}

#[cfg(not(target_arch = "wasm32"))]
pub fn application_root_dir() -> String {
    match env::var("PROFILE") {
        Ok(_) => String::from(env!("CARGO_MANIFEST_DIR")),
        Err(_) => {
            let mut path = env::current_exe().expect("Failed to find executable path.");
            while let Ok(target) = fs::read_link(path.clone()) {
                path = target;
            }
            if cfg!(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux"
            )) {
                path = path.join("../../../").canonicalize().unwrap();
            }
            String::from(
                path.parent()
                    .expect("Failed to get parent directory of the executable.")
                    .to_str()
                    .unwrap(),
            )
        }
    }
}

use lazy_static::*;
use objc::{
    rc::StrongPtr,
    runtime::{Class, Object},
    *,
};
use objc_foundation::{INSString, NSString};

lazy_static! {
    static ref BUNDLE_PATH: &'static str = get_bundle_url();
}

fn get_bundle_url() -> &'static str {
    let cls = class!(NSBundle);
    let path: &str = unsafe {
        // Allocate an instance
        let bundle: *mut Object = msg_send![cls, mainBundle];
        // let url: *mut Object = msg_send![*bundle, resourcePath];
        // 资源路径要用 resourcePath
        let path: &NSString = msg_send![bundle, resourcePath];
        path.as_str()
    };
    path
}

pub fn get_wgsl_path(name: &str) -> PathBuf {
    let base_dir = application_root_dir();
    let p = get_bundle_url().to_string() + "/wgsl_shader/" + name;
    PathBuf::from(&p)
}
