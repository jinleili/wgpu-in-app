mod examples;
mod wgpu_canvas;
pub use wgpu_canvas::WgpuCanvas;
#[cfg(target_env = "ohos")]
pub mod ohos;

#[cfg_attr(target_os = "ios", path = "ffi/ios.rs")]
#[cfg_attr(target_os = "android", path = "ffi/android.rs", allow(non_snake_case))]
mod ffi;

#[cfg(all(target_os = "android", target_os = "ios"))]
pub use ffi::*;

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    all(target_os = "linux", not(target_env = "ohos"))
))]
pub mod desktop;

// Initialize logging in platform dependant ways.
fn init_logger() {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "android")] {
            // 添加 Android 平台的日志初始化
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Info)
            );
            log_panics::init();
        } else {
            // parse_default_env will read the RUST_LOG environment variable and apply it on top
            // of these default filters.
            env_logger::builder()
                .filter_level(log::LevelFilter::Info)
                // We keep wgpu at Error level, as it's very noisy.
                .filter_module("wgpu_core", log::LevelFilter::Info)
                .filter_module("wgpu_hal", log::LevelFilter::Error)
                .filter_module("naga", log::LevelFilter::Error)
                .parse_default_env()
                .init();
        }
    }
}
