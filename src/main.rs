mod cli;
mod ci;
mod classifier;
mod completions;
mod config;
mod conflict;
mod crash;
mod docker;
mod doctor;
mod engine;
mod errors;
mod grouping;
mod history;
mod inspector;
mod log;
mod models;
mod platform;
pub mod plugin;
mod preflight;
mod project;
mod registry;
mod restart;
mod resolver;
mod scanner;
mod stack;
mod status;
mod watch;

use clap::Parser;
use colored::Colorize;
use dialoguer::Confirm;

use cli::output;
use cli::Cli;
use cli::commands::{Commands, RegistryAction};
use classifier::ServiceClassifier;
use engine::{FixEngine, Strategy};
use scanner::PortScanner;

fn main() -> anyhow::Result<()> {
    // Check for conflicting installations before doing anything else
    if conflict::check() {
        std::process::exit(1);
    }

    let cli = Cli::parse();
    let adapter = platform::adapter();

    // ptrm <port>  (shorthand for info)
    if let Some(port) = cli.port
        && cli.command.is_none()
    {
        let scanner = PortScanner::new(adapter.as_ref());
        return cmd_info(&scanner, port, cli.json);
    }

    match cli.command {
        Some(Commands::Scan { ports, dev }) => {
            let scanner = PortScanner::new(adapter.as_ref());
            cmd_scan(&scanner, &ports, dev, cli.json)
        }
        Some(Commands::Kill { ports, yes, force }) => {
            let mut any_failed = false;
            for port in &ports {
                if let Err(e) = cmd_kill(adapter.as_ref(), *port, yes, force) {
                    eprintln!("  {} port {}: {}", "\u{2718}".red(), port, e);
                    any_failed = true;
                }
            }
            if any_failed && ports.len() == 1 {
                anyhow::bail!("kill failed");
            }
            Ok(())
        }
        Some(Commands::Fix { ports, yes, force, run }) => {
            // Auto-detect occupied ports when none specified.
            let ports = if ports.is_empty() {
                let scanner = PortScanner::new(adapter.as_ref());
                let all = scanner.scan_all().unwrap_or_default();
                let detected: Vec<u16> = all.iter().map(|p| p.port).collect();
                if detected.is_empty() {
                    println!();
                    println!("  {} No occupied ports found — nothing to fix.", "\u{2714}".green());
                    println!();
                    return Ok(());
                }
                if !cli.json {
                    println!();
                    println!(
                        "  {} Auto-detected {} occupied port(s): {}",
                        "\u{2192}".dimmed(),
                        detected.len(),
                        detected.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
                    );
                    println!();
                }
                detected
            } else {
                ports
            };
            let mut any_failed = false;
            for port in &ports {
                if let Err(e) = cmd_fix(adapter.as_ref(), *port, yes, force, run.clone(), cli.json) {
                    eprintln!("  {} port {}: {}", "\u{2718}".red(), port, e);
                    any_failed = true;
                }
            }
            if any_failed && ports.len() == 1 {
                anyhow::bail!("fix failed");
            }
            Ok(())
        }
        Some(Commands::Info { port }) => {
            let scanner = PortScanner::new(adapter.as_ref());
            cmd_info(&scanner, port, cli.json)
        }
        Some(Commands::Log { port }) => {
            let scanner = PortScanner::new(adapter.as_ref());
            cmd_log(&scanner, port)
        }
        Some(Commands::Restart { service }) => {
            cmd_restart(adapter.as_ref(), &service)
        }
        Some(Commands::Status) => {
            cmd_status(adapter.as_ref(), cli.json)
        }
        Some(Commands::Interactive) => {
            cli::interactive::run_interactive(adapter.as_ref())
        }
        Some(Commands::Group { dev }) => {
            cmd_group(adapter.as_ref(), dev, cli.json)
        }
        Some(Commands::Doctor { yes }) => {
            cmd_doctor(adapter.as_ref(), yes, cli.json)
        }
        Some(Commands::History { clear, stats }) => {
            cmd_history(clear, stats, cli.json)
        }
        Some(Commands::Project) => {
            cmd_project(cli.json)
        }
        Some(Commands::Watch { port, interval }) => {
            cmd_watch(adapter.as_ref(), port, interval)
        }
        Some(Commands::Up { yes }) => {
            cmd_up(adapter.as_ref(), yes)
        }
        Some(Commands::Down) => {
            cmd_down(adapter.as_ref())
        }
        Some(Commands::Preflight { ports }) => {
            cmd_preflight(adapter.as_ref(), &ports)
        }
        Some(Commands::Init) => {
            cmd_init()
        }
        Some(Commands::Registry { action }) => {
            match action {
                RegistryAction::Check => cmd_registry_check(cli.json),
            }
        }
        Some(Commands::Ci) => {
            cmd_ci(adapter.as_ref(), cli.json)
        }
        Some(Commands::Use { profile }) => {
            cmd_use_profile(&profile)
        }
        Some(Commands::Completions { shell }) => {
            completions::generate_completions(&shell, &mut std::io::stdout())
        }
        Some(Commands::CompletePorts) => {
            completions::list_active_ports()
        }
        None => {
            // No command and no port: default to scan.
            let scanner = PortScanner::new(adapter.as_ref());
            cmd_scan(&scanner, &[], false, cli.json)
        }
    }
}

