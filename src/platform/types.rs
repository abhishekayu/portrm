use crate::errors::Result;

/// Raw port-to-PID binding from the OS.
#[derive(Debug, Clone)]
pub struct RawPortBinding {
    pub port: u16,
    pub pid: u32,
    pub is_tcp: bool,
}

/// Abstraction over OS-specific network and process-control queries.
///
/// Process *information* is handled cross-platform by `ProcessResolver` (sysinfo).
/// This trait only covers operations that genuinely differ per OS:
/// port enumeration, signal delivery, and liveness checks.
pub trait PlatformAdapter: Send + Sync {
    /// List all ports currently in LISTEN state with their owning PIDs.
    fn list_bindings(&self) -> Result<Vec<RawPortBinding>>;

    /// Find which PID owns a specific port (LISTEN state).
    fn find_pid_on_port(&self, port: u16) -> Result<Option<u32>>;

    /// Send SIGTERM (or equivalent) to a process.
    fn graceful_kill(&self, pid: u32) -> Result<()>;

    /// Send SIGKILL (or equivalent) to a process.
    fn force_kill(&self, pid: u32) -> Result<()>;

    /// Check whether a PID is still running.
    fn is_running(&self, pid: u32) -> bool;
}
