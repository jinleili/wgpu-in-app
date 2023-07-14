#!/usr/bin/env bash

# Stop subsequent execution when encountering any errors
set -e

TARGET=${1}
RELEASE_MODE=${2}

if [ ! ${TARGET} ]; then
    : ${TARGET:=aarch64-apple-ios}
fi

if [ "${TARGET}" = "--release" ]; then
    TARGET="aarch64-apple-ios"
    : ${RELEASE_MODE:=--release}
fi

cargo build --target ${TARGET} ${RELEASE_MODE}

# Copy .a file to iOS/ipadOS/visionOS project
# 
# Why copy?
# On Xcode 14.1, when xxx..dylib file exists in the library search path, Xcode will try to reference it and report an error:
# Dylib (/Users/XXX/wgpu-in-app/target/aarch64-apple-ios/debug/libwgpu_in_app.dylib) was built for newer iOS version (16.1) than being linked (13.0)
LIB_FOLDER=
case ${RELEASE_MODE} in
    "--release") : ${LIB_FOLDER:=release} ;;
    *) : ${LIB_FOLDER:=debug} ;;
esac

if [ ! -d "Apple/libs/${LIB_FOLDER}/" ]; then
  mkdir -p "Apple/libs/${LIB_FOLDER}"
fi

cp target/${TARGET}/${LIB_FOLDER}/libwgpu_in_app.a Apple/libs/${LIB_FOLDER}/libwgpu_in_app.a