fn cmd_scan(scanner: &PortScanner, ports: &[u16], dev: bool, json: bool) -> anyhow::Result<()> {
    let mut results = if ports.is_empty() {
        scanner.scan_all()?
    } else {
        scanner.scan_range(ports)?
    };

    if dev {
        results.retain(|p| is_dev_port(p.port));
    }

    // Classify each result using port context.
    for info in &mut results {
        if let Some(ref proc_) = info.process {
            info.service = Some(ServiceClassifier::classify_with_port(proc_, info.port));
        }
    }

    // Merge Docker container info into scan results.
    let docker_map = docker::detect_containers();
    if !docker_map.is_empty() {
        // Attach container info to ports already found.
        for info in &mut results {
            if let Some(container) = docker_map.get(&info.port) {
                info.docker_container = Some(container.clone());
                // Override service classification for Docker-managed ports.
                info.service = Some(models::DevService {
                    kind: models::ServiceKind::Docker,
                    confidence: 1.0,
                    restart_hint: Some(format!("docker restart {}", container.name)),
                });
            }
        }

        // Add Docker-only ports that weren't found in the OS scan
        // (e.g. host-mode or ports only visible via docker ps).
        let existing_ports: std::collections::HashSet<u16> =
            results.iter().map(|r| r.port).collect();
        for (host_port, container) in &docker_map {
            if !existing_ports.contains(host_port) {
                results.push(models::PortInfo {
                    port: *host_port,
                    protocol: models::Protocol::Tcp,
                    process: None,
                    service: Some(models::DevService {
                        kind: models::ServiceKind::Docker,
                        confidence: 1.0,
                        restart_hint: Some(format!("docker restart {}", container.name)),
                    }),
                    docker_container: Some(container.clone()),
                });
            }
        }

        results.sort_by_key(|p| p.port);
    }

    if json {
        output::print_scan_json(&results);
    } else {
        output::print_scan_table(&results);
    }

    Ok(())
}

