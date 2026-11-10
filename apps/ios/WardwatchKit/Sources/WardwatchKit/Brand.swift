/// Brand color tokens from packages/ui/src/tokens/brand.json, the single source of
/// truth. BrandTests fails if these drift from it. Values are "#RRGGBB" or nil for
/// "transparent".
public struct BrandPalette: Sendable {
    public let bg, surface, line, text, textMuted, brand, onBrand, ok: String
    public let act, actTint, actInk, onAct: String
    /// nil in dark mode, where the act-now button has no outline.
    public let actEdge: String?

    public var byToken: [String: String] {
        [
            "bg": bg, "surface": surface, "line": line, "text": text, "text-muted": textMuted,
            "brand": brand, "on-brand": onBrand, "ok": ok, "act": act, "act-tint": actTint,
            "act-ink": actInk, "on-act": onAct, "act-edge": actEdge ?? "transparent",
        ]
    }

    public static let light = BrandPalette(
        bg: "#F3EDE2", surface: "#FBF8F2", line: "#E3D9C6", text: "#23211F", textMuted: "#6B6358",
        brand: "#A23B2C", onBrand: "#FFFFFF", ok: "#2F6F62", act: "#F2B33D", actTint: "#FFF4E0",
        actInk: "#8A5300", onAct: "#23211F", actEdge: "#23211F"
    )

    public static let dark = BrandPalette(
        bg: "#23211F", surface: "#2E2B28", line: "#45403A", text: "#F3EDE2", textMuted: "#B8AE9F",
        brand: "#C4503C", onBrand: "#FFFFFF", ok: "#7FCFA8", act: "#F2B33D", actTint: "#3A2F1C",
        actInk: "#F7C76A", onAct: "#23211F", actEdge: nil
    )

    /// Splits "#RRGGBB" into 0...1 components.
    public static func rgb(_ hex: String) -> (red: Double, green: Double, blue: Double)? {
        guard hex.count == 7, hex.first == "#", let v = UInt32(hex.dropFirst(), radix: 16) else { return nil }
        return (Double((v >> 16) & 0xFF) / 255, Double((v >> 8) & 0xFF) / 255, Double(v & 0xFF) / 255)
    }
}
