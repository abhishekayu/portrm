use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::{Child, Command};
use std::time::Duration;

use colored::Colorize;

use crate::config::{PtrmConfig, ServiceConfig};
use crate::platform::PlatformAdapter;
use crate::preflight;
use crate::scanner::PortScanner;

const PID_FILE: &str = ".ptrm.pids";

/// Saved PID state from last `up`.
type PidMap = HashMap<String, PidEntry>;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct PidEntry {
    pid: u32,
    port: u16,         // declared port
    actual_port: u16,  // port the process actually bound to
}

/// Result of a `ptrm up` run.
pub struct StackUpResult {
    pub started: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub skipped: Vec<(String, String)>,
}

/// Start all services declared in `.ptrm.toml`.
pub fn up(
    adapter: &dyn PlatformAdapter,
    config: &PtrmConfig,
    yes: bool,
    config_dir: &Path,
) -> anyhow::Result<StackUpResult> {
    let scanner = PortScanner::new(adapter);
    let mut result = StackUpResult {
        started: Vec::new(),
        failed: Vec::new(),
        skipped: Vec::new(),
    };
    let mut pids: PidMap = HashMap::new();

    if config.services.is_empty() {
        anyhow::bail!("No services defined in .ptrm.toml");
    }

    println!();
    println!(
        "  \u{1f680} Starting {} service{}...",
        config.services.len(),
        if config.services.len() == 1 { "" } else { "s" }
    );
    println!();

    // Sort services by name for deterministic ordering.
    let mut services: Vec<(&String, &ServiceConfig)> = config.services.iter().collect();
    services.sort_by_key(|(name, _)| name.to_owned());

    for (name, svc) in &services {
        // Pre-flight: check if port is already in use.
        if svc.preflight {
            let conflict = preflight::check_port(&scanner, svc.port)?;
            if let Some(ref info) = conflict {
                let proc_name = info
                    .process
                    .as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                if yes {
                    // Auto-fix: kill the conflicting process.
                    println!(
                        "  {} {} port {} busy ({}) -- fixing...",
                        "\u{26a0}".yellow(),
                        name.bold(),
                        svc.port.to_string().cyan(),
                        proc_name
                    );
                    let engine = crate::engine::FixEngine::new(adapter);
                    let plan = engine.analyze(svc.port)?;
                    if !plan.verdict.is_blocked() {
                        let _ = engine.execute(&plan, |_| {});
                        // Wait for port to be freed.
                        std::thread::sleep(Duration::from_millis(500));
                    } else {
                        result.skipped.push((
                            name.to_string(),
                            format!("port {} blocked by system process", svc.port),
                        ));
                        continue;
                    }
                } else {
                    result.skipped.push((
                        name.to_string(),
                        format!("port {} in use by {}", svc.port, proc_name),
                    ));
                    println!(
                        "  {} {} -- port {} already in use by {}",
                        "\u{2718}".red(),
                        name.bold(),
                        svc.port.to_string().cyan(),
                        proc_name.dimmed()
                    );
                    continue;
                }
            }
        }

        // Snapshot current ports so we can detect new ones after spawn.
        let ports_before: HashSet<u16> = PortScanner::new(adapter)
            .scan_all()
            .unwrap_or_default()
            .iter()
            .map(|p| p.port)
            .collect();

        // Start the service.
        match start_service(name, svc, config_dir) {
            Ok(mut child) => {
                let child_pid = child.id();
                // Wait and verify port comes up.
                print!(
                    "  {} {} starting on port {}...",
                    "\u{25b6}".dimmed(),
                    name.bold(),
                    svc.port.to_string().cyan()
                );
                use std::io::Write;
                let _ = std::io::stdout().flush();

                let up = wait_for_port_or_exit(adapter, svc.port, &mut child, 30);
                print!("\r\x1b[2K");
                let _ = std::io::stdout().flush();

                match up {
                    PortWaitResult::Up => {
                        result.started.push(name.to_string());
                        pids.insert(name.to_string(), PidEntry {
                            pid: child_pid,
                            port: svc.port,
                            actual_port: svc.port,
                        });
                        println!(
                            "  {} {} started on port {}",
                            "\u{2714}".green(),
                            name.bold(),
                            svc.port.to_string().cyan()
                        );
                    }
                    PortWaitResult::ProcessExited => {
                        result.failed.push((name.to_string(), format!("process exited before port {} was ready", svc.port)));
                        println!(
                            "  {} {} process crashed (port {} never came up)",
                            "\u{2718}".red(),
                            name.bold(),
                            svc.port.to_string().cyan()
                        );
                        println!(
                            "      {} try running '{}' manually to see the error",
                            "\u{2192}".dimmed(),
                            svc.run.dimmed()
                        );
                    }
                    PortWaitResult::Timeout => {
                        // Port didn't come up on declared port. Check for port
                        // auto-increment (e.g. Next.js picks next free port).
                        // Try PID match first, then diff ports before/after.
                        let actual = find_port_for_pid(adapter, child_pid)
                            .or_else(|| find_new_port(adapter, &ports_before));
                        if let Some(actual_port) = actual {
                            // Resolve the real PID on this port (may differ from shell PID).
                            let real_pid = adapter
                                .find_pid_on_port(actual_port)
                                .ok()
                                .flatten()
                                .unwrap_or(child_pid);
                            result.started.push(name.to_string());
                            pids.insert(name.to_string(), PidEntry {
                                pid: real_pid,
                                port: svc.port,
                                actual_port,
                            });
                            println!(
                                "  {} {} started on port {} (configured: {})",
                                "\u{2714}".green(),
                                name.bold(),
                                actual_port.to_string().cyan().bold(),
                                svc.port.to_string().dimmed()
                            );
                            println!(
                                "      {} port {} was busy, update .ptrm.toml to match",
                                "\u{26a0}".yellow(),
                                svc.port
                            );
                        } else {
                            // Process alive, no port found yet - still compiling.
                            result.started.push(name.to_string());
                            pids.insert(name.to_string(), PidEntry {
                                pid: child_pid,
                                port: svc.port,
                                actual_port: svc.port,
                            });
                            println!(
                                "  {} {} spawned (port {} still starting, process alive)",
                                "\u{2714}".green(),
                                name.bold(),
                                svc.port.to_string().cyan()
                            );
                        }
                    }
                }
            }
            Err(e) => {
                result.failed.push((name.to_string(), e.to_string()));
                println!(
                    "  {} {} failed: {}",
                    "\u{2718}".red(),
                    name.bold(),
                    e.to_string().dimmed()
                );
            }
        }
    }

    // Save PID file for `down` to use.
    if !pids.is_empty() {
        let pid_path = config_dir.join(PID_FILE);
        if let Ok(json) = serde_json::to_string_pretty(&pids) {
            let _ = std::fs::write(&pid_path, json);
        }
    }

    println!();
    print_stack_summary(&result);

    Ok(result)
}

