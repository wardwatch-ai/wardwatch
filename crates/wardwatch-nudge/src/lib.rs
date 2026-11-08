//! The nudge engine: given what needs updating on this device, decides how
//! hard to nudge for each update and when.
//!
//! It is pure: findings, the user's [`UpdatePlan`], approved broad alerts,
//! what was already sent ([`History`]) and the current local time go in, a
//! [`NudgePlan`] comes out. The desktop app shows the plan, sends its
//! notifications, records them in the history and calls [`plan`] again at
//! [`NudgePlan::next_check`] or whenever the inventory or catalog changes.
//!
//! Rules (design spec "Update plan 4", feasibility brief Option B):
//!
//! - Being exploited right now is always same day, amber, and reminded every
//!   [`URGENT_REMINDER`] until fixed. Unless it's a broad alert (the OS or a
//!   browser) that Matt hasn't approved: that waits for the Sunday Tune-Up.
//! - Everything else follows the rules grid: severity row by System,
//!   Browsers or Other apps. Same day nudges once a day; Sunday Tune-Up and
//!   Monthly are bundled into one notification on Sunday morning; Auto
//!   installs without asking.
//! - Nothing is sent or installed in quiet hours.
//! - Programs the user holds get nothing.

mod alerts;
mod plan;
mod schedule;

use std::collections::BTreeMap;

use jiff::{SignedDuration, Timestamp, Zoned};
use serde::{Deserialize, Serialize};

pub use alerts::{ALERTS_SCHEMA_VERSION, ApprovedAlert, ApprovedAlerts};
pub use plan::{
    Category, ClockTime, Hold, Level, PLAN_VERSION, Preset, QuietHours, Row, Rules, Timing,
    UpdatePlan,
};
pub use schedule::{TUNE_UP_TIME, last_tune_up, next_tune_up};
pub use wardwatch_feed::{AppKind, Finding, Severity};

/// How often an exploited hole is re-nudged while it stays open.
pub const URGENT_REMINDER: SignedDuration = SignedDuration::from_hours(4);
/// How often other same-day updates are re-nudged.
pub const SAME_DAY_REMINDER: SignedDuration = SignedDuration::from_hours(24);
/// How long after Sunday morning the tune-up notification may still go out
/// (a laptop opened Sunday evening still gets it; one opened Wednesday waits).
const TUNE_UP_WINDOW: SignedDuration = SignedDuration::from_hours(24);

/// How a nudge looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Strength {
    /// Being exploited right now: amber, act-now button.
    Urgent,
    /// A quiet outline button with the 3-bar severity meter.
    Quiet { bars: u8 },
}

/// One update this device should get.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nudge {
    /// The finding's key (its catalog app id).
    pub key: String,
    pub app_id: String,
    pub name: String,
    pub category: Category,
    pub level: Level,
    pub exploited: bool,
    pub strength: Strength,
    pub timing: Timing,
    /// An exploited OS or browser hole whose broad alert isn't approved yet.
    /// It waits for the tune-up instead of nudging today.
    pub awaiting_approval: bool,
    /// When this nudges (same day) or installs (auto) next. `None` when it
    /// waits for the tune-up notification.
    pub due_at: Option<Timestamp>,
}

/// The top card of Home and the menu bar popover.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TopCard {
    /// Something here is being exploited right now (beams on).
    UnderAttack { keys: Vec<String> },
    /// Nothing under attack: the Sunday Tune-Up, with everything the user
    /// could update now ("Update all 3"). `keys` may be empty (all clear).
    TuneUp {
        keys: Vec<String>,
        bars: u8,
        at: Timestamp,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    Urgent,
    SameDay,
    TuneUp,
}

/// A system notification to send now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub kind: NotificationKind,
    pub keys: Vec<String>,
    pub title: String,
    pub body: String,
    /// The approved broad alert this carries, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NudgePlan {
    /// Most urgent first.
    pub nudges: Vec<Nudge>,
    pub top_card: TopCard,
    /// Send these now, then [`History::record`] the plan.
    pub notify: Vec<Notification>,
    /// Keys to install now without asking (Auto). The updater still asks
    /// before closing an open app and waits for Wi-Fi and power.
    pub install: Vec<String>,
    /// This run handles this week's tune-up, even if there is nothing in it.
    pub tune_up_due: bool,
    /// Call [`plan`] again by then.
    pub next_check: Timestamp,
}

