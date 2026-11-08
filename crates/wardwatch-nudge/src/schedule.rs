//! Local-time arithmetic: quiet hours and Sunday Tune-Up times. All of it
//! goes through `jiff::Zoned`, so DST changes are handled by the time zone
//! database rather than by us.

use jiff::civil::{Date, Time, Weekday};
use jiff::{ToSpan, Zoned};

use crate::plan::QuietHours;

/// When the Sunday Tune-Up nudges, local time (later if quiet hours cover it).
pub const TUNE_UP_TIME: Time = Time::constant(10, 0, 0, 0);

impl QuietHours {
    pub fn is_off(&self) -> bool {
        self.start == self.end
    }

    /// Whether `t` falls in quiet hours (`start` inclusive, `end` exclusive).
    pub fn contains(&self, t: &Zoned) -> bool {
        let (start, end, now) = (self.start.0, self.end.0, t.time());
        if start < end {
            start <= now && now < end
        } else if start > end {
            now >= start || now < end
        } else {
            false
        }
    }

    /// The first moment at or after `t` that is outside quiet hours.
    pub fn next_allowed(&self, t: &Zoned) -> Zoned {
        if !self.contains(t) {
            return t.clone();
        }
        // In quiet hours: they end today at `end`, or tomorrow when we are
        // in the evening part of hours that wrap past midnight.
        let mut day = t.date();
        if t.time() >= self.end.0 {
            day = day.tomorrow().expect("date in range");
        }
        at_local(day, self.end.0, t)
    }
}

/// `time` on `date` in `like`'s time zone. A time skipped by a DST jump moves
/// forward, like a wall clock.
fn at_local(date: Date, time: Time, like: &Zoned) -> Zoned {
    date.to_datetime(time)
        .to_zoned(like.time_zone().clone())
        .expect("local time in range")
}

/// The most recent Sunday Tune-Up time at or before `now`.
pub fn last_tune_up(now: &Zoned) -> Zoned {
    let today = now.date();
    let back = today.weekday().to_sunday_zero_offset();
    let sunday = today
        .checked_sub(i64::from(back).days())
        .expect("date in range");
    let at = at_local(sunday, TUNE_UP_TIME, now);
    if at <= *now {
        at
    } else {
        at_local(
            sunday.checked_sub(7.days()).expect("date in range"),
            TUNE_UP_TIME,
            now,
        )
    }
}

/// The next Sunday Tune-Up time strictly after `now`.
pub fn next_tune_up(now: &Zoned) -> Zoned {
    let last = last_tune_up(now);
    at_local(
        last.date().checked_add(7.days()).expect("date in range"),
        TUNE_UP_TIME,
        now,
    )
}

/// The monthly tune-up is the first Sunday Tune-Up of each month.
pub fn is_monthly(tune_up: &Zoned) -> bool {
    tune_up.date().weekday() == Weekday::Sunday && tune_up.day() <= 7
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::ClockTime;

    fn ny(s: &str) -> Zoned {
        format!("{s}[America/New_York]").parse().unwrap()
    }

    fn quiet(start: &str, end: &str) -> QuietHours {
        QuietHours {
            start: start.parse::<ClockTime>().unwrap(),
            end: end.parse().unwrap(),
        }
    }

    #[test]
    fn overnight_quiet_hours() {
        let q = quiet("22:00", "08:00");
        assert!(!q.contains(&ny("2026-11-08T21:59")));
        assert!(q.contains(&ny("2026-11-08T22:00")));
        assert!(q.contains(&ny("2026-11-09T07:59")));
        assert!(!q.contains(&ny("2026-11-09T08:00")));
        assert_eq!(
            q.next_allowed(&ny("2026-11-08T23:30")),
            ny("2026-11-09T08:00")
        );
        assert_eq!(
            q.next_allowed(&ny("2026-11-09T03:00")),
            ny("2026-11-09T08:00")
        );
        assert_eq!(
            q.next_allowed(&ny("2026-11-09T12:00")),
            ny("2026-11-09T12:00")
        );
    }

    #[test]
    fn daytime_and_off_quiet_hours() {
        let q = quiet("09:00", "17:00");
        assert!(q.contains(&ny("2026-11-09T12:00")));
        assert!(!q.contains(&ny("2026-11-09T18:00")));
        assert_eq!(
            q.next_allowed(&ny("2026-11-09T12:00")),
            ny("2026-11-09T17:00")
        );

        let off = quiet("00:00", "00:00");
        assert!(off.is_off());
        assert!(!off.contains(&ny("2026-11-09T03:00")));
    }

    #[test]
    fn quiet_hours_ending_in_a_dst_gap_end_at_the_next_wall_clock_time() {
        // 2026-03-08 02:00 does not exist in New York.
        let q = quiet("23:00", "02:30");
        assert_eq!(
            q.next_allowed(&ny("2026-03-08T01:00")),
            ny("2026-03-08T03:30")
        );
    }

    #[test]
    fn tune_up_times() {
        // 2026-11-08 is a Sunday.
        assert_eq!(
            last_tune_up(&ny("2026-11-08T10:00")),
            ny("2026-11-08T10:00")
        );
        assert_eq!(
            last_tune_up(&ny("2026-11-08T09:59")),
            ny("2026-11-01T10:00")
        );
        assert_eq!(
            last_tune_up(&ny("2026-11-11T15:00")),
            ny("2026-11-08T10:00")
        );
        assert_eq!(
            next_tune_up(&ny("2026-11-08T10:00")),
            ny("2026-11-15T10:00")
        );
        assert_eq!(
            next_tune_up(&ny("2026-11-07T23:00")),
            ny("2026-11-08T10:00")
        );
        assert!(is_monthly(&ny("2026-11-01T10:00")));
        assert!(!is_monthly(&ny("2026-11-08T10:00")));
    }
}
