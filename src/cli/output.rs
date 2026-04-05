use colored::Colorize;
use serde::Serialize;

use crate::docker::ContainerInfo;
use crate::doctor::Diagnosis;
use crate::engine::{FixPlan, FixResult, FixStep, SafetyVerdict, Strategy};
use crate::grouping::PortGroup;
use crate::history::{HistoryEntry, HistoryStats};
use crate::models::{PortInfo, ServiceKind};
use crate::project::ProjectInfo;

// ── Icons ─────────────────────────────────────────────────────────────

const ICON_BOLT: &str = "\u{26a1}";      // ⚡
const ICON_ARROW: &str = "\u{2192}";     // →
const ICON_CHECK: &str = "\u{2714}";     // ✔
const ICON_CROSS: &str = "\u{2718}";     // ✘
const ICON_ROCKET: &str = "\u{1f680}";   // 🚀
const ICON_SHIELD: &str = "\u{1f6e1}";   // 🛡
const ICON_WARN: &str = "\u{26a0}";      // ⚠
const ICON_BLOCK: &str = "\u{1f6d1}";    // 🛑
const ICON_DOT: &str = "\u{2022}";       // •
const ICON_GEAR: &str = "\u{2699}";      // ⚙
const ICON_SEARCH: &str = "\u{1f50d}";   // 🔍
const ICON_DOCTOR: &str = "\u{1fa7a}";   // 🩺
const ICON_FOLDER: &str = "\u{1f4c2}";   // 📂
const ICON_DOCKER: &str = "\u{1f433}";   // 🐳
const ICON_CLOCK: &str = "\u{1f552}";    // 🕒

// ── Scan output ───────────────────────────────────────────────────────

pub fn print_scan_table(ports: &[PortInfo]) {
    if ports.is_empty() {
        println!();
        println!("  {} {}", ICON_SEARCH, "No listening ports found.".dimmed());
        println!();
        return;
    }

    let header = format!(
        "  {:<7} {:<22} {:<8} {:<14} {:<10} {:<10} {}",
        "PORT", "PROCESS", "PID", "SERVICE", "MEMORY", "UPTIME", "USER"
    );

    println!();
    println!(
        "  {} {}",
        ICON_BOLT,
        format!("{} active port{}", ports.len(), if ports.len() == 1 { "" } else { "s" })
            .bold()
    );
    println!();
    println!("{}", header.dimmed());
    println!("  {}", "\u{2500}".repeat(80).dimmed());

    for info in ports {
        print_scan_row(info);
    }

    println!();
}

fn print_scan_row(info: &PortInfo) {
    let (pid, name, user, memory, uptime) = match &info.process {
        Some(p) => (
            p.pid.to_string(),
            truncate(&p.name, 20),
            p.user.clone().unwrap_or_default(),
            p.memory_bytes.map(format_bytes).unwrap_or_else(|| "-".into()),
            p.runtime_display(),
        ),
        None => ("-".into(), "-".into(), "-".into(), "-".into(), "-".into()),
    };

    // If this port belongs to a Docker container, override the display name.
    let (display_name, svc_plain, svc_color) = if let Some(ref container) = info.docker_container {
        let docker_label = format!("docker:{}", container.name);
        let svc = container.image.split(':').next().unwrap_or(&container.image);
        (
            truncate(&docker_label, 20),
            svc.to_string(),
            ServiceKind::Docker,
        )
    } else {
        let svc_plain = info
            .service
            .as_ref()
            .map(|s| s.kind.label().to_string())
            .unwrap_or_else(|| "-".into());
        let svc_color = info
            .service
            .as_ref()
            .map(|s| s.kind)
            .unwrap_or(ServiceKind::Unknown);
        (name, svc_plain, svc_color)
    };

    // Pad plain text first, then apply color (ANSI codes break width formatting).
    let col_port = format!("{:<7}", info.port);
    let col_name = format!("{:<22}", display_name);
    let col_pid = format!("{:<8}", pid);
    let col_svc = format!("{:<14}", svc_plain);
    let col_mem = format!("{:<10}", memory);
    let col_up = format!("{:<10}", uptime);

    print!("  {}", col_port.cyan().bold());
    print!(" {}", col_name);
    print!(" {}", col_pid.dimmed());
    print!(" {}", colorize_service_padded(&col_svc, svc_color));
    print!(" {}", col_mem.dimmed());
    print!(" {}", col_up.dimmed());
    println!(" {}", user.dimmed());
}

