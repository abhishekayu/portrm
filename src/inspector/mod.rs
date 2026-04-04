use crate::models::ProcessInfo;
use crate::platform::PlatformAdapter;
use crate::resolver::ProcessResolver;

/// Higher-level process inspection.
#[allow(dead_code)]
pub struct ProcessInspector<'a> {
    adapter: &'a dyn PlatformAdapter,
    resolver: ProcessResolver,
}

#[allow(dead_code)]
impl<'a> ProcessInspector<'a> {
    pub fn new(adapter: &'a dyn PlatformAdapter) -> Self {
        Self {
            adapter,
            resolver: ProcessResolver::new(),
        }
    }

    /// Get full process info via sysinfo (cross-platform, no shell).
    pub fn inspect(&self, pid: u32) -> Option<ProcessInfo> {
        self.resolver.resolve(pid)
    }

    /// Check whether a process is still alive.
    pub fn is_alive(&self, pid: u32) -> bool {
        self.adapter.is_running(pid)
    }
}
