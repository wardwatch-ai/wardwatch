package ai.wardwatch.mobile

import android.content.pm.ApplicationInfo
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.os.Build

data class InstalledApp(val packageName: String, val label: String, val version: String?)

object InstalledApps {
    /**
     * Apps the person installed or updated, with their versions. Preinstalled system
     * apps that were never updated are left out: the system update covers them.
     */
    fun list(pm: PackageManager): List<InstalledApp> =
        installedPackages(pm)
            .mapNotNull { info ->
                val app = info.applicationInfo ?: return@mapNotNull null
                if (!isUserFacing(app.flags)) return@mapNotNull null
                InstalledApp(
                    packageName = info.packageName,
                    label = app.loadLabel(pm).toString(),
                    version = info.versionName,
                )
            }
            .sortedBy { it.label.lowercase() }

    internal fun isUserFacing(flags: Int): Boolean =
        flags and ApplicationInfo.FLAG_SYSTEM == 0 ||
            flags and ApplicationInfo.FLAG_UPDATED_SYSTEM_APP != 0

    private fun installedPackages(pm: PackageManager): List<PackageInfo> =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            pm.getInstalledPackages(PackageManager.PackageInfoFlags.of(0))
        } else {
            @Suppress("DEPRECATION")
            pm.getInstalledPackages(0)
        }
}
