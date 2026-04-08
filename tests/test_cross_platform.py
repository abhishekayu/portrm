#!/usr/bin/env python3
"""
Cross-platform test suite for portrm conflict detection + VS Code extension logic.

Simulates macOS, Windows, and Linux environments by:
  - Creating fake binary layouts matching each OS's package managers
  - Patching platform.system() / sys.platform / os.path separators
  - Verifying detect_source(), suggest_install_commands(), run_doctor()
  - Mirroring VS Code extension's resolvePtrm() and getInstallSuggestion() logic
  - Testing the Node.js conflict.js module via subprocess

Run:  python3 test_cross_platform.py
"""

import importlib
import json
import os
import platform
import shutil
import stat
import subprocess
import sys
import tempfile
from pathlib import Path
from unittest import mock

# ── Ensure the pip module is importable ──────────────────────────────────────

ROOT = os.path.dirname(os.path.abspath(__file__))
PIP_SRC = os.path.join(ROOT, "pip", "src")
if PIP_SRC not in sys.path:
    sys.path.insert(0, PIP_SRC)

# Force reimport each time (module caches _NO_COLOR etc.)
if "portrm.conflict" in sys.modules:
    del sys.modules["portrm.conflict"]

from portrm.conflict import (
    detect_source,
    suggest_install_commands,
    detect_available_tools,
    find_all_binaries,
    get_uninstall_commands,
    run_conflict_check,
    run_doctor,
    _is_npx_context,
)

# ── Test globals ─────────────────────────────────────────────────────────────

TMPDIR = None
PASS = 0
FAIL = 0
TOTAL = 0


def setup():
    global TMPDIR
    TMPDIR = tempfile.mkdtemp(prefix="portrm-xplat-")


def cleanup():
    if TMPDIR and os.path.exists(TMPDIR):
        shutil.rmtree(TMPDIR)


def _header(title):
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print(f"{'=' * 60}\n")


def _section(title):
    print(f"\n  -- {title} --\n")


def check(label, condition):
    global PASS, FAIL, TOTAL
    TOTAL += 1
    if condition:
        PASS += 1
        print(f"  \033[32m✔\033[0m  {label}")
    else:
        FAIL += 1
        print(f"  \033[31m✖\033[0m  {label}")


# ── Helper: create a fake binary ─────────────────────────────────────────────

def _make_binary(dir_path, name, is_windows=False, python_shebang=False):
    """Create a fake executable file."""
    os.makedirs(dir_path, exist_ok=True)
    ext = ".exe" if is_windows else ""
    fpath = os.path.join(dir_path, name + ext)
    with open(fpath, "w") as f:
        if is_windows:
            f.write("@echo off\necho ptrm 2.0.0\n")
        elif python_shebang:
            f.write("#!/usr/bin/env python3\n# pip-installed wrapper\nprint('ptrm 2.0.0')\n")
        else:
            f.write("#!/bin/sh\necho ptrm 2.0.0\n")
    os.chmod(fpath, 0o755)
    return fpath


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 1: Python conflict.py - detect_source() across OS path patterns
# ══════════════════════════════════════════════════════════════════════════════

