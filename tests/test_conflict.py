#!/usr/bin/env python3
"""
Automated local test for portrm multi-install conflict detection.

Simulates npm/pip/cargo/brew installations via fake binaries,
validates conflict detection, tests single-install passthrough,
and cleans up everything.

Usage:
    python3 test_conflict.py
"""

import os
import platform
import shutil
import stat
import subprocess
import sys
import tempfile
import textwrap

# ── Resolve project paths ───────────────────────────────────────────────────

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = SCRIPT_DIR  # script lives at repo root
CONFLICT_MODULE = os.path.join(PROJECT_ROOT, "pip", "src")

# Inject module path so we can import portrm.conflict directly
sys.path.insert(0, CONFLICT_MODULE)

# ── ANSI helpers ─────────────────────────────────────────────────────────────

NO_COLOR = os.environ.get("NO_COLOR") is not None or not sys.stdout.isatty()


def _c(code: str, text: str) -> str:
    return text if NO_COLOR else f"\033[{code}m{text}\033[0m"


def red(t: str) -> str:
    return _c("1;31", t)


def green(t: str) -> str:
    return _c("1;32", t)


def yellow(t: str) -> str:
    return _c("1;33", t)


def cyan(t: str) -> str:
    return _c("36", t)


def dim(t: str) -> str:
    return _c("2", t)


def bold(t: str) -> str:
    return _c("1", t)


# ── Logging ──────────────────────────────────────────────────────────────────

_step = 0


def step(title: str) -> None:
    global _step
    _step += 1
    print(f"\n{'=' * 60}")
    print(f"  {bold(f'[STEP {_step}]')} {cyan(title)}")
    print(f"{'=' * 60}\n")


def info(msg: str) -> None:
    print(f"  {dim('>')} {msg}")


def ok(msg: str) -> None:
    print(f"  {green('✔')} {msg}")


def fail(msg: str) -> None:
    print(f"  {red('✖')} {msg}")


def divider() -> None:
    print(f"  {dim('-' * 50)}")


# ── Simulated install sources ───────────────────────────────────────────────

SOURCES = {
    # name -> (subdirectory pattern that triggers detect_source)
    "npm-local": "node_modules/.bin",
    "pip":       ".local/bin",
    "cargo":     ".cargo/bin",
    "brew":      "homebrew/bin",
}


def create_fake_installations(base_dir: str) -> dict:
    """Create fake portrm executables in ecosystem-specific directories."""
    dirs = {}
    for label, subpath in SOURCES.items():
        bin_dir = os.path.join(base_dir, subpath)
        os.makedirs(bin_dir, exist_ok=True)

        binary = os.path.join(bin_dir, "portrm")
        with open(binary, "w") as f:
            if label == "pip":
                # pip installs create Python scripts with a python shebang
                f.write(textwrap.dedent(f"""\
                    #!/usr/bin/env python3
                    # pip-installed portrm wrapper
                    print("portrm: running from {label} ({bin_dir})")
                """))
            else:
                f.write(textwrap.dedent(f"""\
                    #!/bin/sh
                    echo "portrm: running from {label} ({bin_dir})"
                """))
        os.chmod(binary, stat.S_IRWXU | stat.S_IRGRP | stat.S_IXGRP | stat.S_IROTH | stat.S_IXOTH)

        # Also create ptrm alias
        ptrm = os.path.join(bin_dir, "ptrm")
        with open(ptrm, "w") as f:
            if label == "pip":
                f.write(textwrap.dedent(f"""\
                    #!/usr/bin/env python3
                    # pip-installed ptrm wrapper
                    print("ptrm: running from {label} ({bin_dir})")
                """))
            else:
                f.write(textwrap.dedent(f"""\
                    #!/bin/sh
                    echo "ptrm: running from {label} ({bin_dir})"
                """))
        os.chmod(ptrm, stat.S_IRWXU | stat.S_IRGRP | stat.S_IXGRP | stat.S_IROTH | stat.S_IXOTH)

        dirs[label] = bin_dir
        info(f"Created fake {yellow(label)} binary at {dim(bin_dir)}")

    return dirs


