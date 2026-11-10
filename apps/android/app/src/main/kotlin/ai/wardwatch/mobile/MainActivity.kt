package ai.wardwatch.mobile

import android.os.Bundle
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
import androidx.compose.foundation.lazy.items
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

/**
 * The first mobile screen: this phone, its system update status, and its apps.
 * Placeholder layout until the mobile screens get their own approved design.
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
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            Text(
                "Wardwatch",
                style = MaterialTheme.typography.displaySmall,
                color = c.brand,
                modifier = Modifier.semantics { heading() },
            )
        }
        item { DeviceCard(device) }
        item { SystemUpdateCard(check, onUpdate = { openSystemUpdate(context) }) }
        item {
            Text(
                apps?.let { "Apps on this phone (${it.size})" } ?: "Apps on this phone",
                style = MaterialTheme.typography.titleMedium,
                color = c.text,
                modifier = Modifier.padding(top = 8.dp).semantics { heading() },
            )
        }
        items(apps.orEmpty(), key = { it.packageName }) { app -> AppRow(app) }
    }
}

@Composable
private fun DeviceCard(device: DeviceInfo) {
    val c = LocalWardwatchColors.current
    Card {
        Text(device.name, style = MaterialTheme.typography.titleMedium, color = c.text)
        Text(
            listOfNotNull(
                "Android ${device.androidVersion}",
                device.securityPatch.takeIf { it.isNotBlank() }?.let { "security patch $it" },
            ).joinToString(" · "),
            style = MaterialTheme.typography.bodyMedium,
            color = c.textMuted,
        )
    }
}

@Composable
private fun SystemUpdateCard(check: OsCheck, onUpdate: () -> Unit) {
    val c = LocalWardwatchColors.current
    val actNow = check.status == OsStatus.ActNow
    val (title, detail) = when (check.status) {
        OsStatus.Current -> "✓ Security updates are current" to
            "This phone has a recent security patch."
        OsStatus.Due -> "• A security update is due" to
            "Your security patch is ${check.patchAgeDays} days old. Check for a system update this week."
        OsStatus.ActNow -> "▲ Update now" to
            "Your security patch is ${check.patchAgeDays} days old, so this phone is missing fixes for known attacks."
        OsStatus.Unknown -> "? Check for a system update" to
            "This phone didn't report its security patch. Open Settings to see if an update is waiting."
    }
    Card(background = if (actNow) c.actTint else c.surface) {
        Text(title, style = MaterialTheme.typography.titleMedium, color = if (actNow) c.actInk else c.text)
        Text(detail, style = MaterialTheme.typography.bodyMedium, color = c.textMuted)
        Spacer(Modifier.height(4.dp))
        if (actNow) {
            // Act-now button, locked brand rule 6: amber fill, driftwood label, outline in light mode only.
            Button(
                onClick = onUpdate,
                colors = ButtonDefaults.buttonColors(containerColor = c.act, contentColor = c.onAct),
                border = c.actEdge?.let { BorderStroke(2.dp, it) },
                shape = RoundedCornerShape(8.dp),
            ) { Text("Open system update", style = MaterialTheme.typography.labelLarge) }
        } else if (check.status != OsStatus.Current) {
            OutlinedButton(
                onClick = onUpdate,
                border = BorderStroke(1.dp, c.line),
                shape = RoundedCornerShape(8.dp),
            ) { Text("Check for updates", style = MaterialTheme.typography.labelLarge, color = c.text) }
        }
    }
}

@Composable
private fun AppRow(app: InstalledApp) {
    val c = LocalWardwatchColors.current
    Row(
        Modifier.fillMaxWidth().padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(app.label, style = MaterialTheme.typography.bodyLarge, color = c.text, modifier = Modifier.weight(1f))
        Text(app.version ?: "—", style = MaterialTheme.typography.bodyMedium, color = c.textMuted)
    }
}

@Composable
private fun Card(background: androidx.compose.ui.graphics.Color? = null, content: @Composable () -> Unit) {
    val c = LocalWardwatchColors.current
    Column(
        Modifier
            .fillMaxWidth()
            .background(background ?: c.surface, CardShape)
            .border(1.dp, c.line, CardShape)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) { content() }
}