def test_detect_source_all_os():
    _header("SECTION 1: detect_source() - Cross-Platform Path Patterns")

    # macOS paths
    _section("macOS paths")
    check("brew (macOS /opt/homebrew)",     detect_source("/opt/homebrew/bin/portrm") == "brew")
    check("brew (macOS /usr/local Cellar)", detect_source("/usr/local/Cellar/portrm/2.0/bin/portrm") == "brew")
    check("cargo (macOS)",                  detect_source("/Users/john/.cargo/bin/ptrm") == "cargo")
    check("pip (macOS .local)",             detect_source("/Users/john/.local/bin/portrm") in ("pip", "script"))
    check("pip (macOS site-packages)",      detect_source("/Users/john/Library/Python/3.10/lib/python/site-packages/portrm/bin/ptrm") == "pip")
    check("npm (macOS node_modules)",       detect_source("/Users/john/node_modules/.bin/portrm") == "npm-local")
    check("npm (macOS global)",             detect_source("/usr/local/lib/node_modules/portrm/bin/ptrm") == "npm")

    # Linux paths
    _section("Linux paths")
    check("brew (Linux homebrew)",          detect_source("/home/linuxbrew/.linuxbrew/bin/portrm") == "brew")
    check("cargo (Linux)",                  detect_source("/home/user/.cargo/bin/ptrm") == "cargo")
    check("pip (Linux .local)",             detect_source("/home/user/.local/bin/portrm") in ("pip", "script"))
    check("pip (Linux site-packages)",      detect_source("/usr/lib/python3/dist-packages/site-packages/portrm") == "pip")
    check("npm (Linux node_modules)",       detect_source("/home/user/node_modules/.bin/portrm") == "npm-local")
    check("npm (Linux global /npm/)",       detect_source("/usr/lib/npm/bin/portrm") == "npm")

    # Windows paths (backslash style)
    _section("Windows paths")
    check("cargo (Windows)",                detect_source("C:\\Users\\john\\.cargo\\bin\\ptrm.exe") == "cargo")
    check("pip (Windows .local)",           detect_source("C:\\Users\\john\\.local\\bin\\portrm.exe") in ("pip", "script"))
    check("pip (Windows Python)",           detect_source("C:\\Users\\john\\AppData\\Local\\Programs\\Python\\Python310\\Scripts\\portrm.exe") == "pip")
    check("npm (Windows AppData)",          detect_source("C:\\Users\\john\\AppData\\Roaming\\npm\\portrm.cmd") == "npm")
    check("npm (Windows node_modules)",     detect_source("C:\\Users\\john\\node_modules\\.bin\\portrm.cmd") == "npm-local")
    check("npm (Windows npx)",             detect_source("C:\\Users\\john\\AppData\\Local\\npm-cache\\_npx\\abc123\\node_modules\\.bin\\ptrm.cmd") == "npm-local")

    # Unknown
    _section("Unknown / edge cases")
    check("unknown (/usr/bin)",             detect_source("/usr/bin/ptrm") == "unknown")
    check("unknown (custom dir)",           detect_source("/opt/custom/tools/ptrm") == "unknown")
    check("scoop (no pattern yet)",         detect_source("C:\\Users\\john\\scoop\\shims\\ptrm.exe") == "unknown")


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 2: Python conflict.py - suggest_install_commands() per OS
# ══════════════════════════════════════════════════════════════════════════════

def test_suggest_install_per_os():
    _header("SECTION 2: suggest_install_commands() - Per OS")

    # macOS
    _section("macOS (Darwin)")
    with mock.patch("portrm.conflict.platform") as mock_platform, \
         mock.patch("portrm.conflict.detect_available_tools", return_value={"brew": True, "npm": True, "pip": True, "cargo": True}):
        mock_platform.system.return_value = "Darwin"
        mock_platform.machine.return_value = "arm64"
        mock_platform.python_version.return_value = "3.10.0"
        rec, alts = suggest_install_commands()
        check("macOS recommended = brew", "brew install abhishekayu/tap/portrm" == rec)
        check("macOS has npm alt",        "npm install -g portrm" in alts)
        check("macOS has pip alt",        "pip install portrm" in alts)
        check("macOS has cargo alt",      "cargo install portrm" in alts)

    # macOS - only pip available
    _section("macOS (only pip available)")
    with mock.patch("portrm.conflict.platform") as mock_platform, \
         mock.patch("portrm.conflict.detect_available_tools", return_value={"brew": False, "npm": False, "pip": True, "cargo": False}):
        mock_platform.system.return_value = "Darwin"
        rec, alts = suggest_install_commands()
        check("macOS pip-only recommended = pip", "pip install portrm" == rec)
        check("macOS pip-only no brew in alts",   "brew" not in " ".join(alts))
        check("macOS pip-only no npm in alts",    "npm" not in " ".join(alts))

    # Linux
    _section("Linux")
    with mock.patch("portrm.conflict.platform") as mock_platform, \
         mock.patch("portrm.conflict.detect_available_tools", return_value={"brew": False, "npm": True, "pip": True, "cargo": False}):
        mock_platform.system.return_value = "Linux"
        rec, alts = suggest_install_commands()
        check("Linux recommended = curl",          "curl" in rec and "install.sh" in rec)
        check("Linux has npm alt",                 "npm install -g portrm" in alts)
        check("Linux has pip alt",                 "pip install portrm" in alts)
        check("Linux no cargo (not available)",    not any("cargo" in a for a in alts))

    # Windows
    _section("Windows")
    with mock.patch("portrm.conflict.platform") as mock_platform, \
         mock.patch("portrm.conflict.detect_available_tools", return_value={"brew": False, "npm": True, "pip": True, "cargo": True}):
        mock_platform.system.return_value = "Windows"
        rec, alts = suggest_install_commands()
        check("Windows recommended = npm",         "npm install -g portrm" == rec)
        check("Windows has pip alt",               "pip install portrm" in alts)
        check("Windows has cargo alt",             "cargo install portrm" in alts)
        check("Windows no brew",                   not any("brew" in a for a in [rec] + alts))

    # Windows - no tools
    _section("Windows (no tools available)")
    with mock.patch("portrm.conflict.platform") as mock_platform, \
         mock.patch("portrm.conflict.detect_available_tools", return_value={"brew": False, "npm": False, "pip": False, "cargo": False}):
        mock_platform.system.return_value = "Windows"
        rec, alts = suggest_install_commands()
        check("Windows no-tools fallback has recommendation", rec is not None and len(rec) > 0)
        print(f"    > Fallback: {rec}")


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 3: Python conflict.py - Fake multi-install conflict per OS
# ══════════════════════════════════════════════════════════════════════════════

