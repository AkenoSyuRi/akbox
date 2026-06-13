use crate::config::Config;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const REAL_ADB_ENV: &str = "REAL_ADB";

#[cfg(windows)]
const ADB_REAL_NAME: &str = "adb-real.exe";

#[cfg(not(windows))]
const ADB_REAL_NAME: &str = "adb-real";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealAdbSource {
    Env,
    Config,
    Sibling,
}

#[derive(Debug, Clone)]
pub struct RealAdbCandidate {
    pub source: RealAdbSource,
    pub path: PathBuf,
    pub exists: bool,
}

#[derive(Debug, Clone)]
pub struct RealAdbResolution {
    pub resolved: Option<PathBuf>,
    pub resolved_source: Option<RealAdbSource>,
    pub candidates: Vec<RealAdbCandidate>,
    pub env_value: Option<String>,
    pub recursion_risk: bool,
}

pub fn resolve_real_adb(config: &Config) -> Result<PathBuf> {
    let details = resolve_real_adb_details(config)?;
    if details.recursion_risk {
        anyhow::bail!(
            "adb-wrapper: real adb resolves to this wrapper.\n\
             Point REAL_ADB / config real_adb / adb-real at the official SDK adb."
        );
    }
    details.resolved.ok_or_else(|| {
        anyhow::anyhow!(
            "adb-wrapper: real adb not found.\n\
             Set REAL_ADB, configure real_adb in config, or place adb-real next to this wrapper."
        )
    })
}

pub fn resolve_real_adb_details(config: &Config) -> Result<RealAdbResolution> {
    let wrapper_exe =
        std::env::current_exe().context("failed to resolve current executable path")?;
    let wrapper_dir = wrapper_exe
        .parent()
        .context("wrapper executable has no parent directory")?;

    let env_value = std::env::var(REAL_ADB_ENV).ok();
    let mut candidates = Vec::new();

    if let Some(path) = env_value.as_ref() {
        let path = PathBuf::from(path);
        let exists = path.is_file();
        candidates.push(RealAdbCandidate {
            source: RealAdbSource::Env,
            path,
            exists,
        });
    }

    if let Some(path) = config.real_adb.as_ref() {
        let path = PathBuf::from(path);
        let exists = path.is_file();
        candidates.push(RealAdbCandidate {
            source: RealAdbSource::Config,
            path,
            exists,
        });
    }

    let sibling = wrapper_dir.join(ADB_REAL_NAME);
    let sibling_exists = sibling.is_file();
    candidates.push(RealAdbCandidate {
        source: RealAdbSource::Sibling,
        path: sibling,
        exists: sibling_exists,
    });

    let (resolved, resolved_source) = match candidates.iter().find(|c| c.exists) {
        Some(candidate) => (Some(candidate.path.clone()), Some(candidate.source.clone())),
        None => (None, None),
    };

    let recursion_risk = resolved
        .as_ref()
        .is_some_and(|real| paths_canonical_equal(&wrapper_exe, real));

    Ok(RealAdbResolution {
        resolved,
        resolved_source,
        candidates,
        env_value,
        recursion_risk,
    })
}

fn paths_canonical_equal(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

pub fn real_adb_source_label(source: &RealAdbSource) -> &'static str {
    match source {
        RealAdbSource::Env => "REAL_ADB",
        RealAdbSource::Config => "config real_adb",
        RealAdbSource::Sibling => "adb-real (sibling)",
    }
}

pub fn real_adb_display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_prefers_env_over_config() {
        let dir = std::env::temp_dir().join("adb_wrapper_path_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let env_adb = dir.join("from_env.exe");
        let config_adb = dir.join("from_config.exe");
        std::fs::write(&env_adb, b"").unwrap();
        std::fs::write(&config_adb, b"").unwrap();

        unsafe { std::env::set_var(REAL_ADB_ENV, &env_adb); }
        let config = Config {
            real_adb: Some(config_adb.to_string_lossy().into_owned()),
            ..Config::default()
        };
        let details = resolve_real_adb_details(&config).unwrap();
        assert_eq!(details.resolved, Some(env_adb));
        assert_eq!(details.resolved_source, Some(RealAdbSource::Env));

        unsafe { std::env::remove_var(REAL_ADB_ENV); }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
