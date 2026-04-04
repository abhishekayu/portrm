# Contributing to ptrm

Thanks for your interest in contributing to ptrm! This guide will help you get started.

## Getting Started

```bash
git clone https://github.com/abhishekayu/portrm.git
cd portrm
cargo build
cargo test
```

Requires Rust 1.75+ (stable).

## Development

```bash
# Run in debug mode
cargo run -- scan
cargo run -- fix 3000

# Run tests
cargo test

# Run clippy
cargo clippy -- -D warnings

# Build release binary
cargo build --release
```

## Project Structure

```
src/
  models/       Data types (PortInfo, ProcessInfo, DevService)
  platform/     OS-specific adapters (macOS, Linux, Windows)
  scanner/      Port scanning with batch process resolution
  classifier/   Service identification (13+ categories)
  engine/       Fix engine with safety checks and strategies
  project/      Project type detection from filesystem
  docker/       Docker container awareness
  grouping/     Port role classification
  doctor/       Auto-diagnosis of dev environment issues
  history/      Persistent action history
  plugin/       Extensible service detector system
  cli/          Commands, output formatting, interactive TUI
```

## How to Contribute

### Report Bugs

Open a [GitHub Issue](https://github.com/abhishekayu/portrm/issues) with:

- Your OS and version
- ptrm version (`ptrm --version`)
- Steps to reproduce
- Expected vs actual behavior

### Suggest Features

Open an issue describing the use case and proposed solution.

### Submit Code

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Add tests if applicable
4. Run `cargo test` and `cargo clippy`
5. Open a pull request

### Good First Contributions

- Add a new service classifier (e.g., Bun, Deno, Elixir)
- Improve an existing doctor diagnostic
- Add tests for edge cases
- Fix typos or improve documentation

## Code Style

- Follow standard Rust conventions (`cargo fmt`)
- Use `anyhow` for error handling in CLI code
- Use `thiserror` for library error types
- Keep functions focused and small
- Prefer returning `Result` over panicking

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
