//! Small versioned JSON files in the app config folder (the update plan,
//! settings). Writes are atomic, and a file saved by a newer Wardwatch is never
//! overwritten, so an older build can't silently throw away what it can't read.

use std::path::Path;

use serde::Serialize;

/// Writes `value` to `path` unless the file there has a `v` above `version`.
pub fn save<T: Serialize>(path: &Path, value: &T, version: u64, what: &str) -> Result<(), String> {
    if let Ok(bytes) = std::fs::read(path)
        && let Ok(existing) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && existing
            .get("v")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|v| v > version)
    {
        return Err(format!(
            "this {what} was saved by a newer version of Wardwatch"
        ));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("could not create the config folder: {e}"))?;
    }
    let json = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("could not save the {what}: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("could not save the {what}: {e}"))
}
