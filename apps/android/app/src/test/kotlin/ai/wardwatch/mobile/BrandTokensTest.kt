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
        return Regex("\"([a-z-]+)\"\\s*:\\s*\"([^\"]+)\"").findAll(block)
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

    @Test fun lightMatchesBrand() = assertMatches(theme("light"), LightColors)

    @Test fun darkMatchesBrand() = assertMatches(theme("dark"), DarkColors)
}
