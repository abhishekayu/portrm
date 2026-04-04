use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::{self, ServiceConfig};
use crate::docker;
use crate::platform::PlatformAdapter;
use crate::scanner::PortScanner;

/// Result of a restart operation.
#[allow(dead_code)]
pub struct RestartResult {
    pub service_name: String,
    pub port: u16,
    pub stopped: StopOutcome,
    pub started: bool,
}

#[allow(dead_code)]
pub enum StopOutcome {
    KilledProcess(u32),
    RestartedDocker(String),
    NothingRunning,
    Failed(String),
}

/// Restart a single service by name from `.ptrm.toml`.
pub fn restart_service(
    adapter: &dyn PlatformAdapter,
    service_name: &str,
) -> Result<RestartResult> {
    let cfg = config::load_from_cwd()
        .ok_or_else(|| anyhow::anyhow!("No .ptrm.toml found. Run `ptrm init` to create one."))?;
    let cfg = config::apply_active_profile(&cfg);
    let cfg_dir = config::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let svc = cfg.services.get(service_name).ok_or_else(|| {
        let available: Vec<&String> = cfg.services.keys().collect();
        if available.is_empty() {
            anyhow::anyhow!("Service '{}' not found. No services defined in .ptrm.toml.", service_name)
        } else {
            let mut names: Vec<&str> = available.iter().map(|s| s.as_str()).collect();
            names.sort();
            anyhow::anyhow!(
                "Service '{}' not found. Available: {}",
                service_name,
                names.join(", ")
            )
        }
    })?;

    let port = svc.port;

    println!();

    // Step 1: Stop whatever is on the port.
    let stopped = stop_on_port(adapter, service_name, port);
    println!();

    // Step 2: Start the service.
    let started = start_service(service_name, svc, &cfg_dir, port);

    println!();

    Ok(RestartResult {
        service_name: service_name.to_string(),
        port,
        stopped,
        started,
    })
}

/// Stop whatever is running on the given port.
fn stop_on_port(adapter: &dyn PlatformAdapter, name: &str, port: u16) -> StopOutcome {
    println!(
        "  \u{23f9} Stopping {} (port {})",
        name.bold(),
        port.to_string().cyan()
    );

    // Check Docker first.
    if let Some(container) = docker::find_container_on_port(port) {
        match restart_docker_container(&container.name) {
            Ok(()) => {
                println!(
                    "  {} Restarted Docker container {}",
                    "\u{2714}".green(),
                    container.name.cyan()
                );
                return StopOutcome::RestartedDocker(container.name);
            }
            Err(e) => {
                println!(
                    "  {} Failed to restart Docker container: {}",
                    "\u{2718}".red(),
                    e.to_string().dimmed()
                );
                return StopOutcome::Failed(e.to_string());
            }
        }
    }

    // Check for a local process on the port.
    let scanner = PortScanner::new(adapter);
    let info = scanner.scan_port(port).ok().flatten();
    match info {
        Some(port_info) => {
            let pid = port_info.process.as_ref().map(|p| p.pid).unwrap_or(0);
            if pid == 0 {
                println!("  {} No PID found on port {}", "\u{2796}".dimmed(), port);
                return StopOutcome::NothingRunning;
            }

            // Graceful kill, then force.
            let killed = if adapter.graceful_kill(pid).is_ok() {
                // Wait briefly for process to exit.
                std::thread::sleep(Duration::from_millis(500));
                true
            } else {
                adapter.force_kill(pid).is_ok()
            };

            if killed {
                println!(
                    "  {} Killed process (PID {})",
                    "\u{2714}".green(),
                    pid.to_string().dimmed()
                );
                // Wait for port to free up.
                std::thread::sleep(Duration::from_millis(300));
                StopOutcome::KilledProcess(pid)
            } else {
                let msg = format!("Failed to kill PID {pid}");
                println!("  {} {}", "\u{2718}".red(), msg);
                StopOutcome::Failed(msg)
            }
        }
        None => {
            println!(
                "  {} Nothing running on port {}",
                "\u{2796}".dimmed(),
                port.to_string().dimmed()
            );
            StopOutcome::NothingRunning
        }
    }
}

/// Restart a Docker container by name.
fn restart_docker_container(container_name: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(["restart", container_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("Failed to run `docker restart`. Is Docker installed?")?;

    if !status.success() {
        anyhow::bail!("docker restart {} failed", container_name);
    }
    Ok(())
}

/// Start a service process in the background.
fn start_service(name: &str, svc: &ServiceConfig, config_dir: &Path, port: u16) -> bool {
    println!(
        "  \u{25b6} Starting {}",
        name.bold()
    );

    let mut cmd = Command::new("sh");
    cmd.args(["-c", &svc.run]);

    // Resolve cwd relative to config directory.
    if let Some(ref cwd) = svc.cwd {
        let resolved = config_dir.join(cwd);
        cmd.current_dir(&resolved);
    } else {
        cmd.current_dir(config_dir);
    }

    // Set environment variables.
    for (key, val) in &svc.env {
        cmd.env(key, val);
    }
    cmd.env("PORT", port.to_string());

    // Detach stdout/stderr so service runs independently.
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    match cmd.spawn() {
        Ok(_child) => {
            println!(
                "  {} Running: {}",
                "\u{2714}".green(),
                svc.run.dimmed()
            );
            println!();
            println!(
                "  \u{1f7e2} {} running on port {}",
                name.bold(),
                port.to_string().cyan().bold()
            );
            true
        }
        Err(e) => {
            println!(
                "  {} Failed to start: {}",
                "\u{2718}".red(),
                e.to_string().dimmed()
            );
            false
        }
    }
}
