use serde::Serialize;

/// A recognized dev service category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ServiceKind {
    NextJs,
    Vite,
    CreateReactApp,
    NodeGeneric,
    Python,
    Django,
    Flask,
    Docker,
    Nginx,
    PostgreSQL,
    Redis,
    MySQL,
    Unknown,
}

impl ServiceKind {
    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::NextJs => "Next.js",
            Self::Vite => "Vite",
            Self::CreateReactApp => "Create React App",
            Self::NodeGeneric => "Node.js",
            Self::Python => "Python",
            Self::Django => "Django",
            Self::Flask => "Flask",
            Self::Docker => "Docker",
            Self::Nginx => "Nginx",
            Self::PostgreSQL => "PostgreSQL",
            Self::Redis => "Redis",
            Self::MySQL => "MySQL",
            Self::Unknown => "Unknown",
        }
    }

    /// Whether this is typically safe to kill without data loss.
    pub fn safe_to_kill(&self) -> bool {
        matches!(
            self,
            Self::NextJs
                | Self::Vite
                | Self::CreateReactApp
                | Self::NodeGeneric
                | Self::Python
                | Self::Django
                | Self::Flask
        )
    }
}

impl std::fmt::Display for ServiceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Full service classification result.
#[derive(Debug, Clone, Serialize)]
pub struct DevService {
    pub kind: ServiceKind,
    pub confidence: f32,
    pub restart_hint: Option<String>,
}
