"""
Runtime conflict detection for portrm / ptrm.

Detects multiple installations across package managers (brew, pip, cargo, npm)
and blocks execution with actionable uninstall + reinstall guidance.

Designed for reuse by `portrm doctor` and the main CLI entrypoint.
"""

import os
import platform
import shutil
import subprocess
import sys

# ── ANSI helpers ─────────────────────────────────────────────────────────────

_NO_COLOR = os.environ.get("NO_COLOR") is not None or not hasattr(sys.stderr, "isatty") or not sys.stderr.isatty()


def _red(text: str) -> str:
    return text if _NO_COLOR else f"\033[1;31m{text}\033[0m"


def _green(text: str) -> str:
    return text if _NO_COLOR else f"\033[1;32m{text}\033[0m"


def _yellow(text: str) -> str:
    return text if _NO_COLOR else f"\033[1;33m{text}\033[0m"


def _dim(text: str) -> str:
    return text if _NO_COLOR else f"\033[2m{text}\033[0m"


def _bold(text: str) -> str:
    return text if _NO_COLOR else f"\033[1m{text}\033[0m"


def _cyan(text: str) -> str:
    return text if _NO_COLOR else f"\033[36m{text}\033[0m"


# ── Source detection ─────────────────────────────────────────────────────────

_SOURCE_PATTERNS = [
    # (substrings to match in normalised path, label)
    (("homebrew", "/opt/homebrew", "Cellar", "linuxbrew"), "brew"),
    ((".cargo/bin",), "cargo"),
    (("site-packages", "python", "Python"), "pip"),
    (("node_modules", "/npm/", "/npx/", "AppData/Roaming/npm", "_npx"), "npm"),
]


def _is_python_script(path: str) -> bool:
    """Return True if the file starts with a #! shebang referencing Python."""
    try:
        with open(path, "rb") as f:
            head = f.read(256).decode("utf-8", errors="ignore")
        return head.startswith("#!") and "python" in head.lower()
    except Exception:
        return False


def _pipx_venv_exists() -> bool:
    """Check if the pipx venv for portrm actually exists."""
    home = os.path.expanduser("~")
    return os.path.isdir(os.path.join(home, ".local", "pipx", "venvs", "portrm"))


def _is_local_npm(path: str) -> bool:
    """Detect whether an npm install is local (project-level) vs global."""
    normalised = path.replace("\\", "/")
    # Global npm paths contain /usr/local, /usr/lib, or AppData/Roaming/npm
    if "/usr/local/" in normalised or "/usr/lib/" in normalised:
        return False
    if "appdata/roaming/npm" in normalised.lower():
        return False
    return True


def detect_source(path: str) -> str:
    """Map an absolute binary path to its install ecosystem.

    If the raw path yields 'unknown', resolves symlinks and retries.
    """
    result = _detect_source_inner(path)
    if result == "unknown":
        try:
            resolved = os.path.realpath(path)
            if resolved != path:
                retry = _detect_source_inner(resolved)
                if retry != "unknown":
                    return retry
        except OSError:
            pass
    return result


def _detect_source_inner(path: str) -> str:
    """Core source detection from path patterns."""
    normalised = path.replace("\\", "/")
    lower = normalised.lower()

    # Order matters: more specific patterns first
    if any(p in lower for p in ("homebrew", "/opt/homebrew", "cellar", "linuxbrew")):
        return "brew"
    if ".cargo/bin" in lower:
        return "cargo"
    if any(p in lower for p in ("node_modules", "/npm/", "/npx/", "appdata/roaming/npm", "_npx")):
        if _is_local_npm(path):
            return "npm-local"
        return "npm"
    if any(p in lower for p in ("site-packages", "python")):
        return "pip"
    # ~/.local can be pip, pipx, or install.sh -- inspect the file
    if ".local" in normalised:
        if _is_python_script(path):
            try:
                with open(path) as f:
                    head = f.read(256)
                if "pipx" in head:
                    if _pipx_venv_exists():
                        return "pipx"
                    return "orphan"
            except Exception:
                pass
            return "pip"
        return "script"
    return "unknown"