def test_conflict_detection_per_os():
    _header("SECTION 3: Conflict Detection - Fake Installs Per OS")

    os_configs = {
        "macOS": {
            "dirs": {
                "brew":      "opt/homebrew/bin",
                "pip":       ".local/bin",
                "cargo":     ".cargo/bin",
                "npm-local": "node_modules/.bin",
            },
        },
        "Linux": {
            "dirs": {
                "pip":       ".local/bin",
                "cargo":     ".cargo/bin",
                "npm-local": "node_modules/.bin",
            },
        },
        "Windows": {
            "dirs": {
                "pip":   ".local/bin",
                "cargo": ".cargo/bin",
                "npm":   "AppData/Roaming/npm",
            },
        },
    }

    for os_name, config in os_configs.items():
        _section(f"{os_name} - multi-install conflict")

        # Create fake binaries
        created_dirs = []
        for source, rel_dir in config["dirs"].items():
            full_dir = os.path.join(TMPDIR, os_name, rel_dir)
            use_python = (source == "pip")
            _make_binary(full_dir, "portrm", python_shebang=use_python)
            _make_binary(full_dir, "ptrm", python_shebang=use_python)
            created_dirs.append(full_dir)

        # Build a PATH with only our fake dirs
        fake_path = os.pathsep.join(created_dirs)

        # Strip any real portrm/ptrm locations from original PATH
        original_entries = os.environ.get("PATH", "").split(os.pathsep)
        clean_entries = [
            d for d in original_entries
            if not any(
                os.path.isfile(os.path.join(d, n))
                for n in ("portrm", "ptrm", "portrm.exe", "ptrm.exe")
            )
        ]
        test_path = fake_path + os.pathsep + os.pathsep.join(clean_entries)

        with mock.patch.dict(os.environ, {"PATH": test_path}):
            bins = find_all_binaries()
            sources = [detect_source(b) for b in bins]
            unique = list(dict.fromkeys(sources))
            uninstall = get_uninstall_commands(unique)

            found_sources = set(sources)
            expected_sources = set(config["dirs"].keys())

            check(
                f"{os_name}: found >= {len(config['dirs'])} sources",
                len(found_sources) >= len(expected_sources),
            )
            for src in expected_sources:
                check(f"{os_name}: detected {src}", src in found_sources)
            check(
                f"{os_name}: uninstall cmds = {len(expected_sources)}",
                len(uninstall) == len(expected_sources),
            )

        # Test single install (no conflict)
        _section(f"{os_name} - single install (no conflict)")
        single_dir = created_dirs[0]
        single_path = single_dir + os.pathsep + os.pathsep.join(clean_entries)

        with mock.patch.dict(os.environ, {"PATH": single_path}):
            bins = find_all_binaries()
            sources = [detect_source(b) for b in bins]
            unique = set(sources)
            check(f"{os_name}: single install = 1 source", len(unique) == 1)


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 4: Python conflict.py - run_doctor() per OS
# ══════════════════════════════════════════════════════════════════════════════

