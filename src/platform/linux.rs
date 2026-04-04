use std::fs;
use std::process::Command;

use crate::errors::{PtrmError, Result};

use super::types::{PlatformAdapter, RawPortBinding};

pub struct LinuxAdapter;

impl PlatformAdapter for LinuxAdapter {
    fn list_bindings(&self) -> Result<Vec<RawPortBinding>> {
        // Parse /proc/net/tcp directly -- no shell commands.
        let mut bindings = Vec::new();
        bindings.extend(parse_proc_net("/proc/net/tcp", true)?);
        bindings.extend(parse_proc_net("/proc/net/tcp6", true)?);
        Ok(bindings)
    }

    fn find_pid_on_port(&self, port: u16) -> Result<Option<u32>> {
        // Use ss for targeted lookup -- faster than scanning all of /proc.
        let output = Command::new("ss")
            .args(["-tlnp", &format!("sport = :{port}")])
            .output()?;

        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().skip(1) {
            if let Some(pid) = extract_pid_from_ss_line(line) {
                return Ok(Some(pid));
            }
        }
        Ok(None)
    }

    fn graceful_kill(&self, pid: u32) -> Result<()> {
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
        fs::metadata(format!("/proc/{pid}")).is_ok()
    }
}

/// Parse /proc/net/tcp or tcp6 to find listening ports.
fn parse_proc_net(path: &str, is_tcp: bool) -> Result<Vec<RawPortBinding>> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()),
    };

    let mut bindings = Vec::new();
    for line in content.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }

        // State 0A = LISTEN
        if fields[3] != "0A" {
            continue;
        }

        let local_addr = fields[1];
        if let Some(port) = parse_hex_port(local_addr) {
            let inode = fields[9];
            if let Some(pid) = find_pid_for_inode(inode) {
                bindings.push(RawPortBinding { port, pid, is_tcp });
            }
        }
    }

    Ok(bindings)
}

fn parse_hex_port(addr: &str) -> Option<u16> {
    let port_hex = addr.rsplit(':').next()?;
    u16::from_str_radix(port_hex, 16).ok()
}

fn find_pid_for_inode(inode: &str) -> Option<u32> {
    let target = format!("socket:[{inode}]");
    let entries = fs::read_dir("/proc").ok()?;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let fd_dir = format!("/proc/{name_str}/fd");
        if let Ok(fds) = fs::read_dir(&fd_dir) {
            for fd in fds.flatten() {
                if let Ok(link) = fs::read_link(fd.path())
                    && link.to_string_lossy() == target
                {
                    return name_str.parse().ok();
                }
            }
        }
    }
    None
}

fn extract_pid_from_ss_line(line: &str) -> Option<u32> {
    // ss output contains pid=XXXX
    let pid_marker = "pid=";
    let start = line.find(pid_marker)? + pid_marker.len();
    let rest = &line[start..];
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}


