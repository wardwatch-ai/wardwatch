import SwiftUI
import UIKit
import WardwatchKit

/// The first mobile screen: this device, its iOS version, and how to update it.
/// Placeholder layout until the mobile screens get their own approved design.
struct HomeView: View {
    private let deviceName = UIDevice.current.model
    private let identifier = DeviceModel.identifier(environment: ProcessInfo.processInfo.environment)
    private let systemName = UIDevice.current.systemName
    private let version = OSVersion(UIDevice.current.systemVersion)

    /// The newest iOS release will come from the signed catalog. Until then: guidance only.
    private var status: OSStatus {
        guard let version else { return .unknown }
        return OSNudge.check(current: version, latest: nil)
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                Text("Wardwatch")
                    .font(Typeface.display)
                    .foregroundStyle(Ink.brand)
                    .accessibilityAddTraits(.isHeader)

                Card {
                    Text(deviceName).font(Typeface.title).foregroundStyle(Ink.text)
                    Text("\(systemName) \(version?.description ?? UIDevice.current.systemVersion) · \(identifier)")
                        .font(Typeface.caption)
                        .foregroundStyle(Ink.textMuted)
                }

                UpdateCard(status: status, systemName: systemName)
            }
            .padding(16)
        }
        .background(Ink.bg)
    }
}

private struct UpdateCard: View {
    let status: OSStatus
    let systemName: String

    var body: some View {
        switch status {
        case .current:
            Card {
                Text("✓ \(systemName) is up to date").font(Typeface.title).foregroundStyle(Ink.text)
                Text("Wardwatch checks again every day.").font(Typeface.caption).foregroundStyle(Ink.textMuted)
            }
        case .updateAvailable(let latest):
            Card(background: Ink.actTint) {
                Text("▲ Update to \(systemName) \(latest.description)").font(Typeface.title).foregroundStyle(Ink.actInk)
                Text("This update fixes security problems. Install it today.")
                    .font(Typeface.caption).foregroundStyle(Ink.textMuted)
                Steps(systemName: systemName)
            }
        case .unknown:
            Card {
                Text("Keep \(systemName) up to date").font(Typeface.title).foregroundStyle(Ink.text)
                Text("Most \(systemName) updates fix security problems. Check for one now, and let your device install them by itself.")
                    .font(Typeface.caption).foregroundStyle(Ink.textMuted)
                Steps(systemName: systemName)
            }
        }
    }
}

/// iOS doesn't let apps open the Software Update screen, so we show the path.
private struct Steps: View {
    let systemName: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("1. Open Settings, then General, then Software Update.")
            Text("2. Tap Update Now if an update is waiting.")
            Text("3. Tap Automatic Updates and turn on everything there.")
        }
        .font(Typeface.body)
        .foregroundStyle(Ink.text)
        .padding(.top, 4)
    }
}

private struct Card<Content: View>: View {
    var background: Color = Ink.surface
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 4) { content }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(16)
            .background(background, in: RoundedRectangle(cornerRadius: 12))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Ink.line, lineWidth: 1))
    }
}

#Preview {
    HomeView()
}