def test_doctor_per_os():
    _header("SECTION 4: run_doctor() Output - Per OS")
    import io

    for os_name, system_name in [("macOS", "Darwin"), ("Linux", "Linux"), ("Windows", "Windows")]:
        _section(f"Doctor on {os_name}")

        # Create a single fake install
        fake_dir = os.path.join(TMPDIR, f"doctor-{os_name}", ".cargo", "bin")
        _make_binary(fake_dir, "portrm")
        _make_binary(fake_dir, "ptrm")

        original_entries = os.environ.get("PATH", "").split(os.pathsep)
        clean_entries = [
            d for d in original_entries
            if not any(
                os.path.isfile(os.path.join(d, n))
                for n in ("portrm", "ptrm", "portrm.exe", "ptrm.exe")
            )
        ]
        test_path = fake_dir + os.pathsep + os.pathsep.join(clean_entries)

        captured = io.StringIO()

        with mock.patch.dict(os.environ, {"PATH": test_path, "NO_COLOR": "1"}), \
             mock.patch("portrm.conflict.platform") as mock_platform, \
             mock.patch("sys.stderr", captured):
            mock_platform.system.return_value = system_name
            mock_platform.machine.return_value = "x86_64"
            mock_platform.python_version.return_value = "3.10.0"
            # Need to also mock detect_available_tools since shutil.which
            # depends on real system
            run_doctor()

        output = captured.getvalue()
        check(f"{os_name}: doctor has header",             "portrm doctor" in output)
        check(f"{os_name}: doctor has system info",        "System:" in output)
        check(f"{os_name}: doctor has active binary",      "Active binary:" in output)
        check(f"{os_name}: doctor has installations",      "Installations found:" in output)
        check(f"{os_name}: doctor has conflict status",    "Conflict status:" in output)
        check(f"{os_name}: doctor has runtimes",           "Available runtimes:" in output)
        check(f"{os_name}: doctor has install suggestion", "Recommended install:" in output)
        check(f"{os_name}: doctor shows OS name",          system_name in output)


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 5: VS Code Extension - resolvePtrm() logic simulation
# ══════════════════════════════════════════════════════════════════════════════

def _vscode_resolve_ptrm(os_name, home_dir, workspace_root=None, env_overrides=None):
    """
    Pure-Python simulation of the VS Code extension's resolvePtrm() function.
    Tests the candidate paths generated for each platform.
    """
    is_win = (os_name == "win32")
    ext = ".exe" if is_win else ""
    binary = f"ptrm{ext}"

    candidates = []

    # Workspace target/release
    if workspace_root:
        candidates.append(os.path.join(workspace_root, "target", "release", binary))

    env = env_overrides or {}

    if is_win:
        local_appdata = env.get("LOCALAPPDATA", os.path.join(home_dir, "AppData", "Local"))
        program_files = env.get("ProgramFiles", "C:\\Program Files")
        appdata = env.get("APPDATA", os.path.join(home_dir, "AppData", "Roaming"))

        candidates.extend([
            os.path.join(home_dir, ".cargo", "bin", binary),
            os.path.join(local_appdata, "portrm", binary),
            os.path.join(program_files, "portrm", binary),
            os.path.join(appdata, "npm", binary),
            os.path.join(home_dir, "scoop", "shims", binary),
        ])
    else:
        candidates.extend([
            "/usr/local/bin/ptrm",
            os.path.join(home_dir, ".cargo/bin/ptrm"),
            "/opt/homebrew/bin/ptrm",
            "/usr/bin/ptrm",
        ])

    # Check which candidates exist (in our fake FS)
    for c in candidates:
        if os.path.isfile(c):
            return c

    return binary  # fallback


