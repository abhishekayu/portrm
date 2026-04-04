use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Per-repo configuration loaded from `.ptrm.toml`.
///
/// Declares port ownership, restart commands, env vars, and service stack.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PtrmConfig {
    /// Project-level settings.
    #[serde(default)]
    pub project: ProjectConfig,

    /// Per-service definitions that make up the dev stack.
    #[serde(default)]
    pub services: HashMap<String, ServiceConfig>,

    /// Named profiles with service overrides.
    #[serde(default)]
    pub profiles: Option<HashMap<String, ProfileConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    /// Project name (optional, inferred from directory if missing).
    pub name: Option<String>,
    /// Root directory override.
    pub root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Port this service owns.
    pub port: u16,
    /// Command to start the service.
    pub run: String,
    /// Working directory relative to project root (defaults to ".").
    pub cwd: Option<String>,
    /// Environment variables to set when starting this service.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Whether to run a pre-flight port check before starting.
    #[serde(default = "default_true")]
    pub preflight: bool,
    /// Readiness check: URL to poll or port to probe.
    pub ready: Option<String>,
}

/// A profile overrides service ports/env without redefining everything.
///
/// TOML format (flat, no `.services` nesting):
/// ```toml
/// [profiles.staging]
/// frontend = { port = 3100 }
/// api = { port = 8180 }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(flatten)]
    pub services: HashMap<String, ServiceOverride>,
}

/// Partial override for a service within a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceOverride {
    pub port: Option<u16>,
    pub run: Option<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// State file tracking the active profile.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PtrmState {
    pub active_profile: Option<String>,
}

fn default_true() -> bool {
    true
}

const CONFIG_FILENAMES: &[&str] = &[".ptrm.toml", "ptrm.toml"];
const STATE_FILENAME: &str = ".ptrm.state";

/// Locate the nearest `.ptrm.toml` by walking up from `start`.
pub fn find_config(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        for name in CONFIG_FILENAMES {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Load and parse the config from the nearest `.ptrm.toml`.
pub fn load_config(start: &Path) -> Option<PtrmConfig> {
    let path = find_config(start)?;
    let contents = std::fs::read_to_string(&path).ok()?;
    toml::from_str(&contents).ok()
}

/// Load config from CWD.
pub fn load_from_cwd() -> Option<PtrmConfig> {
    let cwd = std::env::current_dir().ok()?;
    load_config(&cwd)
}

/// Return the directory containing the nearest config file.
pub fn config_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    find_config(&cwd)?.parent().map(|p| p.to_path_buf())
}

/// Load the state file from the same directory as the config.
pub fn load_state() -> Option<PtrmState> {
    let cwd = std::env::current_dir().ok()?;
    let config_dir = find_config(&cwd)?.parent()?.to_path_buf();
    let state_path = config_dir.join(STATE_FILENAME);
    if !state_path.exists() {
        return None;
    }
    let contents = std::fs::read_to_string(&state_path).ok()?;
    toml::from_str(&contents).ok()
}

/// Save the state file next to the config.
pub fn save_state(state: &PtrmState) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config_path = find_config(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .ptrm.toml found"))?;
    let config_dir = config_path.parent().unwrap();
    let state_path = config_dir.join(STATE_FILENAME);
    let contents = toml::to_string_pretty(state)?;
    std::fs::write(state_path, contents)?;
    Ok(())
}

/// Apply the active profile overrides to a config, returning a new config.
pub fn apply_active_profile(config: &PtrmConfig) -> PtrmConfig {
    let state = load_state();
    let profile_name = state.as_ref().and_then(|s| s.active_profile.as_deref());

    match profile_name {
        Some(name) => apply_profile(config, name),
        None => config.clone(),
    }
}

/// Apply a named profile's overrides to a config.
pub fn apply_profile(config: &PtrmConfig, profile_name: &str) -> PtrmConfig {
    let mut result = config.clone();

    let profile = config
        .profiles
        .as_ref()
        .and_then(|p| p.get(profile_name));

    if let Some(profile) = profile {
        for (svc_name, overrides) in &profile.services {
            if let Some(svc) = result.services.get_mut(svc_name) {
                if let Some(port) = overrides.port {
                    svc.port = port;
                }
                if let Some(ref run) = overrides.run {
                    svc.run = run.clone();
                }
                if let Some(ref cwd) = overrides.cwd {
                    svc.cwd = Some(cwd.clone());
                }
                for (k, v) in &overrides.env {
                    svc.env.insert(k.clone(), v.clone());
                }
            }
        }
    }

    result
}
