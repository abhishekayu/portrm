#!/usr/bin/env node

"use strict";

const { execFileSync } = require("child_process");
const path = require("path");
const fs = require("fs");
const { runConflictCheck } = require("./conflict");
const { checkAndUpdate } = require("./update");

runConflictCheck();

// Auto-update check (once per 24h, cached)
const pkg = require("../package.json");
checkAndUpdate(pkg.version);

const binDir = path.join(__dirname);

function getBinaryPath() {
  const platform = process.platform;
  const arch = process.arch;

  let binaryName = "ptrm";
  if (platform === "win32") {
    binaryName = "ptrm.exe";
  }

  const binaryPath = path.join(binDir, binaryName);
  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }

  console.error(
    `ptrm binary not found for ${platform}-${arch}.`,
    `\nRun 'npm rebuild portrm' or install via: curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh | sh`,
  );
  process.exit(1);
}

const binary = getBinaryPath();
const args = process.argv.slice(2);

try {
  const result = execFileSync(binary, args, {
    stdio: "inherit",
    env: process.env,
  });
} catch (err) {
  if (err.status !== undefined) {
    process.exit(err.status);
  }
  process.exit(1);
}
