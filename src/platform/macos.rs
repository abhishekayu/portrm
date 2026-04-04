use std::process::Command;

use crate::errors::{PtrmError, Result};

use super::types::{PlatformAdapter, RawPortBinding};

pub struct MacosAdapter;

impl PlatformAdapter for MacosAdapter {
    fn list_bindings(&self) -> Result<Vec<RawPortBinding>> {
        // lsof is the reliable way to enumerate LISTEN sockets on macOS.
        // Process info is handled cross-platform by sysinfo (no ps/shell).
        let output = Command::new("lsof")
            .args(["-iTCP", "-sTCP:LISTEN", "-nP", "-F", "pcn"])
            .output()?;

        let text = String::from_utf8_lossy(&output.stdout);
        Ok(parse_lsof_fields(&text))
    }

    fn find_pid_on_port(&self, port: u16) -> Result<Option<u32>> {
        let output = Command::new("lsof")
            .args([&format!("-iTCP:{port}"), "-sTCP:LISTEN", "-nP", "-F", "p"])
            .output()?;

        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if let Some(pid_str) = line.strip_prefix('p')
                && let Ok(pid) = pid_str.parse::<u32>()
            {
                return Ok(Some(pid));
            }
        }
        Ok(None)
    }

    fn graceful_kill(&self, pid: u32) -> Result<()> {
        // Use libc directly -- no shell subprocess.
        let ret = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if ret != 0 {
            return Err(PtrmError::PermissionDenied { pid });
        }
        Ok(())
    }

    fn force_kill(&self, pid: u32) -> Result<()> {
        let ret = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        if ret != 0 {
            return Err(PtrmError::PermissionDenied { pid });
        }
        Ok(())
    }

    fn is_running(&self, pid: u32) -> bool {
        // kill(pid, 0) checks existence without sending a signal.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

/// Parse lsof -F pcn output into port bindings.
fn parse_lsof_fields(text: &str) -> Vec<RawPortBinding> {
    let mut bindings = Vec::new();
    let mut current_pid: Option<u32> = None;

    for line in text.lines() {
        if let Some(pid_str) = line.strip_prefix('p') {
            current_pid = pid_str.parse().ok();
        } else if let Some(name_str) = line.strip_prefix('n')
            && let Some(pid) = current_pid
            && let Some(port) = extract_port_from_name(name_str)
        {
            bindings.push(RawPortBinding {
                port,
                pid,
                is_tcp: true,
            });
        }
    }

    bindings
}

/// Extract port from lsof name field like "*:3000" or "127.0.0.1:8080".
fn extract_port_from_name(name: &str) -> Option<u16> {
    name.rsplit(':').next()?.parse().ok()
}
