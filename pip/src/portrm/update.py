"""
Auto-update check for portrm.

Checks GitHub releases for a newer version once per 24 hours (cached).
If found, auto-updates using the detected install method.
"""

import json
import os
import subprocess
import sys
import time
import urllib.request

VERSION = None  # set by caller

GITHUB_API = "https://api.github.com/repos/abhishekayu/portrm/releases/latest"
CHECK_INTERVAL = 86_400  # 24 hours


def _state_file():
    """Return path to ~/.portrm/last_update_check."""
    home = os.path.expanduser("~")
    return os.path.join(home, ".portrm", "last_update_check")


def _last_check_time():
    try:
        with open(_state_file()) as f:
            return int(f.read().strip())
    except Exception:
        return 0


def _save_check_time():
    path = _state_file()
    os.makedirs(os.path.dirname(path), exist_ok=True)
    try:
        with open(path, "w") as f:
            f.write(str(int(time.time())))
    except Exception:
        pass


def _is_newer(current, latest):
    """Return True if latest > current (semver comparison)."""
    def parse(v):
        parts = v.lstrip("v").split(".")
        return tuple(int(x) for x in parts[:3])
    try:
        return parse(latest) > parse(current)
    except (ValueError, IndexError):
        return False


def _fetch_latest_version():
    """Fetch the latest release tag from GitHub."""
    try:
        req = urllib.request.Request(
            GITHUB_API,
            headers={
                "Accept": "application/vnd.github.v3+json",
                "User-Agent": "ptrm-update-check",
            },
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read().decode())
            tag = data.get("tag_name", "")
            return tag.lstrip("v")
    except Exception:
        return None


def _detect_source():
    """Detect how this Python package was installed."""
    exe = sys.executable or ""
    argv0 = sys.argv[0] if sys.argv else ""
    for hint in (exe, argv0):
        lower = hint.replace("\\", "/").lower()
        if "pipx" in lower:
            return "pipx"
    # If we're running from pip, it's pip
    return "pip"


_UPDATE_CMDS = {
    "pip": "pip install --upgrade portrm",
    "pipx": "pipx upgrade portrm",
}

# ANSI helpers
_NO_COLOR = os.environ.get("NO_COLOR") is not None or not hasattr(sys.stderr, "isatty") or not sys.stderr.isatty()

def _cyan(s):
    return s if _NO_COLOR else f"\033[36m{s}\033[0m"

def _bold(s):
    return s if _NO_COLOR else f"\033[1m{s}\033[0m"

def _green(s):
    return s if _NO_COLOR else f"\033[1;32m{s}\033[0m"

def _red(s):
    return s if _NO_COLOR else f"\033[1;31m{s}\033[0m"

def _dim(s):
    return s if _NO_COLOR else f"\033[2m{s}\033[0m"


def check_and_update(current_version):
    """Check for updates and auto-update if a newer version is available."""
    if os.environ.get("PTRM_SKIP_UPDATE_CHECK") == "1":
        return

    now = int(time.time())
    if now - _last_check_time() < CHECK_INTERVAL:
        return

    _save_check_time()

    latest = _fetch_latest_version()
    if not latest:
        return

    if not _is_newer(current_version, latest):
        return

    source = _detect_source()
    cmd = _UPDATE_CMDS.get(source)

    if not cmd:
        print(
            f"\n  {_cyan('⬆')} {_bold('New version available:')} {_cyan(latest)} (you have {_dim(current_version)})",
            file=sys.stderr,
        )
        print(f"  {_dim('→')} Update manually.\n", file=sys.stderr)
        return

    print(file=sys.stderr)
    print(
        f"  {_cyan('⬆')} {_bold('Updating portrm to')} {_cyan(latest)} (you have {_dim(current_version)})",
        file=sys.stderr,
    )
    print(file=sys.stderr)
    print(f"  {_dim('$')} {_cyan(cmd)}", file=sys.stderr)
    print(file=sys.stderr)

    try:
        result = subprocess.run(cmd.split(), capture_output=True, text=True, timeout=120)
        if result.returncode == 0:
            print(
                f"  {_green('✔ Updated successfully! Restart ptrm to use the new version.')}\n",
                file=sys.stderr,
            )
        else:
            print(
                f"  {_red('✖ Auto-update failed.')} Update manually: {cmd}\n",
                file=sys.stderr,
            )
    except Exception:
        print(
            f"  {_red('✖ Auto-update failed.')} Update manually: {cmd}\n",
            file=sys.stderr,
        )
