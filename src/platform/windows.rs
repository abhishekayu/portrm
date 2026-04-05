use std::process::Command;

use crate::errors::{PtrmError, Result};

use super::types::{PlatformAdapter, RawPortBinding};

pub struct WindowsAdapter;

impl PlatformAdapter for WindowsAdapter {
    fn list_bindings(&self) -> Result<Vec<RawPortBinding>> {
        let output = Command::new("netstat")
            .args(["-ano", "-p", "TCP"])
            .output()?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut bindings = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            if !line.contains("LISTENING") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }
            if let (Some(port), Ok(pid)) = (
                parts[1].rsplit(':').next().and_then(|p| p.parse::<u16>().ok()),
                parts[4].parse::<u32>(),
            ) {
                bindings.push(RawPortBinding {
                    port,
                    pid,
                    is_tcp: true,
                });
            }
        }

        Ok(bindings)
    }

    fn find_pid_on_port(&self, port: u16) -> Result<Option<u32>> {
        let output = Command::new("netstat")
            .args(["-ano", "-p", "TCP"])
            .output()?;

        let text = String::from_utf8_lossy(&output.stdout);
        let port_str = format!(":{port}");

        for line in text.lines() {
            let line = line.trim();
            if !line.contains("LISTENING") || !line.contains(&port_str) {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(last) = parts.last() {
                if let Ok(pid) = last.parse::<u32>() {
                    if let Some(local_port) =
                        parts[1].rsplit(':').next().and_then(|p| p.parse::<u16>().ok())
                    {
                        if local_port == port {
                            return Ok(Some(pid));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    fn graceful_kill(&self, pid: u32) -> Result<()> {
        let status = Command::new("taskkill")
            .args(["/T", "/PID", &pid.to_string()])
            .status()?;

        if !status.success() {
            return Err(PtrmError::PermissionDenied { pid });
        }
        Ok(())
    }

    fn force_kill(&self, pid: u32) -> Result<()> {
        let status = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .status()?;

        if !status.success() {
            return Err(PtrmError::PermissionDenied { pid });
        }
        Ok(())
    }

    fn is_running(&self, pid: u32) -> bool {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .is_ok_and(|o| {
                let text = String::from_utf8_lossy(&o.stdout);
                text.contains(&pid.to_string())
            })
    }
}
