package ai.wardwatch.mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale

class HomeCopyTest {
    private val dates = DateTimeFormatter.ofPattern("MMM d, yyyy", Locale.US)
    private val today = LocalDate.parse("2026-11-10")
    private fun copy(patch: String?) = HomeCopy.status(OsNudge.check(patch, today), patch, dates)

    @Test fun actNowIsTheOnlyAmberState() {
        val c = copy("2026-08-06")
        assertEquals(StatusButton.ActNow, c.button)
        assertTrue(c.beams)
        assertEquals("Its security patch is 96 days old, so it's missing fixes for attacks that are happening now.", c.body)
        for (patch in listOf("2026-09-19", "2026-11-01", null, "garbage")) {
            assertTrue(copy(patch).button != StatusButton.ActNow)
            assertFalse(copy(patch).beams)
        }
    }

    @Test fun dueUsesTheQuietButtonAndNamesThePatchDate() {
        val c = copy("2026-09-19")
        assertEquals(StatusButton.Quiet, c.button)
        assertEquals(2, c.bars)
        assertEquals("The security patch is 52 days old (Sep 19, 2026). A newer one is likely waiting.", c.body)
    }

    @Test fun currentHasNoButton() {
        val c = copy("2026-11-01")
        assertEquals(StatusButton.None, c.button)
        assertEquals(StatusTone.Clear, c.tone)
    }

    @Test fun unknownAsksToCheck() {
        val c = copy("")
        assertNull(c.eyebrow)
        assertEquals(StatusButton.Quiet, c.button)
        // We can't say how serious it is, so no severity meter.
        assertEquals(0, c.bars)
    }

    @Test fun deviceLineSkipsAMissingPatch() {
        assertEquals("Android 16 · patch Aug 6, 2026", HomeCopy.deviceLine(DeviceInfo("Google", "Pixel 8", "16", "2026-08-06"), dates))
        assertEquals("Android 16", HomeCopy.deviceLine(DeviceInfo("Google", "Pixel 8", "16", ""), dates))
    }

    @Test fun oneDayIsSingular() = assertEquals("1 day", HomeCopy.days(1))
}
