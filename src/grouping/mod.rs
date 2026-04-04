use serde::Serialize;

use crate::models::{PortInfo, ServiceKind};

/// Port role categories for grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum PortRole {
    Frontend,
    Backend,
    Database,
    Infrastructure,
    Other,
}

impl PortRole {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Frontend => "Frontend",
            Self::Backend => "Backend",
            Self::Database => "Database",
            Self::Infrastructure => "Infrastructure",
            Self::Other => "Other",
        }
    }
}

/// A group of ports sharing the same role.
#[derive(Debug, Serialize)]
pub struct PortGroup {
    pub role: PortRole,
    pub ports: Vec<PortInfo>,
}

/// Classify a port into a role based on service kind and port number.
pub fn classify_role(info: &PortInfo) -> PortRole {
    // Service-based classification takes priority.
    if let Some(ref svc) = info.service {
        match svc.kind {
            ServiceKind::NextJs | ServiceKind::Vite | ServiceKind::CreateReactApp => {
                return PortRole::Frontend;
            }
            ServiceKind::Django | ServiceKind::Flask | ServiceKind::NodeGeneric | ServiceKind::Python => {
                // Could be frontend or backend; use port heuristic.
                if is_frontend_port(info.port) {
                    return PortRole::Frontend;
                }
                return PortRole::Backend;
            }
            ServiceKind::PostgreSQL | ServiceKind::MySQL | ServiceKind::Redis => {
                return PortRole::Database;
            }
            ServiceKind::Docker | ServiceKind::Nginx => {
                return PortRole::Infrastructure;
            }
            ServiceKind::Unknown => {}
        }
    }

    // Port-range heuristic.
    match info.port {
        3000..=3999 | 5173 | 5174 => PortRole::Frontend,
        4000..=4999 | 8000..=8999 | 9000..=9999 => PortRole::Backend,
        5432 | 6379 | 27017 | 6380 => PortRole::Database,
        80 | 443 | 2375 | 2376 => PortRole::Infrastructure,
        _ => PortRole::Other,
    }
}

/// Group ports by role.
pub fn group_ports(ports: Vec<PortInfo>) -> Vec<PortGroup> {
    let mut groups: std::collections::BTreeMap<PortRole, Vec<PortInfo>> = std::collections::BTreeMap::new();

    for info in ports {
        let role = classify_role(&info);
        groups.entry(role).or_default().push(info);
    }

    groups
        .into_iter()
        .map(|(role, ports)| PortGroup { role, ports })
        .collect()
}

fn is_frontend_port(port: u16) -> bool {
    matches!(port, 3000..=3999 | 5173 | 5174)
}
