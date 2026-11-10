package ai.wardwatch.mobile

import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.format.DateTimeParseException
import java.util.Locale

/** What the status card's button looks like. Amber only when the phone must act now. */
enum class StatusButton { ActNow, Quiet, None }

/** The card's mood: amber with beams on, all clear, or neither. */
enum class StatusTone { ActNow, Clear, Neutral }

/** The words on the home screen's status card (approved mobile design, 2026-11-10). */
data class StatusCopy(
    val eyebrow: String?,
    val title: String,
    val body: String,
    val detail: String?,
    val button: StatusButton,
    val tone: StatusTone,
    /** Severity-meter bars on a quiet button; 0 when we can't say how serious it is. */
    val bars: Int = 0,
) {
    /** Lighthouse beams on: the card needs the person now. */
    val beams: Boolean get() = tone == StatusTone.ActNow
}

object HomeCopy {
    // The sentences are English only, so the dates are too, until the app is translated.
    private val defaultDates: DateTimeFormatter = DateTimeFormatter.ofPattern("MMM d, yyyy", Locale.US)

    fun status(check: OsCheck, securityPatch: String?, dates: DateTimeFormatter = defaultDates): StatusCopy {
        val age = check.patchAgeDays
        return when (check.status) {
            OsStatus.ActNow -> StatusCopy(
                eyebrow = "NEEDS YOU TODAY",
                title = "Update this phone.",
                body = "Its security patch is ${days(age)} old, so it's missing fixes for attacks that are happening now.",
                detail = "About 15 minutes. Plug it in first.",
                button = StatusButton.ActNow,
                tone = StatusTone.ActNow,
            )
            OsStatus.Due -> StatusCopy(
                eyebrow = "DUE THIS WEEK",
                title = "A security update is due.",
                body = "The security patch is ${days(age)} old" +
                    (patchDate(securityPatch, dates)?.let { " ($it)" } ?: "") +
                    ". A newer one is likely waiting.",
                detail = null,
                button = StatusButton.Quiet,
                tone = StatusTone.Neutral,
                bars = 2,
            )
            OsStatus.Current -> StatusCopy(
                eyebrow = "ALL CLEAR",
                title = "This phone is up to date.",
                body = "It has a recent security patch. Open Wardwatch now and then to check again.",
                detail = null,
                button = StatusButton.None,
                tone = StatusTone.Clear,
            )
            OsStatus.Unknown -> StatusCopy(
                eyebrow = null,
                title = "Check for a system update.",
                body = "This phone didn't say how old its security patch is. Open Settings to see if an update is waiting.",
                detail = null,
                button = StatusButton.Quiet,
                tone = StatusTone.Neutral,
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
