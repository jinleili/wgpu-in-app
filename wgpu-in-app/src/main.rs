#![allow(clippy::single_match)]

#[cfg(any(target_os = "android", target_os = "ios", target_env = "ohos"))]
fn main() {}

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    all(target_os = "linux", not(target_env = "ohos"))
))]
fn main() -> Result<(), impl std::error::Error> {
    wgpu_in_app::desktop::run()
}
