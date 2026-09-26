ffi_lib := "libmoby_ffi.a"
kit := "ios/MobyKit"
generated := kit / "Sources/MobyKit/Generated"
headers := "target/uniffi/headers"
ios_target := "18.0"
macos_target := "15.0"
# The Swift sources that are not generated.
swift_sources := "ios/MyMoby ios/MobyKit/Tests ios/MobyKit/Package.swift ios/MobyKit/Sources/MobyKit/Extensions.swift ios/Shared ios/MyMobyWidgets ios/Icon"

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

# Build the app for the iOS 18 simulator. Arguments are build settings for
# xcodebuild, for example CODE_SIGNING_ALLOWED=NO.
app *settings: ffi project
    xcodebuild -project ios/MyMoby.xcodeproj -scheme MyMoby \
        -destination 'generic/platform=iOS Simulator' \
        -derivedDataPath ios/build ARCHS=arm64 {{settings}} build

# Draw the app icon into the asset catalog.
icon:
    swift ios/Icon/MakeIcon.swift ios/MyMoby/Assets.xcassets/AppIcon.appiconset

# Check the format of the Swift sources.
swift-lint:
    xcrun swift-format lint --strict -r {{swift_sources}}

# Format the Swift sources.
swift-fmt:
    xcrun swift-format format -i -r {{swift_sources}}

# Build a Release copy and install it on the named iPhone, by default the
# one in MOBY_DEVICE.
device name=env("MOBY_DEVICE"): ffi project
    xcodebuild -project ios/MyMoby.xcodeproj -scheme MyMoby -configuration Release \
        -destination 'generic/platform=iOS' -derivedDataPath ios/build \
        -allowProvisioningUpdates build
    xcrun devicectl device install app --device "{{ name }}" \
        ios/build/Build/Products/Release-iphoneos/MyMoby.app
