# cargo so

Tool for compile Rust package to Android `.so` lib files.
Code modified from [cargo-apk](https://github.com/rust-mobile/cargo-apk)

## Installation

```sh
cargo install --path .
```

## Usage

Assuming your computer already has Android Studio installed, go to `Android Studio` > `Tools` > `SDK Manager` > `Android SDK` > `SDK Tools`. Check the following options for installation and click OK.

- [x] Android SDK Build-Tools
- [x] Android SDK Command-line Tools
- [x] NDK(Side by side)

Set two following environment variables:

```sh
export ANDROID_SDK_ROOT=$HOME/Library/Android/sdk
# Replace the NDK version number with the version you installed
export NDK_HOME=$ANDROID_SDK_ROOT/ndk/28.0.12433566
```

Add build targets

```sh
rustup target add armv7-linux-androideabi aarch64-linux-android i686-linux-android x86_64-linux-android
```

Build

```sh
# build to all android targets
cargo so build --lib
# build to the specified target
cargo so b --lib --target aarch64-linux-android
```
