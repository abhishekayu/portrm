import * as vscode from "vscode";
import { runPtrm, runInTerminal, getOutputChannel, hasConfig, getWorkspaceRoot } from "./utils";
import { PtrmTreeProvider } from "./treeProvider";
import { checkForUpdate, installOrUpdate, getLatestVersion } from "./installer";

// ── Command handlers ───────────────────────────────────────────────

export function registerCommands(
  context: vscode.ExtensionContext,
  provider: PtrmTreeProvider,
  refreshFn: () => Promise<void>
): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("ptrm.refresh", refreshFn),
    vscode.commands.registerCommand("ptrm.restart", (item) => cmdRestart(item, refreshFn)),
    vscode.commands.registerCommand("ptrm.logs", (item) => cmdLogs(item)),
    vscode.commands.registerCommand("ptrm.kill", (item) => cmdKill(item, refreshFn)),
    vscode.commands.registerCommand("ptrm.fix", () => cmdFix(refreshFn)),
    vscode.commands.registerCommand("ptrm.up", () => cmdUp(refreshFn)),
    vscode.commands.registerCommand("ptrm.down", () => cmdDown(refreshFn)),
    vscode.commands.registerCommand("ptrm.init", () => cmdInit(refreshFn)),
    vscode.commands.registerCommand("ptrm.info", (item) => cmdInfo(item)),
    vscode.commands.registerCommand("ptrm.doctor", () => cmdDoctor(refreshFn)),
    vscode.commands.registerCommand("ptrm.watch", (item) => cmdWatch(item)),
    vscode.commands.registerCommand("ptrm.preflight", () => cmdPreflight()),
    vscode.commands.registerCommand("ptrm.interactive", () => cmdInteractive()),
    vscode.commands.registerCommand("ptrm.group", () => cmdGroup()),
    vscode.commands.registerCommand("ptrm.history", () => cmdHistory()),
    vscode.commands.registerCommand("ptrm.scanDev", () => cmdScanDev()),
    vscode.commands.registerCommand("ptrm.registry", () => cmdRegistry()),
    vscode.commands.registerCommand("ptrm.ci", () => cmdCi()),
    vscode.commands.registerCommand("ptrm.useProfile", () => cmdUseProfile(refreshFn)),
    vscode.commands.registerCommand("ptrm.update", () => cmdUpdate()),
    vscode.commands.registerCommand("ptrm.resetProfile", () => cmdResetProfile(refreshFn)),
  );
}

// ── Restart ────────────────────────────────────────────────────────

async function cmdRestart(
  item: { serviceName?: string; port?: number } | undefined,
  refreshFn: () => Promise<void>
): Promise<void> {
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

  const out = getOutputChannel();
  out.appendLine(`[Portrm] Restarting ${serviceName}...`);

  try {
    const result = await runPtrm(`restart ${serviceName}`);
    out.appendLine(result);
    vscode.window.showInformationMessage(`Ptrm: Restarted ${serviceName}`);
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    out.appendLine(`ERROR: ${msg}`);
    vscode.window.showErrorMessage(`Ptrm: Failed to restart ${serviceName} - ${msg}`);
  }

  await refreshFn();
}

// ── Logs ───────────────────────────────────────────────────────────

async function cmdLogs(
  item: { port?: number; name?: string } | undefined
): Promise<void> {
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
  runInTerminal(label, `ptrm log ${port}`);
}

// ── Kill ───────────────────────────────────────────────────────────

async function cmdKill(
  item: { port?: number } | undefined,
  refreshFn: () => Promise<void>
): Promise<void> {
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

  const confirm = await vscode.window.showWarningMessage(
    `Kill the process on port ${port}?`,
    { modal: true },
    "Kill"
  );
  if (confirm !== "Kill") {
    return;
  }

  const out = getOutputChannel();
  out.appendLine(`[Portrm] Killing process on port ${port}...`);

  try {
    const result = await runPtrm(`kill ${port} -y`);
    out.appendLine(result);
    vscode.window.showInformationMessage(`Ptrm: Killed process on port ${port}`);
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    out.appendLine(`ERROR: ${msg}`);
    vscode.window.showErrorMessage(`Ptrm: Failed to kill port ${port} - ${msg}`);
  }

  await refreshFn();
}

// ── Fix ────────────────────────────────────────────────────────────

async function cmdFix(refreshFn: () => Promise<void>): Promise<void> {
  const out = getOutputChannel();
  out.appendLine("[Portrm] Running fix (auto-detect)...");

  runInTerminal("ptrm fix", "ptrm fix");

  setTimeout(() => refreshFn(), 3000);
}

// ── Up ─────────────────────────────────────────────────────────────

async function cmdUp(refreshFn: () => Promise<void>): Promise<void> {
  if (!hasConfig()) {
    vscode.window.showWarningMessage("Portrm: No .ptrm.toml found. Run ptrm init first.");
    return;
  }

  const out = getOutputChannel();
  out.appendLine("[Portrm] Starting all services...");

  try {
    const result = await runPtrm("up -y");
    out.appendLine(result);
    vscode.window.showInformationMessage("Portrm: All services started");
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    out.appendLine(`ERROR: ${msg}`);
    vscode.window.showErrorMessage(`Ptrm: Failed to start services - ${msg}`);
  }

  await refreshFn();
}

// ── Down ───────────────────────────────────────────────────────────

