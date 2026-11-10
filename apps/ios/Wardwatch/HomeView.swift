import SwiftUI
import UIKit
import WardwatchKit

/// Home: a status card for the system update with the steps to take (iOS doesn't let
/// apps start an update), this device's model and version, and how to keep updates
/// automatic. Layout approved by Matt on the mobile design canvas, 2026-11-10.
struct HomeView: View {
    private let deviceName = UIDevice.current.model
    private let identifier = DeviceModel.identifier(environment: ProcessInfo.processInfo.environment)
    private let systemName = UIDevice.current.systemName
    private let version = OSVersion(UIDevice.current.systemVersion)

    /// The newest iOS release will come from the signed catalog. Until then: guidance only.
    private var status: UpdateStatus {
        guard let version else { return .unknown }
        return OSNudge.check(current: version, latest: nil)
    }

    var body: some View {
        let copy = HomeCopy.status(status, systemName: systemName)
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                HStack(spacing: 8) {
                    Lighthouse(beams: false).frame(width: 28, height: 28)
                    Text("Wardwatch")
                        .font(Typeface.brandName)
                        .foregroundStyle(Ink.text)
                        .accessibilityAddTraits(.isHeader)
                }
                .padding(.top, 8)

                StatusCard(copy: copy)

                Card {
                    Text("Let it update by itself").font(Typeface.title).foregroundStyle(Ink.text)
                    Text("In Software Update, tap **Automatic Updates** and turn on everything there. Then most fixes arrive overnight.")
                        .font(Typeface.caption).foregroundStyle(Ink.textMuted)
                }

                Card {
                    Text("Apps").font(Typeface.title).foregroundStyle(Ink.text)
                    Text("\(systemName) doesn't let other apps see your apps. Turn on **App Updates** in Settings, then App Store.")
                        .font(Typeface.caption).foregroundStyle(Ink.textMuted)
                }

                Card {
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(deviceName).font(Typeface.title).foregroundStyle(Ink.text)
                            Text("\(systemName) \(version?.description ?? UIDevice.current.systemVersion) · \(identifier)")
                                .font(Typeface.caption)
                                .foregroundStyle(Ink.textMuted)
                        }
                        Spacer()
                        Text("This device").font(Typeface.caption).foregroundStyle(Ink.textMuted)
                    }
                }
            }
            .padding(16)
        }
        .background(Ink.bg)
    }
}

private struct StatusCard: View {
    let copy: StatusCopy
    /// Step badges grow with Dynamic Type.
    @ScaledMetric(relativeTo: .body) private var badge: CGFloat = 28

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 14) {
                Lighthouse(beams: copy.actNow).frame(width: 56, height: 56)
                if let eyebrow = copy.eyebrow {
                    Text(eyebrow)
                        .font(Typeface.eyebrow)
                        .tracking(1)
                        .foregroundStyle(copy.tone == .actNow ? Ink.actInk : copy.tone == .clear ? Ink.ok : Ink.textMuted)
                }
            }
            Text(copy.title)
                .font(Typeface.display)
                .foregroundStyle(Ink.text)
                .accessibilityAddTraits(.isHeader)
            Text(copy.body).font(Typeface.body).foregroundStyle(Ink.text)
            if !copy.steps.isEmpty {
                VStack(alignment: .leading, spacing: 10) {
                    ForEach(Array(copy.steps.enumerated()), id: \.offset) { index, step in
                        HStack(alignment: .firstTextBaseline, spacing: 12) {
                            Text("\(index + 1)")
                                .font(Typeface.label)
                                .foregroundStyle(copy.actNow ? Ink.onAct : Ink.text)
                                .frame(width: badge, height: badge)
                                .background(copy.actNow ? Ink.act : Ink.bg, in: Circle())
                                .accessibilityHidden(true)
                            Text(step).font(Typeface.body).foregroundStyle(Ink.text)
                        }
                        .accessibilityElement(children: .combine)
                        .accessibilityLabel("Step \(index + 1): \(step)")
                    }
                }
                .padding(.top, 4)
            }
            if copy.actNow {
                Text("iPhone doesn't let apps start updates, so this one is up to you.")
                    .font(Typeface.caption).foregroundStyle(Ink.textMuted)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(20)
        .background(copy.actNow ? Ink.actTint : Ink.surface, in: RoundedRectangle(cornerRadius: 20))
        .overlay {
            if !copy.actNow {
                RoundedRectangle(cornerRadius: 20).strokeBorder(Ink.line, lineWidth: 1)
            }
        }
    }
}

/// The Beacon lighthouse on a 64-unit grid. Beams on means "act now" (brand rule 3:
/// the shape changes, not just the color). Decorative: the card's words carry the meaning.
private struct Lighthouse: View {
    let beams: Bool
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        let dark = scheme == .dark
        Canvas { context, size in
            let u = size.width / 64
            func poly(_ xy: [CGFloat]) -> Path {
                var p = Path()
                p.move(to: CGPoint(x: xy[0] * u, y: xy[1] * u))
                stride(from: 2, to: xy.count, by: 2).forEach { p.addLine(to: CGPoint(x: xy[$0] * u, y: xy[$0 + 1] * u)) }
                p.closeSubpath()
                return p
            }
            func box(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat) -> Path {
                Path(roundedRect: CGRect(x: x * u, y: y * u, width: w * u, height: h * u), cornerRadius: 0)
            }
            let frame = dark ? Ink.textMuted : Ink.text
            if beams {
                context.fill(poly([27, 14, 2, 7, 2, 21]), with: .color(Ink.act))
                context.fill(poly([37, 14, 62, 7, 62, 21]), with: .color(Ink.act))
            }
            context.fill(poly([26, 10, 32, 5, 38, 10]), with: .color(frame))
            context.fill(box(27, 10, 10, 8), with: .color(frame))
            context.fill(box(29, 12, 6, 4), with: .color(beams ? Ink.act : (dark ? Ink.line : Ink.textMuted)))
            context.fill(box(24, 18, 16, 2.5), with: .color(frame))
            context.fill(poly([26, 20.5, 38, 20.5, 39, 36, 25, 36]), with: .color(Ink.brand))
            let lower = poly([25, 36, 39, 36, 41, 54, 23, 54])
            context.fill(lower, with: .color(dark ? Ink.text : .white))
            if !dark { context.stroke(lower, with: .color(frame), lineWidth: 1.5 * u) }
            context.fill(Path(roundedRect: CGRect(x: 20 * u, y: 54 * u, width: 24 * u, height: 3 * u), cornerRadius: u), with: .color(frame))
        }
        .accessibilityHidden(true)
    }
}

private struct Card<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 6) { content }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(16)
            .background(Ink.surface, in: RoundedRectangle(cornerRadius: 12))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Ink.line, lineWidth: 1))
    }
}

#Preview {
    HomeView()
}
