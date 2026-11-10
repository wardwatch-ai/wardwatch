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

    private static func dynamic(_ token: KeyPath<BrandPalette, String>) -> Color {
        Color(UIColor { traits in
            uiColor(traits.userInterfaceStyle == .dark ? BrandPalette.dark[keyPath: token] : BrandPalette.light[keyPath: token])
        })
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
}
