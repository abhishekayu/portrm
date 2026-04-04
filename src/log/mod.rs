use std::process::Command;

use anyhow::{Context, Result};

use crate::docker;
use crate::models::PortInfo;

/// Who owns the port - used to decide the log strategy.
pub enum PortOwner {
    Docker { container_name: String },
    LocalProcess {
        pid: u32,
        name: String,
        command: Option<String>,
        working_dir: Option<String>,
        restart_hint: Option<String>,
    },
}

/// Resolve the port owner from scan results + Docker detection.
pub fn resolve_owner(info: &PortInfo, port: u16) -> Option<PortOwner> {
    // Check Docker first (higher fidelity for container-managed ports).
    if let Some(ref container) = info.docker_container {
        return Some(PortOwner::Docker {
            container_name: container.name.clone(),
        });
    }

    // Fall back to Docker detection even if scan didn't merge it.
    if let Some(container) = docker::find_container_on_port(port) {
        return Some(PortOwner::Docker {
            container_name: container.name,
        });
    }

    // Local process.
    if let Some(ref proc_) = info.process {
        let restart_hint = info.service.as_ref().and_then(|s| s.restart_hint.clone());
        return Some(PortOwner::LocalProcess {
            pid: proc_.pid,
            name: proc_.name.clone(),
            command: Some(proc_.command.clone()),
            working_dir: proc_.working_dir.clone(),
            restart_hint,
        });
    }

    None
}

/// Stream logs for the given port owner. This blocks until the user sends Ctrl-C.
pub fn stream_logs(owner: &PortOwner) -> Result<()> {
    match owner {
        PortOwner::Docker { container_name } => stream_docker_logs(container_name),
        PortOwner::LocalProcess { pid, name, command, working_dir, restart_hint } => {
            stream_local_logs(*pid, name, command.as_deref(), working_dir.as_deref(), restart_hint.as_deref())
        }
    }
}