pub fn print_scan_json(ports: &[PortInfo]) {
    let json = serde_json::to_string_pretty(ports).unwrap_or_default();
    println!("{json}");
}

// ── Port detail (info command) ────────────────────────────────────────

pub fn print_port_detail(info: &PortInfo) {
    let port_label = format!("{} Port {} in use", ICON_BOLT, info.port);
    println!("  {}", port_label.bold());

    if let Some(ref proc_) = info.process {
        let svc_label = info
            .service
            .as_ref()
            .map(|s| s.kind.label())
            .unwrap_or(&proc_.name);

        println!(
            "  {} {} {}",
            ICON_ARROW,
            svc_label.cyan(),
            format!("(PID {})", proc_.pid).dimmed()
        );
        println!(
            "  {} running for {}",
            ICON_ARROW,
            proc_.runtime_display().yellow()
        );

        if let Some(mem) = proc_.memory_bytes {
            println!(
                "  {} memory {}",
                ICON_ARROW,
                format_bytes(mem).dimmed()
            );
        }

        println!(
            "  {} {}",
            ICON_ARROW,
            proc_.command.dimmed()
        );

        if let Some(ref user) = proc_.user {
            println!(
                "  {} user {}",
                ICON_ARROW,
                user.dimmed()
            );
        }

        if let Some(ref cwd) = proc_.working_dir {
            println!(
                "  {} cwd {}",
                ICON_ARROW,
                cwd.dimmed()
            );
        }
    } else {
        println!(
            "  {} {}",
            ICON_ARROW,
            "process unknown".dimmed()
        );
    }

    if let Some(ref svc) = info.service {
        let kind_str = colorize_service(svc.kind);
        println!(
            "  {} detected {} {}",
            ICON_ARROW,
            kind_str,
            format!("({:.0}% confidence)", svc.confidence * 100.0).dimmed()
        );

        let safe_str = if svc.kind.safe_to_kill() {
            format!("{} safe to kill", ICON_SHIELD).green().to_string()
        } else {
            format!("{} use caution", ICON_WARN).red().to_string()
        };
        println!("  {} {}", ICON_ARROW, safe_str);
    }
}

// ── Fix plan / steps / outcome ────────────────────────────────────────

pub fn print_fix_plan(plan: &FixPlan) {
    println!();
    print_port_detail(&plan.port_info);
    println!();

    // Safety verdict.
    match &plan.verdict {
        SafetyVerdict::Safe { reason } => {
            println!(
                "  {} {}  {}",
                ICON_SHIELD,
                "SAFE".green().bold(),
                reason.dimmed()
            );
        }
        SafetyVerdict::Warn { reason } => {
            println!(
                "  {} {}  {}",
                ICON_WARN,
                "WARNING".yellow().bold(),
                reason.dimmed()
            );
        }
        SafetyVerdict::Block { reason } => {
            println!(
                "  {} {}  {}",
                ICON_BLOCK,
                "BLOCKED".red().bold(),
                reason.dimmed()
            );
        }
    }

    println!(
        "  {} strategy: {}",
        ICON_GEAR,
        format!("{}", plan.strategy).cyan()
    );
}

pub fn print_fix_blocked(plan: &FixPlan) {
    println!();
    println!(
        "  {} {} {}",
        ICON_BLOCK,
        "BLOCKED".red().bold(),
        plan.verdict.reason()
    );
    println!(
        "  {}",
        "ptrm will not kill this process. Use a different port instead.".dimmed()
    );
}

