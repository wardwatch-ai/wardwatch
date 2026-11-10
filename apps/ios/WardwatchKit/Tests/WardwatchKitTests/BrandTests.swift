import Foundation
import Testing
@testable import WardwatchKit

/// Fails when Brand.swift drifts from packages/ui/src/tokens/brand.json.
@Suite struct BrandTests {
    struct BrandFile: Decodable {
        struct Colors: Decodable { let light: [String: String]; let dark: [String: String] }
        let color: Colors
    }

    func brandFile() throws -> BrandFile {
        // Tests/WardwatchKitTests -> WardwatchKit -> ios -> apps -> repo root
        let root = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let data = try Data(contentsOf: root.appendingPathComponent("packages/ui/src/tokens/brand.json"))
        return try JSONDecoder().decode(BrandFile.self, from: data)
    }

    func normalized(_ tokens: [String: String]) -> [String: String] {
        tokens.mapValues { $0 == "transparent" ? $0 : $0.uppercased() }
    }

    @Test func lightMatches() throws {
        #expect(normalized(try brandFile().color.light) == BrandPalette.light.byToken)
    }

    @Test func darkMatches() throws {
        #expect(normalized(try brandFile().color.dark) == BrandPalette.dark.byToken)
    }

    @Test func parsesHex() {
        let c = BrandPalette.rgb("#FF8000")!
        #expect(c.red == 1 && c.blue == 0 && abs(c.green - 128.0 / 255) < 1e-9)
        #expect(BrandPalette.rgb("transparent") == nil)
    }
}
