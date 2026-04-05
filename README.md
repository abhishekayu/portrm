<p align="center">
<p align="center">
  <img width="220" height="220" alt="Portrm" src="https://cdn.jsdelivr.net/gh/abhishekayu/portrm@main/assets/logo.png" />
</p>
  <h1 align="center">Portrm</h1>
  <p align="center"><strong>Stop guessing what's running on your machine.</strong></p>
  <p align="center">
    portrm is a blazing-fast, cross-platform CLI for developers who need to move fast and stay unblocked.
    Inspect active ports, understand the processes behind them, kill port conflicts, and recover broken dev environments — all from your terminal, in milliseconds.

Built for real-world development workflows where every second counts.</p>

  <p align="center">
    <a href="https://crates.io/crates/portrm"><img src="https://img.shields.io/crates/v/portrm.svg" alt="crates.io"></a>
    <a href="https://www.npmjs.com/package/portrm"><img src="https://img.shields.io/npm/v/portrm.svg" alt="npm"></a>
    <a href="https://pypi.org/project/portrm/"><img src="https://img.shields.io/pypi/v/portrm.svg" alt="PyPI"></a>
    <a href="https://github.com/abhishekayu/portrm/releases"><img src="https://img.shields.io/github/v/release/abhishekayu/portrm" alt="release"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="license"></a>
    <a href="https://marketplace.visualstudio.com/items?itemName=abhishekayu.portrm-cli"><img src="https://img.shields.io/visual-studio-marketplace/v/abhishekayu.portrm-cli?label=VS%20Code" alt="VS Code Marketplace"></a>
  </p>
</p>

<br>

<div align="center">
  <img src="https://github.com/user-attachments/assets/504fe495-e5d0-4acd-84e6-4f8f578c12dd" width="400" height="400" alt="Portrm Demo" />
</div>

<p align="center">
  <b>Detects the process. Checks if it's safe. Kills it gracefully. Tells you how to restart.</b><br>
  Try it now: <code>npx portrm scan</code>
</p>

---

## The Problem Every Developer Hits

You've seen this before:

```
Error: listen EADDRINUSE: address already in use :::3000
```

A crashed dev server. A zombie process. Something unknown squatting on your port.

So you do the ritual:

```bash
lsof -i :3000           # wall of text
kill -9 <pid>            # hope it's not PostgreSQL
# ...was that important?
# how do I restart this thing?
```

This is fragile. It's blind. It tells you nothing about what you just killed.

**portrm replaces this entire workflow with one command.** A single CLI tool for port conflict resolution, process inspection, and dev server recovery.

---

## Instant Port Inspection

```
$ ptrm 3000

  ⚡ Port 3000 in use
  → Next.js (PID 81106)
  → running for 7m 21s
  → memory 42.9 MB
  → detected Next.js (95% confidence)
  → 🛡 safe to kill

  📂 Detected project: Next.js
  → dev command: npm run dev
  → default port: 3000
```

portrm tells you **what** is running, **whether it's safe** to kill, and **how to restart** it.

---

## Port Debugging: Before vs After

| Task                       | Before                                      | After                    |
| -------------------------- | ------------------------------------------- | ------------------------ |
| Port 3000 stuck            | `lsof -i :3000 \| awk ... \| xargs kill -9` | `ptrm fix 3000`          |
| "What's on my ports?"      | `lsof -iTCP -sTCP:LISTEN` (unreadable)      | `ptrm scan`              |
| "Is this safe to kill?"    | Google the process name                     | portrm tells you         |
| "How do I restart?"        | Dig through package.json                    | portrm shows the command |
| Zombie processes           | Hunt them one by one                        | `ptrm doctor -y`         |
| Which port is my frontend? | Check config files                          | `ptrm group`             |
| Start entire dev stack     | Open 5 terminals, run commands manually     | `ptrm up`                |
| Stop everything            | Find and kill each process                  | `ptrm down`              |
| "Is my port free?"         | `lsof -i :3000` and parse output            | `ptrm preflight 3000`    |
| Monitor a flaky server     | Watch logs + manual restart                 | `ptrm watch 3000`        |
| Duplicate port assignments | Manually diff config files                  | `ptrm registry check`    |
| Run all checks in CI       | Write custom scripts                        | `ptrm ci`                |
| Switch dev/staging config  | Edit .env files manually                    | `ptrm use staging`       |
| Restart a crashed service  | Find port, kill, cd, re-run command         | `ptrm restart frontend`  |
| "Are my services running?" | Check each port manually                    | `ptrm status`            |
| Stream logs from a port    | Figure out which container or log file      | `ptrm log 3000`          |