fn cmd_kill(
    adapter: &dyn platform::PlatformAdapter,
    port: u16,
    yes: bool,
    force: bool,
) -> anyhow::Result<()> {
    let engine = FixEngine::new(adapter);
    let mut plan = engine.analyze(port)?;

    output::print_port_detail(&plan.port_info);
    println!();

    // Block system-critical processes even for explicit kill.
    if plan.verdict.is_blocked() {
        output::print_fix_blocked(&plan);
        return Ok(());
    }

    output::print_safety_verdict(&plan.verdict);
    println!();

    // Override strategy based on flags.
    if force {
        FixEngine::override_strategy(&mut plan, Strategy::Force);
    } else {
        FixEngine::override_strategy(&mut plan, Strategy::Escalating);
    }

    if !yes {
        let msg = output::print_kill_confirm(&plan.port_info);
        let default_yes = plan.verdict.is_safe();
        let confirmed = Confirm::new()
            .with_prompt(msg)
            .default(default_yes)
            .interact()?;
        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    println!();
    let result = engine.execute(&plan, |step| {
        output::print_fix_step(step, 0);
    })?;

    // Record to history.
    history::record(history::HistoryEntry {
        timestamp: chrono::Utc::now(),
        action: history::ActionKind::Kill,
        port,
        pid: result.pid,
        process_name: plan.port_info.process.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        service: plan.port_info.service.as_ref().map(|s| s.kind.label().to_string()),
        strategy: Some(result.strategy_used.to_string()),
        success: result.success,
    });

    println!();
    output::print_fix_outcome(&result);
    Ok(())
}

fn cmd_fix(
    adapter: &dyn platform::PlatformAdapter,
    port: u16,
    yes: bool,
    force: bool,
    run_cmd: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let engine = FixEngine::new(adapter);

    // Phase 1: Analyze the conflict.
    let mut plan = engine.analyze(port)?;

    if !json {
        output::print_fix_plan(&plan);
    }

    // Handle blocked (system-critical) processes.
    if plan.verdict.is_blocked() {
        if json {
            // Emit a minimal JSON error.
            println!(
                "{}",
                serde_json::json!({
                    "port": port,
                    "blocked": true,
                    "reason": plan.verdict.reason(),
                })
            );
        } else {
            output::print_fix_blocked(&plan);
        }
        return Ok(());
    }

    // Override strategy if --force.
    if force {
        FixEngine::override_strategy(&mut plan, Strategy::Force);
        if !json {
            println!(
                "  {} Strategy overridden to {}",
                ">>".dimmed(),
                "Force (SIGKILL)".yellow()
            );
        }
    }

    // Confirm with user.
    if !yes {
        println!();
        let default_yes = plan.verdict.is_safe();
        let msg = format!("Fix port {port} using {}?", plan.strategy);
        let confirmed = Confirm::new()
            .with_prompt(msg)
            .default(default_yes)
            .interact()?;
        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Phase 2: Execute the fix with live step output.
    if !json {
        println!();
    }
    let step_num = std::cell::Cell::new(1u32);
    let result = engine.execute(&plan, |step| {
        if !json {
            output::print_fix_step(step, step_num.get());
            step_num.set(step_num.get() + 1);
        }
    })?;

    // Record to history.
    history::record(history::HistoryEntry {
        timestamp: chrono::Utc::now(),
        action: history::ActionKind::Fix,
        port,
        pid: result.pid,
        process_name: plan.port_info.process.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        service: plan.port_info.service.as_ref().map(|s| s.kind.label().to_string()),
        strategy: Some(result.strategy_used.to_string()),
        success: result.success,
    });

    // Phase 3: Report the outcome.
    println!();
    if json {
        output::print_fix_result_json(&result);
    } else {
        output::print_fix_outcome(&result);
    }

    // Phase 4: Auto-restart if requested and fix succeeded.
    if result.success
        && let Some(ref cmd) = run_cmd
    {
        if !json {
            output::print_auto_restart(cmd);
        }
        auto_restart(cmd)?;
    }

    Ok(())
}

fn auto_restart(cmd: &str) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new("sh")
            .args(["-c", cmd])
            .exec();
        // exec() only returns on error.
        Err(err.into())
    }

    #[cfg(not(unix))]
    {
        let status = std::process::Command::new("cmd")
            .args(["/C", cmd])
            .status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn cmd_info(scanner: &PortScanner, port: u16, json: bool) -> anyhow::Result<()> {
    let info = scanner.scan_port(port)?;

    match info {
        None => {
            if json {
                println!("{}", serde_json::json!({ "port": port, "status": "free" }));
            } else {
                output::print_port_free(port);
            }
            Ok(())
        }
        Some(mut info) => {
            if let Some(ref proc_) = info.process {
                info.service = Some(ServiceClassifier::classify_with_port(proc_, info.port));
            }

            if json {
                output::print_scan_json(&[info]);
            } else {
                println!();
                output::print_port_detail(&info);

                // Docker awareness: show container info if available.
                if let Some(container) = docker::find_container_on_port(port) {
                    println!();
                    output::print_container_info(port, &container);
                }

                // Project detection from process CWD.
                if let Some(ref proc_) = info.process
                    && let Some(proj) = project::detect_from_cwd(proc_.working_dir.as_deref())
                {
                    println!();
                    output::print_project_info(&proj);
                }

                println!();
            }
            Ok(())
        }
    }
}

fn cmd_log(scanner: &PortScanner, port: u16) -> anyhow::Result<()> {
    // First check Docker directly (fast path for container-managed ports).
    if let Some(container) = docker::find_container_on_port(port) {
        println!();
        println!(
            "  \u{1f433} Streaming logs for {} (docker: {})...",
            port.to_string().cyan().bold(),
            container.name.cyan()
        );
        println!();
        return log::stream_logs(&log::PortOwner::Docker {
            container_name: container.name,
        });
    }

    // Fall back to OS-level port scan.
    let info = scanner.scan_port(port)?;
    match info {
        None => {
            anyhow::bail!("No process found on port {port}");
        }
        Some(mut info) => {
            // Classify so we get the restart_hint.
            if let Some(ref proc_) = info.process {
                info.service = Some(classifier::ServiceClassifier::classify_with_port(proc_, info.port));
            }

            let owner = log::resolve_owner(&info, port)
                .ok_or_else(|| anyhow::anyhow!("Could not identify owner of port {port}"))?;

            match &owner {
                log::PortOwner::Docker { container_name } => {
                    println!();
                    println!(
                        "  \u{1f433} Streaming logs for {} (docker: {})...",
                        port.to_string().cyan().bold(),
                        container_name.cyan()
                    );
                    println!();
                }
                log::PortOwner::LocalProcess { pid, name, .. } => {
                    println!();
                    println!(
                        "  \u{26a1} Streaming logs for {} ({}, PID {})...",
                        port.to_string().cyan().bold(),
                        name.cyan(),
                        pid.to_string().dimmed()
                    );
                    println!();
                }
            }

            log::stream_logs(&owner)
        }
    }
}

// ── New commands ──────────────────────────────────────────────────────

fn cmd_group(adapter: &dyn platform::PlatformAdapter, dev: bool, json: bool) -> anyhow::Result<()> {
    let scanner = PortScanner::new(adapter);
    let mut results = scanner.scan_all()?;

    if dev {
        results.retain(|p| is_dev_port(p.port));
    }

    for info in &mut results {
        if let Some(ref proc_) = info.process {
            info.service = Some(ServiceClassifier::classify_with_port(proc_, info.port));
        }
    }

    let groups = grouping::group_ports(results);

    if json {
        println!("{}", serde_json::to_string_pretty(&groups).unwrap_or_default());
    } else {
        output::print_grouped_table(&groups);
    }

    Ok(())
}

fn cmd_doctor(adapter: &dyn platform::PlatformAdapter, yes: bool, json: bool) -> anyhow::Result<()> {
    let diagnoses = doctor::diagnose(adapter);

    if json {
        println!("{}", serde_json::to_string_pretty(&diagnoses.iter().map(|d| {
            serde_json::json!({
                "port": d.port,
                "issue": d.issue.to_string(),
                "suggestion": d.suggestion,
                "auto_fixable": d.auto_fixable,
            })
        }).collect::<Vec<_>>()).unwrap_or_default());
        return Ok(());
    }

    output::print_doctor_results(&diagnoses);

    let auto_fixable: Vec<_> = diagnoses.iter().filter(|d| d.auto_fixable).collect();
    if auto_fixable.is_empty() || !yes {
        return Ok(());
    }

    // Auto-fix.
    println!("  \u{2699} {}", "Auto-fixing...".bold());
    println!();

    let fixed = doctor::auto_fix(adapter, &diagnoses, &mut |msg, ok| {
        output::print_doctor_step(msg, ok);
    });

    println!();
    println!(
        "  {} {}",
        "\u{2714}".green(),
        format!("Fixed {fixed} issue{}", if fixed == 1 { "" } else { "s" }).green().bold()
    );
    println!();

    Ok(())
}

fn cmd_history(clear: bool, stats: bool, json: bool) -> anyhow::Result<()> {
    if clear {
        history::clear();
        println!();
        println!("  {} {}", "\u{2714}".green(), "History cleared.".green());
        println!();
        return Ok(());
    }

    if stats {
        let s = history::stats();
        if json {
            println!("{}", serde_json::to_string_pretty(&s).unwrap_or_default());
        } else {
            output::print_history_stats(&s);
        }
        return Ok(());
    }

    let entries = history::load();
    if json {
        println!("{}", serde_json::to_string_pretty(&entries).unwrap_or_default());
    } else {
        output::print_history(&entries);
    }
    Ok(())
}

fn cmd_project(json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let info = project::detect(&cwd.to_string_lossy());

    if json {
        println!("{}", serde_json::to_string_pretty(&info).unwrap_or_default());
    } else {
        output::print_project_info(&info);
    }

    Ok(())
}

fn is_dev_port(port: u16) -> bool {
    matches!(port, 3000..=3999 | 4000..=4999 | 5000..=5999 | 8000..=8999)
}

// ── Tier 1 commands ──────────────────────────────────────────────────

fn cmd_watch(
    adapter: &dyn platform::PlatformAdapter,
    port: u16,
    interval: u64,
) -> anyhow::Result<()> {
    // Check .ptrm.toml for a restart command for this port.
    let cfg = config::load_from_cwd().map(|c| config::apply_active_profile(&c));
    let restart_cmd = cfg.as_ref().and_then(|c| {
        c.services.values().find(|s| s.port == port).map(|s| s.run.clone())
    });

    watch::watch_port(
        adapter,
        port,
        std::time::Duration::from_secs(interval),
        restart_cmd.as_deref(),
    )
}

fn cmd_restart(adapter: &dyn platform::PlatformAdapter, service: &str) -> anyhow::Result<()> {
    restart::restart_service(adapter, service)?;
    Ok(())
}

fn cmd_status(adapter: &dyn platform::PlatformAdapter, json: bool) -> anyhow::Result<()> {
    let statuses = status::get_status(adapter)?;

    if json {
        status::print_status_json(&statuses)?;
    } else {
        let project_name = config::load_from_cwd()
            .and_then(|c| c.project.name);
        status::print_status(&statuses, project_name.as_deref());
    }

    Ok(())
}

fn cmd_up(adapter: &dyn platform::PlatformAdapter, yes: bool) -> anyhow::Result<()> {
    let cfg = config::load_from_cwd()
        .ok_or_else(|| anyhow::anyhow!("No .ptrm.toml found. Run `ptrm init` to create one."))?;
    let cfg = config::apply_active_profile(&cfg);
    let cfg_dir = config::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    stack::up(adapter, &cfg, yes, &cfg_dir)?;
    Ok(())
}

fn cmd_down(adapter: &dyn platform::PlatformAdapter) -> anyhow::Result<()> {
    let cfg = config::load_from_cwd()
        .ok_or_else(|| anyhow::anyhow!("No .ptrm.toml found. Run `ptrm init` to create one."))?;
    let cfg = config::apply_active_profile(&cfg);
    let cfg_dir = config::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    stack::down(adapter, &cfg, &cfg_dir)?;
    Ok(())
}

fn cmd_preflight(adapter: &dyn platform::PlatformAdapter, ports: &[u16]) -> anyhow::Result<()> {
    let scanner = PortScanner::new(adapter);

    // If no explicit ports, use .ptrm.toml service ports + run registry check.
    let port_list: Vec<u16> = if ports.is_empty() {
        let cfg = config::load_from_cwd().map(|c| config::apply_active_profile(&c));
        match cfg {
            Some(c) if !c.services.is_empty() => {
                // Registry validation as part of preflight.
                let conflicts = registry::check(&c);
                if !conflicts.is_empty() {
                    registry::print_check(&c);
                }

                let mut ps: Vec<u16> = c.services.values().map(|s| s.port).collect();
                ps.sort();
                ps
            }
            _ => {
                anyhow::bail!("No ports specified and no .ptrm.toml found.");
            }
        }
    } else {
        ports.to_vec()
    };

    preflight::run_preflight(&scanner, &port_list)?;
    Ok(())
}

fn cmd_init() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config_path = cwd.join(".ptrm.toml");

    if config_path.exists() {
        println!();
        println!("  {} .ptrm.toml already exists.", "\u{26a0}".yellow());
        println!();
        return Ok(());
    }

    // Detect project type to generate a smart config.
    let cwd_str = cwd.to_string_lossy();
    let info = project::detect(&cwd_str);

    let project_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "my-project".to_string());

    let port = info.default_port.unwrap_or(match info.kind {
        project::ProjectKind::Rust
        | project::ProjectKind::Go
        | project::ProjectKind::Django
        | project::ProjectKind::Flask
        | project::ProjectKind::FastApi => 8080,
        _ => 3000,
    });
    let run_cmd = info.dev_command.as_deref().unwrap_or("npm start");
    let kind_label = info.kind.label();

    let service_name = match info.kind {
        project::ProjectKind::Django
        | project::ProjectKind::Flask
        | project::ProjectKind::FastApi
        | project::ProjectKind::Rust
        | project::ProjectKind::Go => "server",
        _ => "frontend",
    };

    let staging_port = port + 100;

    let template = format!(
        r#"# portrm project configuration
# https://github.com/abhishekayu/portrm

[project]
name = "{project_name}"

[services.{service_name}]
port = {port}
run = "{run_cmd}"
cwd = "."
preflight = true

# ── Profiles ──────────────────────────────────────────────────────────
# Override port/run/cwd/env per profile. Switch with: ptrm use <name>
#
# [profiles.staging]
# {service_name} = {{ port = {staging_port} }}
"#
    );

    std::fs::write(&config_path, template)
        .map_err(|e| anyhow::anyhow!(
            "Failed to write {}: {} (check directory permissions)",
            config_path.display(),
            e
        ))?;

    println!();
    println!(
        "  {} Created {}",
        "\u{2714}".green(),
        ".ptrm.toml".bold()
    );
    println!(
        "  {} Detected {} project (port {})",
        "\u{2192}".dimmed(),
        kind_label.cyan(),
        port.to_string().bold()
    );
    println!("  {} Edit the file to add more services or profiles.", "\u{2192}".dimmed());
    println!();

    Ok(())
}