pub fn print_safety_verdict(verdict: &SafetyVerdict) {
    match verdict {
        SafetyVerdict::Safe { reason } => {
            println!("  {} {} {}", ICON_SHIELD, "SAFE".green().bold(), reason.dimmed());
        }
        SafetyVerdict::Warn { reason } => {
            println!("  {} {} {}", ICON_WARN, "WARNING".yellow().bold(), reason.dimmed());
        }
        SafetyVerdict::Block { reason } => {
            println!("  {} {} {}", ICON_BLOCK, "BLOCKED".red().bold(), reason.dimmed());
        }
    }
}

pub fn print_fix_step(step: &FixStep, _num: u32) {
    if step.ok {
        println!(
            "  {} {}",
            ICON_DOT.dimmed(),
            step.label
        );
    } else {
        println!(
            "  {} {}",
            ICON_DOT.dimmed(),
            step.label.yellow()
        );
    }
}

pub fn print_fix_outcome(result: &FixResult) {
    if result.success {
        println!(
            "  {} {}",
            ICON_CHECK.to_string().green().bold(),
            format!(
                "Killed safely  port {} is now free",
                result.port,
            )
            .green()
        );

        if let Some(ref hint) = result.restart_hint {
            println!();
            println!(
                "  {} {}",
                "Restart:".bold(),
                hint.cyan()
            );
        }
    } else {
        println!(
            "  {} {}",
            ICON_CROSS.to_string().red().bold(),
            format!(
                "Failed to free port {} (PID {} may still be running)",
                result.port, result.pid
            )
            .red()
        );
        println!(
            "  {} {}",
            ICON_ARROW,
            "try: sudo ptrm kill --force <port>".yellow()
        );
    }
}

pub fn print_fix_result_json(result: &FixResult) {
    #[derive(Serialize)]
    struct JsonResult<'a> {
        port: u16,
        pid: u32,
        strategy: &'a str,
        success: bool,
        steps: Vec<JsonStep<'a>>,
        restart_hint: Option<&'a str>,
    }
    #[derive(Serialize)]
    struct JsonStep<'a> {
        label: &'a str,
        ok: bool,
    }

    let jr = JsonResult {
        port: result.port,
        pid: result.pid,
        strategy: match result.strategy_used {
            Strategy::Graceful => "graceful",
            Strategy::Escalating => "escalating",
            Strategy::Force => "force",
        },
        success: result.success,
        steps: result
            .steps
            .iter()
            .map(|s| JsonStep {
                label: &s.label,
                ok: s.ok,
            })
            .collect(),
        restart_hint: result.restart_hint.as_deref(),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&jr).unwrap_or_default()
    );
}

pub fn print_auto_restart(cmd: &str) {
    println!();
    println!(
        "  {} {} {}",
        ICON_ROCKET,
        "Restarting:".bold(),
        cmd.cyan()
    );
    println!();
}

pub fn print_kill_confirm(info: &PortInfo) -> String {
    let proc_name = info
        .process
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("unknown");
    let svc = info
        .service
        .as_ref()
        .map(|s| s.kind.label())
        .unwrap_or("Unknown");

    format!("Kill {} ({}) on port {}?", proc_name, svc, info.port)
}

// ── Nothing on port ───────────────────────────────────────────────────

pub fn print_port_free(port: u16) {
    println!();
    println!(
        "  {} Port {} is {}",
        ICON_CHECK.to_string().green(),
        port.to_string().cyan().bold(),
        "free".green().bold()
    );
    println!();
}

// ── Helpers ───────────────────────────────────────────────────────────

