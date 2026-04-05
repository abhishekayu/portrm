use crate::models::ProcessInfo;
use crate::models::{DevService, ServiceKind};

/// Classify a process into a known dev service category.
///
/// Uses a three-layer approach for accuracy:
/// 1. **Process binary name** (exact match after stripping `.exe`)
/// 2. **Command-line arguments** (framework-specific patterns)
/// 3. **Well-known port** (fallback when the binary is generic like `python`)
pub struct ServiceClassifier;

impl ServiceClassifier {
    /// Classify with port context for maximum accuracy.
    pub fn classify_with_port(process: &ProcessInfo, port: u16) -> DevService {
        let cmd = process.command.to_lowercase();
        let name = {
            let n = process.name.to_lowercase();
            // Strip .exe suffix so classifiers work the same on Windows.
            n.strip_suffix(".exe").unwrap_or(&n).to_string()
        };

        // ── Layer 1: Binary-name exact match (highest confidence) ──
        if let Some(kind) = match_binary_exact(&name) {
            return DevService {
                kind,
                confidence: 0.95,
                restart_hint: restart_hint(kind, process),
            };
        }

        // ── Layer 2: Command-line / framework detection ──
        if let Some((kind, confidence)) = match_command_patterns(&cmd, &name) {
            return DevService {
                kind,
                confidence,
                restart_hint: restart_hint(kind, process),
            };
        }

        // ── Layer 3: Well-known port fallback ──
        if let Some(kind) = well_known_port(port) {
            return DevService {
                kind,
                confidence: 0.60,
                restart_hint: restart_hint(kind, process),
            };
        }

        // ── Fallback: generic runtime detection (only if binary IS the runtime) ──
        if let Some((kind, confidence)) = match_generic_runtime(&name) {
            return DevService {
                kind,
                confidence,
                restart_hint: restart_hint(kind, process),
            };
        }

        DevService {
            kind: ServiceKind::Unknown,
            confidence: 0.0,
            restart_hint: None,
        }
    }
}

// ── Layer 1: Exact binary name ────────────────────────────────────────

fn match_binary_exact(name: &str) -> Option<ServiceKind> {
    match name {
        // Databases
        "postgres" | "postmaster" | "pg_isready" => Some(ServiceKind::PostgreSQL),
        "mysqld" | "mariadbd" | "mariadb" => Some(ServiceKind::MySQL),
        "redis-server" | "redis-sentinel" => Some(ServiceKind::Redis),
        "sqlservr" | "mssql" => Some(ServiceKind::SQLServer),
        "mongod" | "mongos" => Some(ServiceKind::MongoDB),
        // Infrastructure
        "nginx" => Some(ServiceKind::Nginx),
        "httpd" | "apache2" | "apache" => Some(ServiceKind::Apache),
        "w3wp" | "iisexpress" => Some(ServiceKind::IIS),
        "dockerd" | "docker-proxy" | "containerd" => Some(ServiceKind::Docker),
        _ => None,
    }
}

// ── Layer 2: Command-line pattern matching ────────────────────────────

