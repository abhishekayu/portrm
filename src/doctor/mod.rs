use crate::classifier::ServiceClassifier;
use crate::engine::FixEngine;
use crate::platform::PlatformAdapter;
use crate::scanner::PortScanner;
use crate::models::PortInfo;

/// A single finding from the doctor.
#[derive(Debug)]
pub struct Diagnosis {
    pub port: u16,
    pub issue: Issue,
    pub suggestion: String,
    pub auto_fixable: bool,
}

#[derive(Debug)]
pub enum Issue {
    /// A stale dev server holding a port.
    StaleDevServer { pid: u32, name: String, uptime: String },
    /// Multiple processes on common dev ports.
    CrowdedDevPorts { count: usize },
    /// A zombie or idle process holding a port.
    IdleProcess { pid: u32, name: String, cpu: f32 },
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Issue::StaleDevServer { pid, name, uptime } => {
                write!(f, "Stale dev server: {} (PID {}) running for {}", name, pid, uptime)
            }
            Issue::CrowdedDevPorts { count } => {
                write!(f, "{} processes on common dev ports", count)
            }
            Issue::IdleProcess { pid, name, cpu } => {
                write!(f, "Idle process {} (PID {}) at {:.1}% CPU", name, pid, cpu)
            }
        }
    }
}

/// Run diagnostics on all listening ports.
pub fn diagnose(adapter: &dyn PlatformAdapter) -> Vec<Diagnosis> {
    let scanner = PortScanner::new(adapter);
    let mut results = Vec::new();

    let Ok(mut ports) = scanner.scan_all() else {
        return results;
    };

    // Classify all services.
    for info in &mut ports {
        if let Some(ref proc_) = info.process {
            info.service = Some(ServiceClassifier::classify_with_port(proc_, info.port));
        }
    }

    // Filter to dev ports.
    let dev_ports: Vec<&PortInfo> = ports
        .iter()
        .filter(|p| is_dev_port(p.port))
        .collect();

    // Check 1: Stale dev servers (running > 24 hours).
    for info in &dev_ports {
        if let Some(ref proc_) = info.process
            && let Some(ref svc) = info.service
            && svc.kind.safe_to_kill()
            && let Some(runtime) = proc_.runtime
            && runtime.as_secs() > 86400
        {
            results.push(Diagnosis {
                port: info.port,
                issue: Issue::StaleDevServer {
                    pid: proc_.pid,
                    name: proc_.name.clone(),
                    uptime: proc_.runtime_display(),
                },
                suggestion: format!(
                    "Kill stale {} on port {} (running {})",
                    svc.kind.label(),
                    info.port,
                    proc_.runtime_display()
                ),
                auto_fixable: true,
            });
        }
    }

    // Check 2: Idle processes (< 0.1% CPU, dev port, running > 1 hour).
    for info in &dev_ports {
        if let Some(ref proc_) = info.process {
            let cpu = proc_.cpu_usage.unwrap_or(0.0);
            let long_running = proc_
                .runtime
                .is_some_and(|r| r.as_secs() > 3600);

            if cpu < 0.1 && long_running {
                // Avoid duplicating stale server findings.
                let already_found = results.iter().any(|d| d.port == info.port);
                if !already_found {
                    results.push(Diagnosis {
                        port: info.port,
                        issue: Issue::IdleProcess {
                            pid: proc_.pid,
                            name: proc_.name.clone(),
                            cpu,
                        },
                        suggestion: format!(
                            "Idle {} on port {} -- consider killing to free resources",
                            proc_.name,
                            info.port
                        ),
                        auto_fixable: true,
                    });
                }
            }
        }
    }

    // Check 3: Too many dev ports occupied.
    if dev_ports.len() > 5 {
        results.push(Diagnosis {
            port: 0,
            issue: Issue::CrowdedDevPorts {
                count: dev_ports.len(),
            },
            suggestion: format!(
                "{} dev ports in use -- run `ptrm scan --dev` to review",
                dev_ports.len()
            ),
            auto_fixable: false,
        });
    }

    results
}

/// Auto-fix all auto-fixable diagnoses.
pub fn auto_fix(
    adapter: &dyn PlatformAdapter,
    diagnoses: &[Diagnosis],
    on_step: &mut dyn FnMut(&str, bool),
) -> usize {
    let engine = FixEngine::new(adapter);
    let mut fixed = 0;

    for diag in diagnoses {
        if !diag.auto_fixable || diag.port == 0 {
            continue;
        }

        on_step(&format!("Fixing port {}...", diag.port), true);

        match engine.analyze(diag.port) {
            Ok(plan) => {
                if plan.verdict.is_blocked() {
                    on_step(
                        &format!("  Skipped port {} (blocked: {})", diag.port, plan.verdict.reason()),
                        false,
                    );
                    continue;
                }

                match engine.execute(&plan, |_| {}) {
                    Ok(result) if result.success => {
                        on_step(&format!("  Freed port {}", diag.port), true);
                        fixed += 1;
                    }
                    Ok(_) => {
                        on_step(&format!("  Failed to free port {}", diag.port), false);
                    }
                    Err(e) => {
                        on_step(&format!("  Error on port {}: {}", diag.port, e), false);
                    }
                }
            }
            Err(e) => {
                on_step(&format!("  Cannot analyze port {}: {}", diag.port, e), false);
            }
        }
    }

    fixed
}

fn is_dev_port(port: u16) -> bool {
    matches!(port, 3000..=3999 | 4000..=4999 | 5000..=5999 | 8000..=8999)
}
