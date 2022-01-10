# wgpu on iOS 
Run wgpu on iOS without using `winit`. 

winit-free mode can useful for integrating wgpu into existing apps to implement certain graphics features. 

## Interaction between Swift and Rust
[Passing Primitive Data Between Rust and iOS](https://bignerdranch.com/blog/building-an-ios-app-in-rust-part-2-passing-primitive-data-between-rust-and-ios/)

[Passing Owned Objects between Rust and iOS](https://bignerdranch.com/blog/building-an-ios-app-in-rust-part-3-passing-owned-objects-between-rust-and-ios/)

[Sending a boxed trait over FFI](https://users.rust-lang.org/t/sending-a-boxed-trait-over-ffi/21708)

## Run
first of all, build rust:

```sh
cargo build --target aarch64-apple-ios
```

then, use Xcode to open ```iOS/wgpu_test.xcodeproj``` and connect iOS device to run.

