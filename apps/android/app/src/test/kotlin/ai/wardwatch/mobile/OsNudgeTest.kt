package ai.wardwatch.mobile

import org.junit.Assert.assertEquals
import org.junit.Test
import java.time.LocalDate

class OsNudgeTest {
    private val today = LocalDate.of(2026, 11, 10)

    @Test fun recentPatchIsCurrent() =
        assertEquals(OsCheck(OsStatus.Current, 5), OsNudge.check("2026-11-05", today))

    @Test fun boundaryOfDueIsStillCurrent() =
        assertEquals(OsStatus.Current, OsNudge.check(today.minusDays(45).toString(), today).status)

    @Test fun missedBulletinIsDue() =
        assertEquals(OsStatus.Due, OsNudge.check(today.minusDays(46).toString(), today).status)

    @Test fun boundaryOfActNowIsStillDue() =
        assertEquals(OsStatus.Due, OsNudge.check(today.minusDays(90).toString(), today).status)

    @Test fun oldPatchIsActNow() =
        assertEquals(OsCheck(OsStatus.ActNow, 313), OsNudge.check("2026-01-01", today))

    @Test fun futurePatchCountsAsZeroDays() =
        assertEquals(OsCheck(OsStatus.Current, 0), OsNudge.check("2026-12-01", today))

    @Test fun missingOrGarbledPatchIsUnknown() {
        assertEquals(OsCheck(OsStatus.Unknown, null), OsNudge.check(null, today))
        assertEquals(OsCheck(OsStatus.Unknown, null), OsNudge.check("", today))
        assertEquals(OsCheck(OsStatus.Unknown, null), OsNudge.check("Nov 2026", today))
    }
}
