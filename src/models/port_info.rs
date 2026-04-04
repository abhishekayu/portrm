use serde::Serialize;

use super::process_info::ProcessInfo;
use super::service::DevService;
use crate::docker::ContainerInfo;

/// Full picture of what's happening on a single port.
#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub protocol: Protocol,
    pub process: Option<ProcessInfo>,
    pub service: Option<DevService>,
    /// Docker container that owns this port (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_container: Option<ContainerInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "TCP"),
            Protocol::Udp => write!(f, "UDP"),
        }
    }
}
