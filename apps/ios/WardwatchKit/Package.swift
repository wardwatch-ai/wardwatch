// swift-tools-version: 6.0
// The app's logic, with no UIKit or SwiftUI, so `swift test` runs on macOS and Linux.
import PackageDescription

let package = Package(
    name: "WardwatchKit",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [.library(name: "WardwatchKit", targets: ["WardwatchKit"])],
    targets: [
        .target(name: "WardwatchKit"),
        .testTarget(name: "WardwatchKitTests", dependencies: ["WardwatchKit"]),
    ]
)
