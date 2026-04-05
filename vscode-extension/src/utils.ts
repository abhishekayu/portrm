import * as vscode from "vscode";
import { exec } from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";

// ── Types matching CLI JSON output ─────────────────────────────────

export interface ServiceStatus {
  name: string;
  port: number;
  status: "running" | "stopped" | "conflict";
  process?: string;
  pid?: number;
  docker?: string;
}

export interface PortInfo {
  port: number;
  protocol: string;
  process?: {
    pid: number;
    name: string;
    command: string;
    user?: string;
    working_dir?: string;
    cpu_usage?: number;
    memory_bytes?: number;
    runtime?: number;
  };
  service?: {
    kind: string;
    confidence: number;
    restart_hint?: string;
  };
  docker_container?: {
    id: string;
    name: string;
    image: string;
    status: string;
  };
}

// ── CLI Wrapper ────────────────────────────────────────────────────

const OUTPUT_CHANNEL_NAME = "Portrm";
let _outputChannel: vscode.OutputChannel | undefined;

export function getOutputChannel(): vscode.OutputChannel {
  if (!_outputChannel) {
    _outputChannel = vscode.window.createOutputChannel(OUTPUT_CHANNEL_NAME);
  }
  return _outputChannel;
}

/** Resolve the ptrm binary path. Checks common locations on macOS/Linux. */
let _resolvedBinary: string | undefined;
function resolvePtrm(): string {
  if (_resolvedBinary) {
    return _resolvedBinary;
  }
  // Check VS Code setting first
  const configured = vscode.workspace.getConfiguration("ptrm").get<string>("binaryPath");
  if (configured && configured !== "ptrm") {
    if (fs.existsSync(configured)) {
      _resolvedBinary = configured;
      return configured;
    }
  }
  // Check common install locations (Extension Host may have limited PATH)
  const candidates = [
    "/usr/local/bin/ptrm",
    path.join(os.homedir(), ".cargo/bin/ptrm"),
    "/opt/homebrew/bin/ptrm",
    "/usr/bin/ptrm",
  ];
  // Also check workspace target/release
  const root = getWorkspaceRoot();
  if (root) {
    candidates.unshift(path.join(root, "target/release/ptrm"));
  }
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      _resolvedBinary = candidate;
      console.log(`[Ptrm] Resolved binary: ${candidate}`);
      return candidate;
    }
  }
  // Fallback to bare name (rely on PATH)
  return "ptrm";
}

/** Reset cached binary path (e.g. after settings change). */
export function resetBinaryCache(): void {
  _resolvedBinary = undefined;
}

/**
 * Run a ptrm CLI command and return raw stdout.
 * Executes in the workspace root directory.
 */
export function runPtrm(args: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const cwd = getWorkspaceRoot() ?? os.homedir();
    const bin = resolvePtrm();
    const cmd = `${bin} ${args}`;
    console.log(`[Ptrm] exec: ${cmd} (cwd: ${cwd})`);
    exec(cmd, { cwd, timeout: 15_000 }, (err, stdout, stderr) => {
      if (err) {
        const msg = stderr?.trim() || err.message;
        console.error(`[Ptrm] exec error: ${msg}`);
        return reject(new Error(msg));
      }
      resolve(stdout);
    });
  });
}

/**
 * Run a ptrm command and parse JSON output.
 */
export async function runPtrmJson<T>(args: string): Promise<T> {
  const raw = await runPtrm(args);
  return JSON.parse(raw) as T;
}

const TERMINAL_NAME = "Ptrm";
let _sharedTerminal: vscode.Terminal | undefined;
let _terminalMode: "idle" | "interactive" | "busy" = "idle";

