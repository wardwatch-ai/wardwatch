package ai.wardwatch.mobile

import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.format.DateTimeParseException
import java.time.format.FormatStyle

/** What the status card's button looks like. Amber only when the phone must act now. */
enum class StatusButton { ActNow, Quiet, None }

/** The words on the home screen's status card (approved mobile design, 2026-11-10). */
data class StatusCopy(
    val eyebrow: String?,
    val title: String,
    val body: String,
    val detail: String?,
    val button: StatusButton,
    /** Lighthouse beams on: the card needs the person. */
    val beams: Boolean,
    /** Shows the eyebrow in the all-clear color. */
    val allClear: Boolean = false,
)

object HomeCopy {
    private val defaultDates: DateTimeFormatter = DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM)

    fun status(check: OsCheck, securityPatch: String?, dates: DateTimeFormatter = defaultDates): StatusCopy {
        val age = check.patchAgeDays
        return when (check.status) {
            OsStatus.ActNow -> StatusCopy(
                eyebrow = "NEEDS YOU TODAY",
                title = "Update this phone.",
                body = "Its security patch is ${days(age)} old, so it's missing fixes for attacks that are happening now.",
                detail = "About 15 minutes. Plug it in first.",
                button = StatusButton.ActNow,
                beams = true,
            )
            OsStatus.Due -> StatusCopy(
                eyebrow = "DUE THIS WEEK",
                title = "A security update is due.",
                body = "The security patch is ${days(age)} old" +
                    (patchDate(securityPatch, dates)?.let { " ($it)" } ?: "") +
                    ". A newer one is likely waiting.",
                detail = null,
                button = StatusButton.Quiet,
                beams = false,
            )
            OsStatus.Current -> StatusCopy(
                eyebrow = "ALL CLEAR",
                title = "This phone is up to date.",
                body = "It has a recent security patch. We'll tell you when the next one is waiting.",
                detail = null,
                button = StatusButton.None,
                beams = false,
                allClear = true,
            )
            OsStatus.Unknown -> StatusCopy(
                eyebrow = null,
                title = "Check for a system update.",
                body = "This phone didn't say how old its security patch is. Open Settings to see if an update is waiting.",
                detail = null,
                button = StatusButton.Quiet,
                beams = false,
            )
        }
    }

    /** "Android 16 · patch Aug 6, 2026", for the device row. */
    fun deviceLine(device: DeviceInfo, dates: DateTimeFormatter = defaultDates): String =
        listOfNotNull(
            "Android ${device.androidVersion}",
            patchDate(device.securityPatch, dates)?.let { "patch $it" },
        ).joinToString(" · ")

    fun days(n: Long?): String = if (n == 1L) "1 day" else "${n ?: 0} days"

    fun patchDate(securityPatch: String?, dates: DateTimeFormatter = defaultDates): String? =
        securityPatch?.let {
            try {
                LocalDate.parse(it).format(dates)
            } catch (_: DateTimeParseException) {
                null
            }
        }
}