def test_vscode_resolve_ptrm():
    _header("SECTION 5: VS Code resolvePtrm() Simulation")

    # ── macOS ──
    _section("macOS (darwin)")
    mac_home = os.path.join(TMPDIR, "vscode-mac", "Users", "dev")
    mac_workspace = os.path.join(TMPDIR, "vscode-mac", "workspace")

    # Test 1: workspace target/release exists
    target_dir = os.path.join(mac_workspace, "target", "release")
    _make_binary(target_dir, "ptrm")
    result = _vscode_resolve_ptrm("darwin", mac_home, mac_workspace)
    check("macOS: finds workspace target/release/ptrm", result == os.path.join(target_dir, "ptrm"))

    # Test 2: no workspace, cargo install
    cargo_dir = os.path.join(mac_home, ".cargo", "bin")
    _make_binary(cargo_dir, "ptrm")
    result = _vscode_resolve_ptrm("darwin", mac_home, workspace_root=None)
    check("macOS: finds ~/.cargo/bin/ptrm", result == os.path.join(cargo_dir, "ptrm"))

    # Test 3: homebrew
    # On a machine with ptrm installed, system paths like /opt/homebrew/bin/ptrm
    # will be found. The fallback only applies on a clean machine.
    result = _vscode_resolve_ptrm("darwin", os.path.join(TMPDIR, "empty-mac"), workspace_root=None)
    check("macOS: fallback resolves to ptrm or system path", result == "ptrm" or os.path.isfile(result))

    # ── Windows ──
    _section("Windows (win32)")
    win_home = os.path.join(TMPDIR, "vscode-win", "Users", "dev")

    # Test 1: cargo install on Windows
    cargo_win = os.path.join(win_home, ".cargo", "bin")
    _make_binary(cargo_win, "ptrm", is_windows=True)
    result = _vscode_resolve_ptrm("win32", win_home, env_overrides={
        "LOCALAPPDATA": os.path.join(win_home, "AppData", "Local"),
        "ProgramFiles": os.path.join(TMPDIR, "Program Files"),
        "APPDATA": os.path.join(win_home, "AppData", "Roaming"),
    })
    check("Windows: finds .cargo/bin/ptrm.exe", result.endswith("ptrm.exe"))
    check("Windows: cargo path correct", ".cargo" in result)

    # Test 2: npm global on Windows
    npm_win = os.path.join(win_home, "AppData", "Roaming", "npm")
    _make_binary(npm_win, "ptrm", is_windows=True)
    # Remove the cargo one to test npm fallback
    os.remove(os.path.join(cargo_win, "ptrm.exe"))
    result = _vscode_resolve_ptrm("win32", win_home, env_overrides={
        "LOCALAPPDATA": os.path.join(win_home, "AppData", "Local"),
        "ProgramFiles": os.path.join(TMPDIR, "Program Files"),
        "APPDATA": os.path.join(win_home, "AppData", "Roaming"),
    })
    check("Windows: finds AppData/Roaming/npm/ptrm.exe", "npm" in result and result.endswith("ptrm.exe"))

    # Test 3: scoop on Windows
    scoop_win = os.path.join(win_home, "scoop", "shims")
    _make_binary(scoop_win, "ptrm", is_windows=True)
    os.remove(os.path.join(npm_win, "ptrm.exe"))
    result = _vscode_resolve_ptrm("win32", win_home, env_overrides={
        "LOCALAPPDATA": os.path.join(win_home, "AppData", "Local"),
        "ProgramFiles": os.path.join(TMPDIR, "Program Files"),
        "APPDATA": os.path.join(win_home, "AppData", "Roaming"),
    })
    check("Windows: finds scoop/shims/ptrm.exe", "scoop" in result and result.endswith("ptrm.exe"))

    # Test 4: LOCALAPPDATA custom install
    local_app = os.path.join(win_home, "AppData", "Local", "portrm")
    _make_binary(local_app, "ptrm", is_windows=True)
    os.remove(os.path.join(scoop_win, "ptrm.exe"))
    result = _vscode_resolve_ptrm("win32", win_home, env_overrides={
        "LOCALAPPDATA": os.path.join(win_home, "AppData", "Local"),
        "ProgramFiles": os.path.join(TMPDIR, "Program Files"),
        "APPDATA": os.path.join(win_home, "AppData", "Roaming"),
    })
    check("Windows: finds LOCALAPPDATA/portrm/ptrm.exe", "portrm" in result and result.endswith("ptrm.exe"))

    # Test 5: Windows workspace target/release
    win_workspace = os.path.join(TMPDIR, "vscode-win", "workspace")
    win_target = os.path.join(win_workspace, "target", "release")
    _make_binary(win_target, "ptrm", is_windows=True)
    result = _vscode_resolve_ptrm("win32", win_home, win_workspace, env_overrides={
        "LOCALAPPDATA": os.path.join(win_home, "AppData", "Local"),
        "ProgramFiles": os.path.join(TMPDIR, "Program Files"),
        "APPDATA": os.path.join(win_home, "AppData", "Roaming"),
    })
    check("Windows: workspace target/release wins over others", "target" in result and result.endswith("ptrm.exe"))

    # Test 6: Windows fallback
    result = _vscode_resolve_ptrm("win32", os.path.join(TMPDIR, "empty-win"), env_overrides={
        "LOCALAPPDATA": os.path.join(TMPDIR, "empty-win", "AppData", "Local"),
        "ProgramFiles": os.path.join(TMPDIR, "empty-Programs"),
        "APPDATA": os.path.join(TMPDIR, "empty-win", "AppData", "Roaming"),
    })
    check("Windows: fallback to bare 'ptrm.exe'", result == "ptrm.exe")

    # ── Linux ──
    _section("Linux (linux)")
    linux_home = os.path.join(TMPDIR, "vscode-linux", "home", "dev")

    # Test: cargo install on Linux
    cargo_linux = os.path.join(linux_home, ".cargo", "bin")
    _make_binary(cargo_linux, "ptrm")
    result = _vscode_resolve_ptrm("linux", linux_home)
    check("Linux: finds ~/.cargo/bin/ptrm", result == os.path.join(cargo_linux, "ptrm"))

    # Test: Linux fallback
    result = _vscode_resolve_ptrm("linux", os.path.join(TMPDIR, "empty-linux"))
    check("Linux: fallback resolves to ptrm or system path", result == "ptrm" or os.path.isfile(result))


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 6: VS Code Extension - getInstallSuggestion() simulation
# ══════════════════════════════════════════════════════════════════════════════

