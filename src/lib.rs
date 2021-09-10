use std::env;
use std::fs;
use std::ops::Deref;

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
