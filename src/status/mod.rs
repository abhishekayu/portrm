use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::classifier::ServiceClassifier;
use crate::config::{self, PtrmConfig};
use crate::docker;
use crate::platform::PlatformAdapter;
use crate::scanner::PortScanner;

/// Live status of a configured service.
#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub port: u16,
    pub status: StatusType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusType {
    Running,
    Stopped,
    Conflict,
}

/// Get status for all services in `.ptrm.toml`.
pub fn get_status(adapter: &dyn PlatformAdapter) -> Result<Vec<ServiceStatus>> {
    let cfg = config::load_from_cwd()
        .ok_or_else(|| anyhow::anyhow!("No .ptrm.toml found. Run `ptrm init` to create one."))?;
    let cfg = config::apply_active_profile(&cfg);

    get_status_for_config(adapter, &cfg)
}

/// Get status for a given config (used internally and for testing).
fn get_status_for_config(
    adapter: &dyn PlatformAdapter,
    config: &PtrmConfig,
) -> Result<Vec<ServiceStatus>> {
    let scanner = PortScanner::new(adapter);
    let docker_containers = docker::detect_containers();
    let mut results = Vec::new();

    let mut services: Vec<(&String, &crate::config::ServiceConfig)> = config.services.iter().collect();
    services.sort_by_key(|(name, _)| name.to_owned());

    for (name, svc) in &services {
        let port = svc.port;
        let info = scanner.scan_port(port).ok().flatten();

        let status = match info {
            None => {
                // Check Docker containers too.
                if let Some(container) = docker_containers.get(&port) {
                    ServiceStatus {
                        name: name.to_string(),
                        port,
                        status: StatusType::Running,
                        process: None,
                        pid: None,
                        docker: Some(container.name.clone()),
                    }
                } else {
                    ServiceStatus {
                        name: name.to_string(),
                        port,
                        status: StatusType::Stopped,
                        process: None,
                        pid: None,
                        docker: None,
                    }
                }
            }
            Some(mut port_info) => {
                // Check Docker first.
                if let Some(container) = docker_containers.get(&port) {
                    ServiceStatus {
                        name: name.to_string(),
                        port,
                        status: StatusType::Running,
                        process: port_info.process.as_ref().map(|p| p.name.clone()),
                        pid: port_info.process.as_ref().map(|p| p.pid),
                        docker: Some(container.name.clone()),
                    }
                } else if let Some(ref proc_) = port_info.process {
                    // Classify service to check if it matches.
                    let classified = ServiceClassifier::classify_with_port(proc_, port);
                    port_info.service = Some(classified);

                    let is_match = is_expected_process(name, &port_info);
                    ServiceStatus {
                        name: name.to_string(),
                        port,
                        status: if is_match { StatusType::Running } else { StatusType::Conflict },
                        process: Some(proc_.name.clone()),
                        pid: Some(proc_.pid),
                        docker: None,
                    }
                } else {
                    ServiceStatus {
                        name: name.to_string(),
                        port,
                        status: StatusType::Conflict,
                        process: None,
                        pid: None,
                        docker: None,
                    }
                }
            }
        };

        results.push(status);
    }

    Ok(results)
}

/// Heuristic: does the running process match the expected service?
fn is_expected_process(service_name: &str, info: &crate::models::PortInfo) -> bool {
    let proc_ = match &info.process {
        Some(p) => p,
        None => return false,
    };

    let proc_name = proc_.name.to_lowercase();
    let proc_cmd = proc_.command.to_lowercase();
    let svc_name = service_name.to_lowercase();

    // Direct name match.
    if proc_name.contains(&svc_name) || svc_name.contains(&proc_name) {
        return true;
    }

    // Check classified service kind against common mappings.
    if let Some(ref svc) = info.service {
        let kind_label = svc.kind.label().to_lowercase();
        if svc_name.contains(&kind_label) || kind_label.contains(&svc_name) {
            return true;
        }

        // Known mappings: frontend -> node/next/vite, backend -> python/node, database -> postgres/mysql/redis
        let matches = match svc_name.as_str() {
            "frontend" | "web" | "client" | "app" => {
                matches!(svc.kind,
                    crate::models::ServiceKind::NextJs
                    | crate::models::ServiceKind::Vite
                    | crate::models::ServiceKind::CreateReactApp
                    | crate::models::ServiceKind::NodeGeneric
                )
            }
            "backend" | "api" | "server" => {
                matches!(svc.kind,
                    crate::models::ServiceKind::NodeGeneric
                    | crate::models::ServiceKind::Python
                    | crate::models::ServiceKind::Django
                    | crate::models::ServiceKind::Flask
                )
            }
            "database" | "db" | "postgres" | "postgresql" => {
                matches!(svc.kind, crate::models::ServiceKind::PostgreSQL)
            }
            "redis" | "cache" => {
                matches!(svc.kind, crate::models::ServiceKind::Redis)
            }
            "mysql" => {
                matches!(svc.kind, crate::models::ServiceKind::MySQL)
            }
            "nginx" | "proxy" => {
                matches!(svc.kind, crate::models::ServiceKind::Nginx)
            }
            _ => false,
        };
        if matches {
            return true;
        }
    }

    // Check command string for service name.
    if proc_cmd.contains(&svc_name) {
        return true;
    }

    // Common process/service associations.
    let run_cmd_lower = proc_cmd.to_lowercase();
    match svc_name.as_str() {
        "frontend" | "web" | "client" => {
            run_cmd_lower.contains("next") || run_cmd_lower.contains("vite")
                || run_cmd_lower.contains("react") || run_cmd_lower.contains("npm")
                || run_cmd_lower.contains("webpack")
        }
        "backend" | "api" | "server" => {
            run_cmd_lower.contains("uvicorn") || run_cmd_lower.contains("gunicorn")
                || run_cmd_lower.contains("flask") || run_cmd_lower.contains("django")
                || run_cmd_lower.contains("express") || run_cmd_lower.contains("fastapi")
        }
        _ => false,
    }
}

/// Print the status table to stdout.
pub fn print_status(statuses: &[ServiceStatus], project_name: Option<&str>) {
    println!();
    if let Some(name) = project_name {
        println!("  Project: {}", name.bold());
        println!();
    }

    for s in statuses {
        let (icon, color_status) = match s.status {
            StatusType::Running => ("\u{1f7e2}", "running".green()),
            StatusType::Stopped => ("\u{1f534}", "stopped".red()),
            StatusType::Conflict => ("\u{1f7e1}", "conflict".yellow()),
        };

        println!("  {} {}", icon, s.name.bold());
        println!("    port: {}", s.port.to_string().cyan());
        print!("    status: {}", color_status);

        if let Some(ref docker_name) = s.docker {
            print!(" (docker: {})", docker_name.cyan());
        }
        println!();

        if let (Some(proc_name), Some(pid)) = (&s.process, s.pid)
            && s.docker.is_none()
        {
            println!("    process: {} (PID {})", proc_name, pid.to_string().dimmed());
        }

        println!();
    }
}

/// Print the status as JSON.
pub fn print_status_json(statuses: &[ServiceStatus]) -> Result<()> {
    let json = serde_json::to_string_pretty(statuses)?;
    println!("{json}");
    Ok(())
}
