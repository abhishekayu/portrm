use std::thread;
use std::time::{Duration, Instant};

use crate::classifier::ServiceClassifier;
use crate::errors::{PtrmError, Result};
use crate::models::{PortInfo, ProcessInfo, Protocol, ServiceKind};
use crate::platform::PlatformAdapter;
use crate::resolver::ProcessResolver;

// ── Public types ──────────────────────────────────────────────────────

/// Resolution strategy the engine will attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// SIGTERM, wait, verify.
    Graceful,
    /// SIGTERM, wait, SIGKILL if still alive.
    Escalating,
    /// SIGKILL immediately.
    Force,
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Graceful => write!(f, "Graceful (SIGTERM + wait)"),
            Self::Escalating => write!(f, "Escalating (SIGTERM -> wait -> SIGKILL)"),
            Self::Force => write!(f, "Force (SIGKILL)"),
        }
    }
}

/// Safety verdict from analyzing a process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyVerdict {
    /// Safe to kill -- dev server or similar.
    Safe { reason: String },
    /// Can kill but user should be warned (databases, infrastructure).
    Warn { reason: String },
    /// Must NOT kill -- system-critical process.
    Block { reason: String },
}

impl SafetyVerdict {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Block { .. })
    }

    pub fn is_safe(&self) -> bool {
        matches!(self, Self::Safe { .. })
    }

    pub fn reason(&self) -> &str {
        match self {
            Self::Safe { reason } | Self::Warn { reason } | Self::Block { reason } => reason,
        }
    }
}

/// A logged step in the fix process.
#[derive(Debug, Clone)]
pub struct FixStep {
    pub label: String,
    pub ok: bool,
}

impl FixStep {
    fn ok(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ok: true,
        }
    }
    fn fail(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ok: false,
        }
    }
}

/// Full analysis of a port conflict before acting.
pub struct FixPlan {
    pub port: u16,
    pub port_info: PortInfo,
    pub verdict: SafetyVerdict,
    pub strategy: Strategy,
}

/// Outcome of a fix attempt.
#[derive(Debug)]
pub struct FixResult {
    pub port: u16,
    pub pid: u32,
    pub strategy_used: Strategy,
    pub success: bool,
    pub steps: Vec<FixStep>,
    pub restart_hint: Option<String>,
}

// ── System-critical process names (never kill) ────────────────────────

const SYSTEM_CRITICAL: &[&str] = &[
    // macOS kernel & core services
    "launchd",
    "kernel_task",
    "windowserver",
    "loginwindow",
    "opendirectoryd",
    "coreaudiod",
    "coreservicesd",
    "configd",
    "diskarbitrationd",
    "notifyd",
    "securityd",
    "fseventsd",
    "mds",
    "mds_stores",
    // Linux init & kernel threads
    "systemd",
    "init",
    "kthreadd",
    "ksoftirqd",
    "kworker",
    "systemd-journald",
    "systemd-udevd",
    "systemd-logind",
    "dbus-daemon",
    // Windows system processes
    "system",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "lsass.exe",
    "services.exe",
    "svchost.exe",
    "dwm.exe",
    "explorer.exe",
    "spoolsv.exe",
    "lsm.exe",
    "wlanext.exe",
    // Remote access -- killing locks you out
    "sshd",
    "sshd.exe",
    // Scheduling daemons
    "cron",
    "crond",
    "atd",
];

// ── Fix engine ────────────────────────────────────────────────────────

/// Intelligent port conflict resolution engine.
///
/// Flow: analyze (scan + classify + safety check) -> confirm -> execute (kill + verify).
pub struct FixEngine<'a> {
    adapter: &'a dyn PlatformAdapter,
    resolver: ProcessResolver,
    graceful_timeout: Duration,
    graceful_retries: u32,
}

impl<'a> FixEngine<'a> {
    pub fn new(adapter: &'a dyn PlatformAdapter) -> Self {
        Self {
            adapter,
            resolver: ProcessResolver::new(),
            graceful_timeout: Duration::from_secs(5),
            graceful_retries: 2,
        }
    }

    // ── Phase 1: Analyze ──────────────────────────────────────────

    /// Analyze a port and produce a fix plan.
    ///
    /// Scans the port, resolves the owning process, classifies the service,
    /// runs safety checks, and selects the best termination strategy.
    pub fn analyze(&self, port: u16) -> Result<FixPlan> {
        let pid = self
            .adapter
            .find_pid_on_port(port)?
            .ok_or(PtrmError::NoProcessOnPort(port))?;

        let process = self.resolver.resolve(pid);

        let mut port_info = PortInfo {
            port,
            protocol: Protocol::Tcp,
            process,
            service: None,
            docker_container: None,
        };

        if let Some(ref proc_) = port_info.process {
            port_info.service = Some(ServiceClassifier::classify_with_port(proc_, port));
        }

        let verdict = self.safety_checks(
            port_info.process.as_ref(),
            port_info.service.as_ref().map(|s| &s.kind),
        );

        let strategy = self.choose_strategy(&port_info, &verdict);

        Ok(FixPlan {
            port,
            port_info,
            verdict,
            strategy,
        })
    }

