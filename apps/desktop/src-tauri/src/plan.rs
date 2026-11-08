//! The user's Update plan and holds, stored as JSON in the app config folder.
//! The nudge engine reads the same shape (`v: 1`). Being exploited right now is
//! not a rule: it is always same day.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "update-plan.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Timing {
    SameDay,
    SundayTuneup,
    Monthly,
    Auto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindRules {
    pub system: Timing,
    pub browsers: Timing,
    pub apps: Timing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub serious: KindRules,
    pub moderate: KindRules,
    pub minor: KindRules,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuietHours {
    /// "HH:MM", 24-hour.
    pub start: String,
    pub end: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hold {
    /// A catalog `Source` in snake_case, or "os".
    pub source: String,
    pub id: String,
    /// "YYYY-MM-DD"
    pub since: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlan {
    pub v: u32,
    pub rules: Rules,
    pub quiet_hours: QuietHours,
    pub holds: Vec<Hold>,
}

const fn same(t: Timing) -> KindRules {
    KindRules {
        system: t,
        browsers: t,
        apps: t,
    }
}

impl Default for UpdatePlan {
    /// The Balanced preset, 10 pm to 8 am quiet hours, nothing held.
    fn default() -> Self {
        UpdatePlan {
            v: 1,
            rules: Rules {
                serious: same(Timing::SameDay),
                moderate: same(Timing::SundayTuneup),
                minor: KindRules {
                    system: Timing::SundayTuneup,
                    browsers: Timing::SundayTuneup,
                    apps: Timing::Monthly,
                },
            },
            quiet_hours: QuietHours {
                start: "22:00".into(),
                end: "08:00".into(),
            },
            holds: Vec::new(),
        }
    }
}

const MAX_HOLDS: usize = 500;
const MAX_ID_LEN: usize = 512;

fn is_hh_mm(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && b[..2].iter().chain(&b[3..]).all(u8::is_ascii_digit)
        && s[..2].parse::<u8>().is_ok_and(|h| h < 24)
        && s[3..].parse::<u8>().is_ok_and(|m| m < 60)
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

impl UpdatePlan {
    /// Checks what serde can't: the version, time and date formats, and sizes.
    pub fn validate(&self) -> Result<(), String> {
        if self.v != 1 {
            return Err(format!("unsupported plan version {}", self.v));
        }
        if !is_hh_mm(&self.quiet_hours.start) || !is_hh_mm(&self.quiet_hours.end) {
            return Err("quiet hours must be HH:MM".into());
        }
        if self.holds.len() > MAX_HOLDS {
            return Err(format!("at most {MAX_HOLDS} holds"));
        }
        for h in &self.holds {
            if h.source.is_empty()
                || h.source.len() > 32
                || h.id.is_empty()
                || h.id.len() > MAX_ID_LEN
            {
                return Err("hold source or id has a bad length".into());
            }
            if !is_date(&h.since) {
                return Err("hold dates must be YYYY-MM-DD".into());
            }
        }
        Ok(())
    }
}

pub fn path_in(config_dir: &Path) -> PathBuf {
    config_dir.join(FILE_NAME)
}

/// Reads the plan, falling back to the default when the file is missing or
/// unreadable (a corrupt file must not lock the user out of their settings).
pub fn load(path: &Path) -> UpdatePlan {
    let Ok(bytes) = std::fs::read(path) else {
        return UpdatePlan::default();
    };
    match serde_json::from_slice::<UpdatePlan>(&bytes) {
        Ok(plan) if plan.validate().is_ok() => plan,
        _ => {
            eprintln!("wardwatch: ignoring an invalid {FILE_NAME}");
            UpdatePlan::default()
        }
    }
}

/// Validates and writes the plan atomically (temp file, then rename). Refuses to
/// overwrite a plan saved by a newer Wardwatch (`v` above ours), so an older
/// build never silently throws away settings it can't read.
pub fn save(path: &Path, plan: &UpdatePlan) -> Result<(), String> {
    plan.validate()?;
    crate::store::save(path, plan, 1, "plan")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_the_shared_json_shape() {
        let json = r#"{"v":1,"rules":{"serious":{"system":"same_day","browsers":"same_day","apps":"same_day"},
            "moderate":{"system":"sunday_tuneup","browsers":"sunday_tuneup","apps":"sunday_tuneup"},
            "minor":{"system":"sunday_tuneup","browsers":"sunday_tuneup","apps":"monthly"}},
            "quiet_hours":{"start":"22:00","end":"08:00"},
            "holds":[{"source":"mac_app","id":"com.apple.FinalCut","since":"2026-11-02"}]}"#;
        let plan: UpdatePlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.rules, UpdatePlan::default().rules);
        assert_eq!(plan.holds[0].id, "com.apple.FinalCut");
        assert_eq!(plan.validate(), Ok(()));
    }

    #[test]
    fn rejects_unknown_fields_and_bad_values() {
        assert!(serde_json::from_str::<UpdatePlan>(r#"{"v":1,"extra":true}"#).is_err());
        let mut plan = UpdatePlan::default();
        plan.quiet_hours.start = "25:00".into();
        assert!(plan.validate().is_err());
        let mut plan = UpdatePlan::default();
        plan.holds.push(Hold {
            source: "os".into(),
            id: "macos".into(),
            since: "Nov 2".into(),
        });
        assert!(plan.validate().is_err());
        let plan = UpdatePlan {
            v: 2,
            ..UpdatePlan::default()
        };
        assert!(plan.validate().is_err());
    }

    #[test]
    fn saves_then_loads_and_survives_a_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(&dir.path().join("config"));
        assert_eq!(load(&path), UpdatePlan::default());

        let mut plan = UpdatePlan::default();
        plan.rules.serious.apps = Timing::Auto;
        save(&path, &plan).unwrap();
        assert_eq!(load(&path), plan);

        std::fs::write(&path, b"{not json").unwrap();
        assert_eq!(load(&path), UpdatePlan::default());
        // A corrupt file can be replaced; a newer version's file can't.
        save(&path, &plan).unwrap();
        std::fs::write(&path, br#"{"v":2}"#).unwrap();
        assert!(save(&path, &plan).is_err());
    }
}