fn colorize_service(kind: ServiceKind) -> String {
    let label = kind.label();
    match kind {
        ServiceKind::NextJs | ServiceKind::Vite | ServiceKind::CreateReactApp
        | ServiceKind::Java | ServiceKind::DotNet | ServiceKind::Go
        | ServiceKind::Rust | ServiceKind::Ruby => {
            label.green().to_string()
        }
        ServiceKind::Docker | ServiceKind::Nginx | ServiceKind::Apache | ServiceKind::Iis => {
            label.blue().to_string()
        }
        ServiceKind::PostgreSQL | ServiceKind::MySQL | ServiceKind::Redis
        | ServiceKind::SQLServer | ServiceKind::MongoDB => {
            label.yellow().to_string()
        }
        ServiceKind::Unknown => label.dimmed().to_string(),
        _ => label.to_string(),
    }
}

fn colorize_service_padded(padded: &str, kind: ServiceKind) -> String {
    match kind {
        ServiceKind::NextJs | ServiceKind::Vite | ServiceKind::CreateReactApp
        | ServiceKind::Java | ServiceKind::DotNet | ServiceKind::Go
        | ServiceKind::Rust | ServiceKind::Ruby => {
            padded.green().to_string()
        }
        ServiceKind::Docker | ServiceKind::Nginx | ServiceKind::Apache | ServiceKind::Iis => {
            padded.blue().to_string()
        }
        ServiceKind::PostgreSQL | ServiceKind::MySQL | ServiceKind::Redis
        | ServiceKind::SQLServer | ServiceKind::MongoDB => {
            padded.yellow().to_string()
        }
        ServiceKind::Unknown => padded.dimmed().to_string(),
        _ => padded.to_string(),
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}..", &s[..max - 2])
    }
}

// ── Grouped scan output ──────────────────────────────────────────────

pub fn print_grouped_table(groups: &[PortGroup]) {
    if groups.is_empty() {
        println!();
        println!("  {} {}", ICON_SEARCH, "No listening ports found.".dimmed());
        println!();
        return;
    }

    let total: usize = groups.iter().map(|g| g.ports.len()).sum();
    println!();
    println!(
        "  {} {}",
        ICON_BOLT,
        format!("{total} active port{} in {} group{}", 
            if total == 1 { "" } else { "s" },
            groups.len(),
            if groups.len() == 1 { "" } else { "s" },
        ).bold()
    );

    for group in groups {
        println!();
        let role_label = format!("  {} {} ({})", ICON_GEAR, group.role.label(), group.ports.len());
        println!("{}", role_label.cyan().bold());
        println!("  {}", "\u{2500}".repeat(76).dimmed());

        for info in &group.ports {
            print_scan_row(info);
        }
    }

    println!();
}

// ── Docker output ────────────────────────────────────────────────────

pub fn print_container_info(_port: u16, container: &ContainerInfo) {
    println!(
        "  {} {} {} {}",
        ICON_DOCKER,
        "Docker container:".bold(),
        container.name.cyan(),
        format!("({})", container.image).dimmed()
    );
    println!(
        "  {} status: {}",
        ICON_ARROW,
        container.status.dimmed()
    );
    for pm in &container.ports {
        println!(
            "  {} {}:{} {} {}/{}",
            ICON_ARROW,
            "host".dimmed(),
            pm.host_port.to_string().cyan(),
            ICON_ARROW,
            pm.container_port,
            pm.protocol.dimmed()
        );
    }
}

// ── Doctor output ────────────────────────────────────────────────────

pub fn print_doctor_results(diagnoses: &[Diagnosis]) {
    if diagnoses.is_empty() {
        println!();
        println!(
            "  {} {}",
            ICON_CHECK.to_string().green(),
            "All clear! No issues found.".green().bold()
        );
        println!();
        return;
    }

    println!();
    println!(
        "  {} {}",
        ICON_DOCTOR,
        format!("{} issue{} found", diagnoses.len(), if diagnoses.len() == 1 { "" } else { "s" }).bold(),
    );
    println!();

    for (i, diag) in diagnoses.iter().enumerate() {
        let fixable_tag = if diag.auto_fixable {
            "[auto-fixable]".green().to_string()
        } else {
            "[manual]".yellow().to_string()
        };

        println!(
            "  {}. {} {}",
            i + 1,
            diag.issue.to_string().yellow(),
            fixable_tag,
        );
        println!(
            "     {} {}",
            ICON_ARROW,
            diag.suggestion.dimmed()
        );
    }

    let auto_count = diagnoses.iter().filter(|d| d.auto_fixable).count();
    if auto_count > 0 {
        println!();
        println!(
            "  {} Run {} to auto-fix {} issue{}",
            ICON_GEAR,
            "ptrm doctor -y".cyan().bold(),
            auto_count,
            if auto_count == 1 { "" } else { "s" }
        );
    }

    println!();
}

