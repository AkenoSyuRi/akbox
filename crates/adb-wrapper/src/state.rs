//! Runtime device-selection memory, scoped by terminal/session.
//!
//! Preferences live in `config.toml`; ephemeral selections live here so that
//! clearing a remembered device never rewrites user configuration.

use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub serial: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub selections: BTreeMap<String, Selection>,
}

impl State {
    fn prune(&mut self, ttl_secs: u64, now: u64) {
        if ttl_secs == 0 {
            return;
        }
        self.selections
            .retain(|_, selection| now.saturating_sub(selection.updated_at) <= ttl_secs);
    }
}

pub fn state_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|dir| dir.join("adb-wrapper").join("state.json"))
}

pub fn recall(scope: &str, ttl_secs: u64) -> Option<String> {
    let path = state_path()?;
    recall_from_path(&path, scope, ttl_secs)
}

pub fn remember(scope: &str, serial: &str, ttl_secs: u64) {
    let Some(path) = state_path() else {
        return;
    };
    if let Err(err) = remember_at_path(&path, scope, serial, ttl_secs) {
        eprintln!("adb-wrapper: warning: failed to save device memory: {err:#}");
    }
}

pub fn forget(scope: &str) -> Result<()> {
    let Some(path) = state_path() else {
        return Ok(());
    };
    forget_at_path(&path, scope).context("adb-wrapper: failed to clear device memory")
}

pub fn recall_from_path(path: &Path, scope: &str, ttl_secs: u64) -> Option<String> {
    let state = match load_state_from_path(path) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("adb-wrapper: warning: {err:#}, ignoring device memory");
            return None;
        }
    };

    let selection = state.selections.get(scope)?;
    if ttl_secs != 0 && now_secs().saturating_sub(selection.updated_at) > ttl_secs {
        return None;
    }
    Some(selection.serial.clone())
}

pub fn remember_at_path(path: &Path, scope: &str, serial: &str, ttl_secs: u64) -> Result<()> {
    let _lock = StateLock::acquire(path)?;
    let mut state = load_state_or_default(path);
    let now = now_secs();
    state.prune(ttl_secs, now);
    state.selections.insert(
        scope.to_string(),
        Selection {
            serial: serial.to_string(),
            updated_at: now,
        },
    );
    save_state_to_path(&state, path)
}

pub fn forget_at_path(path: &Path, scope: &str) -> Result<()> {
    let _lock = StateLock::acquire(path)?;
    if !path.is_file() {
        return Ok(());
    }
    let mut state = load_state_from_path(path)?;
    if state.selections.remove(scope).is_none() {
        return Ok(());
    }
    save_state_to_path(&state, path)
}

/// Cross-process exclusive lock protecting the read-modify-write cycle of the
/// state file.
struct StateLock {
    file: File,
}

impl StateLock {
    fn acquire(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create device memory directory: {}",
                    parent.display()
                )
            })?;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("state.json");
        let lock_path = path.with_file_name(format!("{file_name}.lock"));
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .with_context(|| {
                format!(
                    "failed to open device memory lock: {}",
                    lock_path.display()
                )
            })?;
        file.lock_exclusive().with_context(|| {
            format!(
                "failed to lock device memory: {}",
                lock_path.display()
            )
        })?;
        Ok(Self { file })
    }
}

impl Drop for StateLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

fn load_state_or_default(path: &Path) -> State {
    match load_state_from_path(path) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("adb-wrapper: warning: {err:#}, resetting device memory");
            State::default()
        }
    }
}

