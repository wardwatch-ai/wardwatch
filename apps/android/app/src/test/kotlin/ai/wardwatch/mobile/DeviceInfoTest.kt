package ai.wardwatch.mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceInfoTest {
    private fun device(manufacturer: String, model: String) =
        DeviceInfo(manufacturer, model, "16", "2026-11-05")

    @Test fun prefixesManufacturer() = assertEquals("Samsung SM-S918B", device("samsung", "SM-S918B").name)

    @Test fun prefixesShortModel() = assertEquals("Google Pixel 9", device("Google", "Pixel 9").name)

    @Test fun keepsModelThatAlreadyNamesMaker() = assertEquals("OnePlus 12", device("OnePlus", "OnePlus 12").name)

    @Test fun userFacingApps() {
        assertTrue(InstalledApps.isUserFacing(0))
        assertFalse(InstalledApps.isUserFacing(android.content.pm.ApplicationInfo.FLAG_SYSTEM))
        assertTrue(
            InstalledApps.isUserFacing(
                android.content.pm.ApplicationInfo.FLAG_SYSTEM or android.content.pm.ApplicationInfo.FLAG_UPDATED_SYSTEM_APP,
            ),
        )
    }
}
