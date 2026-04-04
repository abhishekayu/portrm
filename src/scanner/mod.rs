use crate::errors::Result;
use crate::models::Protocol;
use crate::models::PortInfo;
use crate::platform::PlatformAdapter;
use crate::resolver::ProcessResolver;

/// Scan ports and build enriched PortInfo entries.
pub struct PortScanner<'a> {
    adapter: &'a dyn PlatformAdapter,
    resolver: ProcessResolver,
}

impl<'a> PortScanner<'a> {
    pub fn new(adapter: &'a dyn PlatformAdapter) -> Self {
        Self {
            adapter,
            resolver: ProcessResolver::new(),
        }
    }

    /// Scan all listening ports. Uses batched sysinfo refresh for speed.
    pub fn scan_all(&self) -> Result<Vec<PortInfo>> {
        let bindings = self.adapter.list_bindings()?;
        let mut seen = std::collections::HashSet::new();
        let mut unique_bindings = Vec::new();

        for binding in bindings {
            if seen.insert((binding.port, binding.pid)) {
                unique_bindings.push(binding);
            }
        }

        // Batch-resolve all PIDs in one sysinfo pass.
        let pids: Vec<u32> = unique_bindings.iter().map(|b| b.pid).collect();
        let resolved = self.resolver.resolve_batch(&pids);
        let proc_map: std::collections::HashMap<u32, _> =
            resolved.into_iter().collect();

        let mut results: Vec<PortInfo> = unique_bindings
            .into_iter()
            .map(|binding| PortInfo {
                port: binding.port,
                protocol: if binding.is_tcp {
                    Protocol::Tcp
                } else {
                    Protocol::Udp
                },
                process: proc_map.get(&binding.pid).cloned(),
                service: None,
                docker_container: None,
            })
            .collect();

        results.sort_by_key(|p| p.port);
        Ok(results)
    }

    /// Scan a specific port.
    pub fn scan_port(&self, port: u16) -> Result<Option<PortInfo>> {
        let pid = self.adapter.find_pid_on_port(port)?;

        match pid {
            Some(pid) => {
                let process = self.resolver.resolve(pid);
                Ok(Some(PortInfo {
                    port,
                    protocol: Protocol::Tcp,
                    process,
                    service: None,
                    docker_container: None,
                }))
            }
            None => Ok(None),
        }
    }

    /// Scan a set of specific ports.
    pub fn scan_range(&self, ports: &[u16]) -> Result<Vec<PortInfo>> {
        let mut results = Vec::new();
        for &port in ports {
            if let Some(info) = self.scan_port(port)? {
                results.push(info);
            }
        }
        Ok(results)
    }
}
