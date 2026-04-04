use std::path::Path;

use serde::Serialize;

/// Detected project type from filesystem markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProjectKind {
    NextJs,
    Vite,
    CreateReactApp,
    NodeGeneric,
    Django,
    Flask,
    FastApi,
    PythonGeneric,
    Rust,
    Go,
    DockerCompose,
    Unknown,
}

impl ProjectKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NextJs => "Next.js",
            Self::Vite => "Vite",
            Self::CreateReactApp => "Create React App",
            Self::NodeGeneric => "Node.js",
            Self::Django => "Django",
            Self::Flask => "Flask",
            Self::FastApi => "FastAPI",
            Self::PythonGeneric => "Python",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::DockerCompose => "Docker Compose",
            Self::Unknown => "Unknown",
        }
    }

    pub fn default_port(&self) -> Option<u16> {
        match self {
            Self::NextJs => Some(3000),
            Self::Vite => Some(5173),
            Self::CreateReactApp => Some(3000),
            Self::Django => Some(8000),
            Self::Flask => Some(5000),
            Self::FastApi => Some(8000),
            Self::Rust => None,
            Self::Go => None,
            Self::NodeGeneric => Some(3000),
            Self::PythonGeneric => None,
            Self::DockerCompose => None,
            Self::Unknown => None,
        }
    }

    pub fn dev_command(&self) -> Option<&'static str> {
        match self {
            Self::NextJs => Some("npm run dev"),
            Self::Vite => Some("npm run dev"),
            Self::CreateReactApp => Some("npm start"),
            Self::NodeGeneric => Some("npm start"),
            Self::Django => Some("python manage.py runserver"),
            Self::Flask => Some("flask run"),
            Self::FastApi => Some("uvicorn main:app --reload"),
            Self::PythonGeneric => None,
            Self::Rust => Some("cargo run"),
            Self::Go => Some("go run ."),
            Self::DockerCompose => Some("docker compose up"),
            Self::Unknown => None,
        }
    }
}

/// Detect the project type in a given directory by scanning for marker files.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub kind: ProjectKind,
    pub root: String,
    pub dev_command: Option<String>,
    pub default_port: Option<u16>,
}

/// Detect the project type in a given directory by scanning for marker files.
pub fn detect(dir: &str) -> ProjectInfo {
    let path = Path::new(dir);
    let kind = detect_kind(path);

    // Try to extract a hardcoded port from package.json scripts (e.g. --port 3050).
    let script_port = extract_port_from_scripts(path);

    ProjectInfo {
        kind,
        root: dir.to_string(),
        dev_command: kind.dev_command().map(String::from),
        default_port: script_port.or_else(|| kind.default_port()),
    }
}

/// Detect project from a process's working directory (if available).
pub fn detect_from_cwd(cwd: Option<&str>) -> Option<ProjectInfo> {
    let dir = cwd?;
    let info = detect(dir);
    if info.kind == ProjectKind::Unknown {
        None
    } else {
        Some(info)
    }
}

fn detect_kind(dir: &Path) -> ProjectKind {
    // Check for package.json-based projects first.
    let pkg_json = dir.join("package.json");
    if pkg_json.exists()
        && let Ok(contents) = std::fs::read_to_string(&pkg_json)
    {
        let lower = contents.to_lowercase();
        if lower.contains("\"next\"") || lower.contains("\"next\":") {
            return ProjectKind::NextJs;
        }
        if lower.contains("\"vite\"") || lower.contains("\"vite\":") {
            return ProjectKind::Vite;
        }
        if lower.contains("\"react-scripts\"") {
            return ProjectKind::CreateReactApp;
        }
        return ProjectKind::NodeGeneric;
    }

    // Django: manage.py present.
    if dir.join("manage.py").exists()
        && let Ok(contents) = std::fs::read_to_string(dir.join("manage.py"))
        && contents.contains("django")
    {
        return ProjectKind::Django;
    }

    // FastAPI / Flask / Python.
    if dir.join("pyproject.toml").exists() || dir.join("requirements.txt").exists() {
        // Check for framework markers.
        for marker_file in &["requirements.txt", "pyproject.toml", "setup.py", "Pipfile"] {
            let p = dir.join(marker_file);
            if let Ok(contents) = std::fs::read_to_string(&p) {
                let lower = contents.to_lowercase();
                if lower.contains("fastapi") || lower.contains("uvicorn") {
                    return ProjectKind::FastApi;
                }
                if lower.contains("flask") {
                    return ProjectKind::Flask;
                }
                if lower.contains("django") {
                    return ProjectKind::Django;
                }
            }
        }
        return ProjectKind::PythonGeneric;
    }

    // Rust.
    if dir.join("Cargo.toml").exists() {
        return ProjectKind::Rust;
    }

    // Go.
    if dir.join("go.mod").exists() {
        return ProjectKind::Go;
    }

    // Docker Compose.
    if dir.join("docker-compose.yml").exists() || dir.join("docker-compose.yaml").exists() || dir.join("compose.yml").exists() || dir.join("compose.yaml").exists() {
        return ProjectKind::DockerCompose;
    }

    ProjectKind::Unknown
}

/// Extract a hardcoded port from `package.json` scripts.
///
/// Looks for patterns like `--port 3050`, `-p 8080`, `--port=3050`
/// in the `dev` or `start` scripts.
fn extract_port_from_scripts(dir: &Path) -> Option<u16> {
    let pkg_path = dir.join("package.json");
    let contents = std::fs::read_to_string(&pkg_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&contents).ok()?;
    let scripts = json.get("scripts")?.as_object()?;

    // Check "dev" first, then "start".
    for key in &["dev", "start"] {
        if let Some(script) = scripts.get(*key).and_then(|v| v.as_str())
            && let Some(port) = parse_port_from_command(script)
        {
            return Some(port);
        }
    }
    None
}

/// Parse a port number from a command string.
/// Matches: --port 3050, --port=3050, -p 3050, -p=3050, :8080 (e.g. 0.0.0.0:8080)
fn parse_port_from_command(cmd: &str) -> Option<u16> {
    let tokens: Vec<&str> = cmd.split_whitespace().collect();
    for (i, tok) in tokens.iter().enumerate() {
        // --port=3050 or -p=3050
        if let Some(rest) = tok.strip_prefix("--port=").or_else(|| tok.strip_prefix("-p="))
            && let Ok(p) = rest.parse::<u16>()
        {
            return Some(p);
        }
        // --port 3050 or -p 3050
        if (*tok == "--port" || *tok == "-p")
            && i + 1 < tokens.len()
            && let Ok(p) = tokens[i + 1].parse::<u16>()
        {
            return Some(p);
        }
        // :8080 pattern (uvicorn 0.0.0.0:8080)
        if let Some(colon_idx) = tok.rfind(':')
            && let Ok(p) = tok[colon_idx + 1..].parse::<u16>()
            && p > 0
        {
            return Some(p);
        }
    }
    None
}
