//! Broad alerts approved through the alert approval queue
//! (wardwatch-ai/terraform, `alerts/`).
//!
//! An exploited hole in the operating system or a browser reaches every user
//! of that OS or browser at once. Matt's rule (sprint 1, decision 9: "hold")
//! is that such an alert is held until he approves it, and only the weekly
//! tune-up goes out meanwhile. The queue's dispatcher will publish the
//! approved ones as `alerts.json`, next to `catalog.json` and signed with the
//! same trust chain. Until that exists the app passes [`ApprovedAlerts::none`],
//! so every broad alert waits.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

pub const ALERTS_SCHEMA_VERSION: u32 = 1;

/// One approved alert, with the copy as Matt approved it (edits included).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedAlert {
    /// The queue's proposal id.
    pub id: String,
    /// The catalog app id (`google-chrome`, `macos`).
    pub app_id: String,
    /// Vulnerability ids the alert is about (CVE or OSV ids).
    pub vulns: Vec<String>,
    pub title: String,
    pub body: String,
    pub approved_at: Timestamp,
    /// After this the alert no longer counts; the tune-up covers it.
    pub expires_at: Timestamp,
}

/// The contents of `alerts.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedAlerts {
    pub schema_version: u32,
    /// Refuse the file after this.
    pub expires_at: Timestamp,
    pub alerts: Vec<ApprovedAlert>,
}

impl ApprovedAlerts {
    /// No approvals: every broad alert is held.
    pub fn none() -> Self {
        ApprovedAlerts {
            schema_version: ALERTS_SCHEMA_VERSION,
            expires_at: Timestamp::MAX,
            alerts: Vec::new(),
        }
    }

    /// Parses `alerts.json`. The caller must already have checked its
    /// signature. Refuses unknown schema versions and an expired file, so a
    /// stale or replayed copy can't keep an old alert alive.
    pub fn from_json(bytes: &[u8], now: Timestamp) -> Result<Self, String> {
        let file: ApprovedAlerts = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if file.schema_version != ALERTS_SCHEMA_VERSION {
            return Err(format!(
                "unsupported alerts schema_version {}",
                file.schema_version
            ));
        }
        if file.expires_at <= now {
            return Err("alerts.json has expired".into());
        }
        Ok(file)
    }

    /// The live approval covering any of `vuln_ids` in `app_id`, if any.
    pub fn find(
        &self,
        app_id: &str,
        mut vuln_ids: impl FnMut(&str) -> bool,
        now: Timestamp,
    ) -> Option<&ApprovedAlert> {
        self.alerts.iter().find(|a| {
            self.expires_at > now
                && a.approved_at <= now
                && a.expires_at > now
                && a.app_id == app_id
                && a.vulns.iter().any(|v| vuln_ids(v))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"{"schema_version":1,"expires_at":"2026-11-09T00:00:00Z","alerts":[
        {"id":"p1","app_id":"google-chrome","vulns":["CVE-2026-1"],"title":"Update Chrome.",
         "body":"This is being exploited right now.","approved_at":"2026-11-08T12:00:00Z",
         "expires_at":"2026-11-08T18:00:00Z"}]}"#;

    fn t(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    #[test]
    fn finds_live_approvals_by_app_and_vuln() {
        let now = t("2026-11-08T13:00:00Z");
        let a = ApprovedAlerts::from_json(FILE.as_bytes(), now).unwrap();
        assert!(
            a.find("google-chrome", |v| v == "CVE-2026-1", now)
                .is_some()
        );
        assert!(
            a.find("google-chrome", |v| v == "CVE-2026-2", now)
                .is_none()
        );
        assert!(a.find("firefox", |v| v == "CVE-2026-1", now).is_none());
        let later = t("2026-11-08T19:00:00Z");
        assert!(
            a.find("google-chrome", |v| v == "CVE-2026-1", later)
                .is_none()
        );
    }

    #[test]
    fn refuses_expired_or_unknown_files() {
        assert!(ApprovedAlerts::from_json(FILE.as_bytes(), t("2026-11-09T00:00:00Z")).is_err());
        let v2 = FILE.replace("\"schema_version\":1", "\"schema_version\":2");
        assert!(ApprovedAlerts::from_json(v2.as_bytes(), t("2026-11-08T13:00:00Z")).is_err());
        let no_expiry = FILE.replace("\"expires_at\":\"2026-11-09T00:00:00Z\",", "");
        assert!(
            ApprovedAlerts::from_json(no_expiry.as_bytes(), t("2026-11-08T13:00:00Z")).is_err()
        );
    }
}