pub fn load_state_from_path(path: &Path) -> Result<State> {
    if !path.is_file() {
        return Ok(State::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("invalid device memory: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("invalid device memory: {}", path.display()))
}

fn save_state_to_path(state: &State, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create device memory directory: {}",
                parent.display()
            )
        })?;
    }

    let content =
        serde_json::to_string_pretty(state).context("failed to serialize device memory")?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("state.json");
    let tmp = path.with_file_name(format!("{file_name}.{}.tmp", std::process::id()));

    std::fs::write(&tmp, content)
        .with_context(|| format!("failed to write device memory: {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("failed to replace device memory: {}", path.display()))?;
    Ok(())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remember_and_recall_are_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        remember_at_path(&path, "tty:1", "AAA111", 60).unwrap();
        assert_eq!(
            recall_from_path(&path, "tty:1", 60).as_deref(),
            Some("AAA111")
        );
        assert_eq!(recall_from_path(&path, "tty:2", 60), None);
    }

    #[test]
    fn expired_selection_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        let state = State {
            selections: BTreeMap::from([(
                "tty:1".to_string(),
                Selection {
                    serial: "AAA111".into(),
                    updated_at: now_secs().saturating_sub(120),
                },
            )]),
        };
        save_state_to_path(&state, &path).unwrap();

        assert_eq!(recall_from_path(&path, "tty:1", 60), None);
        assert_eq!(
            recall_from_path(&path, "tty:1", 0).as_deref(),
            Some("AAA111")
        );
    }

    #[test]
    fn forget_removes_only_current_scope() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        remember_at_path(&path, "tty:1", "AAA111", 60).unwrap();
        remember_at_path(&path, "tty:2", "BBB222", 60).unwrap();
        forget_at_path(&path, "tty:1").unwrap();

        assert_eq!(recall_from_path(&path, "tty:1", 60), None);
        assert_eq!(
            recall_from_path(&path, "tty:2", 60).as_deref(),
            Some("BBB222")
        );
    }

    #[test]
    fn forget_propagates_invalid_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(&path, "invalid json").unwrap();

        assert!(forget_at_path(&path, "tty:1").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "invalid json");
    }

    #[cfg(windows)]
    #[test]
    fn forget_propagates_read_failure() {
        use std::os::windows::fs::OpenOptionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        remember_at_path(&path, "tty:1", "AAA", 60).unwrap();

        let result = {
            let _file = OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&path)
                .unwrap();
            assert!(std::fs::read_to_string(&path).is_err());
            forget_at_path(&path, "tty:1")
        };

        assert!(result.is_err());
        assert_eq!(
            recall_from_path(&path, "tty:1", 60).as_deref(),
            Some("AAA")
        );
    }

    #[cfg(windows)]
    #[test]
    fn forget_propagates_save_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        remember_at_path(&path, "tty:1", "AAA", 60).unwrap();

        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions).unwrap();

        assert!(forget_at_path(&path, "tty:1").is_err());
        assert_eq!(
            recall_from_path(&path, "tty:1", 60).as_deref(),
            Some("AAA")
        );

        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        std::fs::set_permissions(&path, permissions).unwrap();
    }

    #[test]
    fn concurrent_remember_keeps_all_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        std::thread::scope(|scope| {
            for index in 0..8 {
                let path = &path;
                scope.spawn(move || {
                    for round in 0..20 {
                        remember_at_path(
                            path,
                            &format!("tty:{index}"),
                            &format!("serial-{round}"),
                            60,
                        )
                        .unwrap();
                    }
                });
            }
        });

        let state = load_state_from_path(&path).unwrap();
        assert_eq!(state.selections.len(), 8);
        for index in 0..8 {
            assert!(state.selections.contains_key(&format!("tty:{index}")));
        }
    }

    #[test]
    fn remember_prunes_expired_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        let state = State {
            selections: BTreeMap::from([(
                "old".to_string(),
                Selection {
                    serial: "OLD".into(),
                    updated_at: 0,
                },
            )]),
        };
        save_state_to_path(&state, &path).unwrap();

        remember_at_path(&path, "new", "NEW", 60).unwrap();

        let state = load_state_from_path(&path).unwrap();
        assert!(!state.selections.contains_key("old"));
        assert_eq!(
            state.selections.get("new").map(|s| s.serial.as_str()),
            Some("NEW")
        );
    }
}
