#[cfg(all(
    not(feature = "winit"),
    any(
        all(not(feature = "mac_catalyst"), target_os = "macos"),
        target_os = "windows",
        target_os = "linux",
    ),
))]
compile_error!(
    "app-surface desktop targets require the `winit` feature. Enable default features or add `features = [\"winit\"]`."
);

#[cfg(all(
    target_arch = "wasm32",
    not(feature = "winit"),
    not(feature = "web_rwh")
))]
compile_error!(
    "app-surface web targets require either the `winit` feature or the `web_rwh` feature."
);
