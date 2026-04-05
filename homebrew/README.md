# Homebrew Tap for Portrm

Stop guessing what's running on your machine. A fast, cross-platform CLI to inspect ports, understand processes, and recover broken dev environments - built for real-world development workflows.

## Install

```bash
brew tap abhishekayu/tap
brew install portrm
```

Or in one line:

```bash
brew tap abhishekayu/tap && brew install portrm
```

## Upgrade

```bash
brew update && brew upgrade portrm
```

## Usage

```bash
ptrm scan                          # see all listening ports
ptrm fix 3000                      # safely kill process on port 3000
ptrm fix 3000 --run "npm run dev"  # kill and auto-restart
ptrm doctor -y                     # auto-fix stale servers
ptrm ui                            # interactive terminal UI
ptrm init                          # create .ptrm.toml config
ptrm up                            # start services from config
ptrm down                          # stop all services
ptrm restart frontend              # restart a single service
ptrm status                        # check status of all services
ptrm log 3000                      # stream logs from a port
ptrm watch 3000                    # monitor a port in real time
ptrm preflight                     # check ports before starting
```

## CLI Commands Reference

| Command                  | Description                                           | Example                             |
| ------------------------ | ----------------------------------------------------- | ----------------------------------- |
| `ptrm scan`              | List all listening ports with service, memory, uptime | `ptrm scan`                         |
| `ptrm <port>`            | Inspect a single port in detail                       | `ptrm 3000`                         |
| `ptrm fix <port>`        | Safely kill the process on a port                     | `ptrm fix 3000`                     |
| `ptrm fix <port> --run`  | Kill and auto-restart a dev server                    | `ptrm fix 3000 --run "npm run dev"` |
| `ptrm fix <port> -y`     | Skip confirmation prompt                              | `ptrm fix 8080 -y`                  |
| `ptrm kill <port>`       | Direct kill with safety confirmation                  | `ptrm kill 3000`                    |
| `ptrm group`             | Ports organized by role (frontend/backend/db/infra)   | `ptrm group --dev`                  |
| `ptrm doctor`            | Find stale servers, idle processes, conflicts         | `ptrm doctor`                       |
| `ptrm doctor -y`         | Auto-fix all safe issues                              | `ptrm doctor -y`                    |
| `ptrm history`           | View past actions with timestamps                     | `ptrm history`                      |
| `ptrm history --stats`   | Kill stats: success rate, top ports, top processes    | `ptrm history --stats`              |
| `ptrm project`           | Detect project type, suggest dev commands             | `ptrm project`                      |
| `ptrm ui`                | Interactive TUI with keyboard navigation              | `ptrm ui`                           |
| `ptrm init`              | Create a `.ptrm.toml` config in current directory     | `ptrm init`                         |
| `ptrm up`                | Start all services from `.ptrm.toml`                  | `ptrm up`                           |
| `ptrm up -y`             | Start services, auto-fix port conflicts first         | `ptrm up -y`                        |
| `ptrm down`              | Stop all services from `.ptrm.toml`                   | `ptrm down`                         |
| `ptrm preflight`         | Check if ports are free before starting               | `ptrm preflight 3000 8080`          |
| `ptrm watch <port>`      | Monitor a port, alert on crash, auto-restart          | `ptrm watch 3000`                   |
| `ptrm restart <service>` | Restart a named service from `.ptrm.toml`             | `ptrm restart frontend`             |
| `ptrm status`            | Show live status of all services from config          | `ptrm status`                       |
| `ptrm log <port>`        | Stream live logs from a port (Docker or local)        | `ptrm log 3000`                     |

> All commands support `--json` for scripting and CI pipelines.

## VS Code Extension

Manage ports and services directly from the VS Code sidebar.

[![Install from VS Code Marketplace](https://img.shields.io/badge/VS%20Code-Install%20Extension-007ACC?logo=visual-studio-code)](https://marketplace.visualstudio.com/items?itemName=abhishekayu.portrm-cli)

```bash
code --install-extension abhishekayu.portrm-cli
```

[View on Marketplace](https://marketplace.visualstudio.com/items?itemName=abhishekayu.portrm-cli)

## What is ptrm?

portrm is a developer productivity CLI tool for port management, process
debugging, and dev environment recovery. It identifies what's running on your ports,
checks if it's safe to kill, shuts it down gracefully, and tells you how to
restart.

Built in Rust for maximum speed (~1.2 MB binary, <50ms startup, zero runtime dependencies).
Works with Next.js, Vite, Django, Flask, Express, Docker, PostgreSQL, Redis, MongoDB, and
22+ service types.

- [Homepage](https://portrm.dev) - official website
- [GitHub](https://github.com/abhishekayu/portrm) - source code, docs, issues
- [crates.io](https://crates.io/crates/portrm) - Rust package
- [npm](https://www.npmjs.com/package/portrm) - Node.js package
- [PyPI](https://pypi.org/project/portrm/) - Python package (`pip install portrm`)

## License

[MIT](https://github.com/abhishekayu/portrm/blob/main/LICENSE)
