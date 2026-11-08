//! The user's Update plan (design spec, "Update plan 4"): a rules grid of
//! severity against kind of software, quiet hours, and holds.
//!
//! The JSON shape is shared with the desktop app, which stores it in the app
//! config dir behind `get_plan` / `set_plan`.

use std::fmt;
use std::str::FromStr;

use jiff::civil::Time;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// When updates in one box of the rules grid happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Timing {
    /// Nudge today, and keep reminding until it's done.
    SameDay,
    /// Wait for the weekly Sunday Tune-Up.
    SundayTuneup,
    /// Wait for the first Sunday Tune-Up of the month.
    Monthly,
    /// Install by itself, without asking (still outside quiet hours).
    Auto,
}

/// The severity rows of the rules grid. "Being exploited right now" is not a
/// row here: it is locked to [`Timing::SameDay`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Minor, or an update with no security fix.
    Minor,
    Moderate,
    Serious,
}

/// The columns of the rules grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    System,
    Browsers,
    Apps,
}

/// One severity row: a timing per column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub system: Timing,
    pub browsers: Timing,
    pub apps: Timing,
}

impl Row {
    const fn new(system: Timing, browsers: Timing, apps: Timing) -> Self {
        Row {
            system,
            browsers,
            apps,
        }
    }

    const fn all(t: Timing) -> Self {
        Row::new(t, t, t)
    }

    pub fn get(&self, category: Category) -> Timing {
        match category {
            Category::System => self.system,
            Category::Browsers => self.browsers,
            Category::Apps => self.apps,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    pub serious: Row,
    pub moderate: Row,
    pub minor: Row,
}

impl Rules {
    pub fn get(&self, level: Level, category: Category) -> Timing {
        match level {
            Level::Serious => self.serious,
            Level::Moderate => self.moderate,
            Level::Minor => self.minor,
        }
        .get(category)
    }
}

use Timing::{Auto, Monthly, SameDay, SundayTuneup};

/// The three presets on the Update plan screen. A preset button is lit only
/// when the grid matches it exactly ([`Preset::matching`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Relaxed,
    Balanced,
    Eager,
}

impl Preset {
    pub const ALL: [Preset; 3] = [Preset::Relaxed, Preset::Balanced, Preset::Eager];

    pub const fn rules(self) -> Rules {
        match self {
            Preset::Relaxed => Rules {
                serious: Row::new(SundayTuneup, SameDay, SundayTuneup),
                moderate: Row::all(Monthly),
                minor: Row::all(Monthly),
            },
            Preset::Balanced => Rules {
                serious: Row::all(SameDay),
                moderate: Row::all(SundayTuneup),
                minor: Row::new(SundayTuneup, SundayTuneup, Monthly),
            },
            Preset::Eager => Rules {
                serious: Row::new(SameDay, Auto, Auto),
                moderate: Row::all(Auto),
                minor: Row::all(Auto),
            },
        }
    }

    /// The preset these rules are exactly equal to, if any.
    pub fn matching(rules: &Rules) -> Option<Preset> {
        Preset::ALL.into_iter().find(|p| p.rules() == *rules)
    }
}

/// A time of day as `"HH:MM"`, the shape the plan file uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockTime(pub Time);

impl ClockTime {
    pub fn new(hour: i8, minute: i8) -> Option<Self> {
        Time::new(hour, minute, 0, 0).ok().map(ClockTime)
    }
}

impl fmt::Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.0.hour(), self.0.minute())
    }
}

impl FromStr for ClockTime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || format!("expected a time like \"22:00\", got {s:?}");
        let (h, m) = s.split_once(':').ok_or_else(bad)?;
        if h.len() != 2 || m.len() != 2 {
            return Err(bad());
        }
        let h: i8 = h.parse().map_err(|_| bad())?;
        let m: i8 = m.parse().map_err(|_| bad())?;
        ClockTime::new(h, m).ok_or_else(bad)
    }
}

