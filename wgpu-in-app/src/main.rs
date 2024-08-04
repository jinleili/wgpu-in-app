#![allow(clippy::single_match)]

#[cfg(any(target_os = "android", target_os = "ios"))]
fn main() {}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn main() -> Result<(), impl std::error::Error> {
    wgpu_in_app::desktop::run()
}
