package ai.wardwatch.mobile

import java.time.LocalDate
import java.time.format.DateTimeParseException
import java.time.temporal.ChronoUnit

/** How urgently this phone needs its system update. */
enum class OsStatus { Current, Due, ActNow, Unknown }

data class OsCheck(val status: OsStatus, val patchAgeDays: Long?)

/**
 * Judges the phone by the age of its Android security patch level.
 *
 * Google publishes a security bulletin every month, and most months fix at least one
 * critical bug. A patch older than [DUE_AFTER_DAYS] has missed a bulletin; one older
 * than [ACT_NOW_AFTER_DAYS] has missed several, so it gets the amber act-now nudge.
 * Once the catalog carries Android bulletins, this can name the exploited bugs instead.
 */
object OsNudge {
    const val DUE_AFTER_DAYS = 45L
    const val ACT_NOW_AFTER_DAYS = 90L

    fun check(securityPatch: String?, today: LocalDate): OsCheck {
        val patch = securityPatch?.let {
            try {
                LocalDate.parse(it)
            } catch (_: DateTimeParseException) {
                null
            }
        } ?: return OsCheck(OsStatus.Unknown, null)

        val age = ChronoUnit.DAYS.between(patch, today).coerceAtLeast(0)
        val status = when {
            age > ACT_NOW_AFTER_DAYS -> OsStatus.ActNow
            age > DUE_AFTER_DAYS -> OsStatus.Due
            else -> OsStatus.Current
        }
        return OsCheck(status, age)
    }
}
