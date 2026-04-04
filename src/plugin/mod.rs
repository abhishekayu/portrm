use crate::models::{DevService, ProcessInfo, ServiceKind};

/// Trait for custom service detectors.
///
/// Implement this trait to add detection for custom/proprietary services
/// that ptrm doesn't recognize out of the box.
///
/// # Example
///
/// ```rust
/// use ptrm::plugin::ServiceDetector;
/// use ptrm::models::{ProcessInfo, DevService, ServiceKind};
///
/// struct MyAppDetector;
///
/// impl ServiceDetector for MyAppDetector {
///     fn name(&self) -> &str { "my-app" }
///
///     fn detect(&self, process: &ProcessInfo) -> Option<DevService> {
///         if process.command.contains("my-app-server") {
///             Some(DevService {
///                 kind: ServiceKind::NodeGeneric,
///                 confidence: 0.9,
///                 restart_hint: Some("my-app start".to_string()),
///             })
///         } else {
///             None
///         }
///     }
/// }
/// ```
pub trait ServiceDetector: Send + Sync {
    /// Unique name for this detector.
    fn name(&self) -> &str;

    /// Try to detect a service from process info.
    /// Return `None` if this detector doesn't recognize the process.
    fn detect(&self, process: &ProcessInfo) -> Option<DevService>;

    /// Priority (higher = checked first). Default is 0.
    fn priority(&self) -> i32 {
        0
    }
}

/// Registry of service detectors.
pub struct DetectorRegistry {
    detectors: Vec<Box<dyn ServiceDetector>>,
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DetectorRegistry {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    /// Register a new detector.
    pub fn register(&mut self, detector: Box<dyn ServiceDetector>) {
        self.detectors.push(detector);
        self.detectors.sort_by_key(|b| std::cmp::Reverse(b.priority()));
    }

    /// Try all registered detectors in priority order.
    /// Returns the first match, or None.
    pub fn detect(&self, process: &ProcessInfo) -> Option<DevService> {
        for detector in &self.detectors {
            if let Some(svc) = detector.detect(process) {
                return Some(svc);
            }
        }
        None
    }
}

// ── Built-in example detector ─────────────────────────────────────────

/// Detects Webpack Dev Server.
pub struct WebpackDetector;

impl ServiceDetector for WebpackDetector {
    fn name(&self) -> &str {
        "webpack-dev-server"
    }

    fn detect(&self, process: &ProcessInfo) -> Option<DevService> {
        let cmd = process.command.to_lowercase();
        if cmd.contains("webpack") && (cmd.contains("serve") || cmd.contains("dev-server")) {
            Some(DevService {
                kind: ServiceKind::NodeGeneric,
                confidence: 0.85,
                restart_hint: Some(format!(
                    "cd {} && npx webpack serve",
                    process.working_dir.as_deref().unwrap_or(".")
                )),
            })
        } else {
            None
        }
    }

    fn priority(&self) -> i32 {
        10
    }
}

/// Detects Uvicorn (ASGI server).
pub struct UvicornDetector;

impl ServiceDetector for UvicornDetector {
    fn name(&self) -> &str {
        "uvicorn"
    }

    fn detect(&self, process: &ProcessInfo) -> Option<DevService> {
        let cmd = process.command.to_lowercase();
        if cmd.contains("uvicorn") {
            Some(DevService {
                kind: ServiceKind::Python,
                confidence: 0.90,
                restart_hint: Some(format!(
                    "cd {} && uvicorn main:app --reload",
                    process.working_dir.as_deref().unwrap_or(".")
                )),
            })
        } else {
            None
        }
    }

    fn priority(&self) -> i32 {
        10
    }
}

/// Create a registry pre-loaded with built-in detectors.
pub fn default_registry() -> DetectorRegistry {
    let mut registry = DetectorRegistry::new();
    registry.register(Box::new(WebpackDetector));
    registry.register(Box::new(UvicornDetector));
    registry
}
