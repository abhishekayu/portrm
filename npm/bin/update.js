/**
 * Auto-update check for portrm (Node.js).
 *
 * Checks GitHub releases for a newer version once per 24 hours (cached).
 * If found, auto-updates using npm.
 */

"use strict";

const https = require("https");
const os = require("os");
const path = require("path");
const fs = require("fs");
const { execSync } = require("child_process");

const GITHUB_API =
  "https://api.github.com/repos/abhishekayu/portrm/releases/latest";
const CHECK_INTERVAL = 86_400; // 24 hours

// ── ANSI helpers ────────────────────────────────────────────────────

const NO_COLOR = "NO_COLOR" in process.env || !process.stderr.isTTY;
const cyan = (s) => (NO_COLOR ? s : `\x1b[36m${s}\x1b[0m`);
const bold = (s) => (NO_COLOR ? s : `\x1b[1m${s}\x1b[0m`);
const green = (s) => (NO_COLOR ? s : `\x1b[1;32m${s}\x1b[0m`);
const red = (s) => (NO_COLOR ? s : `\x1b[1;31m${s}\x1b[0m`);
const dim = (s) => (NO_COLOR ? s : `\x1b[2m${s}\x1b[0m`);

// ── Helpers ─────────────────────────────────────────────────────────

function stateFile() {
  return path.join(os.homedir(), ".portrm", "last_update_check");
}

function lastCheckTime() {
  try {
    return parseInt(fs.readFileSync(stateFile(), "utf8").trim(), 10) || 0;
  } catch {
    return 0;
  }
}

function saveCheckTime() {
  const p = stateFile();
  try {
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, String(Math.floor(Date.now() / 1000)));
  } catch {
    // ignore
  }
}

function isNewer(current, latest) {
  const parse = (v) => v.replace(/^v/, "").split(".").map(Number);
  const c = parse(current);
  const l = parse(latest);
  for (let i = 0; i < 3; i++) {
    if ((l[i] || 0) > (c[i] || 0)) return true;
    if ((l[i] || 0) < (c[i] || 0)) return false;
  }
  return false;
}

function fetchLatestVersionSync() {
  try {
    // Use curl for a simple synchronous HTTP call
    const out = execSync(
      'curl -sS -H "Accept: application/vnd.github.v3+json" -H "User-Agent: ptrm-update-check" "' +
        GITHUB_API +
        '"',
      { encoding: "utf8", timeout: 10000, stdio: ["pipe", "pipe", "pipe"] },
    );
    const data = JSON.parse(out);
    return (data.tag_name || "").replace(/^v/, "");
  } catch {
    return null;
  }
}

/**
 * Check for updates and auto-update if a newer version is available.
 * @param {string} currentVersion - The current installed version.
 */
function checkAndUpdate(currentVersion) {
  if (process.env.PTRM_SKIP_UPDATE_CHECK === "1") return;

  const now = Math.floor(Date.now() / 1000);
  if (now - lastCheckTime() < CHECK_INTERVAL) return;

  saveCheckTime();

  const latest = fetchLatestVersionSync();
  if (!latest) return;
  if (!isNewer(currentVersion, latest)) return;

  const cmd = "npm update -g portrm";

  process.stderr.write("\n");
  process.stderr.write(
    `  ${cyan("⬆")} ${bold("Updating portrm to")} ${cyan(latest)} (you have ${dim(currentVersion)})\n`,
  );
  process.stderr.write("\n");
  process.stderr.write(`  ${dim("$")} ${cyan(cmd)}\n`);
  process.stderr.write("\n");

  try {
    execSync(cmd, { stdio: "inherit", timeout: 120_000 });
    process.stderr.write(
      `  ${green("✔ Updated successfully! Restart ptrm to use the new version.")}\n\n`,
    );
  } catch {
    process.stderr.write(
      `  ${red("✖ Auto-update failed.")} Update manually: ${cmd}\n\n`,
    );
  }
}

module.exports = { checkAndUpdate };
