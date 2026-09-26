// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "MobyKit",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "MobyKit", targets: ["MobyKit"]),
    ],
    targets: [
        .binaryTarget(name: "moby_ffiFFI", path: "MobyFFI.xcframework"),
        .target(name: "MobyKit", dependencies: ["moby_ffiFFI"]),
        .testTarget(name: "MobyKitTests", dependencies: ["MobyKit"]),
    ]
)