def build_test_path(dirs: dict, original_path: str) -> str:
    """Prepend all fake bin directories to PATH."""
    prepend = os.pathsep.join(dirs.values())
    return prepend + os.pathsep + original_path


# ── Test runner ──────────────────────────────────────────────────────────────


def run_which_all() -> list:
    """Run which -a portrm and return found paths."""
    try:
        if sys.platform == "win32":
            out = subprocess.run(["where", "portrm"], capture_output=True, text=True, timeout=5)
        else:
            out = subprocess.run(["which", "-a", "portrm"], capture_output=True, text=True, timeout=5)
        if out.returncode == 0:
            return [l.strip() for l in out.stdout.splitlines() if l.strip()]
    except Exception:
        pass
    return []


def test_conflict_detection() -> bool:
    """Call run_conflict_check and expect sys.exit(1)."""
    from portrm.conflict import run_conflict_check

    try:
        run_conflict_check()
        # If we reach here, no conflict was detected
        return False
    except SystemExit as e:
        return e.code == 1


def test_no_conflict() -> bool:
    """Call run_conflict_check and expect silent return (no exit)."""
    from portrm.conflict import run_conflict_check

    try:
        run_conflict_check()
        return True
    except SystemExit:
        return False


def test_detect_source(dirs: dict) -> bool:
    """Verify detect_source maps each path correctly."""
    from portrm.conflict import detect_source

    all_ok = True
    for label, bin_dir in dirs.items():
        binary_path = os.path.join(bin_dir, "portrm")
        detected = detect_source(binary_path)
        if detected == label:
            ok(f"detect_source({dim(binary_path)}) = {green(detected)}")
        else:
            fail(f"detect_source({dim(binary_path)}) = {red(detected)} (expected {label})")
            all_ok = False
    return all_ok


def test_uninstall_commands() -> bool:
    """Verify uninstall command generation."""
    from portrm.conflict import get_uninstall_commands

    sources = ["brew", "pip", "cargo", "npm-local"]
    cmds = get_uninstall_commands(sources)
    expected = [
        "brew uninstall portrm",
        "pip uninstall portrm",
        "cargo uninstall portrm",
        "npm uninstall portrm",
    ]
    if cmds == expected:
        ok(f"Uninstall commands: {len(cmds)} commands generated correctly")
        return True
    else:
        fail(f"Uninstall commands mismatch: {cmds}")
        return False


def test_suggest_install() -> bool:
    """Verify OS-aware + tool-aware install suggestions."""
    from portrm.conflict import suggest_install_commands

    recommended, alternatives = suggest_install_commands()
    system = platform.system()

    # Smart suggestions depend on what's actually installed, so just verify
    # that recommended is a non-empty string and alternatives is a list
    if not recommended:
        fail("Recommended install command is empty")
        return False

    if not isinstance(alternatives, list):
        fail(f"Alternatives is not a list: {type(alternatives)}")
        return False

    ok(f"Recommended install for {system}: {green(recommended)}")
    for alt in alternatives:
        info(f"  Alternative: {dim(alt)}")
    return True


# ── Main ─────────────────────────────────────────────────────────────────────


