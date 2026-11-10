package ai.wardwatch.mobile

import android.os.Bundle
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.HorizontalDivider
import androidx.compose.ui.Alignment
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.sp
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.time.LocalDate

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { WardwatchTheme { HomeScreen() } }
    }
}

private val CardShape = RoundedCornerShape(12.dp)
private val HeroShape = RoundedCornerShape(20.dp)
private val ButtonShape = RoundedCornerShape(12.dp)

/**
 * Home: a status card for the system update (beams on only when the phone must act
 * now), this phone's model and patch date, and the apps installed on it with versions.
 * Layout approved by Matt on the mobile design canvas, 2026-11-10.
 */
@Composable
fun HomeScreen() {
    val c = LocalWardwatchColors.current
    val context = LocalContext.current
    var device by remember { mutableStateOf(DeviceInfo.current()) }
    val check = remember(device) { OsNudge.check(device.securityPatch, LocalDate.now()) }
    var apps by remember { mutableStateOf<List<InstalledApp>?>(null) }
    // Re-read on every return to the app, such as coming back from a system or app update.
    var resumes by remember { mutableIntStateOf(0) }
    LifecycleResumeEffect(Unit) {
        resumes++
        onPauseOrDispose { }
    }
    LaunchedEffect(resumes) {
        if (resumes == 0) return@LaunchedEffect
        device = DeviceInfo.current()
        apps = withContext(Dispatchers.Default) {
            runCatching { InstalledApps.list(context.packageManager) }.getOrDefault(emptyList())
        }
    }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(c.bg)
            .windowInsetsPadding(WindowInsets.safeDrawing),
        contentPadding = PaddingValues(16.dp),
    ) {
        // Explicit gaps, not spacedBy, so the app rows below join into one card.
        val gap = @Composable { Spacer(Modifier.height(14.dp)) }
        item {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.padding(top = 8.dp),
            ) {
                Lighthouse(size = 28.dp, beams = false)
                Text(
                    "Wardwatch",
                    style = MaterialTheme.typography.titleLarge,
                    color = c.text,
                    modifier = Modifier.semantics { heading() },
                )
            }
        }
        item { gap() }
        item { StatusCard(HomeCopy.status(check, device.securityPatch), onUpdate = { openSystemUpdate(context) }) }
        item { gap() }
        item { DeviceRow(device, check) }
        item { gap() }
        item {
            Text(
                apps?.let { "APPS ON THIS PHONE · ${it.size}" } ?: "APPS ON THIS PHONE",
                style = MaterialTheme.typography.labelSmall,
                color = c.textMuted,
                modifier = Modifier.padding(top = 6.dp, bottom = 10.dp).semantics { heading() },
            )
        }
        val list = apps.orEmpty()
        if (list.isNotEmpty()) {
            // One lazy row per app, drawn as one card: rounded ends on the first and last.
            itemsIndexed(list, key = { _, app -> app.packageName }) { i, app ->
                val top = if (i == 0) 12.dp else 0.dp
                val bottom = if (i == list.lastIndex) 12.dp else 0.dp
                val shape = RoundedCornerShape(top, top, bottom, bottom)
                Column(Modifier.fillMaxWidth().background(c.surface, shape)) {
                    if (i > 0) HorizontalDivider(color = c.line)
                    AppRow(app)
                }
            }
            item { gap() }
            item {
                Text(
                    "The Play Store updates these apps. To keep them current, turn on auto-update in the Play Store settings.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = c.textMuted,
                )
            }
        }
    }
}

@Composable
private fun StatusCard(copy: StatusCopy, onUpdate: () -> Unit) {
    val c = LocalWardwatchColors.current
    val actNow = copy.button == StatusButton.ActNow
    val eyebrowColor = when {
        actNow -> c.actInk
        copy.tone == StatusTone.Clear -> c.ok
        else -> c.textMuted
    }
    Column(
        Modifier
            .fillMaxWidth()
            .background(if (actNow) c.actTint else c.surface, HeroShape)
            .then(if (actNow) Modifier else Modifier.border(1.dp, c.line, HeroShape))
            .padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            Lighthouse(size = 64.dp, beams = copy.beams)
            copy.eyebrow?.let {
                Text(it, style = MaterialTheme.typography.labelSmall, color = eyebrowColor)
            }
        }
        Text(
            copy.title,
            style = MaterialTheme.typography.displaySmall,
            color = c.text,
            modifier = Modifier.semantics { heading() },
        )
        Text(copy.body, style = MaterialTheme.typography.bodyLarge, color = c.text)
        copy.detail?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = c.textMuted) }
        when (copy.button) {
            // Act-now button, locked brand rule 6: amber fill, driftwood label, outline in light mode only.
            StatusButton.ActNow -> Button(
                onClick = onUpdate,
                colors = ButtonDefaults.buttonColors(containerColor = c.act, contentColor = c.onAct),
                border = c.actEdge?.let { BorderStroke(2.dp, it) },
                shape = ButtonShape,
                modifier = Modifier.padding(top = 6.dp).fillMaxWidth().heightIn(min = 52.dp),
            ) { Text("Open system update", style = MaterialTheme.typography.labelLarge) }
            // Not urgent: quiet outline, with the 3-bar severity meter when we know how serious it is.
            StatusButton.Quiet -> OutlinedButton(
                onClick = onUpdate,
                border = BorderStroke(1.dp, c.edge),
                shape = ButtonShape,
                modifier = Modifier.padding(top = 4.dp).heightIn(min = 48.dp),
            ) {
                if (copy.bars > 0) {
                    SeverityMeter(bars = copy.bars)
                    Spacer(Modifier.width(10.dp))
                }
                Text("Open system update", style = MaterialTheme.typography.labelLarge, color = c.text)
            }
            StatusButton.None -> Unit
        }
    }
}

/** The brand's 3-bar severity meter: 2 bars moderate, 1 bar minor. */
@Composable
private fun SeverityMeter(bars: Int) {
    val c = LocalWardwatchColors.current
    val label = if (bars >= 2) "Moderate" else "Minor"
    Row(
        horizontalArrangement = Arrangement.spacedBy(2.dp),
        modifier = Modifier.semantics { contentDescription = label },
    ) {
        repeat(3) { i ->
            val bar = Modifier.width(4.dp).height(14.dp)
            val shape = RoundedCornerShape(1.dp)
            Box(if (i < bars) bar.background(c.text, shape) else bar.border(1.dp, c.edge, shape))
        }
    }
}

@Composable
private fun DeviceRow(device: DeviceInfo, check: OsCheck) {
    val c = LocalWardwatchColors.current
    Row(
        Modifier
            .fillMaxWidth()
            .background(c.surface, CardShape)
            .border(1.dp, c.line, CardShape)
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(device.name, style = MaterialTheme.typography.titleMedium, color = c.text)
            Text(HomeCopy.deviceLine(device), style = MaterialTheme.typography.bodyMedium, color = c.textMuted)
        }
        check.patchAgeDays?.let { age ->
            Text(
                "${HomeCopy.days(age)} old",
                style = MaterialTheme.typography.labelLarge,
                color = when (check.status) {
                    OsStatus.ActNow -> c.actInk
                    OsStatus.Current -> c.ok
                    else -> c.textMuted
                },
            )
        }
    }
}

@Composable
private fun AppRow(app: InstalledApp) {
    val c = LocalWardwatchColors.current
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 13.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(app.label, style = MaterialTheme.typography.labelLarge, color = c.text, modifier = Modifier.weight(1f))
        Text(
            app.version ?: "Unknown",
            style = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace, fontSize = 13.sp),
            color = c.textMuted,
        )
    }
}
