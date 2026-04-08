use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use colored::Colorize;

/// A discovered binary with its source ecosystem.
#[derive(Debug)]
pub(crate) struct Installation {
    pub(crate) path: PathBuf,
    pub(crate) source: &'static str,
}

/// Check whether a file starts with a `#!` shebang referencing Python.
fn is_python_script(path: &Path) -> bool {
    std::fs::read(path)
        .ok()
        .map(|bytes| {
            let head: String = bytes.iter().take(256).map(|&b| b as char).collect();
            head.starts_with("#!") && head.to_lowercase().contains("python")
        })
        .unwrap_or(false)
}

/// Detect which package manager installed a binary based on its path.
pub(crate) fn detect_source(path: &Path) -> &'static str {
    let s = path.to_string_lossy().replace('\\', "/").to_lowercase();

    // Order matters: more specific patterns first
    if s.contains("homebrew") || s.contains("/opt/homebrew") || s.contains("cellar") || s.contains("linuxbrew") {
        return "brew";
    }
    if s.contains(".cargo/bin") {
        return "cargo";
    }
    if s.contains("node_modules") || s.contains("/npm/") || s.contains("/npx/") || s.contains("_npx")
        || s.contains("appdata/roaming/npm")
    {
        return "npm";
    }
    if s.contains("site-packages") || s.contains("python") {
        return "pip";
    }
    // ~/.local/bin could be pip, pipx, or install.sh — inspect the file
    if s.contains(".local") {
        if is_python_script(path) {
            // Check if it's specifically a pipx venv
            let head = std::fs::read_to_string(path).unwrap_or_default();
            if head.contains("pipx") {
                return "pipx";
            }
            return "pip";
        }
        // Compiled binary in ~/.local/bin → from install.sh
        return "script";
    }
    "unknown"
}

/// Find all `ptrm` and `portrm` binaries on PATH.
pub(crate) fn find_all_binaries() -> Vec<Installation> {
    let path_var = env::var("PATH").unwrap_or_default();
    let names: &[&str] = if cfg!(windows) {
        &["ptrm.exe", "portrm.exe"]
    } else {
        &["ptrm", "portrm"]
    };

    let mut seen = std::collections::HashSet::new();
    let mut found = Vec::new();

    for dir in env::split_paths(&path_var) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                // Resolve symlinks for dedup
                let real = candidate.canonicalize().unwrap_or_else(|_| candidate.clone());
                if seen.insert(real) {
                    let source = detect_source(&candidate);
                    found.push(Installation {
                        path: candidate,
                        source,
                    });
                }
            }
        }
    }
    found
}

/// Shorten a path by replacing the home directory with ~.
fn shorten(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(rest) = path.strip_prefix(&home)
    {
        return format!("~/{}", rest.display());
    }
    path.display().to_string()
}

/// Check for conflicting installations. Returns `true` if a conflict was found
/// and an error message was printed.
///
/// Skipped when `PTRM_SKIP_CONFLICT_CHECK=1` is set (for CI / testing).
pub fn check() -> bool {
    if env::var("PTRM_SKIP_CONFLICT_CHECK").unwrap_or_default() == "1" {
        return false;
    }

    let bins = find_all_binaries();
    if bins.len() <= 1 {
        return false;
    }

    // Collect unique sources
    let mut sources: Vec<&str> = bins.iter().map(|b| b.source).collect();
    sources.dedup();
    let unique: Vec<&str> = {
        let mut v = Vec::new();
        for s in &sources {
            if !v.contains(s) {
                v.push(s);
            }
        }
        v
    };

    // All from same source = no real conflict
    if unique.len() <= 1 {
        return false;
    }

    // Print conflict report
    eprintln!();
    eprintln!("  {} {}", "\u{2718}".red().bold(), "Multiple portrm installations detected".red().bold());
    eprintln!();

    // Active binary
    if let Some(first) = bins.first() {
        eprintln!("  {} {}", "Active binary:".dimmed(), shorten(&first.path).cyan());
        eprintln!();
    }

    eprintln!("  {}", "Found:".bold());
    eprintln!();
    for bin in &bins {
        eprintln!(
            "    {} {}  {}",
            "\u{2022}".yellow(),
            shorten(&bin.path),
            format!("({})", bin.source).dimmed()
        );
    }
    eprintln!();

    // Uninstall commands
    let uninstall: BTreeMap<&str, &str> = [
        ("brew", "brew uninstall portrm"),
        ("pip", "pip uninstall portrm"),
        ("pipx", "pipx uninstall portrm"),
        ("cargo", "cargo uninstall portrm"),
        ("npm", "npm uninstall -g portrm"),
    ]
    .into_iter()
    .collect();

    // For "script" (install.sh) installs, suggest rm with the actual path
    let script_cmds: Vec<String> = bins
        .iter()
        .filter(|b| b.source == "script")
        .map(|b| format!("rm {}", shorten(&b.path)))
        .collect();

    let cmds: Vec<&&str> = unique
        .iter()
        .filter_map(|s| uninstall.get(s))
        .collect();

    if !cmds.is_empty() || !script_cmds.is_empty() {
        eprintln!("  {}", "Uninstall duplicates:".bold());
        eprintln!();
        for cmd in &cmds {
            eprintln!("    {} {}", "$".dimmed(), cmd);
        }
        for cmd in &script_cmds {
            eprintln!("    {} {}", "$".dimmed(), cmd);
        }
        eprintln!();
    }

    // Install suggestion
    let recommended = if cfg!(target_os = "macos") {
        "brew install abhishekayu/tap/portrm"
    } else if cfg!(target_os = "linux") {
        "curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh | sh"
    } else if cfg!(target_os = "windows") {
        "npm install -g portrm"
    } else {
        "cargo install portrm"
    };

    eprintln!("  {}", "Install using ONE method:".bold());
    eprintln!();
    eprintln!("    {}  {}", "Recommended:".bold(), recommended.cyan());
    eprintln!();

    true
}
