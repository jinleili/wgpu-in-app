# wgpu on App
Integrate wgpu on your existing iOS | Android apps. 

`winit`-free mode can useful for integrating wgpu into existing apps to implement some graphics features. 

## Screenshot
|![wgpu on iOS](screenshot/on_ios.png)|![wgpu on Android](screenshot/on_android.png)|
|---|---|


## **iOS**
### Add build target

Since iPhone 5 and later are 64-bit devices, `armv7s-apple-ios` and `armv7-apple-ios` are not required.

```sh
# Add hardware iOS target
rustup target add aarch64-apple-ios 
# Add iOS simulator target
# Intel CPU Mac
rustup target add x86_64-apple-ios
# M1 Mac
rustup target add aarch64-apple-ios-sim
```

### Build and run
Build for iOS device and simulator
```sh
# Use Metal backend
cargo build --target aarch64-apple-ios && cargo build --target <aarch64-apple-ios-sim or x86_64-apple-ios>
```

Then, Open `iOS/wgpu_on_iOS.xcodeproj` with Xcode, select `wgpu_phone` scheme run on iOS device, `wgpu_simulator` scheme run on simulator. 

### About Rust and Swift Interaction
[Passing Primitive Data Between Rust and iOS](https://bignerdranch.com/blog/building-an-ios-app-in-rust-part-2-passing-primitive-data-between-rust-and-ios/)

[Passing Owned Objects between Rust and iOS](https://bignerdranch.com/blog/building-an-ios-app-in-rust-part-3-passing-owned-objects-between-rust-and-ios/)

[Sending a boxed trait over FFI](https://users.rust-lang.org/t/sending-a-boxed-trait-over-ffi/21708)

## **Android**
### Set up Android environment

Assuming your computer already has Android Studio installed, go to `Android Studio` > `Tools` > `SDK Manager` > `Android SDK` > `SDK Tools`. Check the following options for installation and click OK. 

- [x] Android SDK Build-Tools
- [x] Android SDK Command-line Tools
- [x] NDK(Side by side)

Then, set two following environment variables:

```sh
export ANDROID_SDK_ROOT=$HOME/Library/Android/sdk
# Replace the NDK version number with the version you installed 
export NDK_HOME=$ANDROID_SDK_ROOT/ndk/23.1.7779620
```


### Add build targets

Since simulator and virtual devices only support GLES, `x86_64-linux-android` and `i686-linux-android` targets are not necessary
```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
```
### Install cargo so subcommand
```sh
# from crates.io
cargo install cargo-so
# frome source code
cargo install --path cargo-so
```

### Build
```sh
# currently, multi-target build only useable on the nightly channel
cargo so b --lib --target aarch64-linux-android --release
cargo so b --lib --target armv7-linux-androideabi --release

# copy .so files to android project jniLibs folder
cp target/aarch64-linux-android/release/libwgpu_on_app.so android/app/libs/arm64-v8a/libwgpu_on_app.so
cp target/armv7-linux-androideabi/release/libwgpu_on_app.so android/app/libs/armeabi-v7a/libwgpu_on_app.so
```

## **Desktop**
```sh
# Use Metal backend.
cargo run
# Use GL backend, need `Angle` libs on your computer.
# https://github.com/gfx-rs/wgpu/pull/2461
WGPU_BACKEND=gl cargo run --features angle
# Use Vulkan backend
# https://github.com/gfx-rs/wgpu/pull/2488
cargo run --features vulkan-portability
# Then, press 0, 1, 2, 3, 4 keys change running example.
```