def _is_npx_context() -> bool:
    """Return True when the current process appears to be running via npx."""
    exe = sys.executable or ""
    argv0 = sys.argv[0] if sys.argv else ""
    env_path = os.environ.get("PATH", "")
    npm_execpath = os.environ.get("npm_execpath", "")
    npm_lifecycle = os.environ.get("npm_lifecycle_event", "")
    npm_config = os.environ.get("npm_config_cache", "")

    hints = (exe, argv0, env_path, npm_execpath, npm_lifecycle, npm_config)
    for h in hints:
        lower = h.lower().replace("\\", "/")
        if "_npx" in lower or "/npx/" in lower or "npx-cli" in lower:
            return True
    return False


# ── Environment detection ────────────────────────────────────────────────────


def detect_available_tools() -> dict:
    """Check which package managers are available on this system."""
    tools = {}
    for name in ("brew", "npm", "pip", "pip3", "cargo"):
        tools[name] = shutil.which(name) is not None
    # Unify pip/pip3
    tools["pip"] = tools.get("pip", False) or tools.get("pip3", False)
    tools.pop("pip3", None)
    return tools


# ── Binary discovery ─────────────────────────────────────────────────────────


def find_all_binaries() -> list:
    """
    Find every `portrm` and `ptrm` binary visible in PATH.

    Returns a deduplicated list of absolute paths.  Binaries from the same
    directory with the same source (e.g. ptrm + portrm from the same pip
    install) are collapsed into a single entry.
    """
    names = ["portrm", "ptrm"]
    found: set = set()

    for name in names:
        found.update(_which_all(name))

    # Resolve symlinks and deduplicate
    resolved: dict = {}
    for p in found:
        try:
            real = os.path.realpath(p)
        except OSError:
            real = p
        resolved[real] = p  # keep original for display

    # Deduplicate by (directory, source) so ptrm + portrm in the same dir
    # from the same package manager count as one entry.
    seen_dirs: set = set()
    result: list = []
    for p in sorted(resolved.values()):
        source = detect_source(p)
        dir_key = (os.path.dirname(p), source)
        if dir_key not in seen_dirs:
            seen_dirs.add(dir_key)
            result.append(p)

    return result


def _which_all(name: str) -> list:
    """Cross-platform `which -a` / `where`."""
    paths: list = []

    if sys.platform == "win32":
        try:
            out = subprocess.run(
                ["where", name],
                capture_output=True,
                text=True,
                timeout=5,
            )
            if out.returncode == 0:
                paths.extend(line.strip() for line in out.stdout.splitlines() if line.strip())
        except Exception:
            pass
    else:
        # which -a on macOS / Linux
        try:
            out = subprocess.run(
                ["which", "-a", name],
                capture_output=True,
                text=True,
                timeout=5,
            )
            if out.returncode == 0:
                paths.extend(line.strip() for line in out.stdout.splitlines() if line.strip())
        except Exception:
            pass

    # Fallback: walk PATH manually (handles edge cases where `which` is missing)
    if not paths:
        ext = ".exe" if sys.platform == "win32" else ""
        for directory in os.environ.get("PATH", "").split(os.pathsep):
            candidate = os.path.join(directory, name + ext)
            if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
                paths.append(candidate)

    return paths


# ── Uninstall / install commands ─────────────────────────────────────────────

_UNINSTALL_CMD = {
    "brew": "brew uninstall portrm",
    "pip": "pip uninstall portrm",
    "pipx": "pipx uninstall portrm",
    "cargo": "cargo uninstall portrm",
    "npm": "npm uninstall -g portrm",
    "npm-local": "npm uninstall portrm",
}