/** Get or create the single shared Ptrm terminal. */
function getSharedTerminal(): vscode.Terminal {
  // Check if the existing terminal is still alive
  if (_sharedTerminal) {
    const alive = vscode.window.terminals.find((t) => t === _sharedTerminal);
    if (!alive) {
      _sharedTerminal = undefined;
      _terminalMode = "idle";
    }
  }
  if (!_sharedTerminal) {
    const cwd = getWorkspaceRoot() ?? os.homedir();
    _sharedTerminal = vscode.window.createTerminal({ name: TERMINAL_NAME, cwd });
    _terminalMode = "idle";
  }
  return _sharedTerminal;
}

/**
 * Run a ptrm command in the shared VS Code terminal.
 * Handles cleanup of any running command before sending the new one:
 * - If interactive TUI is running, sends "q" to exit first
 * - If another command is running (e.g. fix with Y/n prompt), sends Ctrl+C first
 */
export function runInTerminal(_label: string, command: string): vscode.Terminal {
  const terminal = getSharedTerminal();
  terminal.show();

  if (_terminalMode === "interactive") {
    // Exit TUI with "q", then run new command after a short delay
    terminal.sendText("q", true);
    setTimeout(() => {
      terminal.sendText(command);
    }, 500);
  } else if (_terminalMode === "busy") {
    // Interrupt current command (Ctrl+C), then run new command
    terminal.sendText("\x03", false); // Ctrl+C
    setTimeout(() => {
      terminal.sendText(command);
    }, 300);
  } else {
    terminal.sendText(command);
  }

  // Track mode based on command
  if (command.includes("interactive")) {
    _terminalMode = "interactive";
  } else {
    _terminalMode = "busy";
    // After a reasonable timeout, assume command finished
    setTimeout(() => {
      if (_terminalMode === "busy") { _terminalMode = "idle"; }
    }, 30_000);
  }

  return terminal;
}

// ── Workspace helpers ──────────────────────────────────────────────

export function getWorkspaceRoot(): string | undefined {
  return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

export function getConfigPath(): string | undefined {
  const root = getWorkspaceRoot();
  if (!root) {
    return undefined;
  }
  return path.join(root, ".ptrm.toml");
}

export function hasConfig(): boolean {
  const configPath = getConfigPath();
  return !!configPath && fs.existsSync(configPath);
}

/**
 * Read the project name from .ptrm.toml (quick regex parse, no TOML dep).
 */
export function getProjectName(): string | undefined {
  const configPath = getConfigPath();
  if (!configPath || !fs.existsSync(configPath)) {
    return undefined;
  }
  try {
    const content = fs.readFileSync(configPath, "utf-8");
    const match = content.match(/^\s*name\s*=\s*"([^"]+)"/m);
    return match?.[1];
  } catch {
    return undefined;
  }
}

// ── Display helpers ────────────────────────────────────────────────

export function formatPort(port: number): string {
  return String(port);
}

export function formatServiceLabel(info: PortInfo): string {
  if (info.docker_container) {
    return `docker:${info.docker_container.name}`;
  }
  if (info.service && info.service.kind !== "Unknown") {
    return info.service.kind;
  }
  return info.process?.name ?? "unknown";
}

export function formatMemory(bytes: number | undefined): string {
  if (!bytes) {
    return "";
  }
  const GB = 1024 * 1024 * 1024;
  const MB = 1024 * 1024;
  const KB = 1024;
  if (bytes >= GB) {
    return `${(bytes / GB).toFixed(1)} GB`;
  }
  if (bytes >= MB) {
    return `${(bytes / MB).toFixed(1)} MB`;
  }
  if (bytes >= KB) {
    return `${(bytes / KB).toFixed(0)} KB`;
  }
  return `${bytes} B`;
}

export function formatUptime(seconds: number | undefined): string {
  if (!seconds || seconds <= 0) {
    return "";
  }
  const s = Math.floor(seconds);
  if (s < 60) {
    return `${s}s`;
  }
  if (s < 3600) {
    return `${Math.floor(s / 60)}m ${s % 60}s`;
  }
  if (s < 86400) {
    return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
  }
  return `${Math.floor(s / 86400)}d ${Math.floor((s % 86400) / 3600)}h`;
}