/// What was already sent, kept by the app between runs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    /// Last notification per finding key.
    #[serde(default)]
    pub notified: BTreeMap<String, Timestamp>,
    /// When the last weekly tune-up was handled.
    #[serde(default)]
    pub tune_up: Option<Timestamp>,
}

impl History {
    /// Records that `plan`'s notifications went out at `at`, and forgets
    /// programs that no longer need anything.
    pub fn record(&mut self, plan: &NudgePlan, at: Timestamp) {
        for n in plan
            .notify
            .iter()
            .filter(|n| n.kind != NotificationKind::TuneUp)
        {
            for key in &n.keys {
                self.notified.insert(key.clone(), at);
            }
        }
        if plan.tune_up_due {
            self.tune_up = Some(at);
        }
        self.notified
            .retain(|k, _| plan.nudges.iter().any(|n| &n.key == k));
    }
}

fn level(severity: Option<Severity>) -> Level {
    match severity {
        Some(Severity::Critical | Severity::High) => Level::Serious,
        // Unknown severity is not assumed minor.
        Some(Severity::Medium | Severity::Unknown) => Level::Moderate,
        Some(Severity::Low) | None => Level::Minor,
    }
}

fn category(f: &Finding) -> Category {
    match (f.kind, &f.browser) {
        (AppKind::Os, _) => Category::System,
        (_, Some(_)) => Category::Browsers,
        _ => Category::Apps,
    }
}

fn bars(level: Level) -> u8 {
    match level {
        Level::Serious => 3,
        Level::Moderate => 2,
        Level::Minor => 1,
    }
}