impl Serialize for ClockTime {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ClockTime {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// No nudges and no installs from `start` until `end`, in local time. It may
/// wrap past midnight. `start == end` means no quiet hours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuietHours {
    pub start: ClockTime,
    pub end: ClockTime,
}

impl Default for QuietHours {
    fn default() -> Self {
        QuietHours {
            start: ClockTime(Time::constant(22, 0, 0, 0)),
            end: ClockTime(Time::constant(8, 0, 0, 0)),
        }
    }
}

/// A program the user chose to hold back. Keyed like the scanner: the
/// inventory `source` name and the program's `id` in that source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hold {
    pub source: String,
    pub id: String,
    /// Date the hold started, `YYYY-MM-DD`.
    pub since: String,
}

pub const PLAN_VERSION: u32 = 1;

/// Everything the user controls about nudging.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePlan {
    pub v: u32,
    pub rules: Rules,
    pub quiet_hours: QuietHours,
    #[serde(default)]
    pub holds: Vec<Hold>,
}

impl Default for UpdatePlan {
    /// Balanced, quiet from 22:00 to 08:00, nothing held.
    fn default() -> Self {
        UpdatePlan {
            v: PLAN_VERSION,
            rules: Preset::Balanced.rules(),
            quiet_hours: QuietHours::default(),
            holds: Vec::new(),
        }
    }
}

impl UpdatePlan {
    /// Parses a stored plan, refusing versions this build doesn't know.
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let plan: UpdatePlan = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if plan.v != PLAN_VERSION {
            return Err(format!("unsupported plan version {}", plan.v));
        }
        Ok(plan)
    }

    pub fn is_held(&self, source: &str, id: &str) -> bool {
        self.holds.iter().any(|h| h.source == source && h.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_shape_the_desktop_app_stores() {
        let json = br#"{"v":1,
            "rules":{"serious":{"system":"same_day","browsers":"same_day","apps":"same_day"},
                     "moderate":{"system":"sunday_tuneup","browsers":"sunday_tuneup","apps":"sunday_tuneup"},
                     "minor":{"system":"sunday_tuneup","browsers":"sunday_tuneup","apps":"monthly"}},
            "quiet_hours":{"start":"22:00","end":"08:00"},
            "holds":[{"source":"mac_app","id":"com.apple.FinalCut","since":"2026-11-02"}]}"#;
        let plan = UpdatePlan::from_json(json).unwrap();
        assert_eq!(Preset::matching(&plan.rules), Some(Preset::Balanced));
        assert_eq!(plan.quiet_hours, QuietHours::default());
        assert!(plan.is_held("mac_app", "com.apple.FinalCut"));
        assert!(!plan.is_held("homebrew_cask", "com.apple.FinalCut"));

        let back: UpdatePlan =
            serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
        assert_eq!(back, plan);
        assert_eq!(UpdatePlan::default().rules, plan.rules);
    }

    #[test]
    fn refuses_unknown_versions_and_bad_times() {
        let plan = serde_json::to_string(&UpdatePlan::default()).unwrap();
        assert!(UpdatePlan::from_json(plan.replace("\"v\":1", "\"v\":2").as_bytes()).is_err());
        for bad in ["24:00", "7:30", "07:3", "07-30", "ab:cd", "07:60"] {
            assert!(bad.parse::<ClockTime>().is_err(), "{bad}");
        }
        assert_eq!("07:30".parse::<ClockTime>().unwrap().to_string(), "07:30");
    }

    #[test]
    fn editing_any_box_unlights_every_preset() {
        for preset in Preset::ALL {
            let mut rules = preset.rules();
            assert_eq!(Preset::matching(&rules), Some(preset));
            rules.minor.apps = if rules.minor.apps == SameDay {
                Auto
            } else {
                SameDay
            };
            assert_eq!(Preset::matching(&rules), None);
        }
    }
}
