"""portrm - blazing-fast CLI to inspect ports, kill processes, and fix dev environments."""

import io
import os
import platform
import stat
import subprocess
import sys
import tarfile
import urllib.request
import zipfile

VERSION = "2.2.2"
REPO = "abhishekayu/portrm"


def _binary_path():
    """Return the path to the installed ptrm binary."""
    pkg_dir = os.path.dirname(os.path.abspath(__file__))
    bin_dir = os.path.join(pkg_dir, "bin")
    exe = "ptrm.exe" if sys.platform == "win32" else "ptrm"
    return os.path.join(bin_dir, exe)


def _ensure_binary():
    """Download the native binary if not already present."""
    binary = _binary_path()
    if os.path.isfile(binary):
        return binary

    system = platform.system().lower()
    machine = platform.machine().lower()

    arch_map = {"x86_64": "amd64", "amd64": "amd64", "aarch64": "arm64", "arm64": "arm64"}
    arch = arch_map.get(machine)
    if not arch:
        print(f"Unsupported architecture: {machine}", file=sys.stderr)
        sys.exit(1)

    if system == "darwin":
        asset, ext = f"portrm-darwin-{arch}", ".tar.gz"
    elif system == "linux":
        asset, ext = f"portrm-linux-{arch}", ".tar.gz"
    elif system == "windows":
        asset, ext = f"portrm-windows-{arch}", ".zip"
    else:
        print(f"Unsupported platform: {system}", file=sys.stderr)
        sys.exit(1)

    url = f"https://github.com/{REPO}/releases/download/v{VERSION}/{asset}{ext}"
    bin_dir = os.path.dirname(binary)
    os.makedirs(bin_dir, exist_ok=True)

    print(f"Downloading ptrm v{VERSION} for {platform.system()}-{machine}...")
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "portrm-pip-installer"})
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = resp.read()
    except Exception as e:
        print(f"Failed to download ptrm: {e}", file=sys.stderr)
        sys.exit(1)

    if ext == ".zip":
        with zipfile.ZipFile(io.BytesIO(data)) as zf:
            zf.extractall(bin_dir)
    else:
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tf:
            tf.extractall(bin_dir)

    if system != "windows":
        st = os.stat(binary)
        os.chmod(binary, st.st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)

    return binary


def main():
    """Entry point that delegates to the native ptrm binary."""
    from portrm.conflict import run_conflict_check, run_doctor
    from portrm.update import check_and_update

    # Handle `portrm doctor` / `ptrm doctor` at the Python level
    # so it works even when the native binary is missing or broken.
    if len(sys.argv) >= 2 and sys.argv[1] == "doctor":
        run_doctor()
        return

    run_conflict_check()

    # Auto-update check (once per 24h, cached)
    check_and_update(VERSION)

    binary = _ensure_binary()
    try:
        result = subprocess.run([binary] + sys.argv[1:])
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        sys.exit(130)
