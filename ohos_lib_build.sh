#!/usr/bin/env bash

# Stop subsequent execution when encountering any errors
set -e

RELEASE_MODE=${1}
LIB_FOLDER="debug"

# build to OpenHarmony target
cd wgpu-in-app
if [ "${RELEASE_MODE}" = "--release" ]; then
    LIB_FOLDER="release"
    ohrs build ${RELEASE_MODE}
else
    RUST_BACKTRACE=full RUST_LOG=wgpu_hal=debug ohrs build
fi

# copy .so files to jniLibs folder
cd ../
ARM64="OpenHarmony/entry/libs/arm64-v8a"
ARMv7a="OpenHarmony/entry/libs/armeabi-v7a"
X86_64="OpenHarmony/entry/libs/x86_64"

if [ ! -d "$ARM64" ]; then
    mkdir -p "$ARM64"
fi
if [ ! -d "$ARMv7a" ]; then
    mkdir -p "$ARMv7a"
fi
if [ ! -d "$X86_64" ]; then
    mkdir -p "$X86_64"
fi

cp target/aarch64-unknown-linux-ohos/${LIB_FOLDER}/libwgpu_in_app.so "${ARM64}/libwgpu_in_app.so"
cp target/armv7-unknown-linux-ohos/${LIB_FOLDER}/libwgpu_in_app.so "${ARMv7a}/libwgpu_in_app.so"
cp target/x86_64-unknown-linux-ohos/${LIB_FOLDER}/libwgpu_in_app.so "${X86_64}/libwgpu_in_app.so"
