use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single recorded action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub action: ActionKind,
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub service: Option<String>,
    pub strategy: Option<String>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionKind {
    Kill,
    Fix,
    Doctor,
}

impl std::fmt::Display for ActionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kill => write!(f, "kill"),
            Self::Fix => write!(f, "fix"),
            Self::Doctor => write!(f, "doctor"),
        }
    }
}

/// Get the history file path: ~/.ptrm/history.json
fn history_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".ptrm").join("history.json"))
}

/// Load all history entries.
pub fn load() -> Vec<HistoryEntry> {
    let Some(path) = history_path() else {
        return Vec::new();
    };
    let Ok(data) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

/// Append a new entry and persist.
pub fn record(entry: HistoryEntry) {
    let Some(path) = history_path() else {
        return;
    };

    // Ensure directory exists.
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut entries = load();
    entries.push(entry);

    // Keep last 500 entries.
    if entries.len() > 500 {
        entries.drain(..entries.len() - 500);
    }

    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let _ = fs::write(&path, json);
    }
}

/// Clear all history.
pub fn clear() {
    if let Some(path) = history_path() {
        let _ = fs::remove_file(path);
    }
}

/// Summary stats from history.
#[derive(Debug, Serialize)]
pub struct HistoryStats {
    pub total_actions: usize,
    pub kills: usize,
    pub fixes: usize,
    pub success_rate: f64,
    pub most_killed_port: Option<u16>,
    pub most_killed_process: Option<String>,
}

pub fn stats() -> HistoryStats {
    let entries = load();
    let total = entries.len();
    let kills = entries.iter().filter(|e| matches!(e.action, ActionKind::Kill)).count();
    let fixes = entries.iter().filter(|e| matches!(e.action, ActionKind::Fix)).count();
    let successes = entries.iter().filter(|e| e.success).count();

    let success_rate = if total > 0 {
        successes as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    // Most killed port.
    let mut port_counts: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
    for e in &entries {
        *port_counts.entry(e.port).or_default() += 1;
    }
    let most_killed_port = port_counts.into_iter().max_by_key(|(_, c)| *c).map(|(p, _)| p);

    // Most killed process.
    let mut proc_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in &entries {
        *proc_counts.entry(e.process_name.clone()).or_default() += 1;
    }
    let most_killed_process = proc_counts.into_iter().max_by_key(|(_, c)| *c).map(|(p, _)| p);

    HistoryStats {
        total_actions: total,
        kills,
        fixes,
        success_rate,
        most_killed_port,
        most_killed_process,
    }
}
