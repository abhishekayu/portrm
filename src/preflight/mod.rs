use colored::Colorize;

use crate::classifier::ServiceClassifier;
use crate::models::PortInfo;
use crate::scanner::PortScanner;

/// Check if a port is already in use before starting a service.
///
/// Returns `Some(PortInfo)` if the port is occupied, `None` if free.
pub fn check_port(scanner: &PortScanner, port: u16) -> anyhow::Result<Option<PortInfo>> {
    let info = scanner.scan_port(port)?;

    match info {
        Some(mut port_info) => {
            if let Some(ref proc_) = port_info.process {
                port_info.service = Some(ServiceClassifier::classify(proc_));
            }
            Ok(Some(port_info))
        }
        None => Ok(None),
    }
}

/// Run pre-flight checks for one or more ports and print results.
///
/// Returns the list of conflicting ports.
pub fn run_preflight(scanner: &PortScanner, ports: &[u16]) -> anyhow::Result<Vec<PortInfo>> {
    let mut conflicts = Vec::new();

    println!();
    println!(
        "  \u{1f50d} Pre-flight check for {} port{}...",
        ports.len(),
        if ports.len() == 1 { "" } else { "s" }
    );
    println!();

    for &port in ports {
        match check_port(scanner, port)? {
            Some(info) => {
                let proc_name = info
                    .process
                    .as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let svc_label = info
                    .service
                    .as_ref()
                    .map(|s| s.kind.label().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                let pid = info.process.as_ref().map(|p| p.pid).unwrap_or(0);

                println!(
                    "  {} Port {} is {} -- {} (PID {}, {})",
                    "\u{2718}".red(),
                    port.to_string().cyan().bold(),
                    "busy".red().bold(),
                    proc_name,
                    pid,
                    svc_label.dimmed()
                );

                conflicts.push(info);
            }
            None => {
                println!(
                    "  {} Port {} is {}",
                    "\u{2714}".green(),
                    port.to_string().cyan(),
                    "free".green().bold()
                );
            }
        }
    }

    println!();
    if conflicts.is_empty() {
        println!(
            "  {} {}",
            "\u{2714}".green(),
            "All ports are available.".green().bold()
        );
    } else {
        println!(
            "  {} {} port{} already in use. Run {} to fix.",
            "\u{26a0}".yellow(),
            conflicts.len(),
            if conflicts.len() == 1 { " is" } else { "s are" },
            "ptrm fix <port>".bold()
        );
    }
    println!();

    Ok(conflicts)
}
