/**
 * Runtime conflict detection for portrm / ptrm (Node.js).
 *
 * Mirrors the Python conflict.py module. Detects multiple installations
 * across package managers and blocks execution with actionable guidance.
 */

"use strict";

const { execSync } = require("child_process");
const os = require("os");
const path = require("path");

// ── ANSI helpers ────────────────────────────────────────────────────────────

const NO_COLOR = "NO_COLOR" in process.env || !process.stderr.isTTY;

const red = (s) => (NO_COLOR ? s : `\x1b[1;31m${s}\x1b[0m`);
const yellow = (s) => (NO_COLOR ? s : `\x1b[1;33m${s}\x1b[0m`);
const dim = (s) => (NO_COLOR ? s : `\x1b[2m${s}\x1b[0m`);
const bold = (s) => (NO_COLOR ? s : `\x1b[1m${s}\x1b[0m`);
const cyan = (s) => (NO_COLOR ? s : `\x1b[36m${s}\x1b[0m`);

// ── Source detection ────────────────────────────────────────────────────────

const SOURCE_PATTERNS = [
  {
    patterns: ["homebrew", "/opt/homebrew", "Cellar", "linuxbrew"],
    label: "brew",
  },
  { patterns: [".cargo/bin"], label: "cargo" },
  { patterns: ["site-packages", ".local", "python", "Python"], label: "pip" },
  {
    patterns: ["node_modules", "/npm/", "/npx/", "AppData/Roaming/npm", "_npx"],
    label: "npm",
  },
];

function detectSource(binPath) {
  const normalised = binPath.replace(/\\/g, "/").toLowerCase();
  for (const { patterns, label } of SOURCE_PATTERNS) {
    for (const pat of patterns) {
      if (normalised.includes(pat.toLowerCase())) {
        return label;
      }
    }
  }
  return "unknown";
}

// ── npx detection ───────────────────────────────────────────────────────────

function isNpxContext() {
  const hints = [
    process.argv[1] || "",
    process.env.npm_execpath || "",
    process.env.npm_lifecycle_event || "",
    process.env.npm_config_cache || "",
    process.env.PATH || "",
  ];
  for (const h of hints) {
    const lower = h.replace(/\\/g, "/").toLowerCase();
    if (lower.includes("_npx") || lower.includes("/npx/")) {
      return true;
    }
  }
  return false;
}

// ── Binary discovery ────────────────────────────────────────────────────────

function whichAll(name) {
  const paths = [];
  try {
    const cmd =
      process.platform === "win32" ? `where ${name}` : `which -a ${name}`;
    const out = execSync(cmd, {
      encoding: "utf8",
      timeout: 5000,
      stdio: ["pipe", "pipe", "pipe"],
    });
    for (const line of out.split("\n")) {
      const trimmed = line.trim();
      if (trimmed) paths.push(trimmed);
    }
  } catch {
    // command not found or error — ignore
  }
  return paths;
}

function findAllBinaries() {
  const raw = new Set();
  for (const name of ["portrm", "ptrm"]) {
    for (const p of whichAll(name)) {
      raw.add(p);
    }
  }

  // Resolve symlinks and deduplicate
  const resolved = new Map();
  for (const p of raw) {
    let real;
    try {
      const fs = require("fs");
      real = fs.realpathSync(p);
    } catch {
      real = p;
    }
    if (!resolved.has(real)) {
      resolved.set(real, p);
    }
  }
  return Array.from(resolved.values()).sort();
}

// ── Uninstall / install commands ────────────────────────────────────────────

const UNINSTALL_CMD = {
  brew: "brew uninstall portrm",
  pip: "pip uninstall portrm",
  cargo: "cargo uninstall portrm",
  npm: "npm uninstall -g portrm",
};

function getUninstallCommands(sources) {
  const seen = new Set();
  const cmds = [];
  for (const src of sources) {
    const cmd = UNINSTALL_CMD[src];
    if (cmd && !seen.has(cmd)) {
      seen.add(cmd);
      cmds.push(cmd);
    }
  }
  return cmds;
}

function suggestInstallCommands() {
  const system = os.platform();

  if (system === "darwin") {
    return {
      recommended: "brew install abhishekayu/tap/portrm",
      alternatives: [
        "npm install -g portrm",
        "pip install portrm",
        "cargo install portrm",
      ],
    };
  }
  if (system === "linux") {
    return {
      recommended:
        "curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh | sh",
      alternatives: [
        "npm install -g portrm",
        "pip install portrm",
        "cargo install portrm",
      ],
    };
  }
  if (system === "win32") {
    return {
      recommended: "npm install -g portrm",
      alternatives: ["pip install portrm", "cargo install portrm"],
    };
  }
  return { recommended: "cargo install portrm", alternatives: [] };
}

// ── Main check ──────────────────────────────────────────────────────────────

function printConflict(binaries, sources, uniqueSources) {
  const w = (s) => process.stderr.write(s);
  const home = os.homedir();

  w("\n");
  w(`  ${red("✖ Multiple portrm installations detected")}\n`);
  w("\n");

  // Active binary
  let active;
  try {
    const cmd = process.platform === "win32" ? "where ptrm" : "which ptrm";
    active = execSync(cmd, {
      encoding: "utf8",
      timeout: 3000,
      stdio: ["pipe", "pipe", "pipe"],
    })
      .split("\n")[0]
      .trim();
  } catch {
    // ignore
  }
  if (active) {
    w(`  ${dim("Active binary:")} ${cyan(active)}\n`);
    w("\n");
  }

  w(`  ${bold("Found:")}\n`);
  w("\n");
  for (let i = 0; i < binaries.length; i++) {
    const shortened = binaries[i].replace(home, "~");
    w(`    ${yellow("•")} ${shortened}  ${dim("(" + sources[i] + ")")}\n`);
  }
  w("\n");

  const uninstallCmds = getUninstallCommands(uniqueSources);
  if (uninstallCmds.length) {
    w(`  ${bold("Uninstall duplicates:")}\n`);
    w("\n");
    for (const cmd of uninstallCmds) {
      w(`    ${dim("$")} ${cmd}\n`);
    }
    w("\n");
  }

  const { recommended, alternatives } = suggestInstallCommands();
  w(`  ${bold("Install using ONE method:")}\n`);
  w("\n");
  w(`    Recommended:  ${cyan(recommended)}\n`);
  if (alternatives.length) {
    w(`    Alternative:  ${dim(alternatives[0])}\n`);
    for (let i = 1; i < alternatives.length; i++) {
      w(`                  ${dim(alternatives[i])}\n`);
    }
  }
  w("\n");
}

/**
 * Detect conflicting portrm installations and exit with code 1 if found.
 *
 * Safe to call from any entrypoint. Silently returns when:
 * - running via npx
 * - zero or one binary found
 * - detection itself fails
 */
function runConflictCheck() {
  try {
    if (isNpxContext()) return;

    const binaries = findAllBinaries();
    if (binaries.length <= 1) return;

    const sources = binaries.map(detectSource);
    const uniqueSources = [...new Set(sources)];

    // All from the same ecosystem — no real conflict
    if (uniqueSources.length <= 1) return;

    printConflict(binaries, sources, uniqueSources);
    process.exit(1);
  } catch {
    // Never crash the CLI due to conflict detection
  }
}

module.exports = {
  findAllBinaries,
  detectSource,
  getUninstallCommands,
  suggestInstallCommands,
  runConflictCheck,
};
