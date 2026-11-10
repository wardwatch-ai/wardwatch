package ai.wardwatch.mobile

import android.os.Build

/** What the phone says about itself. Read locally; nothing is sent anywhere yet. */
data class DeviceInfo(
    val manufacturer: String,
    val model: String,
    val androidVersion: String,
    val sdkInt: Int,
    val securityPatch: String,
) {
    val name: String
        get() = if (model.startsWith(manufacturer, ignoreCase = true)) {
            model
        } else {
            "${manufacturer.replaceFirstChar { it.uppercase() }} $model"
        }

    companion object {
        fun current() = DeviceInfo(
            manufacturer = Build.MANUFACTURER,
            model = Build.MODEL,
            androidVersion = Build.VERSION.RELEASE,
            sdkInt = Build.VERSION.SDK_INT,
            securityPatch = Build.VERSION.SECURITY_PATCH,
        )
    }
}
