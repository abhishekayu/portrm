"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const os = require("os");

const VERSION = require("./package.json").version;
const REPO = "abhishekayu/portrm";

function getPlatformInfo() {
  const platform = process.platform;
  const arch = process.arch;

  const platformMap = {
    "darwin-x64": "portrm-darwin-amd64",
    "darwin-arm64": "portrm-darwin-arm64",
    "linux-x64": "portrm-linux-amd64",
    "linux-arm64": "portrm-linux-arm64",
    "win32-x64": "portrm-windows-amd64",
  };

  const key = `${platform}-${arch}`;
  const name = platformMap[key];

  if (!name) {
    console.error(`Unsupported platform: ${key}`);
    console.error("Install manually: cargo install portrm");
    process.exit(0); // Don't fail npm install
  }

  const ext = platform === "win32" ? ".zip" : ".tar.gz";
  return { name, ext, platform };
}

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        if (
          res.statusCode >= 300 &&
          res.statusCode < 400 &&
          res.headers.location
        ) {
          return download(res.headers.location).then(resolve).catch(reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function install() {
  const { name, ext, platform } = getPlatformInfo();
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${name}${ext}`;
  const binDir = path.join(__dirname, "bin");

  console.log(
    `Downloading ptrm v${VERSION} for ${process.platform}-${process.arch}...`,
  );

  try {
    const data = await download(url);
    const tmpFile = path.join(os.tmpdir(), `${name}${ext}`);
    fs.writeFileSync(tmpFile, data);

    if (platform === "win32") {
      // Use PowerShell to extract zip
      execSync(
        `powershell -Command "Expand-Archive -Force '${tmpFile}' '${binDir}'"`,
        { stdio: "pipe" },
      );
    } else {
      execSync(`tar xzf "${tmpFile}" -C "${binDir}"`, { stdio: "pipe" });
      const binary = path.join(binDir, "ptrm");
      fs.chmodSync(binary, 0o755);
    }

    fs.unlinkSync(tmpFile);
    console.log("portrm installed successfully.");
  } catch (err) {
    console.error(`Failed to download portrm: ${err.message}`);
    console.error("Install manually: cargo install portrm");
    // Don't fail npm install
  }
}

install();
