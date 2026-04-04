use serde::Serialize;

/// Reason a process died or is in a bad state.
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub enum CrashReason {
    /// Killed by a signal (e.g. SIGKILL, SIGTERM, SIGSEGV).
    Signal { signal: i32, name: String },
    /// Exited with a non-zero exit code.
    ExitCode { code: i32, meaning: String },
    /// Out-of-memory kill (OOM killer on Linux, jetsam on macOS).
    OutOfMemory,
    /// Process is not running and port is stuck (zombie/orphan).
    ZombiePort,
    /// Could not determine reason.
    Unknown,
}

impl std::fmt::Display for CrashReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Signal { signal, name } => write!(f, "Killed by signal {signal} ({name})"),
            Self::ExitCode { code, meaning } => write!(f, "Exited with code {code} ({meaning})"),
            Self::OutOfMemory => write!(f, "Killed by OOM (out of memory)"),
            Self::ZombiePort => write!(f, "Port held by zombie/orphan process"),
            Self::Unknown => write!(f, "Unknown crash reason"),
        }
    }
}

/// Try to determine why a process died or is misbehaving.
///
/// Checks: process state, OOM markers, exit status interpretation.
pub fn detect_crash_reason(pid: u32) -> CrashReason {
    // Check if process is a zombie first.
    if is_zombie(pid) {
        return CrashReason::ZombiePort;
    }

    // Check for OOM kill.
    if was_oom_killed(pid) {
        return CrashReason::OutOfMemory;
    }

    CrashReason::Unknown
}

/// Interpret a wait status / exit code into a CrashReason.
#[allow(dead_code)]
pub fn interpret_exit(status: i32, from_signal: bool) -> CrashReason {
    if from_signal {
        let name = signal_name(status);
        return CrashReason::Signal {
            signal: status,
            name,
        };
    }

    if status == 0 {
        return CrashReason::Unknown; // Clean exit, not a crash.
    }

    // Common exit codes.
    let meaning = match status {
        1 => "General error".to_string(),
        2 => "Misuse of shell command".to_string(),
        126 => "Command not executable".to_string(),
        127 => "Command not found".to_string(),
        128 => "Invalid exit argument".to_string(),
        code if (129..=192).contains(&code) => {
            let sig = code - 128;
            format!("Killed by signal {} ({})", sig, signal_name(sig))
        }
        137 => "Killed by SIGKILL (likely OOM or forced)".to_string(),
        139 => "Segmentation fault (SIGSEGV)".to_string(),
        143 => "Terminated by SIGTERM".to_string(),
        255 => "Exit status out of range".to_string(),
        _ => format!("Non-zero exit code {status}"),
    };

    CrashReason::ExitCode {
        code: status,
        meaning,
    }
}

fn signal_name(sig: i32) -> String {
    match sig {
        1 => "SIGHUP".to_string(),
        2 => "SIGINT".to_string(),
        3 => "SIGQUIT".to_string(),
        4 => "SIGILL".to_string(),
        6 => "SIGABRT".to_string(),
        8 => "SIGFPE".to_string(),
        9 => "SIGKILL".to_string(),
        11 => "SIGSEGV".to_string(),
        13 => "SIGPIPE".to_string(),
        14 => "SIGALRM".to_string(),
        15 => "SIGTERM".to_string(),
        _ => format!("SIG{sig}"),
    }
}

/// Check if a PID is a zombie process.
#[cfg(unix)]
fn is_zombie(pid: u32) -> bool {
    // On Linux, check /proc/<pid>/status.
    #[cfg(target_os = "linux")]
    {
        let status_path = format!("/proc/{pid}/status");
        if let Ok(contents) = std::fs::read_to_string(&status_path) {
            for line in contents.lines() {
                if line.starts_with("State:") && line.contains("Z") {
                    return true;
                }
            }
        }
    }

    // On macOS, use kill(pid, 0) to check existence, then ps to check state.
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("ps")
            .args(["-o", "state=", "-p", &pid.to_string()])
            .output();
        if let Ok(out) = output {
            let state = String::from_utf8_lossy(&out.stdout);
            if state.trim().starts_with('Z') {
                return true;
            }
        }
    }

    false
}

#[cfg(not(unix))]
fn is_zombie(_pid: u32) -> bool {
    false
}

/// Check if a process was killed by the OOM killer.
#[cfg(target_os = "linux")]
fn was_oom_killed(pid: u32) -> bool {
    // Check dmesg or /proc/<pid>/oom_score.
    // The most reliable check is /proc/<pid>/oom_score_adj existing but process gone.
    let oom_path = format!("/proc/{pid}/oom_score");
    if !std::path::Path::new(&oom_path).exists() {
        // Process is dead; check kernel log for OOM mention.
        if let Ok(output) = std::process::Command::new("dmesg")
            .args(["--time-format", "reltime"])
            .output()
        {
            let log = String::from_utf8_lossy(&output.stdout);
            let pid_str = pid.to_string();
            return log.lines().rev().take(100).any(|line| {
                line.contains("oom-kill") && line.contains(&pid_str)
            });
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn was_oom_killed(_pid: u32) -> bool {
    // macOS: jetsam kills are harder to detect programmatically.
    // For now, return false. Could check log stream in the future.
    false
}
