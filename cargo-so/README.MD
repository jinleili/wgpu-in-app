# cargo so

Tool for compile Rust package to Android `.so` lib files. 
Code modified from [cargo-apk](https://github.com/rust-windowing/android-ndk-rs/tree/master/cargo-apk)

## Installation

```sh
cargo install --path .
```

## Usage

```sh
# add build targets
rustup target add armv7-linux-androideabi aarch64-linux-android i686-linux-android x86_64-linux-android
# build to all android targets
cargo so build
# build to the specified target
cargo so b --target aarch64-linux-android
```