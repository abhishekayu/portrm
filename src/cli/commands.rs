use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ptrm",
    version,
    about = "Developer environment recovery tool -- detect, explain, and fix port conflicts",
    long_about = "portrm intelligently detects port conflicts, identifies what's using them \
                  (Next.js, Vite, Docker, etc.), and safely resolves issues without blindly killing processes."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Quick-check a port (shorthand for `ptrm info <port>`).
    #[arg(value_name = "PORT", global = false)]
    pub port: Option<u16>,

    /// Output as JSON instead of a table.
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan ports and show what's listening.
    Scan {
        /// Specific port(s) to check. Omit to scan all.
        #[arg(value_name = "PORT")]
        ports: Vec<u16>,

        /// Show only common dev ports (3000-3999, 4000-4999, 5000-5999, 8000-8999).
        #[arg(long)]
        dev: bool,
    },

    /// Kill the process on a port (with safety checks).
    Kill {
        /// The port(s) to free.
        #[arg(required = true, num_args = 1..)]
        ports: Vec<u16>,

        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,

        /// Force kill (SIGKILL) immediately without graceful shutdown.
        #[arg(short, long)]
        force: bool,
    },

    /// Intelligent fix: detect, explain, and resolve a port conflict.
    /// When no ports are given, auto-detects all occupied ports and fixes them.
    Fix {
        /// The port(s) to fix.  Omit to auto-detect all conflicts.
        #[arg(num_args = 0..)]
        ports: Vec<u16>,

        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,

        /// Force kill (SIGKILL) instead of the recommended strategy.
        #[arg(short, long)]
        force: bool,

        /// Command to run after freeing the port (auto-restart).
        #[arg(long, value_name = "CMD")]
        run: Option<String>,
    },

    /// Show detailed info about what's running on a port.
    Info {
        /// The port to inspect.
        #[arg(required = true)]
        port: u16,
    },

    /// Stream logs from the process or container on a port.
    Log {
        /// The port whose logs to stream.
        #[arg(required = true)]
        port: u16,
    },

    /// Restart a service from .ptrm.toml.
    Restart {
        /// Service name to restart.
        #[arg(required = true)]
        service: String,
    },

    /// Show live status of all services from .ptrm.toml.
    Status,

    /// Interactive TUI: browse ports, inspect, and act.
    #[command(alias = "ui")]
    Interactive,

    /// Scan ports grouped by role (frontend, backend, database, etc.).
    Group {
        /// Show only common dev ports.
        #[arg(long)]
        dev: bool,
    },

    /// Diagnose and fix common port issues automatically.
    Doctor {
        /// Auto-fix all safe issues without prompting.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Show action history.
    History {
        /// Clear all history.
        #[arg(long)]
        clear: bool,

        /// Show summary statistics.
        #[arg(long)]
        stats: bool,
    },

    /// Detect the project type in the current directory.
    Project,

    /// Watch a port and alert when it goes down.
    Watch {
        /// The port to monitor.
        #[arg(required = true)]
        port: u16,

        /// Poll interval in seconds (default: 2).
        #[arg(short, long, default_value = "2")]
        interval: u64,
    },

    /// Start all services from .ptrm.toml.
    Up {
        /// Auto-fix port conflicts before starting.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Stop all services from .ptrm.toml.
    Down,

    /// Check if ports are free before starting.
    Preflight {
        /// Ports to check. If omitted, uses .ptrm.toml.
        #[arg(value_name = "PORT")]
        ports: Vec<u16>,
    },

    /// Create a .ptrm.toml in the current directory.
    Init,

    /// Check port registry for conflicts in .ptrm.toml.
    Registry {
        #[command(subcommand)]
        action: RegistryAction,
    },

    /// Run all checks non-interactively (CI/CD mode).
    Ci,

    /// Switch to a named profile from .ptrm.toml.
    Use {
        /// Profile name to activate.
        #[arg(required = true)]
        profile: String,
    },

    /// Generate shell completions (bash, zsh, fish, powershell).
    Completions {
        /// Shell to generate completions for.
        #[arg(value_parser = ["bash", "zsh", "fish", "powershell"])]
        shell: String,
    },

    /// Print active port numbers (used internally by shell completions).
    #[command(name = "_complete-ports", hide = true)]
    CompletePorts,
}

#[derive(Subcommand)]
pub enum RegistryAction {
    /// Validate port assignments for conflicts.
    Check,
}
