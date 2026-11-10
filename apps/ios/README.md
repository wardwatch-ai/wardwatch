# Wardwatch for iOS

A native SwiftUI app. Bundle ID `ai.wardwatch.mobile`.

What it does today, all on the device (nothing is sent anywhere yet):

- Shows the device type, iOS version and hardware identifier.
- Shows how to check for and install iOS updates and turn on Automatic Updates. iOS doesn't let apps read other apps' versions or open the Software Update screen, so guidance is the whole job here.
- `OSNudge` already compares the running version with the newest release. Once the signed catalog carries the newest iOS version, the app turns that into the amber act-now nudge.

The logic lives in `WardwatchKit`, a Swift package with no UIKit, so its tests run on macOS and Linux. The home screen follows the approved mobile design (2026-11-10). Its words live in `WardwatchKit` (`HomeCopy.swift`) so they are tested there.

## Build

You need Xcode 16 or newer and [XcodeGen](https://github.com/yonaskolb/XcodeGen). The Xcode project is generated from `project.yml` and not committed.

```sh
swift test --package-path WardwatchKit
xcodegen
xcodebuild build -project Wardwatch.xcodeproj -scheme Wardwatch \
  -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO
```

Builds are unsigned. Signing and App Store uploads are done by hand under Matthew Ward's developer account, never by CI.