    /// Override the strategy in a plan (e.g. for --force).
    pub fn override_strategy(plan: &mut FixPlan, strategy: Strategy) {
        plan.strategy = strategy;
    }

    // ── Phase 2: Safety checks ────────────────────────────────────

    /// Determine whether a process is safe to kill, should warn, or must be blocked.
    pub fn safety_checks(
        &self,
        process: Option<&ProcessInfo>,
        service_kind: Option<&ServiceKind>,
    ) -> SafetyVerdict {
        let Some(proc_) = process else {
            return SafetyVerdict::Warn {
                reason: "Could not identify the owning process".into(),
            };
        };

        // Rule 1: Never kill PID 0 or 1.
        if proc_.pid <= 1 {
            return SafetyVerdict::Block {
                reason: format!(
                    "PID {} ({}) is {} -- killing it would crash the system",
                    proc_.pid,
                    proc_.name,
                    if proc_.pid == 0 {
                        "the kernel"
                    } else {
                        "the init process"
                    }
                ),
            };
        }

        // Rule 2: System-critical process names.
        let name_lower = proc_.name.to_lowercase();
        let name_bare = name_lower.strip_suffix(".exe").unwrap_or(&name_lower);
        for &critical in SYSTEM_CRITICAL {
            if name_lower == critical || name_bare == critical {
                return SafetyVerdict::Block {
                    reason: format!(
                        "{} is a system-critical process -- killing it could destabilize the system",
                        proc_.name
                    ),
                };
            }
        }

        // Rule 3: Database services -- warn about data loss.
        if matches!(
            service_kind,
            Some(ServiceKind::PostgreSQL | ServiceKind::MySQL | ServiceKind::Redis
                | ServiceKind::SQLServer | ServiceKind::MongoDB)
        ) {
            return SafetyVerdict::Warn {
                reason: format!(
                    "{} is a database server -- killing it risks data loss or corruption",
                    service_kind.unwrap().label()
                ),
            };
        }

        // Rule 4: Infrastructure services -- warn about cascading effects.
        if matches!(service_kind, Some(ServiceKind::Docker | ServiceKind::Nginx
            | ServiceKind::Apache | ServiceKind::Iis)) {
            return SafetyVerdict::Warn {
                reason: format!(
                    "{} manages other services -- killing it may have cascading effects",
                    service_kind.unwrap().label()
                ),
            };
        }

        // Rule 5: Root-owned, unclassified processes -- warn.
        let is_root = proc_
            .user
            .as_deref()
            .is_some_and(|u| u == "root" || u == "SYSTEM");
        if is_root && matches!(service_kind, Some(ServiceKind::Unknown) | None) {
            return SafetyVerdict::Warn {
                reason: format!(
                    "{} is owned by root and not recognized -- could be a system service",
                    proc_.name
                ),
            };
        }

        // Rule 6: Known dev servers -- safe.
        if matches!(service_kind, Some(kind) if kind.safe_to_kill()) {
            return SafetyVerdict::Safe {
                reason: format!(
                    "{} is a dev server -- safe to kill and restart",
                    service_kind.unwrap().label()
                ),
            };
        }

        // Default: unclassified user-owned process.
        SafetyVerdict::Warn {
            reason: format!("Could not classify {} -- proceed with caution", proc_.name),
        }
    }

    // ── Strategy selection ────────────────────────────────────────

    /// Choose termination strategy based on service kind and safety verdict.
    fn choose_strategy(&self, port_info: &PortInfo, verdict: &SafetyVerdict) -> Strategy {
        if verdict.is_blocked() {
            return Strategy::Graceful; // Won't execute anyway.
        }

        match port_info.service.as_ref().map(|s| s.kind) {
            // Databases: always graceful only -- never escalate to SIGKILL.
            Some(ServiceKind::PostgreSQL | ServiceKind::MySQL | ServiceKind::Redis
                | ServiceKind::SQLServer | ServiceKind::MongoDB) => {
                Strategy::Graceful
            }
            // Infrastructure: graceful only.
            Some(ServiceKind::Docker | ServiceKind::Nginx | ServiceKind::Apache | ServiceKind::Iis) => Strategy::Graceful,
            // Dev servers: safe to escalate.
            Some(kind) if kind.safe_to_kill() => Strategy::Escalating,
            // Unknown: be cautious.
            _ => Strategy::Graceful,
        }
    }