/// Stream Docker container logs via `docker logs -f`.
fn stream_docker_logs(container_name: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(["logs", "-f", "--tail", "100", container_name])
        .status()
        .context("Failed to run `docker logs`. Is Docker installed?")?;

    if !status.success() {
        anyhow::bail!(
            "`docker logs {}` exited with code {}",
            container_name,
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

/// Try to stream local process logs.
///
/// Strategy (tried in order):
/// 1. Check if stdout/stderr point to a regular file -> tail -f
/// 2. Scan open file descriptors for .log files -> tail -f
/// 3. Linux: try journalctl or /proc/<pid>/fd/1
/// 4. macOS: try `log stream --process <pid>`
/// 5. Helpful fallback message with the TTY/socket info
fn stream_local_logs(pid: u32, name: &str, command: Option<&str>, working_dir: Option<&str>, restart_hint: Option<&str>) -> Result<()> {
    // Step 1 & 2: Use lsof to find log files or stdout destinations.
    if let Some(log_file) = find_log_file(pid) {
        eprintln!("  Tailing {log_file} ...\n");
        let status = Command::new("tail")
            .args(["-f", "-n", "100", &log_file])
            .status()
            .context("Failed to run `tail`")?;
        if status.success() {
            return Ok(());
        }
    }

    // Linux: try journalctl for the PID.
    #[cfg(target_os = "linux")]
    {
        if try_journalctl(pid) {
            return Ok(());
        }
    }

    // Linux: try /proc/<pid>/fd/1 (stdout).
    #[cfg(target_os = "linux")]
    {
        let stdout_path = format!("/proc/{pid}/fd/1");
        if std::path::Path::new(&stdout_path).exists() {
            eprintln!("  Tailing /proc/{pid}/fd/1 ...\n");
            let status = Command::new("tail")
                .args(["-f", &stdout_path])
                .status();
            if let Ok(s) = status
                && s.success()
            {
                return Ok(());
            }
        }
    }

    // macOS: try unified log stream for the process.
    #[cfg(target_os = "macos")]
    {
        if try_log_stream(pid, name) {
            return Ok(());
        }
    }

    // Fallback: detect where stdout goes and give a helpful message.
    let stdout_dest = detect_stdout_dest(pid);
    let restart_cmd = build_restart_hint(name, command, working_dir, restart_hint);
    let hint = match stdout_dest.as_deref() {
        Some(tty) if tty.starts_with("/dev/tty") => {
            format!(
                "Process {name} (PID {pid}) writes to terminal {tty}.\n\
                 \n  Its output is in the terminal where it was started.\n\
                 \n  Tip: Restart with output redirected to a file:\n\
                 \n    {restart_cmd} > /tmp/{name}.log 2>&1\n\
                 \n  Then run: ptrm log <port>"
            )
        }
        _ => {
            format!(
                "Could not locate log output for {name} (PID {pid}).\n\
                 \n  Tip: Restart with output redirected to a file:\n\
                 \n    {restart_cmd} > /tmp/{name}.log 2>&1\n\
                 \n  Then run: ptrm log <port>"
            )
        }
    };

    anyhow::bail!("{hint}");
}

/// Build a restart command hint from the process command and working dir.
fn build_restart_hint(name: &str, command: Option<&str>, working_dir: Option<&str>, restart_hint: Option<&str>) -> String {
    // Best: use the service's restart_hint (e.g. "cd /path && npm run dev")
    if let Some(hint) = restart_hint
        && !hint.is_empty()
    {
        return hint.to_string();
    }

    let cmd = match command {
        Some(c) if !c.is_empty() && c != name => {
            // Trim the full binary path to just the last component for readability,
            // but keep the arguments.
            let parts: Vec<&str> = c.splitn(2, char::is_whitespace).collect();
            let bin = parts[0]
                .rsplit('/')
                .next()
                .unwrap_or(parts[0]);
            if parts.len() > 1 {
                format!("{} {}", bin, parts[1])
            } else {
                bin.to_string()
            }
        }
        _ => name.to_string(),
    };

    match working_dir {
        Some(dir) if !dir.is_empty() => format!("cd {} && {}", dir, cmd),
        _ => cmd,
    }
}

/// Use lsof to find a log file the process has open.
/// Prefers: stdout/stderr pointing to a regular file, then *.log files.
fn find_log_file(pid: u32) -> Option<String> {
    let output = Command::new("lsof")
        .args(["-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);

    // First: check if fd 1 (stdout) or fd 2 (stderr) point to a regular file.
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 9 {
            let fd = cols[3];
            let ftype = cols[4];
            let path = cols[8];
            if (fd == "1w" || fd == "1u" || fd == "2w" || fd == "2u")
                && ftype == "REG"
                && !path.is_empty()
            {
                return Some(path.to_string());
            }
        }
    }

    // Second: look for any open .log file.
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 9 {
            let ftype = cols[4];
            let path = cols[8];
            if ftype == "REG" && path.ends_with(".log") {
                return Some(path.to_string());
            }
        }
    }

    None
}

/// Detect where a process's stdout is connected (tty, pipe, file).
fn detect_stdout_dest(pid: u32) -> Option<String> {
    let output = Command::new("lsof")
        .args(["-p", &pid.to_string(), "-a", "-d", "1"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 9 {
            return Some(cols[8].to_string());
        }
    }
    None
}

/// macOS: use `log stream --process <pid>` for unified logging.
#[cfg(target_os = "macos")]
fn try_log_stream(pid: u32, name: &str) -> bool {
    // Only use this for processes that actually emit os_log messages.
    // Skip for typical dev tools (node, python, ruby) since they
    // write to stdout, not the unified log.
    let skip = ["node", "python", "python3", "ruby", "java", "deno", "bun", "npm", "npx", "cargo"];
    let lower = name.to_lowercase();
    if skip.iter().any(|s| lower.contains(s)) {
        return false;
    }

    eprintln!("  Streaming macOS unified log for PID {pid} ({name})...\n");
    let status = Command::new("log")
        .args(["stream", "--process", &pid.to_string(), "--style", "compact"])
        .status();
    matches!(status, Ok(s) if s.success())
}

/// Try journalctl for a specific PID.
#[cfg(target_os = "linux")]
fn try_journalctl(pid: u32) -> bool {
    // Only attempt if journalctl exists.
    if !Command::new("which")
        .arg("journalctl")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return false;
    }

    let status = Command::new("journalctl")
        .args(["-f", &format!("_PID={pid}")])
        .status();

    matches!(status, Ok(s) if s.success())
}
