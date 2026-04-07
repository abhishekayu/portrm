import * as vscode from "vscode";
import * as os from "os";
import { exec } from "child_process";
import { resolvePtrm } from "./utils";

const GLOBAL_STATE_CLI_MISSING_WARNED = "portrm.cliMissingWarned";

// ── CLI Detection ──────────────────────────────────────────────────

/**
 * Check if the ptrm CLI is reachable using the resolved binary path.
 * Returns the version string on success, undefined on failure.
 */
export function getInstalledVersion(): Promise<string | undefined> {
  return new Promise((resolve) => {
    const bin = resolvePtrm();
    exec(`${bin} --version`, { timeout: 5_000 }, (err, stdout) => {
      if (err) { return resolve(undefined); }
      const match = stdout.trim().match(/(\d+\.\d+\.\d+)/);
      resolve(match?.[1]);
    });
  });
}

/**
 * Return the platform-appropriate install command suggestion.
 */
function getInstallSuggestion(): string {
  const platform = os.platform();
  switch (platform) {
    case "darwin":
      return "brew install abhishekayu/tap/portrm";
    case "linux":
      return "curl -fsSL https://raw.githubusercontent.com/abhishekayu/portrm/main/install.sh | sh";
    case "win32":
      return "npm install -g portrm";
    default:
      return "cargo install portrm";
  }
}

// ── One-time CLI check on activation ───────────────────────────────

/**
 * Check if the CLI is installed. If not, show a ONE-TIME warning with
 * the install command. Uses globalState to never repeat the popup.
 *
 * The extension does NOT install, download, or manage the CLI itself.
 */
export async function ensureInstalled(context: vscode.ExtensionContext): Promise<boolean> {
  const version = await getInstalledVersion();
  if (version) {
    // CLI found -- clear any previous warning flag so future removals get caught
    await context.globalState.update(GLOBAL_STATE_CLI_MISSING_WARNED, undefined);
    return true;
  }

  // CLI not found -- show warning only once
  const alreadyWarned = context.globalState.get<boolean>(GLOBAL_STATE_CLI_MISSING_WARNED);
  if (alreadyWarned) {
    return false;
  }

  await context.globalState.update(GLOBAL_STATE_CLI_MISSING_WARNED, true);

  const suggestion = getInstallSuggestion();
  const action = await vscode.window.showWarningMessage(
    `portrm CLI not found. Install it to use all features.`,
    "Copy Install Command",
    "Dismiss"
  );

  if (action === "Copy Install Command") {
    await vscode.env.clipboard.writeText(suggestion);
    vscode.window.showInformationMessage(`Copied: ${suggestion}`);
  }

  return false;
}