def main() -> int:
    original_path = os.environ.get("PATH", "")
    base_dir = os.path.join(tempfile.gettempdir(), "portrm-test")
    passed = 0
    failed = 0
    total = 0

    def record(success: bool) -> None:
        nonlocal passed, failed, total
        total += 1
        if success:
            passed += 1
        else:
            failed += 1

    try:
        # ── STEP 1: Create fake installations ────────────────────────────
        step("Creating fake installations")

        # Clean any leftover from previous runs
        if os.path.exists(base_dir):
            shutil.rmtree(base_dir)

        dirs = create_fake_installations(base_dir)
        divider()
        info(f"Base directory: {dim(base_dir)}")
        info(f"Created {len(dirs)} fake installations")

        # ── STEP 2: Inject into PATH ────────────────────────────────────
        step("Updating PATH")

        test_path = build_test_path(dirs, original_path)
        os.environ["PATH"] = test_path

        for label, bin_dir in dirs.items():
            info(f"PATH += {dim(bin_dir)} ({yellow(label)})")
        divider()
        info(f"Total PATH entries: {len(test_path.split(os.pathsep))}")

        # ── STEP 3: Verify binaries visible ─────────────────────────────
        step("Checking binaries via which -a")

        found = run_which_all()
        for p in found:
            info(f"Found: {cyan(p)}")
        divider()

        has_all = len(found) >= len(dirs)
        if has_all:
            ok(f"Detected {len(found)} binaries (expected >= {len(dirs)})")
        else:
            fail(f"Detected {len(found)} binaries (expected >= {len(dirs)})")
        record(has_all)

        # Verify each fake binary is executable
        for label, bin_dir in dirs.items():
            binary = os.path.join(bin_dir, "portrm")
            try:
                out = subprocess.run([binary], capture_output=True, text=True, timeout=5)
                info(f"{yellow(label)}: {out.stdout.strip()}")
            except Exception as e:
                info(f"{red(label)}: exec failed - {e}")

        # ── STEP 4: Test detect_source ──────────────────────────────────
        step("Testing detect_source()")

        record(test_detect_source(dirs))

        # ── STEP 5: Test uninstall commands ─────────────────────────────
        step("Testing get_uninstall_commands()")

        record(test_uninstall_commands())

        # ── STEP 6: Test install suggestions ────────────────────────────
        step("Testing suggest_install_commands()")

        record(test_suggest_install())

        # ── STEP 7: Run conflict detection (expect conflict) ────────────
        step("Running conflict detection (multi-install)")

        info("Calling run_conflict_check() with all 4 sources in PATH...")
        info("Expected: conflict detected, sys.exit(1)")
        divider()

        conflict_detected = test_conflict_detection()
        divider()
        if conflict_detected:
            ok("Conflict correctly detected and reported")
        else:
            fail("Conflict was NOT detected (expected exit code 1)")
        record(conflict_detected)

        # ── STEP 8: Test single-install scenario ────────────────────────
        step("Testing single install (no conflict)")

        # Keep only one source in PATH, stripping any real portrm locations
        single_label = "brew"
        # Build a clean PATH: fake brew dir + original dirs that do NOT contain portrm/ptrm
        clean_entries = []
        for entry in original_path.split(os.pathsep):
            has_portrm = any(
                os.path.isfile(os.path.join(entry, name))
                for name in ("portrm", "ptrm", "portrm.exe", "ptrm.exe")
            )
            if not has_portrm:
                clean_entries.append(entry)
        single_path = dirs[single_label] + os.pathsep + os.pathsep.join(clean_entries)
        os.environ["PATH"] = single_path
        info(f"PATH now contains only: {yellow(single_label)} ({dim(dirs[single_label])})")
        info(f"Stripped {len(original_path.split(os.pathsep)) - len(clean_entries)} real portrm dirs from PATH")

        # Clear any cached module state
        found_single = run_which_all()
        info(f"Binaries visible: {len(found_single)}")
        for p in found_single:
            info(f"  {cyan(p)}")
        divider()

        no_conflict = test_no_conflict()
        if no_conflict:
            ok("No conflict detected with single installation")
        else:
            fail("Unexpected conflict detected with single installation")
        record(no_conflict)

        # ── STEP 9: Test npx bypass ─────────────────────────────────────
        step("Testing npx bypass")

        # Restore all sources + add fake npx marker to PATH
        os.environ["PATH"] = test_path
        os.environ["npm_execpath"] = "/tmp/_npx/some/path"
        info("Set npm_execpath to simulate npx context")

        from portrm.conflict import _is_npx_context
        npx_detected = _is_npx_context()
        if npx_detected:
            ok("npx context correctly detected - conflict check would be skipped")
        else:
            fail("npx context NOT detected")
        record(npx_detected)

        # Clean up npx env
        del os.environ["npm_execpath"]

        # ── STEP 10: Test environment detection ─────────────────────────
        step("Testing detect_available_tools()")

        os.environ["PATH"] = test_path
        from portrm.conflict import detect_available_tools
        tools = detect_available_tools()
        info(f"Detected tools: {tools}")
        # Should be a dict with bool values
        tools_ok = isinstance(tools, dict) and all(isinstance(v, bool) for v in tools.values())
        if tools_ok:
            for name, avail in tools.items():
                status = green("available") if avail else dim("not found")
                info(f"  {name}: {status}")
            ok("Environment detection returned valid results")
        else:
            fail(f"Invalid tools result: {tools}")
        record(tools_ok)

        # ── STEP 11: Test smart suggestions filter unavailable tools ────
        step("Testing smart install suggestions")

        from portrm.conflict import suggest_install_commands
        recommended, alternatives = suggest_install_commands()
        info(f"Recommended: {cyan(recommended)}")
        for alt in alternatives:
            info(f"  Alternative: {dim(alt)}")
        # Verify none suggest a tool that isn't installed (except curl which is always ok)
        all_suggestions = [recommended] + alternatives
        suggestion_ok = True
        for cmd in all_suggestions:
            # Extract tool name from command
            tool = cmd.split()[0] if cmd else ""
            if tool in ("curl",):
                continue  # curl is always available
            tool_key = tool if tool != "brew" else "brew"
            if tool_key in tools and not tools[tool_key]:
                fail(f"Suggested unavailable tool: {cmd}")
                suggestion_ok = False
        if suggestion_ok:
            ok("All suggestions use available tools")
        record(suggestion_ok)

        # ── STEP 12: Test doctor output ─────────────────────────────────
        step("Testing run_doctor()")

        from portrm.conflict import run_doctor
        import io
        old_stderr = sys.stderr
        sys.stderr = captured = io.StringIO()
        try:
            run_doctor()
        finally:
            sys.stderr = old_stderr
        doctor_output = captured.getvalue()
        # Verify key sections are present
        checks = [
            ("portrm doctor" in doctor_output, "header"),
            ("System:" in doctor_output, "system info"),
            ("Active binary:" in doctor_output, "active binary"),
            ("Installations found:" in doctor_output, "installations list"),
            ("Conflict status:" in doctor_output, "conflict status"),
            ("Available runtimes:" in doctor_output, "runtimes"),
            ("Recommended install:" in doctor_output, "install suggestion"),
        ]
        doctor_ok = True
        for present, section_name in checks:
            if present:
                ok(f"Doctor output contains: {section_name}")
            else:
                fail(f"Doctor output missing: {section_name}")
                doctor_ok = False
        record(doctor_ok)

        # ── STEP 13: Cleanup ────────────────────────────────────────────
        step("Cleaning up")

        os.environ["PATH"] = original_path
        ok("PATH restored to original")

        shutil.rmtree(base_dir, ignore_errors=True)
        if not os.path.exists(base_dir):
            ok(f"Removed {dim(base_dir)}")
        else:
            fail(f"Failed to remove {base_dir}")

        # Verify cleanup
        post_cleanup = run_which_all()
        test_dirs_in_path = [p for p in post_cleanup if base_dir in p]
        if not test_dirs_in_path:
            ok("No test binaries remain in PATH")
        else:
            fail(f"Test binaries still found: {test_dirs_in_path}")

        # Rehash (for zsh/bash)
        try:
            subprocess.run(["hash", "-r"], capture_output=True, timeout=2)
        except Exception:
            pass

    except Exception as e:
        print(f"\n  {red('FATAL:')} {e}")
        import traceback
        traceback.print_exc()
        failed += 1
    finally:
        # Safety: always restore PATH
        os.environ["PATH"] = original_path
        if os.path.exists(base_dir):
            shutil.rmtree(base_dir, ignore_errors=True)

    # ── Summary ──────────────────────────────────────────────────────────
    print(f"\n{'=' * 60}")
    print(f"  {bold('RESULTS')}")
    print(f"{'=' * 60}\n")

    if failed == 0:
        print(f"  {green('✔')} Test completed")
        print(f"  {green('✔')} Conflict detection working")
        print(f"  {green('✔')} Environment cleaned successfully")
        print(f"\n  {bold(green(f'{passed}/{total} tests passed'))}\n")
        return 0
    else:
        print(f"  {green('✔') if passed else red('✖')} {passed} passed")
        print(f"  {red('✖')} {failed} failed")
        print(f"\n  {bold(red(f'{passed}/{total} tests passed'))}\n")
        return 1


if __name__ == "__main__":
    sys.exit(main())