/// Stop all services declared in `.ptrm.toml`.
pub fn down(
    adapter: &dyn PlatformAdapter,
    config: &PtrmConfig,
    config_dir: &Path,
) -> anyhow::Result<Vec<(String, bool)>> {
    let scanner = PortScanner::new(adapter);
    let mut results = Vec::new();

    if config.services.is_empty() {
        anyhow::bail!("No services defined in .ptrm.toml");
    }

    // Load saved PIDs from last `up`.
    let pid_path = config_dir.join(PID_FILE);
    let saved_pids: PidMap = std::fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    println!();
    println!(
        "  \u{1f6d1} Stopping {} service{}...",
        config.services.len(),
        if config.services.len() == 1 { "" } else { "s" }
    );
    println!();

    let mut services: Vec<(&String, &ServiceConfig)> = config.services.iter().collect();
    services.sort_by_key(|(name, _)| name.to_owned());

    for (name, svc) in &services {
        // Try 1: check the declared port.
        let info = scanner.scan_port(svc.port)?;
        if let Some(port_info) = info {
            let pid = port_info.process.as_ref().map(|p| p.pid).unwrap_or(0);
            let stopped = kill_pid(adapter, pid);
            let label = if stopped { "\u{2714}".green() } else { "\u{2718}".red() };
            println!(
                "  {} {} stopped (port {})",
                label,
                name.bold(),
                svc.port.to_string().cyan()
            );
            results.push((name.to_string(), stopped));
            continue;
        }

        // Try 2: check the actual port from PID file (e.g., Next.js on 3050 instead of 3000).
        if let Some(entry) = saved_pids.get(name.as_str()) {
            if entry.actual_port != svc.port {
                let info2 = scanner.scan_port(entry.actual_port)?;
                if let Some(port_info) = info2 {
                    let pid = port_info.process.as_ref().map(|p| p.pid).unwrap_or(0);
                    let stopped = kill_pid(adapter, pid);
                    let label = if stopped { "\u{2714}".green() } else { "\u{2718}".red() };
                    println!(
                        "  {} {} stopped (was on port {}, configured: {})",
                        label,
                        name.bold(),
                        entry.actual_port.to_string().cyan(),
                        svc.port.to_string().dimmed()
                    );
                    results.push((name.to_string(), stopped));
                    continue;
                }
            }

            // Try 3: kill by saved PID directly.
            let stopped = kill_pid(adapter, entry.pid);
            if stopped {
                println!(
                    "  {} {} stopped (PID {})",
                    "\u{2714}".green(),
                    name.bold(),
                    entry.pid.to_string().cyan()
                );
                results.push((name.to_string(), true));
                continue;
            }
        }

        // Nothing found.
        println!(
            "  \u{2796} {} already stopped (port {} free)",
            name.dimmed(),
            svc.port.to_string().dimmed()
        );
        results.push((name.to_string(), true));
    }

    // Clean up PID file.
    let _ = std::fs::remove_file(&pid_path);

    println!();
    let stopped = results.iter().filter(|(_, ok)| *ok).count();
    println!(
        "  {} {}",
        "\u{2714}".green(),
        format!("{stopped}/{} services stopped.", results.len())
            .green()
            .bold()
    );
    println!();

    Ok(results)
}

