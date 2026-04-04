use std::collections::HashMap;
use std::process::Command;

use serde::Serialize;

/// Information about a Docker container bound to a port.
#[derive(Debug, Clone, Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: Vec<PortMapping>,
}

/// A single host:container port mapping.
#[derive(Debug, Clone, Serialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

/// Query Docker for all running containers and their port mappings.
/// Returns a map from host port to container info.
pub fn detect_containers() -> HashMap<u16, ContainerInfo> {
    let mut map = HashMap::new();

    let output = match Command::new("docker")
        .args([
            "ps",
            "--format",
            "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}",
        ])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return map, // Docker not installed or not running.
    };

    let text = String::from_utf8_lossy(&output.stdout);

    for line in text.lines() {
        let fields: Vec<&str> = line.splitn(5, '\t').collect();
        if fields.len() < 5 {
            continue;
        }

        let id = fields[0].to_string();
        let name = fields[1].to_string();
        let image = fields[2].to_string();
        let status = fields[3].to_string();
        let ports_str = fields[4];

        let port_mappings = parse_port_mappings(ports_str);

        for pm in &port_mappings {
            map.insert(
                pm.host_port,
                ContainerInfo {
                    id: id.clone(),
                    name: name.clone(),
                    image: image.clone(),
                    status: status.clone(),
                    ports: port_mappings.clone(),
                },
            );
        }
    }

    map
}

/// Look up a single container by host port.
pub fn find_container_on_port(port: u16) -> Option<ContainerInfo> {
    let containers = detect_containers();
    containers.into_values().find(|c| c.ports.iter().any(|p| p.host_port == port))
}

/// Parse Docker's port string like "0.0.0.0:3000->3000/tcp, 0.0.0.0:3001->3001/tcp".
fn parse_port_mappings(ports_str: &str) -> Vec<PortMapping> {
    let mut mappings = Vec::new();

    for part in ports_str.split(", ") {
        let part = part.trim();
        // Format: 0.0.0.0:HOST->CONTAINER/proto  or  :::HOST->CONTAINER/proto
        if let Some(arrow_pos) = part.find("->") {
            let left = &part[..arrow_pos];
            let right = &part[arrow_pos + 2..];

            // Host port: last colon-separated segment on the left.
            let host_port = left
                .rsplit(':')
                .next()
                .and_then(|p| p.parse::<u16>().ok());

            // Container port + protocol.
            let (container_port, protocol) = if let Some(slash) = right.find('/') {
                (
                    right[..slash].parse::<u16>().ok(),
                    right[slash + 1..].to_string(),
                )
            } else {
                (right.parse::<u16>().ok(), "tcp".to_string())
            };

            if let (Some(hp), Some(cp)) = (host_port, container_port) {
                mappings.push(PortMapping {
                    host_port: hp,
                    container_port: cp,
                    protocol,
                });
            }
        }
    }

    mappings
}