async function cmdDown(refreshFn: () => Promise<void>): Promise<void> {
  if (!hasConfig()) {
    vscode.window.showWarningMessage("Portrm: No .ptrm.toml found.");
    return;
  }

  const out = getOutputChannel();
  out.appendLine("[Portrm] Stopping all services...");

  try {
    const result = await runPtrm("down");
    out.appendLine(result);
    vscode.window.showInformationMessage("Portrm: All services stopped");
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    out.appendLine(`ERROR: ${msg}`);
    vscode.window.showErrorMessage(`Ptrm: Failed to stop services - ${msg}`);
  }

  await refreshFn();
}

// ── Init ───────────────────────────────────────────────────────────

async function cmdInit(refreshFn: () => Promise<void>): Promise<void> {
  const root = getWorkspaceRoot();
  if (!root) {
    vscode.window.showErrorMessage("Portrm: No workspace folder open. Please open a folder first.");
    return;
  }

  if (hasConfig()) {
    vscode.window.showInformationMessage("Portrm: .ptrm.toml already exists in this workspace.");
    return;
  }

  const out = getOutputChannel();
  out.appendLine("[Portrm] Initializing project...");

  try {
    const result = await runPtrm("init");
    out.appendLine(result);
    vscode.window.showInformationMessage("Portrm: Created .ptrm.toml");
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    out.appendLine(`ERROR: ${msg}`);
    vscode.window.showErrorMessage(`Ptrm: Init failed - ${msg}`);
  }

  await refreshFn();
}

// ── Info ───────────────────────────────────────────────────────────

async function cmdInfo(
  item: { port?: number } | undefined
): Promise<void> {
  let port = item?.port;

  if (!port) {
    const input = await vscode.window.showInputBox({
      prompt: "Port number to inspect",
      placeHolder: "e.g. 3000",
    });
    if (!input) { return; }
    port = parseInt(input, 10);
    if (isNaN(port)) {
      vscode.window.showErrorMessage("Portrm: Invalid port number");
      return;
    }
  }

  runInTerminal(`ptrm info: ${port}`, `ptrm info ${port}`);
}

// ── Doctor ─────────────────────────────────────────────────────────

async function cmdDoctor(refreshFn: () => Promise<void>): Promise<void> {
  runInTerminal("ptrm doctor", "ptrm doctor");
  setTimeout(() => refreshFn(), 3000);
}

// ── Watch ──────────────────────────────────────────────────────────

async function cmdWatch(
  item: { port?: number } | undefined
): Promise<void> {
  let port = item?.port;

  if (!port) {
    const input = await vscode.window.showInputBox({
      prompt: "Port number to watch",
      placeHolder: "e.g. 3000",
    });
    if (!input) { return; }
    port = parseInt(input, 10);
    if (isNaN(port)) {
      vscode.window.showErrorMessage("Portrm: Invalid port number");
      return;
    }
  }

  runInTerminal(`ptrm watch: ${port}`, `ptrm watch ${port}`);
}

// ── Preflight ──────────────────────────────────────────────────────

async function cmdPreflight(): Promise<void> {
  if (!hasConfig()) {
    vscode.window.showWarningMessage("Portrm: No .ptrm.toml found. Run ptrm init first.");
    return;
  }
  runInTerminal("ptrm preflight", "ptrm preflight");
}

// ── Interactive TUI ────────────────────────────────────────────────

function cmdInteractive(): void {
  runInTerminal("ptrm interactive", "ptrm interactive");
}

// ── Group ──────────────────────────────────────────────────────────

function cmdGroup(): void {
  runInTerminal("ptrm group", "ptrm group");
}

// ── History ────────────────────────────────────────────────────────

function cmdHistory(): void {
  runInTerminal("ptrm history", "ptrm history");
}

// ── Scan Dev ───────────────────────────────────────────────────────

function cmdScanDev(): void {
  runInTerminal("ptrm scan --dev", "ptrm scan --dev");
}

// ── Registry ───────────────────────────────────────────────────────

async function cmdRegistry(): Promise<void> {
  if (!hasConfig()) {
    vscode.window.showWarningMessage("Portrm: No .ptrm.toml found.");
    return;
  }
  runInTerminal("ptrm registry", "ptrm registry check");
}

// ── CI ─────────────────────────────────────────────────────────────

function cmdCi(): void {
  runInTerminal("ptrm ci", "ptrm ci");
}

// ── Use Profile ────────────────────────────────────────────────────

async function cmdUseProfile(refreshFn: () => Promise<void>): Promise<void> {
  if (!hasConfig()) {
    vscode.window.showWarningMessage("Portrm: No .ptrm.toml found.");
    return;
  }

  const profile = await vscode.window.showInputBox({
    prompt: "Profile name to switch to",
    placeHolder: "e.g. staging, production",
    value: "staging",
  });
  if (!profile) { return; }

  runInTerminal("ptrm use", `ptrm down && ptrm use ${profile} && ptrm up`);

  setTimeout(() => refreshFn(), 3000);
}

// ── Reset Profile (switch back to default) ─────────────────────────

async function cmdResetProfile(refreshFn: () => Promise<void>): Promise<void> {
  if (!hasConfig()) {
    vscode.window.showWarningMessage("Portrm: No .ptrm.toml found.");
    return;
  }

  runInTerminal("ptrm use", "ptrm down && ptrm use default && ptrm up");

  setTimeout(() => refreshFn(), 3000);
}

// ── Update CLI ─────────────────────────────────────────────────────

async function cmdUpdate(): Promise<void> {
  try {
    const latest = await getLatestVersion();
    const binPath = await installOrUpdate(latest);
    vscode.window.showInformationMessage(`Ptrm: Updated to v${latest} at ${binPath}`);
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    vscode.window.showErrorMessage(`Ptrm: Update failed - ${msg}`);
  }
}
