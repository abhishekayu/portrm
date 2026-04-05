<p align="center">
  <img width="220" height="220" alt="Portrm" src="https://cdn.jsdelivr.net/gh/abhishekayu/portrm@main/assets/logo.png" />
</p>
<h1 align="center">portrm</h1>
<p align="center"><strong>Stop guessing what's running on your machine.</strong></p>
<p align="center">
portrm is a blazing-fast, cross-platform port management CLI built in <strong>Rust</strong>.<br>
Inspect active ports, understand the processes behind them, kill port conflicts, and recover broken dev environments &mdash; all from your terminal, in milliseconds.
</p>
<p align="center">
  <a href="https://pypi.org/project/portrm/"><img src="https://img.shields.io/pypi/v/portrm.svg" alt="PyPI"></a>
  <a href="https://crates.io/crates/portrm"><img src="https://img.shields.io/crates/v/portrm.svg" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/portrm"><img src="https://img.shields.io/npm/v/portrm.svg" alt="npm"></a>
  <a href="https://github.com/abhishekayu/portrm/releases"><img src="https://img.shields.io/github/v/release/abhishekayu/portrm" alt="release"></a>
  <a href="https://github.com/abhishekayu/portrm/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="license"></a>
</p>

<p align="center">
  <a href="https://portrm.dev">Homepage</a> &bull;
  <a href="https://github.com/abhishekayu/portrm">GitHub</a> &bull;
  <a href="https://marketplace.visualstudio.com/items?itemName=abhishekayu.portrm-cli">VS Code Extension</a>
</p>

---

## Why portrm?

```
Error: listen EADDRINUSE: address already in use :::3000
```

You know the drill: `lsof -i :3000`, parse the output, `kill -9 <pid>`, hope it wasn't PostgreSQL. portrm replaces that entire workflow with **one command**.

```
$ ptrm fix 3000

  ⚡ Port 3000 in use
  → Next.js (PID 81106) running for 7m 21s
  → 🛡 safe to kill

  • Sending SIGTERM to PID 81106
  • Verified: PID 81106 has exited

  ✔ Port 3000 is now free
  Restart: npm run dev
```

Detects the process. Checks if it's safe. Kills it gracefully. Tells you how to restart.

---

## Install

```bash
pip install portrm
```

The native Rust binary (~1.2 MB) is downloaded automatically on first run. No runtime dependencies.

### Other installation methods

| Platform            | Command                                                                                       |
| ------------------- | --------------------------------------------------------------------------------------------- |
| **curl** (fastest)  | `curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh \| sh`       |
| **Homebrew**        | `brew install abhishekayu/tap/portrm`                                                         |
| **Cargo**           | `cargo install portrm`                                                                        |
| **npm**             | `npm install -g portrm`                                                                       |
| **Scoop** (Windows) | `scoop bucket add portrm https://github.com/abhishekayu/scoop-portrm && scoop install portrm` |

> **Supports** macOS (Intel + Apple Silicon), Linux (x86_64 + ARM64), and Windows.

---

## Quick Start

```bash
# Scan all listening ports
ptrm scan

# Inspect a specific port
ptrm 3000

# Fix a stuck port (kill process + suggest restart)
ptrm fix 3000

# Fix and auto-restart
ptrm fix 3000 --run "npm run dev"

# Interactive terminal UI
ptrm ui
```

---

## Commands

### Port Inspection & Management

| Command                 | Description                                           | Example                             |
| ----------------------- | ----------------------------------------------------- | ----------------------------------- |
| `ptrm scan`             | List all listening ports with service, memory, uptime | `ptrm scan`                         |
| `ptrm <port>`           | Inspect a single port in detail                       | `ptrm 3000`                         |
| `ptrm fix <ports>`      | Safely kill the process on one or more ports          | `ptrm fix 3000 8080`                |
| `ptrm fix <port> --run` | Kill and auto-restart a dev server                    | `ptrm fix 3000 --run "npm run dev"` |
| `ptrm kill <ports>`     | Direct kill with safety confirmation                  | `ptrm kill 3000 8080`               |
| `ptrm group`            | Ports organized by role (frontend/backend/db/infra)   | `ptrm group --dev`                  |
| `ptrm doctor`           | Find stale servers, idle processes, conflicts         | `ptrm doctor`                       |
| `ptrm doctor -y`        | Auto-fix all safe issues                              | `ptrm doctor -y`                    |
| `ptrm history`          | View past actions with timestamps                     | `ptrm history`                      |
| `ptrm project`          | Detect project type, suggest dev commands             | `ptrm project`                      |
| `ptrm ui`               | Interactive TUI with keyboard navigation              | `ptrm ui`                           |
| `ptrm log <port>`       | Stream live logs from a port (Docker or local)        | `ptrm log 3000`                     |

### Dev Stack Orchestration (`.ptrm.toml`)