def get_uninstall_commands(sources: list, binaries: list = None) -> list:
    """Return deduplicated uninstall commands for the given source labels."""
    seen: set = set()
    cmds: list = []
    for src in sources:
        cmd = _UNINSTALL_CMD.get(src)
        if cmd and cmd not in seen:
            seen.add(cmd)
            cmds.append(cmd)
    # For "script" and "orphan" installs, suggest rm with the path
    if binaries:
        for path, src in zip(binaries, sources):
            if src in ("script", "orphan"):
                home = os.path.expanduser("~")
                display = path.replace(home, "~") if path.startswith(home) else path
                cmd = f"rm {display}"
                if cmd not in seen:
                    seen.add(cmd)
                    cmds.append(cmd)
    return cmds


def suggest_install_commands() -> tuple:
    """
    Return (recommended: str, alternatives: list[str]) based on OS + available tools.

    Never suggests a tool that isn't installed on the system.
    """
    system = platform.system()
    tools = detect_available_tools()

    if system == "Darwin":
        candidates = [
            ("brew", "brew install abhishekayu/tap/portrm"),
            ("npm", "npm install -g portrm"),
            ("pip", "pip install portrm"),
            ("cargo", "cargo install portrm"),
        ]
    elif system == "Linux":
        # curl is always available; list it first then package managers
        candidates = [
            (None, "curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh | sh"),
            ("npm", "npm install -g portrm"),
            ("pip", "pip install portrm"),
            ("cargo", "cargo install portrm"),
        ]
    elif system == "Windows":
        candidates = [
            ("npm", "npm install -g portrm"),
            ("pip", "pip install portrm"),
            ("cargo", "cargo install portrm"),
        ]
    else:
        candidates = [
            ("cargo", "cargo install portrm"),
        ]

    available = [cmd for tool, cmd in candidates if tool is None or tools.get(tool)]

    if not available:
        # Nothing detected -- show all with a note
        all_cmds = [cmd for _, cmd in candidates]
        return all_cmds[0] if all_cmds else "cargo install portrm", all_cmds[1:]

    return available[0], available[1:]


# ── Main check ───────────────────────────────────────────────────────────────


def run_conflict_check() -> None:
    """
    Detect conflicting portrm installations and block execution if found.

    Safe to call from any entrypoint. Silently returns when:
    - running via npx
    - zero or one binary found
    - detection itself fails
    """
    try:
        if _is_npx_context():
            return

        binaries = find_all_binaries()

        if len(binaries) <= 1:
            return

        sources = [detect_source(b) for b in binaries]
        unique_sources = list(dict.fromkeys(sources))  # deduplicate, preserve order

        # If all binaries resolve to the same source, no real conflict
        if len(set(unique_sources)) <= 1:
            return

        _print_conflict(binaries, sources, unique_sources)
        sys.exit(1)

    except Exception:
        # Never crash the CLI due to conflict detection
        return


def _print_conflict(binaries: list, sources: list, unique_sources: list) -> None:
    """Print the formatted conflict report to stderr."""
    w = sys.stderr.write

    w("\n")
    w(f"  {_red('✖ Multiple portrm installations detected')}\n")
    w("\n")

    # Active binary
    active = shutil.which("ptrm") or shutil.which("portrm")
    if active:
        w(f"  {_dim('Active binary:')} {_cyan(active)}\n")
        w("\n")

    # Found list
    w(f"  {_bold('Found:')}\n")
    w("\n")
    for binary, src in zip(binaries, sources):
        shortened = binary.replace(os.path.expanduser("~"), "~")
        label = {"orphan": "stale pipx - orphaned wrapper", "npm-local": "npm (local)"}.get(src, src)
        w(f"    {_yellow('•')} {shortened}  {_dim('(' + label + ')')}\n")
    w("\n")

    # Uninstall commands
    uninstall_cmds = get_uninstall_commands(unique_sources, binaries)
    if uninstall_cmds:
        w(f"  {_bold('Uninstall duplicates:')}\n")
        w("\n")
        for cmd in uninstall_cmds:
            w(f"    {_dim('$')} {cmd}\n")
        w("\n")

    # Install recommendation
    recommended, alternatives = suggest_install_commands()
    w(f"  {_bold('Install using ONE method:')}\n")
    w("\n")
    w(f"    Recommended:  {_cyan(recommended)}\n")
    if alternatives:
        w(f"    Alternative:  {_dim(alternatives[0])}\n")
        for alt in alternatives[1:]:
            w(f"                  {_dim(alt)}\n")
    w("\n")


