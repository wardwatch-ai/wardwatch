package ai.wardwatch.mobile

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

/**
 * Brand tokens from packages/ui/src/tokens/brand.json (plus `edge` from ui.json), the
 * single source of truth. BrandTokensTest fails if these drift from it.
 */
data class WardwatchColors(
    val bg: Color,
    val surface: Color,
    val line: Color,
    val text: Color,
    val textMuted: Color,
    val brand: Color,
    val onBrand: Color,
    val ok: Color,
    val act: Color,
    val actTint: Color,
    val actInk: Color,
    val onAct: Color,
    /** Null in dark mode, where the act-now button has no outline. */
    val actEdge: Color?,
    /** Outline for quiet buttons and empty severity bars (ui.json `edge`, 3:1 on every surface). */
    val edge: Color,
    val isDark: Boolean,
)

val LightColors = WardwatchColors(
    bg = Color(0xFFF3EDE2),
    surface = Color(0xFFFBF8F2),
    line = Color(0xFFE3D9C6),
    text = Color(0xFF23211F),
    textMuted = Color(0xFF6B6358),
    brand = Color(0xFFA23B2C),
    onBrand = Color(0xFFFFFFFF),
    ok = Color(0xFF2F6F62),
    act = Color(0xFFF2B33D),
    actTint = Color(0xFFFFF4E0),
    actInk = Color(0xFF8A5300),
    onAct = Color(0xFF23211F),
    actEdge = Color(0xFF23211F),
    edge = Color(0xFF857B6D),
    isDark = false,
)

val DarkColors = WardwatchColors(
    bg = Color(0xFF23211F),
    surface = Color(0xFF2E2B28),
    line = Color(0xFF45403A),
    text = Color(0xFFF3EDE2),
    textMuted = Color(0xFFB8AE9F),
    brand = Color(0xFFC4503C),
    onBrand = Color(0xFFFFFFFF),
    ok = Color(0xFF7FCFA8),
    act = Color(0xFFF2B33D),
    actTint = Color(0xFF3A2F1C),
    actInk = Color(0xFFF7C76A),
    onAct = Color(0xFF23211F),
    actEdge = null,
    edge = Color(0xFF8A8072),
    isDark = true,
)

val Inter = FontFamily(
    Font(R.font.inter_regular, FontWeight.Normal),
    Font(R.font.inter_semibold, FontWeight.SemiBold),
)
val Manrope = FontFamily(Font(R.font.manrope_extrabold, FontWeight.ExtraBold))

val LocalWardwatchColors = staticCompositionLocalOf { LightColors }

@Composable
fun WardwatchTheme(dark: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    val c = if (dark) DarkColors else LightColors
    val scheme = if (dark) {
        darkColorScheme(primary = c.brand, onPrimary = c.onBrand, background = c.bg, surface = c.surface, onBackground = c.text, onSurface = c.text, outline = c.line)
    } else {
        lightColorScheme(primary = c.brand, onPrimary = c.onBrand, background = c.bg, surface = c.surface, onBackground = c.text, onSurface = c.text, outline = c.line)
    }
    val body = TextStyle(fontFamily = Inter, fontSize = 16.sp, lineHeight = 24.sp)
    val typography = Typography(
        displaySmall = TextStyle(fontFamily = Manrope, fontWeight = FontWeight.ExtraBold, fontSize = 28.sp, lineHeight = 32.sp),
        titleLarge = TextStyle(fontFamily = Manrope, fontWeight = FontWeight.ExtraBold, fontSize = 20.sp),
        titleMedium = TextStyle(fontFamily = Inter, fontWeight = FontWeight.SemiBold, fontSize = 17.sp),
        bodyLarge = body,
        bodyMedium = body.copy(fontSize = 14.sp, lineHeight = 20.sp),
        labelLarge = TextStyle(fontFamily = Inter, fontWeight = FontWeight.SemiBold, fontSize = 15.sp),
        // Eyebrows such as "NEEDS YOU TODAY".
        labelSmall = TextStyle(fontFamily = Inter, fontWeight = FontWeight.SemiBold, fontSize = 12.sp, letterSpacing = 1.sp),
    )
    androidx.compose.runtime.CompositionLocalProvider(LocalWardwatchColors provides c) {
        MaterialTheme(colorScheme = scheme, typography = typography, content = content)
    }
}
