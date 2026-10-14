// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "VcCore",
    platforms: [.iOS(.v17)],
    products: [
        .library(name: "VcCore", targets: ["VcCore"])
    ],
    targets: [
        .binaryTarget(
            name: "VcFFI",
            path: "VcFFI.xcframework"
        ),
        .target(
            name: "VcCore",
            dependencies: ["VcFFI"],
            path: "Sources/VcCore"
        )
    ]
)