# build to Android targets
cargo so b --lib --target aarch64-linux-android --release
cargo so b --lib --target armv7-linux-androideabi --release
# copy .so files to jniLibs folder
cp target/aarch64-linux-android/release/libwgpu_on_app.so android/app/libs/arm64-v8a/libwgpu_on_app.so
cp target/armv7-linux-androideabi/release/libwgpu_on_app.so android/app/libs/armeabi-v7a/libwgpu_on_app.so