// ── Tier 2 commands ──────────────────────────────────────────────────

fn cmd_registry_check(json: bool) -> anyhow::Result<()> {
    let cfg = config::load_from_cwd()
        .ok_or_else(|| anyhow::anyhow!("No .ptrm.toml found. Run `ptrm init` to create one."))?;

    let cfg = config::apply_active_profile(&cfg);

    if json {
        let conflicts = registry::check(&cfg);
        println!("{}", serde_json::to_string_pretty(&conflicts).unwrap_or_default());
        if !conflicts.is_empty() {
            std::process::exit(1);
        }
    } else {
        let valid = registry::print_check(&cfg);
        if !valid {
            std::process::exit(1);
        }
    }

    Ok(())
}

fn cmd_ci(adapter: &dyn platform::PlatformAdapter, json: bool) -> anyhow::Result<()> {
    let result = ci::run(adapter, json)?;
    if !result.passed {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_use_profile(profile: &str) -> anyhow::Result<()> {
    let cfg = config::load_from_cwd()
        .ok_or_else(|| anyhow::anyhow!("No .ptrm.toml found. Run `ptrm init` to create one."))?;

    // "default" clears the active profile, reverting to base config.
    if profile == "default" {
        config::save_state(&config::PtrmState {
            active_profile: None,
        })?;

        println!();
        println!(
            "  {} Switched to {}: using base config",
            "\u{2714}".green(),
            "default".cyan().bold()
        );
        println!();
        for (name, svc) in &cfg.services {
            println!(
                "      {} {} \u{2192} port {}",
                "\u{2192}".dimmed(),
                name.cyan(),
                svc.port.to_string().bold()
            );
        }
        println!();
        return Ok(());
    }

    // Validate profile exists.
    let has_profile = cfg
        .profiles
        .as_ref()
        .is_some_and(|p| p.contains_key(profile));

    if !has_profile {
        let available: Vec<String> = cfg
            .profiles
            .as_ref()
            .map(|p| p.keys().cloned().collect())
            .unwrap_or_default();

        if available.is_empty() {
            anyhow::bail!("No profiles defined in .ptrm.toml");
        } else {
            anyhow::bail!(
                "Profile '{}' not found. Available: {}",
                profile,
                available.join(", ")
            );
        }
    }

    // Save state.
    config::save_state(&config::PtrmState {
        active_profile: Some(profile.to_string()),
    })?;

    // Show what changed.
    let effective = config::apply_profile(&cfg, profile);
    println!();
    println!(
        "  {} Switched to profile: {}",
        "\u{2714}".green(),
        profile.cyan().bold()
    );
    println!();
    for (name, svc) in &effective.services {
        println!(
            "      {} {} \u{2192} port {}",
            "\u{2192}".dimmed(),
            name.cyan(),
            svc.port.to_string().bold()
        );
    }
    println!();

    Ok(())
}