fn match_command_patterns(cmd: &str, name: &str) -> Option<(ServiceKind, f32)> {
    // Next.js
    if cmd.contains("next") && (cmd.contains("dev") || cmd.contains("start"))
        || cmd.contains(".next/")
        || cmd.contains("next-server")
    {
        return Some((ServiceKind::NextJs, 0.95));
    }

    // Vite
    if cmd.contains("vite") && !cmd.contains("invite") {
        return Some((ServiceKind::Vite, 0.95));
    }

    // Create React App
    if cmd.contains("react-scripts") || cmd.contains("react-app-rewired") {
        return Some((ServiceKind::CreateReactApp, 0.90));
    }

    // Django
    if cmd.contains("manage.py") && cmd.contains("runserver") || cmd.contains("django") {
        return Some((ServiceKind::Django, 0.90));
    }

    // Flask
    if cmd.contains("flask") && cmd.contains("run") {
        return Some((ServiceKind::Flask, 0.85));
    }

    // .NET / Kestrel
    if name == "dotnet" || cmd.contains("dotnet run") || cmd.contains("dotnet watch") {
        return Some((ServiceKind::DotNet, 0.85));
    }

    // Java / Spring Boot / Tomcat
    if name == "java" || name == "javaw" {
        return Some((ServiceKind::Java, 0.80));
    }
    if cmd.contains("spring-boot") || cmd.contains("tomcat") || cmd.contains(".jar") {
        return Some((ServiceKind::Java, 0.85));
    }

    // Ruby / Rails
    if cmd.contains("rails") || cmd.contains("puma") || cmd.contains("unicorn")
        || cmd.contains("webrick")
    {
        return Some((ServiceKind::Ruby, 0.85));
    }
    if name == "ruby" {
        return Some((ServiceKind::Ruby, 0.75));
    }

    // Go
    if cmd.contains("go run") || (name == "air" || cmd.contains("/air")) {
        return Some((ServiceKind::Go, 0.80));
    }

    // Rust
    if cmd.contains("cargo run") || cmd.contains("cargo watch") {
        return Some((ServiceKind::Rust, 0.80));
    }

    // Python frameworks (uvicorn, gunicorn, fastapi) -- more specific than generic python
    if cmd.contains("uvicorn") || cmd.contains("gunicorn") || cmd.contains("fastapi") {
        return Some((ServiceKind::Python, 0.85));
    }

    None
}

// ── Layer 3: Well-known port mapping ──────────────────────────────────

fn well_known_port(port: u16) -> Option<ServiceKind> {
    match port {
        5432 => Some(ServiceKind::PostgreSQL),
        3306 | 3307 => Some(ServiceKind::MySQL),
        6379 | 6380 => Some(ServiceKind::Redis),
        1433 | 1434 => Some(ServiceKind::SQLServer),
        27017 | 27018 | 27019 => Some(ServiceKind::MongoDB),
        _ => None,
    }
}

// ── Fallback: Generic runtime detection ───────────────────────────────
// Only match when the binary name IS exactly the runtime executable.

fn match_generic_runtime(name: &str) -> Option<(ServiceKind, f32)> {
    // Node.js -- exact binary names only
    if name == "node" || name == "nodejs" {
        return Some((ServiceKind::NodeGeneric, 0.70));
    }

    // Python -- exact binary names only (python, python3, python3.12, etc.)
    if name == "python" || name.starts_with("python3") || name.starts_with("python2") {
        return Some((ServiceKind::Python, 0.65));
    }

    // Go binary (standalone compiled binary won't be detected, that's expected)
    if name == "go" {
        return Some((ServiceKind::Go, 0.60));
    }

    None
}

fn restart_hint(kind: ServiceKind, process: &ProcessInfo) -> Option<String> {
    let cwd = process.working_dir.as_deref().unwrap_or(".");

    match kind {
        ServiceKind::NextJs => Some(format!("cd {cwd} && npm run dev")),
        ServiceKind::Vite => Some(format!("cd {cwd} && npm run dev")),
        ServiceKind::CreateReactApp => Some(format!("cd {cwd} && npm start")),
        ServiceKind::Django => Some(format!("cd {cwd} && python manage.py runserver")),
        ServiceKind::Flask => Some(format!("cd {cwd} && flask run")),
        ServiceKind::NodeGeneric => Some(format!("cd {cwd} && npm start")),
        ServiceKind::Python => Some(format!("cd {cwd} && python {}", process.command)),
        ServiceKind::Docker => Some("docker compose up".to_string()),
        ServiceKind::Java => Some(format!("cd {cwd} && ./mvnw spring-boot:run")),
        ServiceKind::DotNet => Some(format!("cd {cwd} && dotnet run")),
        ServiceKind::Ruby => Some(format!("cd {cwd} && rails server")),
        ServiceKind::Go => Some(format!("cd {cwd} && go run .")),
        ServiceKind::Rust => Some(format!("cd {cwd} && cargo run")),
        _ => None,
    }
}
