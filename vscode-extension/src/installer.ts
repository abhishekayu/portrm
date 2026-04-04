import * as vscode from "vscode";
import * as os from "os";
import * as fs from "fs";
import * as path from "path";
import * as https from "https";
import { exec } from "child_process";

const REPO = "abhishekayu/portrm";
const BINARY = "ptrm";
const API_URL = `https://api.github.com/repos/${REPO}/releases/latest`;

// ── Platform helpers ───────────────────────────────────────────────

function getTarget(): string | undefined {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === "darwin" && arch === "arm64") { return "ptrm-darwin-arm64"; }
  if (platform === "darwin" && arch === "x64") { return "ptrm-darwin-amd64"; }
  if (platform === "linux" && arch === "x64") { return "ptrm-linux-amd64"; }
  if (platform === "linux" && arch === "arm64") { return "ptrm-linux-arm64"; }
  if (platform === "win32" && arch === "x64") { return "ptrm-windows-amd64"; }
  return undefined;
}

function getInstallDir(): string {
  const platform = os.platform();
  if (platform === "win32") {
    const appData = process.env.LOCALAPPDATA ?? path.join(os.homedir(), "AppData", "Local");
    const dir = path.join(appData, "ptrm", "bin");
    if (!fs.existsSync(dir)) { fs.mkdirSync(dir, { recursive: true }); }
    return dir;
  }
  // macOS / Linux
  const usr = "/usr/local/bin";
  try {
    fs.accessSync(usr, fs.constants.W_OK);
    return usr;
  } catch {
    const dir = path.join(os.homedir(), ".local", "bin");
    if (!fs.existsSync(dir)) { fs.mkdirSync(dir, { recursive: true }); }
    return dir;
  }
}

// ── HTTP helpers ───────────────────────────────────────────────────

function httpsGet(url: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const req = https.get(url, { headers: { "User-Agent": "portrm-vscode" } }, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        const loc = res.headers.location;
        if (loc) { return httpsGet(loc).then(resolve, reject); }
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`HTTP ${res.statusCode}`));
      }
      let data = "";
      res.on("data", (chunk) => (data += chunk));
      res.on("end", () => resolve(data));
    });
    req.on("error", reject);
    req.setTimeout(15_000, () => { req.destroy(); reject(new Error("timeout")); });
  });
}

