use serde::Serialize;
use std::time::Duration;

/// Information about an OS process occupying a port.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub user: Option<String>,
    pub working_dir: Option<String>,
    pub parent_pid: Option<u32>,
    pub cpu_usage: Option<f32>,
    pub memory_bytes: Option<u64>,
    #[serde(serialize_with = "serialize_duration_opt")]
    pub runtime: Option<Duration>,
}

impl std::fmt::Display for ProcessInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PID {} ({})", self.pid, self.name)
    }
}

impl ProcessInfo {
    pub fn runtime_display(&self) -> String {
        match self.runtime {
            Some(d) => {
                let secs = d.as_secs();
                if secs < 60 {
                    format!("{secs}s")
                } else if secs < 3600 {
                    format!("{}m {}s", secs / 60, secs % 60)
                } else if secs < 86400 {
                    format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
                } else {
                    format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
                }
            }
            None => "-".to_string(),
        }
    }
}

fn serialize_duration_opt<S: serde::Serializer>(
    val: &Option<Duration>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match val {
        Some(d) => s.serialize_f64(d.as_secs_f64()),
        None => s.serialize_none(),
    }
}
