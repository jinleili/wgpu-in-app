## test wgpu run on iOS

first, build rust:

```sh
cargo build --target aarch64-apple-ios
```

then, use Xcode to open ```iOS/wgpu_test.xcodeproj``` and connect iOS device to run.

