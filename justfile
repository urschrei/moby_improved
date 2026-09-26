ffi_lib := "libmoby_ffi.a"
kit := "ios/MobyKit"
generated := kit / "Sources/MobyKit/Generated"
headers := "target/uniffi/headers"
ios_target := "18.0"
macos_target := "15.0"

# Build the Rust library for iOS, the simulator and macOS, and regenerate the Swift bindings.
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

# Generate the Xcode project.
project:
    cd ios && xcodegen generate

# Build the app for the iOS 18 simulator.
app: ffi project
    xcodebuild -project ios/MyMoby.xcodeproj -scheme MyMoby \
        -destination 'generic/platform=iOS Simulator' \
        -derivedDataPath ios/build ARCHS=arm64 build

# Format the Swift sources.
swift-fmt:
    xcrun swift-format format -i -r ios/MyMoby ios/MobyKit/Tests ios/MobyKit/Package.swift ios/MobyKit/Sources/MobyKit/Extensions.swift ios/Shared ios/MyMobyWidgets

# Build a Release copy and install it on the iPhone named in MOBY_DEVICE.
device: ffi project
    xcodebuild -project ios/MyMoby.xcodeproj -scheme MyMoby -configuration Release \
        -destination 'generic/platform=iOS' -derivedDataPath ios/build \
        -allowProvisioningUpdates build
    xcrun devicectl device install app --device "$MOBY_DEVICE" \
        ios/build/Build/Products/Release-iphoneos/MyMoby.app