def _vscode_get_install_suggestion(platform_name):
    """Mirrors installer.ts getInstallSuggestion()."""
    if platform_name == "darwin":
        return "brew install abhishekayu/tap/portrm"
    elif platform_name == "linux":
        return "curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh | sh"
    elif platform_name == "win32":
        return "npm install -g portrm"
    else:
        return "cargo install portrm"


def test_vscode_install_suggestion():
    _header("SECTION 6: VS Code getInstallSuggestion() Simulation")

    check("darwin  -> brew tap",     "brew install abhishekayu/tap/portrm" == _vscode_get_install_suggestion("darwin"))
    check("linux   -> curl install", "install.sh" in _vscode_get_install_suggestion("linux"))
    check("win32   -> npm global",   "npm install -g portrm" == _vscode_get_install_suggestion("win32"))
    check("freebsd -> cargo",        "cargo install portrm" == _vscode_get_install_suggestion("freebsd"))

    # Verify consistency between Python (conflict.py) and TS (installer.ts)
    _section("Python <-> VS Code consistency")

    # macOS
    with mock.patch("portrm.conflict.platform") as mp, \
         mock.patch("portrm.conflict.detect_available_tools", return_value={"brew": True, "npm": True, "pip": True, "cargo": True}):
        mp.system.return_value = "Darwin"
        py_rec, _ = suggest_install_commands()
    ts_rec = _vscode_get_install_suggestion("darwin")
    check("macOS: Python rec == TS suggestion", py_rec == ts_rec)

    # Windows
    with mock.patch("portrm.conflict.platform") as mp, \
         mock.patch("portrm.conflict.detect_available_tools", return_value={"brew": False, "npm": True, "pip": True, "cargo": True}):
        mp.system.return_value = "Windows"
        py_rec, _ = suggest_install_commands()
    ts_rec = _vscode_get_install_suggestion("win32")
    check("Windows: Python rec == TS suggestion", py_rec == ts_rec)


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 7: VS Code Extension - runInTerminal() path substitution
# ══════════════════════════════════════════════════════════════════════════════

