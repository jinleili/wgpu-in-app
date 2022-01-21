# wgpu on App
Integrate wgpu on your existing iOS | Android apps. 

## boids demo screenshot
![wgpu on iOS](screenshot/on_ios.jpg) ![wgpu on Android](screenshot/on_android.jpg) 

## iOS
Adding build target for iOS. Since iPhone 5 and later are 64-bit devices, add `aarch64-apple-ios` is enough.
```sh
rustup target add aarch64-apple-ios
```

Build to iOS target.
```sh
cargo build --target aarch64-apple-ios
```

Then, use Xcode to open ```iOS/wgpu_test.xcodeproj``` and connect iOS device to run.

##### More about Rust and Swift Interaction
[Passing Primitive Data Between Rust and iOS](https://bignerdranch.com/blog/building-an-ios-app-in-rust-part-2-passing-primitive-data-between-rust-and-ios/)

[Passing Owned Objects between Rust and iOS](https://bignerdranch.com/blog/building-an-ios-app-in-rust-part-3-passing-owned-objects-between-rust-and-ios/)

[Sending a boxed trait over FFI](https://users.rust-lang.org/t/sending-a-boxed-trait-over-ffi/21708)

## Android
```sh
# Adding build targets for Android
rustup target add aarch64-linux-android armv7-linux-androideabi

# install cargo-so subcommand
cargo install --path cargo-so

# build (currently, multi-target build only useable on the nightly channel)
cargo so b --target aarch64-linux-android --release
cargo so b --target armv7-linux-androideabi --release

# copy .so files to android project jniLibs folder
cp target/aarch64-linux-android/release/libwgpu_on_app.so android/app/libs/arm64-v8a/libwgpu_on_app.so
cp target/armv7-linux-androideabi/release/libwgpu_on_app.so android/app/libs/armeabi-v7a/libwgpu_on_app.so
```