pub fn print_doctor_step(msg: &str, ok: bool) {
    if ok {
        println!("  {} {}", ICON_DOT.dimmed(), msg);
    } else {
        println!("  {} {}", ICON_CROSS.to_string().red(), msg.yellow());
    }
}

// ── History output ───────────────────────────────────────────────────

pub fn print_history(entries: &[HistoryEntry]) {
    if entries.is_empty() {
        println!();
        println!("  {} {}", ICON_SEARCH, "No history yet.".dimmed());
        println!();
        return;
    }

    println!();
    println!(
        "  {} {}",
        ICON_CLOCK,
        format!("{} recorded action{}", entries.len(), if entries.len() == 1 { "" } else { "s" }).bold()
    );
    println!();

    let header = format!(
        "  {:<22} {:<8} {:<7} {:<8} {:<20} {}",
        "TIME", "ACTION", "PORT", "PID", "PROCESS", "RESULT"
    );
    println!("{}", header.dimmed());
    println!("  {}", "\u{2500}".repeat(76).dimmed());

    // Show last 20.
    let start = entries.len().saturating_sub(20);
    for entry in &entries[start..] {
        let time_str = entry.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let result_str = if entry.success {
            ICON_CHECK.to_string().green().to_string()
        } else {
            ICON_CROSS.to_string().red().to_string()
        };

        let col_time = format!("{:<22}", time_str);
        let col_action = format!("{:<8}", entry.action.to_string());
        let col_port = format!("{:<7}", entry.port);
        let col_pid = format!("{:<8}", entry.pid);
        let col_proc = format!("{:<20}", truncate(&entry.process_name, 18));

        print!("  {}", col_time.dimmed());
        print!(" {}", col_action);
        print!(" {}", col_port.cyan());
        print!(" {}", col_pid.dimmed());
        print!(" {}", col_proc);
        println!(" {}", result_str);
    }

    println!();
}

pub fn print_history_stats(stats: &HistoryStats) {
    println!();
    println!("  {} {}", ICON_GEAR, "History Statistics".bold());
    println!();
    println!("  {} Total actions:    {}", ICON_ARROW, stats.total_actions.to_string().cyan());
    println!("  {} Kills:            {}", ICON_ARROW, stats.kills);
    println!("  {} Fixes:            {}", ICON_ARROW, stats.fixes);
    println!("  {} Success rate:     {}", ICON_ARROW, format!("{:.1}%", stats.success_rate).green());

    if let Some(port) = stats.most_killed_port {
        println!("  {} Most killed port: {}", ICON_ARROW, port.to_string().yellow());
    }
    if let Some(ref proc_name) = stats.most_killed_process {
        println!("  {} Most killed proc: {}", ICON_ARROW, proc_name.yellow());
    }

    println!();
}

// ── Project output ───────────────────────────────────────────────────

pub fn print_project_info(info: &ProjectInfo) {
    println!();
    println!(
        "  {} {} {}",
        ICON_FOLDER,
        "Detected project:".bold(),
        info.kind.label().cyan().bold()
    );
    println!("  {} root: {}", ICON_ARROW, info.root.dimmed());

    if let Some(ref cmd) = info.dev_command {
        println!("  {} dev command: {}", ICON_ARROW, cmd.cyan());
    }
    if let Some(port) = info.default_port {
        println!("  {} default port: {}", ICON_ARROW, port.to_string().yellow());
    }

    println!();
}
