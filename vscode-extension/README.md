# Portrm - Stop guessing what's running on your machine.

**portrm** is a CLI + VS Code extension to see, fix, and control your dev environment ports in seconds.
No more lsof commands. No more EADDRINUSE errors.
Just a simple, reliable way to keep your dev stack running smoothly.

> See every port, kill rogue processes, manage services, and fix conflicts -- all from the VS Code sidebar.

---

<p align="center">
  <img src="https://raw.githubusercontent.com/abhishekayu/portrm/main/vscode-extension/resources/ptrm.gif" alt="Portrm CLI Demo" width="800" height="400" />
</p>

---

## Features

### Sidebar Port Dashboard

Instantly see every listening port on your machine with process name, PID, and service status -- no terminal needed.

### Project-Aware Service Management

Detects `.ptrm.toml` in your workspace and shows your services (frontend, backend, database) with live status indicators:

- **Running** -- green dot
- **Stopped** -- red dot
- **Conflict** -- yellow warning

### One-Click Actions

| Action                 | What it does                                       |
| ---------------------- | -------------------------------------------------- |
| **Start All**          | Launch all services from `.ptrm.toml`              |
| **Stop All**           | Gracefully stop all project services               |
| **Fix Port Conflicts** | Auto-detect and resolve port collisions            |
| **Switch Profile**     | Switch between dev/staging/production port configs |
| **Reset to Default**   | Restore original port assignments                  |
| **Preflight Check**    | Verify all ports are free before starting          |
| **Doctor**             | Diagnose common port and service issues            |
| **Scan Dev Ports**     | Find all development server ports                  |
| **Group by Role**      | Organize ports by frontend/backend/database        |
| **Interactive TUI**    | Full terminal UI for advanced management           |
| **History**            | View port usage history                            |
| **Registry Check**     | Check ports against known service registry         |
| **CI Check**           | Validate config for CI/CD pipelines                |
| **Update CLI**         | Auto-update ptrm binary from GitHub                |

### Smart Terminal Integration

- Single shared terminal for all actions
- Automatic cleanup when switching between commands
- Interactive TUI support with graceful exit handling

### Auto-Install & Update

Extension automatically detects if `ptrm` CLI is installed. If not, it prompts to download the correct binary for your platform (macOS/Linux/Windows, x64/ARM64) from GitHub releases.

---

## Quick Start

1. **Install the extension** from the VS Code Marketplace
2. **Open a project** -- the Ptrm icon appears in the activity bar
3. **Click the icon** to see all listening ports on your machine
4. _(Optional)_ Run `ptrm init` to create a `.ptrm.toml` for service management

### Example `.ptrm.toml`

```toml
[project]
name = "my-app"

[services.frontend]
port = 3000
run = "npm run dev"
cwd = "."

[services.api]
port = 8080
run = "cargo run"
cwd = "./api"

[profiles.staging]
frontend = { port = 3100 }
api = { port = 8180 }
```

---

## Commands

All commands are available via the Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`) under the **Ptrm** category:

- `Ptrm: Refresh` -- Refresh port and service data
- `Ptrm: Start All Services` -- Start all services from config
- `Ptrm: Stop All Services` -- Stop all running services
- `Ptrm: Kill Process on Port` -- Kill a process by port number
- `Ptrm: Restart Service` -- Restart a specific service
- `Ptrm: View Logs` -- Show logs for a service or port
- `Ptrm: Fix Port Conflicts` -- Auto-resolve port conflicts
- `Ptrm: Inspect Port` -- Deep inspect a specific port
- `Ptrm: Watch Port` -- Monitor a port for changes
- `Ptrm: Doctor` -- Run diagnostic checks
- `Ptrm: Preflight Check` -- Verify ports before launch
- `Ptrm: Switch Profile` -- Switch port profile (staging, production)
- `Ptrm: Reset to Default Profile` -- Restore default port config
- `Ptrm: Initialize Project` -- Create `.ptrm.toml` in workspace
- `Ptrm: Interactive TUI` -- Open full terminal interface
- `Ptrm: Group by Role` -- Group ports by type
- `Ptrm: History` -- View port usage history
- `Ptrm: Scan Dev Ports` -- Scan for development servers
- `Ptrm: Registry Check` -- Check against known services
- `Ptrm: CI Check` -- Validate for CI/CD
- `Ptrm: Update CLI` -- Update ptrm to latest version

---

## Settings

| Setting           | Default | Description                                              |
| ----------------- | ------- | -------------------------------------------------------- |
| `ptrm.binaryPath` | `ptrm`  | Path to the ptrm binary. Leave as `ptrm` to auto-detect. |

---

## Requirements

- **ptrm CLI** -- automatically installed via the extension, or install manually:

```bash
# macOS (Homebrew)
brew install abhishekayu/tap/portrm

# npm (cross-platform)
npm install -g portrm

# From source
cargo install portrm
```

---

## Supported Platforms

| Platform | Architecture                       |
| -------- | ---------------------------------- |
| macOS    | Apple Silicon (ARM64), Intel (x64) |
| Linux    | x64, ARM64                         |
| Windows  | x64                                |

---

## Links

- [GitHub Repository](https://github.com/abhishekayu/portrm)
- [CLI Documentation](https://github.com/abhishekayu/portrm#readme)
- [Report Issues](https://github.com/abhishekayu/portrm/issues)

---

## License

MIT
