use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind, Users};

use crate::models::ProcessInfo;

/// Cross-platform process resolver backed entirely by `sysinfo`.
/// No shell calls (`ps`, `wmic`) needed.
pub struct ProcessResolver {
    sys: Mutex<System>,
}

impl ProcessResolver {
    pub fn new() -> Self {
        Self {
            sys: Mutex::new(System::new()),
        }
    }

    /// Refresh and resolve a single PID into a full ProcessInfo.
    pub fn resolve(&self, pid: u32) -> Option<ProcessInfo> {
        let sysinfo_pid = Pid::from_u32(pid);
        let mut sys = self.sys.lock().unwrap();

        let refresh = ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_user(UpdateKind::Always)
            .with_exe(UpdateKind::Always);

        // First refresh initialises baselines; second yields accurate memory
        // and CPU readings (required on Windows where the first pass may
        // return zeroes for a fresh System instance).
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[sysinfo_pid]),
            true,
            refresh,
        );
        std::thread::sleep(Duration::from_millis(50));
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[sysinfo_pid]),
            true,
            refresh,
        );

        let proc_ = sys.process(sysinfo_pid)?;

        let command = {
            let cmd = proc_.cmd();
            if cmd.is_empty() {
                proc_.name().to_string_lossy().to_string()
            } else {
                cmd.iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        };

        let name = proc_.name().to_string_lossy().to_string();

        let working_dir = proc_
            .cwd()
            .map(|p| p.to_string_lossy().to_string());

        let parent_pid = proc_.parent().map(|p| p.as_u32());

        let user_name = proc_.user_id().and_then(|uid| {
            let users = Users::new_with_refreshed_list();
            users
                .iter()
                .find(|u| u.id() == uid)
                .map(|u| u.name().to_string())
        });

        let runtime = {
            let start = proc_.start_time(); // seconds since epoch
            if start > 0 {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                Some(Duration::from_secs(now.saturating_sub(start)))
            } else {
                None
            }
        };

        Some(ProcessInfo {
            pid,
            name,
            command,
            user: user_name,
            working_dir,
            parent_pid,
            cpu_usage: Some(proc_.cpu_usage()),
            memory_bytes: Some(proc_.memory()),
            runtime,
        })
    }

    /// Resolve multiple PIDs in a single refresh pass (batched, faster).
    pub fn resolve_batch(&self, pids: &[u32]) -> Vec<(u32, ProcessInfo)> {
        let sysinfo_pids: Vec<Pid> = pids.iter().map(|&p| Pid::from_u32(p)).collect();
        let mut sys = self.sys.lock().unwrap();

        let refresh = ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_user(UpdateKind::Always)
            .with_exe(UpdateKind::Always);

        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&sysinfo_pids),
            true,
            refresh,
        );
        std::thread::sleep(Duration::from_millis(50));
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&sysinfo_pids),
            true,
            refresh,
        );

        let users = Users::new_with_refreshed_list();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut results = Vec::with_capacity(pids.len());

        for &pid in pids {
            let sysinfo_pid = Pid::from_u32(pid);
            if let Some(proc_) = sys.process(sysinfo_pid) {
                let command = {
                    let cmd = proc_.cmd();
                    if cmd.is_empty() {
                        proc_.name().to_string_lossy().to_string()
                    } else {
                        cmd.iter()
                            .map(|s| s.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                };

                let user_name = proc_.user_id().and_then(|uid| {
                    users
                        .iter()
                        .find(|u| u.id() == uid)
                        .map(|u| u.name().to_string())
                });

                let runtime = {
                    let start = proc_.start_time();
                    if start > 0 {
                        Some(Duration::from_secs(now.saturating_sub(start)))
                    } else {
                        None
                    }
                };

                results.push((
                    pid,
                    ProcessInfo {
                        pid,
                        name: proc_.name().to_string_lossy().to_string(),
                        command,
                        user: user_name,
                        working_dir: proc_.cwd().map(|p| p.to_string_lossy().to_string()),
                        parent_pid: proc_.parent().map(|p| p.as_u32()),
                        cpu_usage: Some(proc_.cpu_usage()),
                        memory_bytes: Some(proc_.memory()),
                        runtime,
                    },
                ));
            }
        }

        results
    }
}