| Command                  | Description                                          | Example                 |
| ------------------------ | ---------------------------------------------------- | ----------------------- |
| `ptrm init`              | Create `.ptrm.toml` (auto-detects ports from config) | `ptrm init`             |
| `ptrm up`                | Start all services from `.ptrm.toml`                 | `ptrm up`               |
| `ptrm down`              | Stop all services from `.ptrm.toml`                  | `ptrm down`             |
| `ptrm restart <service>` | Restart a named service                              | `ptrm restart frontend` |
| `ptrm status`            | Show live status of all services                     | `ptrm status`           |
| `ptrm watch <port>`      | Monitor a port, auto-restart on crash                | `ptrm watch 3000`       |
| `ptrm preflight`         | Check if ports are free before starting              | `ptrm preflight`        |
| `ptrm registry check`    | Validate port assignments for conflicts              | `ptrm registry check`   |
| `ptrm use <profile>`     | Switch between dev/staging/prod profiles             | `ptrm use staging`      |
| `ptrm ci`                | Run all checks non-interactively (CI/CD mode)        | `ptrm ci --json`        |

> All commands support `--json` for scripting and CI pipelines.

---

## Usage Examples

### Scan all listening ports

```
$ ptrm scan

  ⚡ 5 active ports

  PORT    PROCESS                PID      SERVICE        MEMORY     UPTIME     USER
  ────────────────────────────────────────────────────────────────────────────────
  3000    node                   81106    Next.js        42.9 MB    7m 17s     user
  5432    postgres               1234     PostgreSQL     28.1 MB    2d 5h      user
  8080    java                   5678     Java           512.0 MB   1h 12m     user
  27017   mongod                 9012     MongoDB        64.3 MB    3d 8h      user
  6379    redis-server           3456     Redis          8.2 MB     3d 8h      user
```

### Diagnose dev environment issues

```
$ ptrm doctor

  🩺 2 issues found

  1. Idle process node (PID 34290) at 0.0% CPU [auto-fixable]
  2. Stale server on port 8080, running for 3d [auto-fixable]

  ⚙ Run ptrm doctor -y to auto-fix 2 issues
```

### Define and manage your dev stack

```toml
# .ptrm.toml
[project]
name = "my-app"

[services.frontend]
port = 3000
run = "npm run dev"

[services.api]
port = 8080
run = "cargo run"

[services.db]
port = 5432
run = "docker compose up postgres"

[profiles.staging]
frontend = { port = 3100 }
api = { port = 8180 }
```

```bash
ptrm up        # Start everything with pre-flight checks
ptrm status    # See what's running
ptrm down      # Stop everything
```

---

## Built in Rust

portrm is a native Rust binary. No Python runtime, no Node.js, no JVM.

- **~1.2 MB** binary size
- **< 50ms** startup time
- **Zero runtime dependencies**
- Cross-platform: macOS, Linux, Windows

This pip package is a thin wrapper that downloads the pre-built native binary on first run and delegates all commands to it. You get the full speed of Rust with the convenience of `pip install`.

---

## Smart Process Classification

portrm identifies 22+ service types using a 3-layer detection engine:

1. **Binary name matching** &mdash; postgres, nginx, mongod, redis-server, etc.
2. **Command-line pattern analysis** &mdash; `next dev`, `spring-boot`, `flask run`, etc.
3. **Well-known port fallback** &mdash; 5432 = PostgreSQL, 27017 = MongoDB, 6379 = Redis, etc.

### Safety system

| Verdict     | Examples                              | Behavior                                        |
| ----------- | ------------------------------------- | ----------------------------------------------- |
| **BLOCKED** | PID 0/1, launchd, systemd, sshd       | Refuses to kill                                 |
| **WARNING** | PostgreSQL, MySQL, Redis, Docker      | Warns about data loss, asks for confirmation    |
| **SAFE**    | Next.js, Vite, Django, Flask, Node.js | Kills gracefully (SIGTERM first, then escalate) |

---

## Comparison

|                        | kill-port | fkill     | **portrm**                             |
| ---------------------- | --------- | --------- | -------------------------------------- |
| Service identification | No        | Name only | Full (22+ services, confidence scores) |
| Safety checks          | No        | No        | Yes (safe / warn / block)              |
| Graceful shutdown      | No        | No        | Yes (SIGTERM, then escalate)           |
| Auto-restart           | No        | No        | Yes (`--run`)                          |
| Docker awareness       | No        | No        | Yes                                    |
| Dev stack management   | No        | No        | Yes (`up` / `down` / `status`)         |
| Port monitoring        | No        | No        | Yes (`watch` with auto-restart)        |
| CI/CD mode             | No        | No        | Yes (`ci --json`)                      |
| Interactive TUI        | No        | Yes       | Yes                                    |
| Platform               | Node.js   | Node.js   | Native Rust binary                     |
| Size                   | ~50 MB    | ~50 MB    | ~1.2 MB                                |

---

## VS Code Extension

Manage ports directly from the VS Code sidebar with rich tooltips, service-colored icons, and one-click actions.

[![Install from VS Code Marketplace](https://img.shields.io/badge/VS%20Code-Install%20Extension-007ACC?logo=visual-studio-code)](https://marketplace.visualstudio.com/items?itemName=abhishekayu.portrm-cli)

---

## Links

- **Homepage**: [portrm.dev](https://portrm.dev)
- **GitHub**: [github.com/abhishekayu/portrm](https://github.com/abhishekayu/portrm)
- **npm**: [npmjs.com/package/portrm](https://www.npmjs.com/package/portrm)
- **Crates.io**: [crates.io/crates/portrm](https://crates.io/crates/portrm)
- **VS Code**: [marketplace.visualstudio.com](https://marketplace.visualstudio.com/items?itemName=abhishekayu.portrm-cli)
- **PyPI**: [pypi.org/project/portrm](https://pypi.org/project/portrm/)

## License

MIT
