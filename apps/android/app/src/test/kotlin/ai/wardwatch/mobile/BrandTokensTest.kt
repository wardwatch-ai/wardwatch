package ai.wardwatch.mobile

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import org.junit.Assert.assertEquals
import org.junit.Test
import java.io.File

/** Fails when Theme.kt drifts from packages/ui/src/tokens/brand.json. */
class BrandTokensTest {
    private val brandJson = File(System.getProperty("wardwatch.repoRoot"), "packages/ui/src/tokens/brand.json").readText()

    /** The `"name": "value"` pairs inside one theme's block, e.g. `"light": { ... }`. */
    private fun theme(name: String): Map<String, String> {
        val block = Regex("\"$name\"\\s*:\\s*\\{([^}]*)\\}").find(brandJson)!!.groupValues[1]
        return Regex("\"([a-z0-9-]+)\"\\s*:\\s*\"([^\"]+)\"").findAll(block)
            .associate { it.groupValues[1] to it.groupValues[2] }
    }

    private fun hex(color: Color?) = color?.let { "#%06X".format(it.toArgb() and 0xFFFFFF) } ?: "transparent"

    private fun assertMatches(tokens: Map<String, String>, c: WardwatchColors) {
        val ours = mapOf(
            "bg" to c.bg, "surface" to c.surface, "line" to c.line, "text" to c.text,
            "text-muted" to c.textMuted, "brand" to c.brand, "on-brand" to c.onBrand, "ok" to c.ok,
            "act" to c.act, "act-tint" to c.actTint, "act-ink" to c.actInk, "on-act" to c.onAct,
            "act-edge" to c.actEdge,
        ).mapValues { hex(it.value) }
        assertEquals(tokens.mapValues { it.value.uppercase().replace("TRANSPARENT", "transparent") }, ours)
    }

    /** `edge` comes from the app-only tokens in ui.json. */
    @Test fun edgeMatchesUiTokens() {
        val ui = File(System.getProperty("wardwatch.repoRoot"), "packages/ui/src/tokens/ui.json").readText()
        fun edge(theme: String) = Regex("\"$theme\"\\s*:\\s*\\{[^}]*\"edge\"\\s*:\\s*\"(#[0-9A-Fa-f]{6})\"")
            .find(ui)!!.groupValues[1].uppercase()
        assertEquals(edge("light"), hex(LightColors.edge))
        assertEquals(edge("dark"), hex(DarkColors.edge))
    }

    @Test fun lightMatchesBrand() = assertMatches(theme("light"), LightColors)

    @Test fun darkMatchesBrand() = assertMatches(theme("dark"), DarkColors)

    /** The window background drawn before Compose's first frame. */
    @Test fun windowBackgroundMatchesBrand() {
        val res = File(System.getProperty("wardwatch.repoRoot"), "apps/android/app/src/main/res")
        fun bg(dir: String) = Regex("name=\"ww_bg\">(#[0-9A-Fa-f]{6})<")
            .find(File(res, "$dir/colors.xml").readText())!!.groupValues[1].uppercase()
        assertEquals(theme("light")["bg"], bg("values"))
        assertEquals(theme("dark")["bg"], bg("values-night"))
    }
}
