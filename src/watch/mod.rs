use std::thread;
use std::time::{Duration, Instant};

use colored::Colorize;

use crate::classifier::ServiceClassifier;
use crate::crash;
use crate::platform::PlatformAdapter;
use crate::scanner::PortScanner;

/// Outcome of a single watch tick.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum WatchEvent {
    /// Port is up and serving.
    Up { pid: u32, service: String },
    /// Port went down (was up, now nothing listening).
    Down { last_pid: u32, reason: String },
    /// Port came back up after being down.
    Recovered { pid: u32, service: String },
    /// Port is free (nothing ever seen).
    Free,
}

/// Continuously monitor a port and react when it goes down.
///
/// Returns only on Ctrl-C or fatal error.
pub fn watch_port(
    adapter: &dyn PlatformAdapter,
    port: u16,
    interval: Duration,
    auto_restart: Option<&str>,
) -> anyhow::Result<()> {
    let scanner = PortScanner::new(adapter);

    let mut last_pid: Option<u32> = None;
    let mut was_up = false;
    let mut down_since: Option<Instant> = None;
    let mut printed_free = false;

    println!();
    println!(
        "  \u{1f440} {} port {} (every {}s, Ctrl-C to stop)",
        "Watching".bold(),
        port.to_string().cyan().bold(),
        interval.as_secs()
    );
    println!();

    // Install Ctrl-C handler.
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .ok();

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        let info = scanner.scan_port(port)?;

        match info {
            Some(mut port_info) => {
                // Port is up.
                if let Some(ref proc_) = port_info.process {
                    port_info.service = Some(ServiceClassifier::classify_with_port(proc_, port));
                }

                let pid = port_info.process.as_ref().map(|p| p.pid).unwrap_or(0);
                let service = port_info
                    .service
                    .as_ref()
                    .map(|s| s.kind.label().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                if !was_up {
                    if down_since.is_some() {
                        // Was down, now recovered.
                        let downtime = down_since.unwrap().elapsed();
                        println!(
                            "  {} Port {} {} (PID {}, {}) -- downtime: {}s",
                            "\u{2714}".green(),
                            port.to_string().cyan(),
                            "recovered".green().bold(),
                            pid,
                            service,
                            downtime.as_secs()
                        );
                        down_since = None;
                    } else {
                        // First time seeing it up.
                        println!(
                            "  {} Port {} is {} -- {} (PID {})",
                            "\u{2714}".green(),
                            port.to_string().cyan(),
                            "up".green().bold(),
                            service,
                            pid
                        );
                    }
                } else if last_pid != Some(pid) {
                    // PID changed (restarted by something else).
                    println!(
                        "  {} Port {} -- PID changed {} -> {} ({})",
                        "\u{26a0}".yellow(),
                        port.to_string().cyan(),
                        last_pid.unwrap_or(0),
                        pid,
                        service
                    );
                }

                last_pid = Some(pid);
                was_up = true;
            }
            None => {
                // Port is down.
                if was_up {
                    let reason = if let Some(pid) = last_pid {
                        let crash_reason = crash::detect_crash_reason(pid);
                        format!("{crash_reason}")
                    } else {
                        "Unknown".to_string()
                    };

                    println!(
                        "  {} Port {} {} -- {} (was PID {})",
                        "\u{2718}".red(),
                        port.to_string().cyan(),
                        "went down".red().bold(),
                        reason,
                        last_pid.unwrap_or(0)
                    );

                    down_since = Some(Instant::now());

                    // Auto-restart if configured.
                    if let Some(cmd) = auto_restart {
                        println!(
                            "  \u{1f680} Auto-restarting: {}",
                            cmd.bold()
                        );
                        let _ = std::process::Command::new("sh")
                            .args(["-c", cmd])
                            .spawn();
                        // Give the process time to bind the port.
                        thread::sleep(Duration::from_secs(2));
                    }
                } else if !printed_free {
                    println!(
                        "  \u{2796} Port {} is {} -- waiting for a process to bind...",
                        port.to_string().cyan(),
                        "free".dimmed()
                    );
                    printed_free = true;
                }

                was_up = false;
            }
        }

        thread::sleep(interval);
    }

    println!();
    println!("  \u{1f6d1} Watch stopped.");
    println!();

    Ok(())
}
