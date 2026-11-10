import SwiftUI
import UIKit
import WardwatchKit

/// Brand colors that follow light and dark mode.
enum Ink {
    static let bg = dynamic(\.bg)
    static let surface = dynamic(\.surface)
    static let line = dynamic(\.line)
    static let text = dynamic(\.text)
    static let textMuted = dynamic(\.textMuted)
    static let brand = dynamic(\.brand)
    static let actTint = dynamic(\.actTint)
    static let actInk = dynamic(\.actInk)
    static let ok = dynamic(\.ok)
    static let act = dynamic(\.act)
    static let onAct = dynamic(\.onAct)

    private static func dynamic(_ token: KeyPath<BrandPalette, String>) -> Color {
        let light = uiColor(BrandPalette.light[keyPath: token])
        let dark = uiColor(BrandPalette.dark[keyPath: token])
        return Color(UIColor { $0.userInterfaceStyle == .dark ? dark : light })
    }

    private static func uiColor(_ hex: String) -> UIColor {
        guard let c = BrandPalette.rgb(hex) else { return .clear }
        return UIColor(red: c.red, green: c.green, blue: c.blue, alpha: 1)
    }
}

/// Brand fonts, scaled with Dynamic Type.
enum Typeface {
    static let display = Font.custom("Manrope-ExtraBold", size: 28, relativeTo: .largeTitle)
    static let title = Font.custom("Inter-SemiBold", size: 17, relativeTo: .headline)
    static let body = Font.custom("Inter-Regular", size: 16, relativeTo: .body)
    static let caption = Font.custom("Inter-Regular", size: 14, relativeTo: .subheadline)
    static let brandName = Font.custom("Manrope-ExtraBold", size: 20, relativeTo: .title3)
    static let label = Font.custom("Inter-SemiBold", size: 15, relativeTo: .subheadline)
    /// Eyebrows such as "NEEDS YOU TODAY".
    static let eyebrow = Font.custom("Inter-SemiBold", size: 12, relativeTo: .caption)
}
