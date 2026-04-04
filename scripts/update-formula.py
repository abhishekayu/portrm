#!/usr/bin/env python3
"""
Update the Homebrew formula with SHA256 checksums from a GitHub release.

Usage:
    python3 scripts/update-formula.py v0.1.0
"""

import hashlib
import json
import sys
import urllib.request
import re
from pathlib import Path

REPO = "abhishekayu/portrm"
FORMULA_PATH = Path(__file__).parent.parent / "Formula" / "portrm.rb"

TARGETS = [
    "ptrm-darwin-amd64.tar.gz",
    "ptrm-darwin-arm64.tar.gz",
    "ptrm-linux-amd64.tar.gz",
    "ptrm-linux-arm64.tar.gz",
]


def get_release_assets(tag: str):
    url = f"https://api.github.com/repos/{REPO}/releases/tags/{tag}"
    req = urllib.request.Request(url, headers={"Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read())["assets"]


def download_and_hash(url: str) -> str:
    with urllib.request.urlopen(url) as resp:
        data = resp.read()
    return hashlib.sha256(data).hexdigest()


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <tag>", file=sys.stderr)
        sys.exit(1)

    tag = sys.argv[1]
    version = tag.lstrip("v")

    print(f"Fetching release {tag}...")
    assets = get_release_assets(tag)
    asset_map = {a["name"]: a["browser_download_url"] for a in assets}

    formula = FORMULA_PATH.read_text()

    # Update version
    formula = re.sub(r'version ".*?"', f'version "{version}"', formula)

    for target in TARGETS:
        if target not in asset_map:
            print(f"  WARNING: {target} not found in release assets")
            continue

        print(f"  Hashing {target}...")
        sha = download_and_hash(asset_map[target])
        print(f"    {sha}")

        # Find the url line for this target and replace the sha256 on the next line
        lines = formula.split("\n")
        for i, line in enumerate(lines):
            if target in line:
                # Next sha256 line
                for j in range(i + 1, min(i + 3, len(lines))):
                    if "sha256" in lines[j]:
                        lines[j] = re.sub(r'sha256 ".*?"', f'sha256 "{sha}"', lines[j])
                        break
        formula = "\n".join(lines)

    FORMULA_PATH.write_text(formula)
    print(f"\nFormula updated: {FORMULA_PATH}")


if __name__ == "__main__":
    main()