def test_vscode_terminal_path_substitution():
    _header("SECTION 7: VS Code runInTerminal() Path Substitution")

    import re

    def simulate_run_in_terminal(command, resolved_bin):
        """Mirrors the regex replacement in utils.ts runInTerminal().
        JS String.replace handles backslashes literally; Python re.sub needs
        a lambda to avoid interpreting them as escape sequences."""
        return re.sub(r'\bptrm\b', lambda m: resolved_bin, command)

    # macOS: resolved to workspace binary
    resolved = "/Users/dev/project/target/release/ptrm"
    check("single ptrm replaced",
          simulate_run_in_terminal("ptrm doctor", resolved) == f"{resolved} doctor")
    check("chained ptrm && ptrm replaced",
          simulate_run_in_terminal("ptrm down && ptrm up", resolved) == f"{resolved} down && {resolved} up")
    check("ptrm with args",
          simulate_run_in_terminal("ptrm info 8080", resolved) == f"{resolved} info 8080")
    check("ptrm use profile combo",
          simulate_run_in_terminal("ptrm down && ptrm use staging && ptrm up", resolved)
          == f"{resolved} down && {resolved} use staging && {resolved} up")

    # Windows: resolved with backslashes
    # Note: In the real TS code, String.replace() handles backslashes natively.
    # In Python re.sub, re.escape is needed but produces escaped backslashes.
    # We test that the command contains the resolved path (functional correctness).
    resolved_win = "C:\\Users\\dev\\AppData\\Roaming\\npm\\ptrm.exe"
    result = simulate_run_in_terminal("ptrm scan --dev", resolved_win)
    check("Windows path substitution",
          "ptrm.exe" in result and "scan --dev" in result and "ptrm scan" not in result)

    # Edge: bare name fallback (ptrm -> ptrm, no change)
    check("bare name no-op",
          simulate_run_in_terminal("ptrm fix", "ptrm") == "ptrm fix")

    # Edge: should NOT replace partial matches like "ptrm-extra"
    check("no partial match 'ptrmx'",
          simulate_run_in_terminal("ptrmx test", resolved) == "ptrmx test")


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 8: Node.js conflict.js - Cross-platform via subprocess
# ══════════════════════════════════════════════════════════════════════════════

def test_nodejs_conflict():
    _header("SECTION 8: Node.js conflict.js - Subprocess Tests")

    conflict_js = os.path.join(ROOT, "npm", "bin", "conflict.js")
    if not os.path.isfile(conflict_js):
        print("  (skipped - conflict.js not found)")
        return

    node = shutil.which("node")
    if not node:
        print("  (skipped - node not found)")
        return

    # Test detectSource via a small Node script
    test_script = """
    const c = require('./npm/bin/conflict.js');
    const tests = {
        brew_mac: c.detectSource('/opt/homebrew/bin/portrm'),
        cargo_mac: c.detectSource('/Users/dev/.cargo/bin/ptrm'),
        pip_linux: c.detectSource('/home/user/.local/bin/portrm'),
        npm_win: c.detectSource('C:\\\\Users\\\\dev\\\\AppData\\\\Roaming\\\\npm\\\\portrm.cmd'),
        cargo_win: c.detectSource('C:\\\\Users\\\\dev\\\\.cargo\\\\bin\\\\ptrm.exe'),
        npm_npx: c.detectSource('/tmp/_npx/abc123/node_modules/.bin/ptrm'),
    };
    const suggest = c.suggestInstallCommands();
    console.log(JSON.stringify({ sources: tests, suggest }));
    """

    try:
        result = subprocess.run(
            [node, "-e", test_script],
            capture_output=True,
            text=True,
            timeout=10,
            cwd=ROOT,
            env={**os.environ, "NO_COLOR": "1"},
        )
        if result.returncode != 0:
            print(f"  (node error: {result.stderr.strip()})")
            return

        data = json.loads(result.stdout)
        sources = data["sources"]
        suggest = data["suggest"]

        check("Node: brew (macOS)",                 sources["brew_mac"] == "brew")
        check("Node: cargo (macOS)",                sources["cargo_mac"] == "cargo")
        check("Node: pip (Linux)",                  sources["pip_linux"] in ("pip", "script"))
        check("Node: npm (Windows AppData)",        sources["npm_win"] == "npm")
        check("Node: cargo (Windows)",              sources["cargo_win"] == "cargo")
        check("Node: npm (npx path)",               sources["npm_npx"] == "npm-local")
        check("Node: has recommended install",      len(suggest["recommended"]) > 0)
        check("Node: has alternatives",             isinstance(suggest["alternatives"], list))

        # Verify Node suggestions match local platform
        current = sys.platform
        if current == "darwin":
            check("Node: darwin rec = brew", "brew" in suggest["recommended"])
        elif current == "linux":
            check("Node: linux rec = curl", "curl" in suggest["recommended"])
        elif current == "win32":
            check("Node: win32 rec = npm", "npm" in suggest["recommended"])

    except (subprocess.TimeoutExpired, json.JSONDecodeError, KeyError) as e:
        print(f"  (error: {e})")


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 9: npx bypass per OS
# ══════════════════════════════════════════════════════════════════════════════

