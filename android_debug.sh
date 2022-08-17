#!/usr/bin/env bash

# build to Android target
RUST_BACKTRACE=full RUST_LOG=wgpu_hal=debug cargo so b --lib --target aarch64-linux-android
# RUST_LOG=wgpu_hal=debug cargo so b --features angle --lib --target armv7-linux-androideabi
# RUST_BACKTRACE=full RUST_LOG=wgpu_hal=debug cargo so b --lib --target aarch64-linux-android 
# RUST_BACKTRACE=full RUST_LOG=wgpu_hal=debug cargo so b --lib --target armv7-linux-androideabi

# copy .so files to jniLibs folder
ARM64="android/app/libs/arm64-v8a"
ARMv7a="android/app/libs/armeabi-v7a"

if [ ! -d "$ARM64" ]; then
    mkdir "$ARM64"
fi
if [ ! -d "$ARMv7a" ]; then
    mkdir "$ARMv7a"
fi

cp target/aarch64-linux-android/debug/libwgpu_on_app.so "${ARM64}/libwgpu_on_app.so"
# cp target/armv7-linux-androideabi/debug/libwgpu_on_app.so "${ARMv7a}/libwgpu_on_app.so"
