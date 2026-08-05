// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "super_native_extensions",
    platforms: [
        .macOS("10.11")
    ],
    products: [
        .library(name: "super-native-extensions", targets: ["super_native_extensions"])
    ],
    dependencies: [
        .package(name: "FlutterFramework", path: "../FlutterFramework")
    ],
    targets: [
        .target(
            name: "super_native_extensions",
            dependencies: [
                .product(name: "FlutterFramework", package: "FlutterFramework")
            ],
            cSettings: [
                .headerSearchPath("include/super_native_extensions")
            ],
            linkerSettings: [
                .linkedFramework("Carbon")
            ]
        )
    ]
)