---

## Install

> **Native binary. No Node. No runtime dependencies.** ~1.2MB, runs instantly.

### One-line install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh | sh
```

### Package managers

| Platform            | Command                                                                                       |
| ------------------- | --------------------------------------------------------------------------------------------- |
| **Homebrew**        | `brew install abhishekayu/tap/portrm`                                                         |
| **Cargo**           | `cargo install portrm`                                                                        |
| **npm**             | `npm install -g portrm`                                                                       |
| **pip**             | `pip install portrm`                                                                          |
| **Scoop** (Windows) | `scoop bucket add portrm https://github.com/abhishekayu/scoop-portrm && scoop install portrm` |
| **Debian/Ubuntu**   | Download `.deb` from [releases](https://github.com/abhishekayu/portrm/releases)               |

### Try instantly (no install)

```bash
npx portrm scan
```

### Build from source

```bash
git clone https://github.com/abhishekayu/portrm.git
cd portrm
cargo install --path .
```

> **Supports** macOS (Intel + Apple Silicon), Linux (x86_64 + ARM64), and Windows.

---

## Usage Examples

### Scan all listening ports

```
$ ptrm scan

  ⚡ 5 active ports

  PORT    PROCESS                PID      SERVICE        MEMORY     UPTIME     USER
  ────────────────────────────────────────────────────────────────────────────────
  3000    node                   81106    Next.js        42.9 MB    7m 17s     abhishek
  3898    Code Helper (Plugin)   34290    Python         20.2 MB    3h 12m     abhishek
  5237    Code Helper (Plugin)   34073    Python         20.1 MB    3h 12m     abhishek
  5932    Code Helper (Plugin)   61773    Python         57.5 MB    58m 35s    abhishek
  42050   OneDrive Sync Serv..   36643    Unknown        14.4 MB    3d 1h      abhishek
```

### Fix port conflicts and auto-restart

```
$ ptrm fix 3000 --run "npm run dev"

  ✔ Killed safely  port 3000 is now free
  🚀 Restarting: npm run dev
```

One command. Port cleared, dev server restarted.

### Diagnose dev environment issues

```
$ ptrm doctor

  🩺 2 issues found

  1. Idle process Code Helper (PID 34290) at 0.0% CPU [auto-fixable]
     → Idle Code Helper on port 3898 -- consider killing to free resources
  2. Idle process Code Helper (PID 34073) at 0.0% CPU [auto-fixable]
     → Idle Code Helper on port 5237 -- consider killing to free resources

  ⚙ Run ptrm doctor -y to auto-fix 2 issues
```

### Group ports by service role

```
$ ptrm group --dev

  ⚡ 4 active ports in 2 groups

  ⚙ Frontend (2)
  ────────────────────────────────────────────────────────────────────────────
  3000    node                   81106    Next.js        42.9 MB    7m 21s     abhishek
  3898    Code Helper (Plugin)   34290    Python         20.2 MB    3h 12m     abhishek

  ⚙ Backend (2)
  ────────────────────────────────────────────────────────────────────────────
  5237    Code Helper (Plugin)   34073    Python         20.0 MB    3h 12m     abhishek
  5932    Code Helper (Plugin)   61773    Python         57.5 MB    58m 39s    abhishek
```

### Interactive terminal UI

```
$ ptrm ui
```

Arrow keys to navigate, enter to inspect, `f` to fix. Full interactive TUI for port management, powered by ratatui.

### Define your dev stack with `.ptrm.toml`

```bash
$ ptrm init

  ✔ Created .ptrm.toml
  → Detected Next.js project (port 3050)
  → Port 3050 found in package.json scripts
```

portrm reads your `package.json` scripts and detects hardcoded ports (`--port 3050`, `-p 8080`, etc.). In a monorepo, it scans subdirectories and generates a multi-service config automatically.

Edit the generated config to declare your services:

```toml
[project]
name = "my-app"

[services.frontend]
port = 3000
run = "npm run dev"
cwd = "./frontend"
preflight = true

[services.api]
port = 8080
run = "cargo run"
cwd = "./backend"
preflight = true

[services.worker]
port = 9090
run = "python worker.py"
env = { PYTHON_ENV = "development" }

[profiles.staging]
frontend = { port = 3100 }
api = { port = 8180, env = { RUST_LOG = "info" } }
```

### Start your entire dev stack

```
$ ptrm up

  🚀 Starting 3 services...

  ✔ api started on port 8080
  ✔ frontend started on port 3050
  ✔ worker started on port 9090

  ✔ 3/3 services started.
```

Pre-flight checks run automatically. If a port is busy, portrm tells you. Add `-y` to auto-fix conflicts before starting.

portrm tracks spawned PIDs in `.ptrm.pids`. If a framework binds a different port than configured (e.g., Next.js auto-increments when a port is taken), ptrm detects the actual port and reports it:

```
  ✔ frontend started on port 3001 (configured: 3000)
      ⚠ port 3000 was busy, update .ptrm.toml to match
```

### Stop everything

```
$ ptrm down

  🛑 Stopping 3 services...

  ✔ api stopped (port 8080)
  ✔ frontend stopped (port 3050)
  ✔ worker stopped (port 9090)

  ✔ 3/3 services stopped.
```

`down` uses a 3-tier strategy to find and stop processes: checks the declared port, then the actual port from `.ptrm.pids` (if the process moved), then kills by saved PID directly.

### Pre-flight port check

```
$ ptrm preflight 3000 8080 5432

  🔍 Pre-flight check for 3 ports...

  ✔ Port 3000 is free
  ✔ Port 8080 is free
  ✘ Port 5432 is busy -- postgres (PID 1234, PostgreSQL)

  ⚠ 1 port is already in use. Run ptrm fix <port> to fix.
```

Run without arguments to check all ports from `.ptrm.toml`.

### Watch a port and auto-restart on crash

```
$ ptrm watch 3000

  👀 Watching port 3000 (every 2s, Ctrl-C to stop)

  ✔ Port 3000 is up -- Next.js (PID 81106)
  ✘ Port 3000 went down -- Unknown crash reason (was PID 81106)
  🚀 Auto-restarting: npm run dev
  ✔ Port 3000 recovered (PID 82001, Next.js) -- downtime: 3s
```

If a `.ptrm.toml` defines a `run` command for the watched port, portrm auto-restarts it when it crashes.

### Validate port assignments

```
$ ptrm registry check

  🔍 Checking port registry...

  ✔ No port conflicts found across 3 services.
```

Detects duplicate ports across services and profile overrides in `.ptrm.toml`.

### Run all checks in CI

```
$ ptrm ci

  ▶ Step 1/4: Validate config... ✔
  ▶ Step 2/4: Registry check... ✔
  ▶ Step 3/4: Pre-flight check... ✔
  ▶ Step 4/4: Doctor... ✔

  ✔ All checks passed.
```

Non-interactive runner for CI/CD pipelines. Runs config validation, registry check, preflight, and doctor in sequence. Exits 1 on failure. Supports `--json`.

### Switch profiles

```
$ ptrm use staging

  ✔ Switched to profile: staging
  → frontend port: 3100
  → api port: 8180
```

Switch between named profiles defined in `.ptrm.toml`. The active profile is persisted to `.ptrm.state` and applied automatically to `up`, `down`, `watch`, and `preflight`.

### Restart a single service

```
$ ptrm restart frontend

  🔄 Restarting frontend (port 3000)...
  ✔ Stopped process on port 3000 (PID 81106)
  ✔ Started frontend on port 3000
```

Restarts a named service from `.ptrm.toml`. Stops whatever is on the port (local process or Docker container), then re-runs the configured `run` command. Respects the active profile.

### Check service status

```
$ ptrm status

  📊 test-app status

  SERVICE      PORT    STATUS     PROCESS           PID
  ──────────────────────────────────────────────────────
  frontend     3000    🟢 Running  node              81106
  api          8080    🔴 Stopped
  worker       9090    🟡 Conflict python3           92001
```

Shows the live status of every service in `.ptrm.toml`. Compares the actual process on each port against the expected service to flag conflicts. Supports `--json`.

### Stream logs from a port

```
$ ptrm log 3000
```

Streams live logs from the process on a port. Works with Docker containers (`docker logs -f`) and local processes (detects log files via `lsof`). If the process writes to a TTY, suggests how to redirect output to a file.

---

## CLI Commands Reference

| Command                  | Description                                           | Example                             |
| ------------------------ | ----------------------------------------------------- | ----------------------------------- |
| `ptrm scan`              | List all listening ports with service, memory, uptime | `ptrm scan`                         |
| `ptrm <port>`            | Inspect a single port in detail                       | `ptrm 3000`                         |
| `ptrm fix <ports>`       | Safely kill the process on one or more ports          | `ptrm fix 3000 8080`                |
| `ptrm fix <ports> --run` | Kill and auto-restart a dev server                    | `ptrm fix 3000 --run "npm run dev"` |
| `ptrm fix <ports> -y`    | Skip confirmation prompt                              | `ptrm fix 3000 8080 -y`             |
| `ptrm kill <ports>`      | Direct kill with safety confirmation                  | `ptrm kill 3000 8080`               |
| `ptrm group`             | Ports organized by role (frontend/backend/db/infra)   | `ptrm group --dev`                  |
| `ptrm doctor`            | Find stale servers, idle processes, conflicts         | `ptrm doctor`                       |
| `ptrm doctor -y`         | Auto-fix all safe issues                              | `ptrm doctor -y`                    |
| `ptrm history`           | View past actions with timestamps                     | `ptrm history`                      |
| `ptrm history --stats`   | Kill stats: success rate, top ports, top processes    | `ptrm history --stats`              |
| `ptrm project`           | Detect project type, suggest dev commands             | `ptrm project`                      |
| `ptrm ui`                | Interactive TUI with keyboard navigation              | `ptrm ui`                           |
| `ptrm init`              | Create a `.ptrm.toml` (auto-detects ports)            | `ptrm init`                         |
| `ptrm up`                | Start all services from `.ptrm.toml`                  | `ptrm up`                           |
| `ptrm up -y`             | Start services, auto-fix port conflicts first         | `ptrm up -y`                        |
| `ptrm down`              | Stop all services from `.ptrm.toml`                   | `ptrm down`                         |
| `ptrm preflight`         | Check if ports are free before starting               | `ptrm preflight 3000 8080`          |
| `ptrm watch <port>`      | Monitor a port, alert on crash, auto-restart          | `ptrm watch 3000`                   |
| `ptrm registry check`    | Validate port assignments for conflicts               | `ptrm registry check`               |
| `ptrm ci`                | Run all checks non-interactively (CI/CD mode)         | `ptrm ci --json`                    |
| `ptrm use <profile>`     | Switch to a named profile from `.ptrm.toml`           | `ptrm use staging`                  |
| `ptrm restart <service>` | Restart a named service from `.ptrm.toml`             | `ptrm restart frontend`             |
| `ptrm status`            | Show live status of all services from config          | `ptrm status`                       |
| `ptrm log <port>`        | Stream live logs from a port (Docker or local)        | `ptrm log 3000`                     |

> All commands support `--json` for scripting and CI pipelines.

---

## Why portrm Over kill-port, fkill, or lsof

**It's not `kill -9` with extra steps.**

portrm is a process classification engine built for developer productivity:

- **Identifies services** -- Next.js, Vite, Django, Flask, Express, PostgreSQL, Redis, Docker, and 13+ categories with confidence scores
- **Safety system** -- blocks system-critical processes (PID 1, sshd, launchd), warns about databases (data loss risk), approves dev servers
- **Graceful shutdown** -- SIGTERM first, waits for clean exit, escalates to SIGKILL only if needed
- **Project-aware** -- reads package.json, Cargo.toml, pyproject.toml to suggest the right restart command
- **Docker-aware** -- detects container ports vs host ports
- **History** -- every action logged to `~/.ptrm/history.json` with timestamps and outcomes

### How portrm works under the hood

1. **Scan** -- queries the OS for all listening ports, resolves PIDs via sysinfo
2. **Classify** -- identifies the service type (Next.js, PostgreSQL, Docker, etc.)
3. **Assess** -- safety check: SAFE / WARN / BLOCK
4. **Strategy** -- picks the right approach: Graceful, Escalating, or Force
5. **Execute** -- sends signals, waits for exit, verifies the port is free
6. **Recover** -- detects the project, suggests restart, or auto-restarts with `--run`

### Safety tiers

| Verdict     | Examples                                                    | Behavior                                        |
| ----------- | ----------------------------------------------------------- | ----------------------------------------------- |
| **BLOCKED** | PID 0/1, launchd, systemd, sshd, kernel_task                | Refuses to kill                                 |
| **WARNING** | PostgreSQL, MySQL, Redis, Docker, Nginx                     | Warns about consequences, asks for confirmation |
| **SAFE**    | Next.js, Vite, Create React App, Django dev, Flask, Node.js | Kills gracefully                                |

---

## Developer Productivity Workflows

### "Port 3000 is already in use" after a crash

Your Next.js server crashed. The port is stuck. You just want to get back to coding.

```bash
ptrm fix 3000 -y --run "npm run dev"
```

Port cleared, server restarted. One line.

### Make `npm run dev` crash-proof

Add ptrm to your scripts so port conflicts resolve themselves:

```json
{
  "scripts": {
    "dev": "ptrm fix 3000 -y --run 'next dev'",
    "dev:api": "ptrm fix 8080 -y --run 'node server.js'",
    "dev:clean": "ptrm doctor -y && npm run dev"
  }
}
```

Now `npm run dev` works every time, even if something is already on port 3000.

### Morning dev environment reset

You open your laptop. Stale servers from yesterday are hogging ports and memory.

```bash
ptrm doctor -y
```

Finds zombie processes, idle servers, and cleans them up automatically.

### "What is using port 8080?"

Something is squatting on your API port but you have no idea what.

```bash
ptrm 8080
```

Shows the process name, PID, service type, memory, uptime, project directory, and whether it's safe to kill.

### Full-stack dev with `.ptrm.toml`

Frontend on 3000, API on 8080, worker on 9090. Define them once, manage them forever:

```bash
# Initialize config
ptrm init

# Start everything (runs preflight checks automatically)
ptrm up

# Stop everything at end of day
ptrm down
```

No more opening 5 terminals and running commands manually.

### Monitor a flaky dev server

Your dev server keeps crashing. Let ptrm watch it and auto-restart:

```bash
ptrm watch 3000
```

If a `.ptrm.toml` defines a `run` command for port 3000, portrm auto-restarts it when it goes down.

### Pre-flight check before deployment scripts

```bash
# Check all ports from .ptrm.toml are free
ptrm preflight

# Check specific ports
ptrm preflight 3000 8080 5432
```

### Shell aliases for daily use

```bash
# ~/.zshrc or ~/.bashrc
alias pf='ptrm fix'
alias pfs='ptrm scan'
alias pfd='ptrm doctor -y'
alias pu='ptrm up'
alias pd='ptrm down'
alias dev3='ptrm fix 3000 -y --run "npm run dev"'
alias dev8='ptrm fix 8080 -y --run "node server.js"'
```

### Fix multiple ports at once

Clearing out a full dev environment before starting fresh:

```bash
# Fix multiple ports in one command
ptrm fix 3000 8080 5173 -y

# Or with .ptrm.toml
ptrm down && ptrm up
```

### CI / pre-commit: ensure clean ports

```bash
# Run all checks in one command (exits 1 on failure)
ptrm ci

# Or with JSON output for CI parsing
ptrm ci --json
```

### Validate port assignments before deploying

```bash
# Check for duplicate ports across services and profiles
ptrm registry check
```

### Switch between dev and staging

```bash
# Define profiles in .ptrm.toml, then switch:
ptrm use staging
ptrm up

# Switch back to default
ptrm use default
ptrm up
```

### Pipe to scripts with JSON output

```bash
# Get all listening ports as JSON
ptrm scan --json | jq '.[] | select(.service == "Next.js")'

# Count active dev servers
ptrm scan --json | jq '[.[] | select(.service != "Unknown")] | length'
```

---

## Comparison: portrm vs kill-port vs fkill

|                        | kill-port | fkill     | **portrm**                          |
| ---------------------- | --------- | --------- | ----------------------------------- |
| Service identification | No        | Name only | Full (service, memory, uptime, CWD) |
| Safety checks          | No        | No        | Yes (safe / warn / block)           |
| Graceful shutdown      | No        | No        | Yes (SIGTERM, then escalate)        |
| Restart hints          | No        | No        | Yes (project-aware)                 |
| Auto-restart           | No        | No        | Yes (`--run`)                       |
| Docker awareness       | No        | No        | Yes                                 |
| Auto-diagnosis         | No        | No        | Yes (`doctor`)                      |
| Port grouping          | No        | No        | Yes (by role)                       |
| Action history         | No        | No        | Yes                                 |
| Interactive TUI        | No        | Yes       | Yes                                 |
| Project config file    | No        | No        | Yes (`.ptrm.toml`)                  |
| Dev stack up/down      | No        | No        | Yes (`up` / `down`)                 |
| Service restart        | No        | No        | Yes (`restart <service>`)           |
| Service status         | No        | No        | Yes (`status`)                      |
| Log streaming          | No        | No        | Yes (`log <port>`)                  |
| Port monitoring        | No        | No        | Yes (`watch`)                       |
| Pre-flight checks      | No        | No        | Yes (`preflight`)                   |
| Crash detection        | No        | No        | Yes (signal, OOM, zombie)           |
| Port registry          | No        | No        | Yes (conflict detection)            |
| CI/CD mode             | No        | No        | Yes (`ci` command)                  |
| Profiles               | No        | No        | Yes (`use` for dev/staging/prod)    |
| Platform               | Node.js   | Node.js   | Native binary                       |
| Size                   | ~50MB     | ~50MB     | ~1.2MB                              |

---

## VS Code Extension

Manage ports and services directly from the VS Code sidebar -- no terminal needed.

[![Install from VS Code Marketplace](https://img.shields.io/badge/VS%20Code-Install%20Extension-007ACC?logo=visual-studio-code)](https://marketplace.visualstudio.com/items?itemName=abhishekayu.portrm-cli)

**Features:**

- Sidebar dashboard showing all listening ports with process info
- Project-aware service management (reads `.ptrm.toml`)
- One-click Start All, Stop All, Fix, Doctor, Preflight, and 15+ actions
- Switch between dev/staging/production profiles
- Auto-install and update the ptrm CLI binary
- Smart single-terminal integration with interactive TUI support

**Install:**

1. Open VS Code
2. Go to Extensions (`Cmd+Shift+X`)
3. Search for "Portrm"
4. Click Install

Or install from the command line:

```bash
code --install-extension abhishekayu.portrm-cli
```

[View on Marketplace](https://marketplace.visualstudio.com/items?itemName=abhishekayu.portrm-cli)

---

## Architecture & Performance

```
src/
  scanner/      Batch port scanning with sysinfo process resolution
  classifier/   13+ service classifiers with confidence scoring
  engine/       Safety checks, strategy selection, graceful kill with retry
  platform/     macOS (lsof + libc) / Linux (/proc/net/tcp) / Windows (netstat)
  project/      Filesystem project detection (package.json, Cargo.toml, etc.)
  docker/       Container awareness via docker ps
  grouping/     Port role classification (frontend/backend/database/infra)
  doctor/       Stale servers, idle processes, crowded ports
  history/      Action log persisted to ~/.ptrm/history.json
  config/       .ptrm.toml project config loader (monorepo, port detection)
  watch/        Continuous port monitoring with crash detection
  stack/        Dev stack orchestration (up/down with PID tracking)
  preflight/    Pre-flight port availability checks
  crash/        Crash reason detection (signal, OOM, zombie)
  restart/      Single-service restart (stop + start from config)
  status/       Live service status dashboard (running/stopped/conflict)
  log/          Log streaming for Docker containers and local processes
  registry/     Port conflict detection across services and profiles
  ci/           Non-interactive CI/CD runner (config + registry + preflight + doctor)
  plugin/       Extensible ServiceDetector trait for custom detectors
  cli/          Clap v4 + colored output + ratatui TUI
```

Built in Rust for speed and reliability. Ships as a single ~1.2MB static binary with zero runtime dependencies. No Node.js, no Python -- just a fast native CLI tool for managing ports and debugging dev environments.

---

## Contributing

```bash
git clone https://github.com/abhishekayu/portrm.git
cd portrm
cargo build
cargo test
```

- Report bugs or request features via [Issues](https://github.com/abhishekayu/portrm/issues)
- Add new service classifiers
- Improve platform support
- Write new doctor diagnostics

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

## License

[MIT](LICENSE) -- free for personal and commercial use.

---

<p align="center">
  <strong>portrm</strong> -- an open-source CLI tool for port management, process debugging, and developer environment recovery.<br>
  Built for developers who are tired of <code>lsof</code> + <code>kill -9</code>.<br>
  Define your dev stack in <code>.ptrm.toml</code>, start with <code>ptrm up</code>, stop with <code>ptrm down</code>.<br>
  Restart a service with <code>ptrm restart frontend</code>, check status with <code>ptrm status</code>.<br>
  Switch profiles with <code>ptrm use staging</code>, validate with <code>ptrm ci</code>.<br>
  Works with Next.js, Vite, Django, Flask, Express, Docker, PostgreSQL, Redis, and more.
</p>
