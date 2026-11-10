package ai.wardwatch.mobile

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp

/**
 * The Beacon lighthouse (Old Barney: brick-red top, white bottom, dark lantern room),
 * drawn on a 64-unit grid. Beams on means "act now" (brand rule 3: the shape changes,
 * not just the color). Decorative: the card's words carry the meaning.
 */
@Composable
fun Lighthouse(size: Dp, beams: Boolean, modifier: Modifier = Modifier) {
    val c = LocalWardwatchColors.current
    val dark = c.isDark
    val frame = if (dark) c.textMuted else c.text
    // Old Barney's white lower half; on dark it takes the sand text color, as the brand mark does.
    val lowerTower = if (dark) c.text else Color.White
    val lamp = if (beams) c.act else if (dark) c.line else c.textMuted
    Canvas(modifier.size(size)) {
        val u = this.size.width / 64f
        fun poly(vararg xy: Float) = Path().apply {
            moveTo(xy[0] * u, xy[1] * u)
            for (i in 2 until xy.size step 2) lineTo(xy[i] * u, xy[i + 1] * u)
            close()
        }
        fun DrawScope.box(x: Float, y: Float, w: Float, h: Float, color: Color, r: Float = 0f) =
            drawRoundRect(color, Offset(x * u, y * u), Size(w * u, h * u), CornerRadius(r * u))

        if (beams) {
            drawPath(poly(27f, 14f, 2f, 7f, 2f, 21f), c.act)
            drawPath(poly(37f, 14f, 62f, 7f, 62f, 21f), c.act)
        }
        drawPath(poly(26f, 10f, 32f, 5f, 38f, 10f), frame)
        box(27f, 10f, 10f, 8f, frame)
        box(29f, 12f, 6f, 4f, lamp)
        box(24f, 18f, 16f, 2.5f, frame)
        drawPath(poly(26f, 20.5f, 38f, 20.5f, 39f, 36f, 25f, 36f), c.brand)
        val lower = poly(25f, 36f, 39f, 36f, 41f, 54f, 23f, 54f)
        drawPath(lower, lowerTower)
        if (!dark) drawPath(lower, frame, style = Stroke(width = 1.5f * u))
        box(20f, 54f, 24f, 3f, frame, r = 1f)
    }
}
