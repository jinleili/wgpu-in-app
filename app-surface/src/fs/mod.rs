use std::env;
use std::fs;
use std::path::PathBuf;

#[cfg_attr(target_os = "ios", path = "ios_fs.rs")]
#[cfg_attr(target_arch = "wasm32", path = "web_fs.rs")]
mod file_sys;

pub use file_sys::FileSystem;

pub fn get_texture_file_path(name: &str) -> PathBuf {
    let base_dir = application_root_dir();
    let f = FileSystem::new(&&base_dir);
    f.get_texture_file_path(name)
}

// Returns the cargo manifest directory when running the executable with cargo
// or the directory in which the executable resides otherwise,
// traversing symlinks if necessary.
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
