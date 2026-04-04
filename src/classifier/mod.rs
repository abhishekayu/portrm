use crate::models::ProcessInfo;
use crate::models::{DevService, ServiceKind};

/// Classify a process into a known dev service category.
pub struct ServiceClassifier;

impl ServiceClassifier {
    pub fn classify(process: &ProcessInfo) -> DevService {
        let cmd = process.command.to_lowercase();
        let name = process.name.to_lowercase();

        // Order matters: more specific patterns first.
        let (kind, confidence) = if is_nextjs(&cmd) {
            (ServiceKind::NextJs, 0.95)
        } else if is_vite(&cmd) {
            (ServiceKind::Vite, 0.95)
        } else if is_cra(&cmd) {
            (ServiceKind::CreateReactApp, 0.90)
        } else if is_django(&cmd) {
            (ServiceKind::Django, 0.90)
        } else if is_flask(&cmd) {
            (ServiceKind::Flask, 0.85)
        } else if is_docker(&name) {
            (ServiceKind::Docker, 0.95)
        } else if is_nginx(&name) {
            (ServiceKind::Nginx, 0.95)
        } else if is_postgres(&name) {
            (ServiceKind::PostgreSQL, 0.95)
        } else if is_redis(&name) {
            (ServiceKind::Redis, 0.95)
        } else if is_mysql(&name) {
            (ServiceKind::MySQL, 0.95)
        } else if is_node(&cmd, &name) {
            (ServiceKind::NodeGeneric, 0.75)
        } else if is_python(&cmd, &name) {
            (ServiceKind::Python, 0.70)
        } else {
            (ServiceKind::Unknown, 0.0)
        };

        DevService {
            kind,
            confidence,
            restart_hint: restart_hint(kind, process),
        }
    }
}

fn is_nextjs(cmd: &str) -> bool {
    cmd.contains("next") && (cmd.contains("dev") || cmd.contains("start"))
        || cmd.contains(".next/")
        || cmd.contains("next-server")
}

fn is_vite(cmd: &str) -> bool {
    cmd.contains("vite") && !cmd.contains("invite")
}

fn is_cra(cmd: &str) -> bool {
    cmd.contains("react-scripts") || cmd.contains("react-app-rewired")
}

fn is_django(cmd: &str) -> bool {
    cmd.contains("manage.py") && cmd.contains("runserver") || cmd.contains("django")
}

fn is_flask(cmd: &str) -> bool {
    cmd.contains("flask") && cmd.contains("run")
}

fn is_docker(name: &str) -> bool {
    name.contains("docker") || name.contains("containerd")
}

fn is_nginx(name: &str) -> bool {
    name.contains("nginx")
}

fn is_postgres(name: &str) -> bool {
    name.contains("postgres") || name.contains("postmaster")
}

fn is_redis(name: &str) -> bool {
    name.contains("redis-server") || name.contains("redis")
}

fn is_mysql(name: &str) -> bool {
    name.contains("mysql") || name.contains("mariadbd")
}

fn is_node(cmd: &str, name: &str) -> bool {
    name.contains("node") || cmd.contains("node ") || cmd.contains("npx") || cmd.contains("tsx")
}

fn is_python(cmd: &str, name: &str) -> bool {
    name.contains("python") || cmd.contains("python") || cmd.contains("uvicorn")
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
        _ => None,
    }
}
