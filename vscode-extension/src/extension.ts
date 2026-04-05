import * as vscode from "vscode";
import { PtrmTreeProvider } from "./treeProvider";
import { registerCommands } from "./commands";
import { hasConfig, resetBinaryCache, formatMemory } from "./utils";
import { ensureInstalled, checkForUpdate } from "./installer";

const REFRESH_INTERVAL_MS = 4_000;

let refreshTimer: ReturnType<typeof setInterval> | undefined;
let statusBarItem: vscode.StatusBarItem;

// ── Activation ─────────────────────────────────────────────────────

export function activate(context: vscode.ExtensionContext): void {
  console.log("[Portrm] Extension activating...");

  const provider = new PtrmTreeProvider();

  // Register tree view
  const treeView = vscode.window.createTreeView("ptrmView", {
    treeDataProvider: provider,
    showCollapseAll: true,
  });
  context.subscriptions.push(treeView);

  console.log("[Portrm] TreeView registered");

  // Set context for conditional menus
  vscode.commands.executeCommand("setContext", "ptrm.hasConfig", hasConfig());

  // Shared refresh function
  const doRefresh = async (): Promise<void> => {
    try {
      await provider.loadData();
      provider.refresh();
      updateStatusBar(provider);
      vscode.commands.executeCommand("setContext", "ptrm.hasConfig", hasConfig());
    } catch (e) {
      console.error("[Portrm] Refresh error:", e);
    }
  };

  // Register commands
  registerCommands(context, provider, doRefresh);

  // Status bar
  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 50);
  statusBarItem.command = "ptrmView.focus";
  statusBarItem.tooltip = "Portrm - Click to open sidebar";
  context.subscriptions.push(statusBarItem);
  statusBarItem.show();

  // Initial load (fire-and-forget with error handling)
  doRefresh();

  // Check if ptrm CLI is installed; prompt to install if not
  ensureInstalled().then((ok) => {
    if (ok) {
      // Check for updates after a short delay (don't block activation)
      setTimeout(() => checkForUpdate(), 10_000);
    }
  });

  // Auto-refresh
  refreshTimer = setInterval(() => doRefresh(), REFRESH_INTERVAL_MS);
  context.subscriptions.push({ dispose: () => clearInterval(refreshTimer) });

  // Watch for .ptrm.toml changes
  const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (root) {
    const watcher = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(root, ".ptrm.toml")
    );
    watcher.onDidChange(() => doRefresh());
    watcher.onDidCreate(() => doRefresh());
    watcher.onDidDelete(() => doRefresh());
    context.subscriptions.push(watcher);
  }

  // Reset binary cache when settings change
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("ptrm.binaryPath")) {
        resetBinaryCache();
        doRefresh();
      }
    })
  );
}

// ── Status Bar ─────────────────────────────────────────────────────

function updateStatusBar(provider: PtrmTreeProvider): void {
  const portCount = provider.ports.length;

  if (provider.projectMode) {
    const services = provider.services;
    const running = services.filter((s) => s.status === "running").length;
    const stopped = services.filter((s) => s.status === "stopped").length;
    const conflict = services.filter((s) => s.status === "conflict").length;

    const parts: string[] = [];
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
  } else {
    statusBarItem.text = portCount > 0 ? `$(plug) ${portCount} ports` : "$(plug) ptrm";
  }

  // Build rich tooltip
  const tipParts: string[] = ["Portrm"];
  if (portCount > 0) {
    tipParts.push(`${portCount} active port${portCount === 1 ? "" : "s"}`);
    const totalMem = provider.ports.reduce(
      (sum, p) => sum + (p.process?.memory_bytes ?? 0), 0
    );
    if (totalMem > 0) {
      tipParts.push(`Total memory: ${formatMemory(totalMem)}`);
    }
  }
  statusBarItem.tooltip = tipParts.join(" \u00b7 ");
}

// ── Deactivation ───────────────────────────────────────────────────

export function deactivate(): void {
  if (refreshTimer) {
    clearInterval(refreshTimer);
  }
}
