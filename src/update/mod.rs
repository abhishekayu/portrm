use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use colored::Colorize;

use crate::conflict;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_API: &str =
    "https://api.github.com/repos/abhishekayu/portrm/releases/latest";
const CHECK_INTERVAL_SECS: u64 = 86_400; // 24 hours

/// Cached state file path: `~/.portrm/last_update_check`
fn state_file() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".portrm").join("last_update_check"))
}

/// Return the epoch timestamp stored in the state file, or 0.
fn last_check_time() -> u64 {
    state_file()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

/// Write the current epoch timestamp to the state file.
fn save_check_time() {
    if let Some(p) = state_file() {
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = fs::write(p, now.to_string());
    }
}

/// Compare two semver strings. Returns true if `latest` > `current`.
fn is_newer(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v
            .trim_start_matches('v')
            .splitn(3, '.')
            .filter_map(|s| s.parse().ok())
            .collect();
        (
            *parts.first().unwrap_or(&0),
            *parts.get(1).unwrap_or(&0),
            *parts.get(2).unwrap_or(&0),
        )
    };
    parse(latest) > parse(current)
}

/// Fetch the latest release tag from GitHub.
fn fetch_latest_version() -> Option<String> {
    let mut resp = ureq::get(GITHUB_API)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "ptrm-update-check")
        .call()
        .ok()?;

    let body: serde_json::Value = resp.body_mut().read_json().ok()?;
    body["tag_name"]
        .as_str()
        .map(|t| t.trim_start_matches('v').to_string())
}

/// Detect how the currently-running binary was installed.
fn detect_my_source() -> &'static str {
    let exe = env::current_exe().unwrap_or_default();
    conflict::detect_source(&exe)
}

/// Return the appropriate update command for a given source.
fn update_command(source: &str) -> Option<&'static str> {
    match source {
        "brew" => Some("brew upgrade portrm"),
        "cargo" => Some("cargo install portrm"),
        "pip" => Some("pip install --upgrade portrm"),
        "pipx" => Some("pipx upgrade portrm"),
        "npm" | "npm-local" => Some("npm update -g portrm"),
        "script" => Some(
            "curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh | sh",
        ),
        // orphan: stale file, nothing to update
        _ => None,
    }
}

/// Run the update command in a child process.
fn run_update(cmd_str: &str) -> bool {
    eprintln!(
        "  {} {}",
        "$".dimmed(),
        cmd_str.cyan()
    );
    eprintln!();

    let status = if cmd_str.contains('|') {
        // Pipe commands need a shell
        Command::new("sh")
            .arg("-c")
            .arg(cmd_str)
            .status()
    } else {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            return false;
        }
        Command::new(parts[0])
            .args(&parts[1..])
            .status()
    };

    match status {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

/// Check for updates and auto-update if a newer version is available.
///
/// Skipped when `PTRM_SKIP_UPDATE_CHECK=1` is set.
/// Only checks the network once per 24 hours (cached in `~/.portrm/last_update_check`).
pub fn check_and_update() {
    if env::var("PTRM_SKIP_UPDATE_CHECK").unwrap_or_default() == "1" {
        return;
    }

    // Throttle: only check once per day
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if now.saturating_sub(last_check_time()) < CHECK_INTERVAL_SECS {
        return;
    }

    save_check_time();

    let latest = match fetch_latest_version() {
        Some(v) => v,
        None => return, // Network error — silently skip
    };

    if !is_newer(CURRENT_VERSION, &latest) {
        return;
    }

    let source = detect_my_source();
    let cmd = match update_command(source) {
        Some(c) => c,
        None => {
            eprintln!(
                "\n  {} {} {} (you have {})",
                "⬆".cyan().bold(),
                "New version available:".bold(),
                latest.cyan(),
                CURRENT_VERSION.dimmed()
            );
            eprintln!(
                "  {} Update manually — could not detect install method.\n",
                "→".dimmed()
            );
            return;
        }
    };

    eprintln!();
    eprintln!(
        "  {} {} {} (you have {})",
        "⬆".cyan().bold(),
        "Updating portrm to".bold(),
        latest.cyan(),
        CURRENT_VERSION.dimmed()
    );
    eprintln!();

    if run_update(cmd) {
        eprintln!(
            "  {} {}\n",
            "✔".green().bold(),
            "Updated successfully! Restart ptrm to use the new version.".green()
        );
    } else {
        eprintln!(
            "  {} {} {}\n",
            "✖".red().bold(),
            "Auto-update failed. Update manually:".red(),
            cmd
        );
    }
}