    // ── Phase 3: Execute ──────────────────────────────────────────

    /// Execute a fix plan, calling `on_step` for each action taken.
    ///
    /// Returns an error if the plan's verdict is `Block`.
    pub fn execute(
        &self,
        plan: &FixPlan,
        on_step: impl Fn(&FixStep),
    ) -> Result<FixResult> {
        if plan.verdict.is_blocked() {
            return Err(PtrmError::Other(anyhow::anyhow!(
                "{}",
                plan.verdict.reason()
            )));
        }

        let process = plan
            .port_info
            .process
            .as_ref()
            .ok_or(PtrmError::NoProcessOnPort(plan.port))?;

        let pid = process.pid;
        let restart_hint = plan
            .port_info
            .service
            .as_ref()
            .and_then(|s| s.restart_hint.clone());

        let (success, steps) = self.safe_kill(pid, plan.strategy, &on_step);

        Ok(FixResult {
            port: plan.port,
            pid,
            strategy_used: plan.strategy,
            success,
            steps,
            restart_hint,
        })
    }

    // ── Kill implementation ───────────────────────────────────────

    /// Kill a process using the selected strategy, with retry logic and logging.
    fn safe_kill(
        &self,
        pid: u32,
        strategy: Strategy,
        on_step: &impl Fn(&FixStep),
    ) -> (bool, Vec<FixStep>) {
        let mut steps = Vec::new();

        match strategy {
            Strategy::Graceful => {
                self.do_graceful(pid, &mut steps, on_step);
            }
            Strategy::Escalating => {
                self.do_graceful(pid, &mut steps, on_step);

                if self.adapter.is_running(pid) {
                    let step =
                        FixStep::ok("Process survived graceful shutdown -- escalating to SIGKILL");
                    on_step(&step);
                    steps.push(step);

                    self.do_force(pid, &mut steps, on_step);
                }
            }
            Strategy::Force => {
                self.do_force(pid, &mut steps, on_step);
            }
        }

        // Final verification.
        let dead = !self.adapter.is_running(pid);
        let step = if dead {
            FixStep::ok(format!("Verified: PID {pid} has exited"))
        } else {
            FixStep::fail(format!("PID {pid} is still running after all attempts"))
        };
        on_step(&step);
        steps.push(step);

        (dead, steps)
    }

    /// Send SIGTERM and wait, with retries.
    fn do_graceful(
        &self,
        pid: u32,
        steps: &mut Vec<FixStep>,
        on_step: &impl Fn(&FixStep),
    ) {
        for attempt in 1..=self.graceful_retries {
            let label = if self.graceful_retries == 1 || attempt == 1 {
                format!("Sending SIGTERM to PID {pid}")
            } else {
                format!(
                    "Retry {}/{}: sending SIGTERM to PID {pid}",
                    attempt, self.graceful_retries
                )
            };

            match self.adapter.graceful_kill(pid) {
                Ok(()) => {
                    let step = FixStep::ok(&label);
                    on_step(&step);
                    steps.push(step);
                }
                Err(e) => {
                    let step = FixStep::fail(format!("{label} -- failed: {e}"));
                    on_step(&step);
                    steps.push(step);
                    return;
                }
            }

            let wait_step = FixStep::ok(format!(
                "Waiting up to {}s for graceful shutdown",
                self.graceful_timeout.as_secs()
            ));
            on_step(&wait_step);
            steps.push(wait_step);

            if self.wait_for_exit(pid) {
                return; // Exited successfully.
            }

            if attempt < self.graceful_retries {
                let step = FixStep::fail(format!(
                    "Process did not exit within {}s",
                    self.graceful_timeout.as_secs()
                ));
                on_step(&step);
                steps.push(step);
            }
        }
    }

    /// Send SIGKILL.
    fn do_force(
        &self,
        pid: u32,
        steps: &mut Vec<FixStep>,
        on_step: &impl Fn(&FixStep),
    ) {
        let label = format!("Sending SIGKILL to PID {pid}");
        match self.adapter.force_kill(pid) {
            Ok(()) => {
                let step = FixStep::ok(&label);
                on_step(&step);
                steps.push(step);
            }
            Err(e) => {
                let step = FixStep::fail(format!("{label} -- failed: {e}"));
                on_step(&step);
                steps.push(step);
                return;
            }
        }

        thread::sleep(Duration::from_millis(500));
    }

    /// Poll until PID exits or timeout.
    fn wait_for_exit(&self, pid: u32) -> bool {
        let deadline = Instant::now() + self.graceful_timeout;
        while Instant::now() < deadline {
            if !self.adapter.is_running(pid) {
                return true;
            }
            thread::sleep(Duration::from_millis(100));
        }
        false
    }
}
