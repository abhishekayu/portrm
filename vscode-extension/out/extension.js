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
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const treeProvider_1 = require("./treeProvider");
const commands_1 = require("./commands");
const utils_1 = require("./utils");
const installer_1 = require("./installer");
const REFRESH_INTERVAL_MS = 4000;
let refreshTimer;
let statusBarItem;
// ── Activation ─────────────────────────────────────────────────────
function activate(context) {
    console.log("[Portrm] Extension activating...");
    const provider = new treeProvider_1.PtrmTreeProvider();
    // Register tree view
    const treeView = vscode.window.createTreeView("ptrmView", {
        treeDataProvider: provider,
        showCollapseAll: true,
    });
    context.subscriptions.push(treeView);
    console.log("[Portrm] TreeView registered");
    // Set context for conditional menus
    vscode.commands.executeCommand("setContext", "ptrm.hasConfig", (0, utils_1.hasConfig)());
    // Shared refresh function
    const doRefresh = async () => {
        try {
            await provider.loadData();
            provider.refresh();
            updateStatusBar(provider);
            vscode.commands.executeCommand("setContext", "ptrm.hasConfig", (0, utils_1.hasConfig)());
        }
        catch (e) {
            console.error("[Portrm] Refresh error:", e);
        }
    };
    // Register commands
    (0, commands_1.registerCommands)(context, provider, doRefresh);
    // Status bar
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 50);
    statusBarItem.command = "ptrmView.focus";
    statusBarItem.tooltip = "Portrm - Click to open sidebar";
    context.subscriptions.push(statusBarItem);
    statusBarItem.show();
    // Initial load (fire-and-forget with error handling)
    doRefresh();
    // One-time CLI presence check (no install, no update loop)
    (0, installer_1.ensureInstalled)(context);
    // Auto-refresh
    refreshTimer = setInterval(() => doRefresh(), REFRESH_INTERVAL_MS);
    context.subscriptions.push({ dispose: () => clearInterval(refreshTimer) });
    // Watch for .ptrm.toml changes
    const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    if (root) {
        const watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(root, ".ptrm.toml"));
        watcher.onDidChange(() => doRefresh());
        watcher.onDidCreate(() => doRefresh());
        watcher.onDidDelete(() => doRefresh());
        context.subscriptions.push(watcher);
    }
    // Reset binary cache when settings change
    context.subscriptions.push(vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration("ptrm.binaryPath")) {
            (0, utils_1.resetBinaryCache)();
            doRefresh();
        }
    }));
}
// ── Status Bar ─────────────────────────────────────────────────────
function updateStatusBar(provider) {
    const portCount = provider.ports.length;
    if (provider.projectMode) {
        const services = provider.services;
        const running = services.filter((s) => s.status === "running").length;
        const stopped = services.filter((s) => s.status === "stopped").length;
        const conflict = services.filter((s) => s.status === "conflict").length;
        const parts = [];
        if (running > 0) {
            parts.push(`$(pass-filled) ${running}`);
        }
        if (stopped > 0) {
            parts.push(`$(error) ${stopped}`);
        }
        if (conflict > 0) {
            parts.push(`$(warning) ${conflict}`);
        }
        statusBarItem.text = parts.length > 0 ? `$(plug) ${parts.join("  ")}` : "$(plug) ptrm";
    }
    else {
        statusBarItem.text = portCount > 0 ? `$(plug) ${portCount} ports` : "$(plug) ptrm";
    }
    // Build rich tooltip
    const tipParts = ["Portrm"];
    if (portCount > 0) {
        tipParts.push(`${portCount} active port${portCount === 1 ? "" : "s"}`);
        const totalMem = provider.ports.reduce((sum, p) => sum + (p.process?.memory_bytes ?? 0), 0);
        if (totalMem > 0) {
            tipParts.push(`Total memory: ${(0, utils_1.formatMemory)(totalMem)}`);
        }
    }
    statusBarItem.tooltip = tipParts.join(" \u00b7 ");
}
// ── Deactivation ───────────────────────────────────────────────────
function deactivate() {
    if (refreshTimer) {
        clearInterval(refreshTimer);
    }
}
//# sourceMappingURL=extension.js.map