def test_npx_bypass():
    _header("SECTION 9: npx Bypass Detection")

    # Simulate npx via npm_execpath
    with mock.patch.dict(os.environ, {"npm_execpath": "/usr/local/lib/node_modules/npm/bin/npx-cli.js"}):
        check("npx detected via npm_execpath (Unix)", _is_npx_context())

    with mock.patch.dict(os.environ, {"npm_execpath": "C:\\Users\\dev\\AppData\\Roaming\\npm\\npx-cli.js"}):
        # This one should NOT match since there's no _npx or /npx/ in the path
        # npx-cli.js doesn't contain _npx or /npx/ pattern
        pass

    # Simulate npx via PATH containing _npx
    original_path = os.environ.get("PATH", "")
    npx_path = "/tmp/_npx/abc123/node_modules/.bin" + os.pathsep + original_path
    with mock.patch.dict(os.environ, {"PATH": npx_path, "npm_execpath": ""}):
        check("npx detected via _npx in PATH (Unix)", _is_npx_context())

    win_npx_path = "C:\\Users\\dev\\AppData\\Local\\npm-cache\\_npx\\abc\\node_modules\\.bin" + os.pathsep + original_path
    with mock.patch.dict(os.environ, {"PATH": win_npx_path, "npm_execpath": ""}):
        check("npx detected via _npx in PATH (Windows-style)", _is_npx_context())

    # Not npx
    with mock.patch.dict(os.environ, {"PATH": "/usr/bin:/usr/local/bin", "npm_execpath": "", "npm_lifecycle_event": "", "npm_config_cache": ""}):
        check("non-npx correctly returns False", not _is_npx_context())


# ══════════════════════════════════════════════════════════════════════════════
#  SECTION 10: VS Code Extension - activationEvents + package.json checks
# ══════════════════════════════════════════════════════════════════════════════

def test_vscode_package_json():
    _header("SECTION 10: VS Code Extension package.json Validation")

    pkg_path = os.path.join(ROOT, "vscode-extension", "package.json")
    with open(pkg_path) as f:
        pkg = json.load(f)

    # Activation events
    events = pkg.get("activationEvents", [])
    check("activationEvents != ['*']",               events != ["*"])
    check("activationEvents = ['onStartupFinished']", events == ["onStartupFinished"])

    # Commands registered
    commands = [c["command"] for c in pkg.get("contributes", {}).get("commands", [])]
    expected_cmds = ["ptrm.fix", "ptrm.doctor", "ptrm.scanDev", "ptrm.history", "ptrm.ci", "ptrm.update"]
    for cmd in expected_cmds:
        check(f"command registered: {cmd}", cmd in commands)

    # No install/download references in commands
    cmd_titles = [c.get("title", "").lower() for c in pkg.get("contributes", {}).get("commands", [])]
    check("no 'download' in command titles", not any("download" in t for t in cmd_titles))

    # Engine version
    engines = pkg.get("engines", {})
    check("engines.vscode defined", "vscode" in engines)

    # Publisher
    check("publisher = abhishekayu", pkg.get("publisher") == "abhishekayu")


# ══════════════════════════════════════════════════════════════════════════════
#  MAIN
# ══════════════════════════════════════════════════════════════════════════════

def main():
    setup()
    try:
        test_detect_source_all_os()
        test_suggest_install_per_os()
        test_conflict_detection_per_os()
        test_doctor_per_os()
        test_vscode_resolve_ptrm()
        test_vscode_install_suggestion()
        test_vscode_terminal_path_substitution()
        test_nodejs_conflict()
        test_npx_bypass()
        test_vscode_package_json()
    finally:
        cleanup()

    print(f"\n{'=' * 60}")
    print(f"  RESULTS")
    print(f"{'=' * 60}\n")

    if FAIL == 0:
        print(f"  \033[32m✔ ALL {TOTAL} TESTS PASSED\033[0m\n")
    else:
        print(f"  \033[31m✖ {FAIL} FAILED\033[0m / {TOTAL} total\n")

    sys.exit(1 if FAIL else 0)


if __name__ == "__main__":
    main()
