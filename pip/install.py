"""Post-install script: downloads the correct ptrm binary for this platform."""

import hashlib
import io
import os
import platform
import shutil
import stat
import sys
import tarfile
import tempfile
import urllib.request
import zipfile

VERSION = "2.2.4"
REPO = "abhishekayu/portrm"


def _get_asset_name():
    """Determine the GitHub release asset name for this platform."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    arch_map = {
        "x86_64": "amd64",
        "amd64": "amd64",
        "aarch64": "arm64",
        "arm64": "arm64",
    }
    arch = arch_map.get(machine)
    if not arch:
        return None, None

    if system == "darwin":
        return f"portrm-darwin-{arch}", ".tar.gz"
    elif system == "linux":
        return f"portrm-linux-{arch}", ".tar.gz"
    elif system == "windows":
        return f"portrm-windows-{arch}", ".zip"
    return None, None


def _download(url):
    """Download a URL, following redirects."""
    req = urllib.request.Request(url, headers={"User-Agent": "portrm-pip-installer"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        return resp.read()


def install():
    """Download and install the ptrm binary into the package bin/ directory."""
    asset, ext = _get_asset_name()
    if not asset:
        print(
            f"Unsupported platform: {platform.system()} {platform.machine()}",
            file=sys.stderr,
        )
        print("Install manually: cargo install portrm", file=sys.stderr)
        return

    url = f"https://github.com/{REPO}/releases/download/v{VERSION}/{asset}{ext}"
    bin_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "src", "portrm", "bin")
    os.makedirs(bin_dir, exist_ok=True)

    print(f"Downloading ptrm v{VERSION} for {platform.system()}-{platform.machine()}...")

    try:
        data = _download(url)
    except Exception as e:
        print(f"Failed to download ptrm: {e}", file=sys.stderr)
        print("Install manually: cargo install portrm", file=sys.stderr)
        return

    try:
        if ext == ".zip":
            with zipfile.ZipFile(io.BytesIO(data)) as zf:
                zf.extractall(bin_dir)
        else:
            with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tf:
                tf.extractall(bin_dir)

        # Make binary executable on Unix
        if platform.system().lower() != "windows":
            binary = os.path.join(bin_dir, "ptrm")
            if os.path.isfile(binary):
                st = os.stat(binary)
                os.chmod(binary, st.st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)

        print("ptrm installed successfully.")
    except Exception as e:
        print(f"Failed to extract ptrm: {e}", file=sys.stderr)
        print("Install manually: cargo install portrm", file=sys.stderr)


if __name__ == "__main__":
    install()
