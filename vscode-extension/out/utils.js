"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.getOutputChannel = getOutputChannel;
exports.resetBinaryCache = resetBinaryCache;
exports.runPtrm = runPtrm;
exports.runPtrmJson = runPtrmJson;
exports.runInTerminal = runInTerminal;
exports.getWorkspaceRoot = getWorkspaceRoot;
exports.getConfigPath = getConfigPath;
exports.hasConfig = hasConfig;
exports.getProjectName = getProjectName;
exports.formatPort = formatPort;
exports.formatServiceLabel = formatServiceLabel;
exports.formatMemory = formatMemory;
const vscode = __importStar(require("vscode"));
const child_process_1 = require("child_process");
const fs = __importStar(require("fs"));
const path = __importStar(require("path"));
const os = __importStar(require("os"));
// ── CLI Wrapper ────────────────────────────────────────────────────
const OUTPUT_CHANNEL_NAME = "Ptrm";
let _outputChannel;
function getOutputChannel() {
    if (!_outputChannel) {
        _outputChannel = vscode.window.createOutputChannel(OUTPUT_CHANNEL_NAME);
    }
    return _outputChannel;
}
/** Resolve the ptrm binary path. Checks common locations on macOS/Linux. */
let _resolvedBinary;
function resolvePtrm() {
    if (_resolvedBinary) {
        return _resolvedBinary;
    }
    // Check VS Code setting first
    const configured = vscode.workspace.getConfiguration("ptrm").get("binaryPath");
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
function resetBinaryCache() {
    _resolvedBinary = undefined;
}
/**
 * Run a ptrm CLI command and return raw stdout.
 * Executes in the workspace root directory.
 */
function runPtrm(args) {
    return new Promise((resolve, reject) => {
        const cwd = getWorkspaceRoot() ?? os.homedir();
        const bin = resolvePtrm();
        const cmd = `${bin} ${args}`;
        console.log(`[Ptrm] exec: ${cmd} (cwd: ${cwd})`);
        (0, child_process_1.exec)(cmd, { cwd, timeout: 15000 }, (err, stdout, stderr) => {
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
async function runPtrmJson(args) {
    const raw = await runPtrm(args);
    return JSON.parse(raw);
}
const TERMINAL_NAME = "Ptrm";
let _sharedTerminal;
let _terminalMode = "idle";
/** Get or create the single shared Ptrm terminal. */
function getSharedTerminal() {
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
function runInTerminal(_label, command) {
    const terminal = getSharedTerminal();
    terminal.show();
    if (_terminalMode === "interactive") {
        // Exit TUI with "q", then run new command after a short delay
        terminal.sendText("q", true);
        setTimeout(() => {
            terminal.sendText(command);
        }, 500);
    }
    else if (_terminalMode === "busy") {
        // Interrupt current command (Ctrl+C), then run new command
        terminal.sendText("\x03", false); // Ctrl+C
        setTimeout(() => {
            terminal.sendText(command);
        }, 300);
    }
    else {
        terminal.sendText(command);
    }
    // Track mode based on command
    if (command.includes("interactive")) {
        _terminalMode = "interactive";
    }
    else {
        _terminalMode = "busy";
        // After a reasonable timeout, assume command finished
        setTimeout(() => {
            if (_terminalMode === "busy") {
                _terminalMode = "idle";
            }
        }, 30000);
    }
    return terminal;
}
// ── Workspace helpers ──────────────────────────────────────────────
function getWorkspaceRoot() {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}
function getConfigPath() {
    const root = getWorkspaceRoot();
    if (!root) {
        return undefined;
    }
    return path.join(root, ".ptrm.toml");
}
function hasConfig() {
    const configPath = getConfigPath();
    return !!configPath && fs.existsSync(configPath);
}
/**
 * Read the project name from .ptrm.toml (quick regex parse, no TOML dep).
 */
function getProjectName() {
    const configPath = getConfigPath();
    if (!configPath || !fs.existsSync(configPath)) {
        return undefined;
    }
    try {
        const content = fs.readFileSync(configPath, "utf-8");
        const match = content.match(/^\s*name\s*=\s*"([^"]+)"/m);
        return match?.[1];
    }
    catch {
        return undefined;
    }
}
// ── Display helpers ────────────────────────────────────────────────
function formatPort(port) {
    return String(port);
}
function formatServiceLabel(info) {
    if (info.docker_container) {
        return `docker:${info.docker_container.name}`;
    }
    if (info.service && info.service.kind !== "Unknown") {
        return info.service.kind;
    }
    return info.process?.name ?? "unknown";
}
function formatMemory(bytes) {
    if (!bytes) {
        return "";
    }
    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(0)} KB`;
    }
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
//# sourceMappingURL=utils.js.map