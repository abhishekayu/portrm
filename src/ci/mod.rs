use colored::Colorize;

use crate::config;
use crate::doctor;
use crate::platform::PlatformAdapter;
use crate::preflight;
use crate::registry;
use crate::scanner::PortScanner;

/// Result of a CI run.
#[derive(Debug, serde::Serialize)]
pub struct CiResult {
    pub config_valid: bool,
    pub registry_valid: bool,
    pub preflight_passed: bool,
    pub doctor_issues: usize,
    pub passed: bool,
}

/// Run all CI checks non-interactively.
pub fn run(adapter: &dyn PlatformAdapter, json: bool) -> anyhow::Result<CiResult> {
    let mut result = CiResult {
        config_valid: false,
        registry_valid: false,
        preflight_passed: false,
        doctor_issues: 0,
        passed: false,
    };

    // Step 1: Validate config.
    if !json {
        println!();
        println!("  {} {}", "\u{25b6}".dimmed(), "Validating config...".bold());
    }

    let cfg = match config::load_from_cwd() {
        Some(c) => {
            if !json {
                println!("  {} Config loaded", "\u{2714}".green());
            }
            result.config_valid = true;
            c
        }
        None => {
            if !json {
                println!("  {} No .ptrm.toml found", "\u{2718}".red());
            }
            result.passed = false;
            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                print_summary(&result);
            }
            return Ok(result);
        }
    };

    // Apply active profile.
    let cfg = config::apply_active_profile(&cfg);

    // Step 2: Registry check.
    if !json {
        println!("  {} {}", "\u{25b6}".dimmed(), "Running registry check...".bold());
    }

    let conflicts = registry::check(&cfg);
    if conflicts.is_empty() {
        result.registry_valid = true;
        if !json {
            println!("  {} No port conflicts", "\u{2714}".green());
        }
    } else {
        if !json {
            println!(
                "  {} {} port conflict{}",
                "\u{2718}".red(),
                conflicts.len(),
                if conflicts.len() == 1 { "" } else { "s" }
            );
            for c in &conflicts {
                println!(
                    "      {} {} \u{2192} port {}",
                    "\u{2192}".dimmed(),
                    c.services.join(", ").cyan(),
                    c.port
                );
            }
        }
    }

    // Step 3: Preflight.
    if !json {
        println!("  {} {}", "\u{25b6}".dimmed(), "Running preflight checks...".bold());
    }

    let scanner = PortScanner::new(adapter);
    let ports: Vec<u16> = cfg.services.values().map(|s| s.port).collect();

    if ports.is_empty() {
        result.preflight_passed = true;
        if !json {
            println!("  {} No ports to check", "\u{2714}".green());
        }
    } else {
        let mut all_free = true;
        for &port in &ports {
            match preflight::check_port(&scanner, port)? {
                Some(info) => {
                    all_free = false;
                    if !json {
                        let proc_name = info
                            .process
                            .as_ref()
                            .map(|p| p.name.as_str())
                            .unwrap_or("unknown");
                        println!(
                            "  {} Port {} busy ({})",
                            "\u{2718}".red(),
                            port.to_string().cyan(),
                            proc_name
                        );
                    }
                }
                None => {
                    if !json {
                        println!("  {} Port {} free", "\u{2714}".green(), port.to_string().cyan());
                    }
                }
            }
        }
        result.preflight_passed = all_free;
    }

    // Step 4: Doctor (safe mode - read-only).
    if !json {
        println!("  {} {}", "\u{25b6}".dimmed(), "Running diagnostics...".bold());
    }

    let diagnoses = doctor::diagnose(adapter);
    result.doctor_issues = diagnoses.len();

    if diagnoses.is_empty() {
        if !json {
            println!("  {} No issues found", "\u{2714}".green());
        }
    } else if !json {
        println!(
            "  {} {} issue{}",
            "\u{26a0}".yellow(),
            diagnoses.len(),
            if diagnoses.len() == 1 { "" } else { "s" }
        );
    }

    // Overall result.
    result.passed = result.config_valid && result.registry_valid && result.preflight_passed;

    if json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
    } else {
        print_summary(&result);
    }

    Ok(result)
}

fn print_summary(result: &CiResult) {
    println!();
    if result.passed {
        println!(
            "  {} {}",
            "\u{2714}".green(),
            "CI passed".green().bold()
        );
    } else {
        println!(
            "  {} {}",
            "\u{2718}".red(),
            "CI failed".red().bold()
        );
    }
    println!();
}
