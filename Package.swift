// swift-tools-version:5.9
// Root manifest so that Swift Package Manager can resolve the iOS SDK from this repository.
import PackageDescription

let package = Package(
    name: "Kurzhaar",
    platforms: [.iOS(.v13), .macOS(.v10_15)],
    products: [
        .library(name: "Kurzhaar", targets: ["Kurzhaar"]),
    ],
    targets: [
        .target(name: "Kurzhaar", path: "sdks/ios/Sources/Kurzhaar"),
    ]
)
