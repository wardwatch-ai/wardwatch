//! App settings (the Settings page), stored as JSON in the app config folder.
//! For now that is the release channel the user wants updates from.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "settings.json";

/// Where updates come from. Mainline is numbered releases; nightly is a build
/// of `main` on each day it changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    #[default]
    Mainline,
    Nightly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub v: u32,
    pub channel: Channel,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            v: 1,
            channel: Channel::Mainline,
        }
    }
}

pub fn path_in(config_dir: &Path) -> PathBuf {
    config_dir.join(FILE_NAME)
}

/// Reads the settings, falling back to the defaults when the file is missing or
/// unreadable.
pub fn load(path: &Path) -> Settings {
    let Ok(bytes) = std::fs::read(path) else {
        return Settings::default();
    };
    match serde_json::from_slice::<Settings>(&bytes) {
        Ok(s) if s.v == 1 => s,
        _ => {
            eprintln!("wardwatch: ignoring an invalid {FILE_NAME}");
            Settings::default()
        }
    }
}

pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    if settings.v != 1 {
        return Err(format!("unsupported settings version {}", settings.v));
    }
    crate::store::save(path, settings, 1, "settings file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_mainline_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(dir.path());
        assert_eq!(load(&path).channel, Channel::Mainline);
        let nightly = Settings {
            v: 1,
            channel: Channel::Nightly,
        };
        save(&path, &nightly).unwrap();
        assert_eq!(load(&path), nightly);
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains(r#""channel": "nightly""#)
        );
    }

    #[test]
    fn ignores_bad_files_and_keeps_newer_ones() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(dir.path());
        std::fs::write(&path, r#"{"v":1,"channel":"beta"}"#).unwrap();
        assert_eq!(load(&path), Settings::default());
        std::fs::write(&path, r#"{"v":2,"channel":"nightly","more":1}"#).unwrap();
        assert_eq!(load(&path), Settings::default());
        assert!(save(&path, &Settings::default()).is_err());
        let bad = Settings {
            v: 2,
            ..Settings::default()
        };
        assert!(save(&dir.path().join("other.json"), &bad).is_err());
    }
}
