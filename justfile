ffi_lib := "libmoby_ffi.a"
kit := "ios/MobyKit"
generated := kit / "Sources/MobyKit/Generated"
headers := "target/uniffi/headers"
ios_target := "18.0"
macos_target := "15.0"

# Build the Rust library for iOS device, iOS simulator and macOS, then
# regenerate the Swift bindings and the XCFramework.
ffi:
    IPHONEOS_DEPLOYMENT_TARGET={{ios_target}} cargo build -p moby-ffi --release --target aarch64-apple-ios
    IPHONEOS_DEPLOYMENT_TARGET={{ios_target}} cargo build -p moby-ffi --release --target aarch64-apple-ios-sim
    MACOSX_DEPLOYMENT_TARGET={{macos_target}} cargo build -p moby-ffi --release --target aarch64-apple-darwin
    rm -rf {{headers}} {{generated}} {{kit}}/MobyFFI.xcframework
    mkdir -p {{headers}} {{generated}}
    cargo run -p moby-ffi --features bindgen --bin uniffi-bindgen -- \
        --headers --modulemap --module-name moby_ffiFFI --modulemap-filename module.modulemap \
        target/aarch64-apple-darwin/release/{{ffi_lib}} {{headers}}
    cargo run -p moby-ffi --features bindgen --bin uniffi-bindgen -- \
        --swift-sources target/aarch64-apple-darwin/release/{{ffi_lib}} {{generated}}
    xcodebuild -create-xcframework \
        -library target/aarch64-apple-ios/release/{{ffi_lib}} -headers {{headers}} \
        -library target/aarch64-apple-ios-sim/release/{{ffi_lib}} -headers {{headers}} \
        -library target/aarch64-apple-darwin/release/{{ffi_lib}} -headers {{headers}} \
        -output {{kit}}/MobyFFI.xcframework

# Run the Rust tests.
test:
    cargo nextest r

# Run the Swift smoke tests against the macOS slice.
swift-test: ffi
    cd {{kit}} && swift test