/// Builds the nudge plan for `now` (local time; quiet hours and Sunday
/// morning are in its time zone).
pub fn plan(
    findings: &[Finding],
    update_plan: &UpdatePlan,
    approvals: &ApprovedAlerts,
    history: &History,
    now: &Zoned,
) -> NudgePlan {
    let quiet = update_plan.quiet_hours;
    let at = now.timestamp();
    let allowed = |t: Timestamp| quiet.next_allowed(&t.to_zoned(now.time_zone().clone()));

    let mut nudges = Vec::new();
    let mut notify = Vec::new();
    let mut install = Vec::new();
    let mut wake: Vec<Timestamp> = Vec::new();

    for f in findings {
        let held = f
            .installs
            .iter()
            .any(|i| update_plan.is_held(&i.source, &i.id));
        if !f.update_available || held {
            continue;
        }
        let category = category(f);
        let level = level(f.severity);
        let rule = update_plan.rules.get(level, category);

        // The approval queue only gates exploited holes in the OS or a browser.
        let approval = (f.exploited && category != Category::Apps).then(|| {
            approvals.find(
                &f.app_id,
                |id| {
                    f.vulns
                        .iter()
                        .any(|v| v.exploited && (v.id == id || v.aliases.iter().any(|a| a == id)))
                },
                at,
            )
        });
        let awaiting_approval = matches!(approval, Some(None)) && rule != Timing::Auto;
        let alert = approval.flatten();

        let timing = match (f.exploited, rule) {
            (_, Timing::Auto) => Timing::Auto,
            (true, _) if awaiting_approval => Timing::SundayTuneup,
            (true, _) => Timing::SameDay,
            (false, rule) => rule,
        };
        let strength = if f.exploited {
            Strength::Urgent
        } else {
            Strength::Quiet { bars: bars(level) }
        };

        let every = if f.exploited {
            URGENT_REMINDER
        } else {
            SAME_DAY_REMINDER
        };
        let due_at = match timing {
            Timing::Auto => Some(allowed(at).timestamp()),
            Timing::SameDay => {
                let next = history
                    .notified
                    .get(&f.key)
                    .map_or(at, |last| (*last + every).max(at));
                Some(allowed(next).timestamp())
            }
            Timing::SundayTuneup | Timing::Monthly => None,
        };

        match (timing, due_at) {
            (_, Some(due)) if due > at => wake.push(due),
            (Timing::Auto, Some(_)) => install.push(f.key.clone()),
            (Timing::SameDay, Some(_)) => {
                notify.push(same_day_notification(f, level, alert));
                wake.push(allowed(at + every).timestamp());
            }
            _ => {}
        }

        nudges.push(Nudge {
            key: f.key.clone(),
            app_id: f.app_id.clone(),
            name: f.name.clone(),
            category,
            level,
            exploited: f.exploited,
            strength,
            timing,
            awaiting_approval,
            due_at,
        });
    }

    nudges.sort_by(|a, b| {
        let urgent = |n: &Nudge| n.exploited && !n.awaiting_approval;
        urgent(b)
            .cmp(&urgent(a))
            .then(b.exploited.cmp(&a.exploited))
            .then(b.level.cmp(&a.level))
            .then(a.name.cmp(&b.name))
    });

    // This week's tune-up: Sunday morning, after quiet hours, for a day.
    let last_sunday = last_tune_up(now);
    let sunday_at = allowed(last_sunday.timestamp()).timestamp();
    let handled = history
        .tune_up
        .is_some_and(|t| t >= last_sunday.timestamp());
    let this_week_open = !handled && at < sunday_at + TUNE_UP_WINDOW;
    let tune_up_due = this_week_open && sunday_at <= at;
    let next_sunday = allowed(next_tune_up(now).timestamp()).timestamp();
    // The tune-up the user sees coming: this week's until it's handled or missed.
    let upcoming = if this_week_open {
        sunday_at
    } else {
        next_sunday
    };
    wake.push(if upcoming > at { upcoming } else { next_sunday });

    if tune_up_due {
        let monthly = schedule::is_monthly(&last_sunday);
        let keys: Vec<String> = nudges
            .iter()
            .filter(|n| {
                n.timing == Timing::SundayTuneup || (monthly && n.timing == Timing::Monthly)
            })
            .map(|n| n.key.clone())
            .collect();
        if !keys.is_empty() {
            notify.push(tune_up_notification(keys));
        }
    }

    let under_attack: Vec<String> = nudges
        .iter()
        .filter(|n| n.exploited && !n.awaiting_approval)
        .map(|n| n.key.clone())
        .collect();
    let top_card = if under_attack.is_empty() {
        let waiting: Vec<&Nudge> = nudges.iter().filter(|n| n.timing != Timing::Auto).collect();
        TopCard::TuneUp {
            keys: waiting.iter().map(|n| n.key.clone()).collect(),
            bars: waiting.iter().map(|n| bars(n.level)).max().unwrap_or(0),
            at: upcoming,
        }
    } else {
        TopCard::UnderAttack { keys: under_attack }
    };

    NudgePlan {
        nudges,
        top_card,
        notify,
        install,
        tune_up_due,
        next_check: wake
            .into_iter()
            .min()
            .expect("the next tune-up is always pushed"),
    }
}

fn same_day_notification(f: &Finding, level: Level, alert: Option<&ApprovedAlert>) -> Notification {
    let (kind, title, body) = match alert {
        Some(a) => (NotificationKind::Urgent, a.title.clone(), a.body.clone()),
        None if f.exploited => (
            NotificationKind::Urgent,
            format!("Update {}.", f.name),
            if f.browser.is_some() {
                "This is being exploited right now. 1 minute to update. Tabs will come back.".into()
            } else {
                "This is being exploited right now.".into()
            },
        ),
        None => (
            NotificationKind::SameDay,
            format!("Update {} today.", f.name),
            match level {
                Level::Serious => "It fixes a serious security hole.",
                Level::Moderate => "It fixes a security hole.",
                Level::Minor => "An update is ready.",
            }
            .into(),
        ),
    };
    Notification {
        kind,
        keys: vec![f.key.clone()],
        title,
        body,
        alert_id: alert.map(|a| a.id.clone()),
    }
}

fn tune_up_notification(keys: Vec<String>) -> Notification {
    let body = match keys.len() {
        1 => "1 update is ready.".to_string(),
        n => format!("{n} updates are ready."),
    };
    Notification {
        kind: NotificationKind::TuneUp,
        keys,
        title: "Sunday Tune-Up".into(),
        body,
        alert_id: None,
    }
}
