use std::collections::HashMap;

use colored::Colorize;

use crate::config::PtrmConfig;

/// A registry conflict found in the config.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Conflict {
    pub port: u16,
    pub services: Vec<String>,
    pub context: String,
}

/// Validate the config for port conflicts.
///
/// Returns a list of conflicts (empty = valid).
pub fn check(config: &PtrmConfig) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    // Check 1: Duplicate ports across services.
    let mut port_owners: HashMap<u16, Vec<String>> = HashMap::new();
    for (name, svc) in &config.services {
        port_owners
            .entry(svc.port)
            .or_default()
            .push(name.clone());
    }

    for (port, owners) in &port_owners {
        if owners.len() > 1 {
            conflicts.push(Conflict {
                port: *port,
                services: owners.clone(),
                context: "duplicate port in services".to_string(),
            });
        }
    }

    // Check 2: Profile conflicts (if profiles exist).
    if let Some(ref profiles) = config.profiles {
        for (profile_name, profile) in profiles {
            let mut profile_ports: HashMap<u16, Vec<String>> = port_owners
                .iter()
                .map(|(p, o)| (*p, o.clone()))
                .collect();

            // Apply profile overrides.
            for (svc_name, svc_override) in &profile.services {
                if let Some(new_port) = svc_override.port {
                    // Remove old mapping for this service.
                    for owners in profile_ports.values_mut() {
                        owners.retain(|n| n != svc_name);
                    }
                    profile_ports.retain(|_, o| !o.is_empty());

                    // Add new mapping.
                    profile_ports
                        .entry(new_port)
                        .or_default()
                        .push(svc_name.clone());
                }
            }

            for (port, owners) in &profile_ports {
                if owners.len() > 1 {
                    conflicts.push(Conflict {
                        port: *port,
                        services: owners.clone(),
                        context: format!("conflict in profile '{profile_name}'"),
                    });
                }
            }
        }
    }

    conflicts
}

/// Print registry check results.
///
/// Returns `true` if valid (no conflicts).
pub fn print_check(config: &PtrmConfig) -> bool {
    let conflicts = check(config);

    println!();
    if conflicts.is_empty() {
        println!(
            "  {} {}",
            "\u{2714}".green(),
            "Registry valid -- no port conflicts.".green().bold()
        );
        println!();
        return true;
    }

    println!(
        "  {} {}",
        "\u{2718}".red(),
        format!(
            "Found {} port conflict{}:",
            conflicts.len(),
            if conflicts.len() == 1 { "" } else { "s" }
        )
        .red()
        .bold()
    );
    println!();

    for conflict in &conflicts {
        println!(
            "  {} Port conflict: {}",
            "\u{2718}".red(),
            conflict.context.dimmed()
        );
        for svc in &conflict.services {
            println!(
                "      {} {} {}",
                "\u{2192}".dimmed(),
                svc.cyan(),
                format!("\u{2192} {}", conflict.port).dimmed()
            );
        }
        println!();
    }

    false
}