function downloadFile(url: string, dest: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const follow = (u: string) => {
      https.get(u, { headers: { "User-Agent": "portrm-vscode" } }, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          const loc = res.headers.location;
          if (loc) { return follow(loc); }
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode}`));
        }
        const file = fs.createWriteStream(dest);
        res.pipe(file);
        file.on("finish", () => { file.close(); resolve(); });
        file.on("error", reject);
      }).on("error", reject);
    };
    follow(url);
  });
}

// ── Version helpers ────────────────────────────────────────────────

export async function getLatestVersion(): Promise<string> {
  const body = await httpsGet(API_URL);
  const json = JSON.parse(body);
  const tag: string = json.tag_name;
  return tag.startsWith("v") ? tag.slice(1) : tag;
}

export function getInstalledVersion(): Promise<string | undefined> {
  return new Promise((resolve) => {
    exec(`${BINARY} --version`, { timeout: 5_000 }, (err, stdout) => {
      if (err) { return resolve(undefined); }
      const match = stdout.trim().match(/(\d+\.\d+\.\d+)/);
      resolve(match?.[1]);
    });
  });
}

export function isInstalled(): boolean {
  const candidates = [
    "/usr/local/bin/ptrm",
    "/opt/homebrew/bin/ptrm",
    path.join(os.homedir(), ".local/bin/ptrm"),
    path.join(os.homedir(), ".cargo/bin/ptrm"),
  ];
  if (os.platform() === "win32") {
    const appData = process.env.LOCALAPPDATA ?? path.join(os.homedir(), "AppData", "Local");
    candidates.push(path.join(appData, "ptrm", "bin", "ptrm.exe"));
  }
  return candidates.some((p) => fs.existsSync(p));
}

// ── Install / Update ───────────────────────────────────────────────

export async function installOrUpdate(version: string): Promise<string> {
  const target = getTarget();
  if (!target) {
    throw new Error(`Unsupported platform: ${os.platform()} ${os.arch()}`);
  }

  const isWin = os.platform() === "win32";
  const ext = isWin ? "zip" : "tar.gz";
  const url = `https://github.com/${REPO}/releases/download/v${version}/${target}.${ext}`;
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "ptrm-"));
  const archive = path.join(tmpDir, `ptrm.${ext}`);

  await vscode.window.withProgress(
    { location: vscode.ProgressLocation.Notification, title: `Installing portrm v${version}...`, cancellable: false },
    async (progress) => {
      progress.report({ message: "Downloading..." });
      await downloadFile(url, archive);

      progress.report({ message: "Extracting..." });
      const installDir = getInstallDir();

      if (isWin) {
        await execPromise(`powershell -Command "Expand-Archive -Path '${archive}' -DestinationPath '${tmpDir}' -Force"`, tmpDir);
        const src = path.join(tmpDir, "ptrm.exe");
        const dest = path.join(installDir, "ptrm.exe");
        fs.copyFileSync(src, dest);
      } else {
        await execPromise(`tar xzf "${archive}" -C "${tmpDir}"`, tmpDir);
        const src = path.join(tmpDir, BINARY);
        const dest = path.join(installDir, BINARY);
        fs.copyFileSync(src, dest);
        fs.chmodSync(dest, 0o755);
      }

      progress.report({ message: "Done!" });
      fs.rmSync(tmpDir, { recursive: true, force: true });
      return installDir;
    }
  );

  const installDir = getInstallDir();
  return path.join(installDir, isWin ? "ptrm.exe" : BINARY);
}

// ── Check on activation ────────────────────────────────────────────

export async function ensureInstalled(): Promise<boolean> {
  const installed = isInstalled();
  const currentVersion = await getInstalledVersion();

  if (!installed || !currentVersion) {
    const action = await vscode.window.showWarningMessage(
      "portrm CLI is not installed. Install it now?",
      "Install",
      "Later"
    );
    if (action !== "Install") { return false; }

    try {
      const latest = await getLatestVersion();
      const binPath = await installOrUpdate(latest);
      vscode.window.showInformationMessage(`portrm v${latest} installed to ${binPath}`);
      return true;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      vscode.window.showErrorMessage(`Failed to install portrm: ${msg}`);
      return false;
    }
  }

  return true;
}

export async function checkForUpdate(): Promise<void> {
  try {
    const current = await getInstalledVersion();
    if (!current) { return; }

    const latest = await getLatestVersion();
    if (latest === current) { return; }

    // Simple semver compare
    const cParts = current.split(".").map(Number);
    const lParts = latest.split(".").map(Number);
    const isNewer = lParts[0] > cParts[0]
      || (lParts[0] === cParts[0] && lParts[1] > cParts[1])
      || (lParts[0] === cParts[0] && lParts[1] === cParts[1] && lParts[2] > cParts[2]);

    if (!isNewer) { return; }

    const action = await vscode.window.showInformationMessage(
      `portrm v${latest} is available (current: v${current}). Update now?`,
      "Update",
      "Later"
    );
    if (action !== "Update") { return; }

    const binPath = await installOrUpdate(latest);
    vscode.window.showInformationMessage(`ptrm updated to v${latest} at ${binPath}`);
  } catch {
    // Silently ignore update check failures
  }
}

// ── Helpers ────────────────────────────────────────────────────────

function execPromise(cmd: string, cwd: string): Promise<string> {
  return new Promise((resolve, reject) => {
    exec(cmd, { cwd, timeout: 30_000 }, (err, stdout) => {
      if (err) { return reject(err); }
      resolve(stdout);
    });
  });
}