fn kill_pid(adapter: &dyn PlatformAdapter, pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    if adapter.graceful_kill(pid).is_ok() {
        return true;
    }
    adapter.force_kill(pid).is_ok()
}

/// Spawn a service process in the background.
fn start_service(name: &str, svc: &ServiceConfig, config_dir: &Path) -> anyhow::Result<Child> {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", &svc.run]);

    // Resolve cwd relative to config file directory (not current dir).
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

    // Set PORT env for frameworks that respect it.
    cmd.env("PORT", svc.port.to_string());

    // Redirect stdout/stderr to /dev/null so the service doesn't block.
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    let child = cmd.spawn().map_err(|e| {
        anyhow::anyhow!("Failed to start {name}: {e}")
    })?;

    Ok(child)
}

enum PortWaitResult {
    Up,
    ProcessExited,
    Timeout,
}

/// Wait for a port to become active, checking process liveness.
fn wait_for_port_or_exit(
    adapter: &dyn PlatformAdapter,
    port: u16,
    child: &mut Child,
    max_secs: u64,
) -> PortWaitResult {
    for _ in 0..max_secs * 2 {
        std::thread::sleep(Duration::from_millis(500));
        // Check if port is up.
        if let Ok(Some(_)) = adapter.find_pid_on_port(port) {
            return PortWaitResult::Up;
        }
        // Check if process died.
        if let Ok(Some(_status)) = child.try_wait() {
            return PortWaitResult::ProcessExited;
        }
    }
    PortWaitResult::Timeout
}

/// Scan all listening ports and find which port a given PID (or its children) bound to.
fn find_port_for_pid(adapter: &dyn PlatformAdapter, pid: u32) -> Option<u16> {
    let scanner = PortScanner::new(adapter);
    let all = scanner.scan_all().ok()?;
    // Direct PID match.
    for info in &all {
        if let Some(ref proc) = info.process
            && proc.pid == pid
        {
            return Some(info.port);
        }
    }
    // Check if any port's process is a child of our PID (sh -> node).
    for info in &all {
        if let Some(ref proc) = info.process
            && is_child_of(proc.pid, pid)
        {
            return Some(info.port);
        }
    }
    None
}

/// Find a port that appeared after we spawned (not in the before-set).
fn find_new_port(adapter: &dyn PlatformAdapter, ports_before: &HashSet<u16>) -> Option<u16> {
    let scanner = PortScanner::new(adapter);
    let all = scanner.scan_all().ok()?;
    let new_ports: Vec<u16> = all
        .iter()
        .map(|p| p.port)
        .filter(|p| !ports_before.contains(p))
        .collect();
    if new_ports.len() == 1 {
        return Some(new_ports[0]);
    }
    None
}

/// Check if `child` has `parent` as its parent PID.
fn is_child_of(child: u32, parent: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;
        if let Ok(out) = Command::new("ps")
            .args(["-o", "ppid=", "-p", &child.to_string()])
            .output()
            && let Ok(s) = String::from_utf8(out.stdout)
            && let Ok(ppid) = s.trim().parse::<u32>()
        {
            return ppid == parent;
        }
    }
    false
}

fn print_stack_summary(result: &StackUpResult) {
    let total = result.started.len() + result.failed.len() + result.skipped.len();
    if result.failed.is_empty() && result.skipped.is_empty() {
        println!(
            "  {} {}",
            "\u{2714}".green(),
            format!("{total}/{total} services started.").green().bold()
        );
    } else {
        println!(
            "  {} started, {} failed, {} skipped",
            result.started.len().to_string().green(),
            result.failed.len().to_string().red(),
            result.skipped.len().to_string().yellow()
        );
    }
    println!();
}
