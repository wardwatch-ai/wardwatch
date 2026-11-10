package ai.wardwatch.mobile

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.provider.Settings

/**
 * Opens the phone's system update screen. Android has no public action for it, so this
 * tries the one most phones register, then About phone, then the main Settings screen.
 */
fun openSystemUpdate(context: Context) {
    val actions = listOf(
        "android.settings.SYSTEM_UPDATE_SETTINGS",
        Settings.ACTION_DEVICE_INFO_SETTINGS,
        Settings.ACTION_SETTINGS,
    )
    for (action in actions) {
        try {
            context.startActivity(Intent(action).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            return
        } catch (_: ActivityNotFoundException) {
            // Try the next screen.
        }
    }
}