# ── Doctor ───────────────────────────────────────────────────────────────────


def run_doctor() -> None:
    """
    Print a full diagnostic report: OS, active binary, all installations,
    available runtimes, and actionable fix suggestions.

    Designed for `portrm doctor` subcommand.
    """
    w = sys.stderr.write

    system = platform.system()
    machine = platform.machine()
    tools = detect_available_tools()
    binaries = find_all_binaries()
    sources = [detect_source(b) for b in binaries]
    unique_sources = list(dict.fromkeys(sources))
    active = shutil.which("ptrm") or shutil.which("portrm")

    w("\n")
    w(f"  {_bold('portrm doctor')}\n")
    w(f"  {_dim('=' * 40)}\n")
    w("\n")

    # OS
    w(f"  {_bold('System:')}\n")
    w(f"    OS:           {_cyan(f'{system} {machine}')}\n")
    w(f"    Python:       {_cyan(platform.python_version())}\n")
    w("\n")

    # Active binary
    w(f"  {_bold('Active binary:')}\n")
    if active:
        src = detect_source(active)
        w(f"    Path:         {_cyan(active)}\n")
        w(f"    Source:       {_cyan(src)}\n")
    else:
        w(f"    {_yellow('No portrm/ptrm binary found in PATH')}\n")
    w("\n")

    # All installations
    w(f"  {_bold('Installations found:')}\n")
    if binaries:
        for binary, src in zip(binaries, sources):
            shortened = binary.replace(os.path.expanduser("~"), "~")
            w(f"    {_yellow('•')} {shortened}  {_dim('(' + src + ')')}\n")
    else:
        w(f"    {_dim('None')}\n")
    w("\n")

    # Conflict status
    has_conflict = len(set(unique_sources)) > 1
    w(f"  {_bold('Conflict status:')}\n")
    if has_conflict:
        w(f"    {_red('✖ CONFLICT')} - multiple sources detected\n")
        uninstall_cmds = get_uninstall_commands(unique_sources, binaries)
        if uninstall_cmds:
            w(f"\n    {_bold('Fix:')} uninstall all, then reinstall with one method:\n")
            for cmd in uninstall_cmds:
                w(f"      {_dim('$')} {cmd}\n")
    elif binaries:
        w(f"    {_green('✔ OK')} - single installation\n")
    else:
        w(f"    {_yellow('⚠ No installation found')}\n")
    w("\n")

    # Available runtimes
    w(f"  {_bold('Available runtimes:')}\n")
    tool_labels = {"brew": "Homebrew", "npm": "npm", "pip": "pip", "cargo": "Cargo"}
    for tool, label in tool_labels.items():
        if tools.get(tool):
            path_str = shutil.which(tool) or ""
            w(f"    {_green('✔')} {label:<10} {_dim(path_str)}\n")
        else:
            w(f"    {_dim('✖')} {label:<10} {_dim('not found')}\n")
    w("\n")

    # Install suggestion
    recommended, alternatives = suggest_install_commands()
    w(f"  {_bold('Recommended install:')}\n")
    w(f"    {_cyan(recommended)}\n")
    if alternatives:
        w(f"\n  {_bold('Alternatives:')}\n")
        for alt in alternatives:
            w(f"    {_dim(alt)}\n")
    w("\n")
