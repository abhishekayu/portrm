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
exports.registerCommands = registerCommands;
const vscode = __importStar(require("vscode"));
const utils_1 = require("./utils");
// ── Command handlers ───────────────────────────────────────────────
function registerCommands(context, provider, refreshFn) {
    context.subscriptions.push(vscode.commands.registerCommand("ptrm.refresh", refreshFn), vscode.commands.registerCommand("ptrm.restart", (item) => cmdRestart(item, refreshFn)), vscode.commands.registerCommand("ptrm.logs", (item) => cmdLogs(item)), vscode.commands.registerCommand("ptrm.kill", (item) => cmdKill(item, refreshFn)), vscode.commands.registerCommand("ptrm.fix", () => cmdFix(refreshFn)), vscode.commands.registerCommand("ptrm.up", () => cmdUp(refreshFn)), vscode.commands.registerCommand("ptrm.down", () => cmdDown(refreshFn)), vscode.commands.registerCommand("ptrm.init", () => cmdInit(refreshFn)), vscode.commands.registerCommand("ptrm.info", (item) => cmdInfo(item)), vscode.commands.registerCommand("ptrm.doctor", () => cmdDoctor(refreshFn)), vscode.commands.registerCommand("ptrm.watch", (item) => cmdWatch(item)), vscode.commands.registerCommand("ptrm.preflight", () => cmdPreflight()), vscode.commands.registerCommand("ptrm.interactive", () => cmdInteractive()), vscode.commands.registerCommand("ptrm.group", () => cmdGroup()), vscode.commands.registerCommand("ptrm.history", () => cmdHistory()), vscode.commands.registerCommand("ptrm.scanDev", () => cmdScanDev()), vscode.commands.registerCommand("ptrm.registry", () => cmdRegistry()), vscode.commands.registerCommand("ptrm.ci", () => cmdCi()), vscode.commands.registerCommand("ptrm.useProfile", () => cmdUseProfile(refreshFn)), vscode.commands.registerCommand("ptrm.update", () => cmdUpdate()), vscode.commands.registerCommand("ptrm.resetProfile", () => cmdResetProfile(refreshFn)));
}
// ── Restart ────────────────────────────────────────────────────────
async function cmdRestart(item, refreshFn) {
    let serviceName = item?.serviceName;
    if (!serviceName) {
        serviceName = await vscode.window.showInputBox({
            prompt: "Service name to restart",
            placeHolder: "e.g. frontend",
        });
    }
    if (!serviceName) {
        return;
    }
    const out = (0, utils_1.getOutputChannel)();
    out.appendLine(`[Portrm] Restarting ${serviceName}...`);
    try {
        const result = await (0, utils_1.runPtrm)(`restart ${serviceName}`);
        out.appendLine(result);
        vscode.window.showInformationMessage(`Ptrm: Restarted ${serviceName}`);
    }
    catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        out.appendLine(`ERROR: ${msg}`);
        vscode.window.showErrorMessage(`Ptrm: Failed to restart ${serviceName} - ${msg}`);
    }
    await refreshFn();
}
// ── Logs ───────────────────────────────────────────────────────────
async function cmdLogs(item) {
    let port = item?.port;
    if (!port) {
        const input = await vscode.window.showInputBox({
            prompt: "Port number for log streaming",
            placeHolder: "e.g. 3000",
        });
        if (!input) {
            return;
        }
        port = parseInt(input, 10);
        if (isNaN(port)) {
            vscode.window.showErrorMessage("Portrm: Invalid port number");
            return;
        }
    }
    const label = item?.name ? `ptrm log: ${item.name} (${port})` : `ptrm log: ${port}`;
    (0, utils_1.runInTerminal)(label, `ptrm log ${port}`);
}
// ── Kill ───────────────────────────────────────────────────────────
async function cmdKill(item, refreshFn) {
    let port = item?.port;
    if (!port) {
        const input = await vscode.window.showInputBox({
            prompt: "Port number to kill",
            placeHolder: "e.g. 3000",
        });
        if (!input) {
            return;
        }
        port = parseInt(input, 10);
        if (isNaN(port)) {
            vscode.window.showErrorMessage("Portrm: Invalid port number");
            return;
        }
    }
    const confirm = await vscode.window.showWarningMessage(`Kill the process on port ${port}?`, { modal: true }, "Kill");
    if (confirm !== "Kill") {
        return;
    }
    const out = (0, utils_1.getOutputChannel)();
    out.appendLine(`[Portrm] Killing process on port ${port}...`);
    try {
        const result = await (0, utils_1.runPtrm)(`kill ${port} -y`);
        out.appendLine(result);
        vscode.window.showInformationMessage(`Ptrm: Killed process on port ${port}`);
    }
    catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        out.appendLine(`ERROR: ${msg}`);
        vscode.window.showErrorMessage(`Ptrm: Failed to kill port ${port} - ${msg}`);
    }
    await refreshFn();
}
// ── Fix ────────────────────────────────────────────────────────────
async function cmdFix(refreshFn) {
    const out = (0, utils_1.getOutputChannel)();
    out.appendLine("[Portrm] Running fix (auto-detect)...");
    (0, utils_1.runInTerminal)("ptrm fix", "ptrm fix");
    setTimeout(() => refreshFn(), 3000);
}
// ── Up ─────────────────────────────────────────────────────────────
async function cmdUp(refreshFn) {
    if (!(0, utils_1.hasConfig)()) {
        vscode.window.showWarningMessage("Portrm: No .ptrm.toml found. Run ptrm init first.");
        return;
    }
    const out = (0, utils_1.getOutputChannel)();
    out.appendLine("[Portrm] Starting all services...");
    try {
        const result = await (0, utils_1.runPtrm)("up -y");
        out.appendLine(result);
        vscode.window.showInformationMessage("Portrm: All services started");
    }
    catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        out.appendLine(`ERROR: ${msg}`);
        vscode.window.showErrorMessage(`Ptrm: Failed to start services - ${msg}`);
    }
    await refreshFn();
}
// ── Down ───────────────────────────────────────────────────────────
async function cmdDown(refreshFn) {
    if (!(0, utils_1.hasConfig)()) {
        vscode.window.showWarningMessage("Portrm: No .ptrm.toml found.");
        return;
    }
    const out = (0, utils_1.getOutputChannel)();
    out.appendLine("[Portrm] Stopping all services...");
    try {
        const result = await (0, utils_1.runPtrm)("down");
        out.appendLine(result);
        vscode.window.showInformationMessage("Portrm: All services stopped");
    }
    catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        out.appendLine(`ERROR: ${msg}`);
        vscode.window.showErrorMessage(`Ptrm: Failed to stop services - ${msg}`);
    }
    await refreshFn();
}
// ── Init ───────────────────────────────────────────────────────────
async function cmdInit(refreshFn) {
    const root = (0, utils_1.getWorkspaceRoot)();
    if (!root) {
        vscode.window.showErrorMessage("Portrm: No workspace folder open. Please open a folder first.");
        return;
    }
    if ((0, utils_1.hasConfig)()) {
        vscode.window.showInformationMessage("Portrm: .ptrm.toml already exists in this workspace.");
        return;
    }
    const out = (0, utils_1.getOutputChannel)();
    out.appendLine("[Portrm] Initializing project...");
    try {
        const result = await (0, utils_1.runPtrm)("init");
        out.appendLine(result);
        vscode.window.showInformationMessage("Portrm: Created .ptrm.toml");
    }
    catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        out.appendLine(`ERROR: ${msg}`);
        vscode.window.showErrorMessage(`Ptrm: Init failed - ${msg}`);
    }
    await refreshFn();
}
// ── Info ───────────────────────────────────────────────────────────
async function cmdInfo(item) {
    let port = item?.port;
    if (!port) {
        const input = await vscode.window.showInputBox({
            prompt: "Port number to inspect",
            placeHolder: "e.g. 3000",
        });
        if (!input) {
            return;
        }
        port = parseInt(input, 10);
        if (isNaN(port)) {
            vscode.window.showErrorMessage("Portrm: Invalid port number");
            return;
        }
    }
    (0, utils_1.runInTerminal)(`ptrm info: ${port}`, `ptrm info ${port}`);
}
// ── Doctor ─────────────────────────────────────────────────────────
async function cmdDoctor(refreshFn) {
    (0, utils_1.runInTerminal)("ptrm doctor", "ptrm doctor");
    setTimeout(() => refreshFn(), 3000);
}
// ── Watch ──────────────────────────────────────────────────────────
async function cmdWatch(item) {
    let port = item?.port;
    if (!port) {
        const input = await vscode.window.showInputBox({
            prompt: "Port number to watch",
            placeHolder: "e.g. 3000",
        });
        if (!input) {
            return;
        }
        port = parseInt(input, 10);
        if (isNaN(port)) {
            vscode.window.showErrorMessage("Portrm: Invalid port number");
            return;
        }
    }
    (0, utils_1.runInTerminal)(`ptrm watch: ${port}`, `ptrm watch ${port}`);
}
// ── Preflight ──────────────────────────────────────────────────────
async function cmdPreflight() {
    if (!(0, utils_1.hasConfig)()) {
        vscode.window.showWarningMessage("Portrm: No .ptrm.toml found. Run ptrm init first.");
        return;
    }
    (0, utils_1.runInTerminal)("ptrm preflight", "ptrm preflight");
}
// ── Interactive TUI ────────────────────────────────────────────────
function cmdInteractive() {
    (0, utils_1.runInTerminal)("ptrm interactive", "ptrm interactive");
}
// ── Group ──────────────────────────────────────────────────────────
function cmdGroup() {
    (0, utils_1.runInTerminal)("ptrm group", "ptrm group");
}
// ── History ────────────────────────────────────────────────────────
function cmdHistory() {
    (0, utils_1.runInTerminal)("ptrm history", "ptrm history");
}
// ── Scan Dev ───────────────────────────────────────────────────────
function cmdScanDev() {
    (0, utils_1.runInTerminal)("ptrm scan --dev", "ptrm scan --dev");
}
// ── Registry ───────────────────────────────────────────────────────
async function cmdRegistry() {
    if (!(0, utils_1.hasConfig)()) {
        vscode.window.showWarningMessage("Portrm: No .ptrm.toml found.");
        return;
    }
    (0, utils_1.runInTerminal)("ptrm registry", "ptrm registry check");
}
// ── CI ─────────────────────────────────────────────────────────────
function cmdCi() {
    (0, utils_1.runInTerminal)("ptrm ci", "ptrm ci");
}
// ── Use Profile ────────────────────────────────────────────────────
async function cmdUseProfile(refreshFn) {
    if (!(0, utils_1.hasConfig)()) {
        vscode.window.showWarningMessage("Portrm: No .ptrm.toml found.");
        return;
    }
    const profile = await vscode.window.showInputBox({
        prompt: "Profile name to switch to",
        placeHolder: "e.g. staging, production",
        value: "staging",
    });
    if (!profile) {
        return;
    }
    (0, utils_1.runInTerminal)("ptrm use", `ptrm down && ptrm use ${profile} && ptrm up`);
    setTimeout(() => refreshFn(), 3000);
}
// ── Reset Profile (switch back to default) ─────────────────────────
async function cmdResetProfile(refreshFn) {
    if (!(0, utils_1.hasConfig)()) {
        vscode.window.showWarningMessage("Portrm: No .ptrm.toml found.");
        return;
    }
    (0, utils_1.runInTerminal)("ptrm use", "ptrm down && ptrm use default && ptrm up");
    setTimeout(() => refreshFn(), 3000);
}
// ── Update CLI ─────────────────────────────────────────────────────
async function cmdUpdate() {
    (0, utils_1.runInTerminal)("ptrm update", "ptrm update");
}
//# sourceMappingURL=commands.js.map