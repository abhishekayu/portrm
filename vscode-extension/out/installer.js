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
exports.getInstalledVersion = getInstalledVersion;
exports.ensureInstalled = ensureInstalled;
const vscode = __importStar(require("vscode"));
const os = __importStar(require("os"));
const child_process_1 = require("child_process");
const utils_1 = require("./utils");
const GLOBAL_STATE_CLI_MISSING_WARNED = "portrm.cliMissingWarned";
// ── CLI Detection ──────────────────────────────────────────────────
/**
 * Check if the ptrm CLI is reachable using the resolved binary path.
 * Returns the version string on success, undefined on failure.
 */
function getInstalledVersion() {
    return new Promise((resolve) => {
        const bin = (0, utils_1.resolvePtrm)();
        (0, child_process_1.exec)(`${bin} --version`, { timeout: 5000 }, (err, stdout) => {
            if (err) {
                return resolve(undefined);
            }
            const match = stdout.trim().match(/(\d+\.\d+\.\d+)/);
            resolve(match?.[1]);
        });
    });
}
/**
 * Return the platform-appropriate install command suggestion.
 */
function getInstallSuggestion() {
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
async function ensureInstalled(context) {
    const version = await getInstalledVersion();
    if (version) {
        // CLI found -- clear any previous warning flag so future removals get caught
        await context.globalState.update(GLOBAL_STATE_CLI_MISSING_WARNED, undefined);
        return true;
    }
    // CLI not found -- show warning only once
    const alreadyWarned = context.globalState.get(GLOBAL_STATE_CLI_MISSING_WARNED);
    if (alreadyWarned) {
        return false;
    }
    await context.globalState.update(GLOBAL_STATE_CLI_MISSING_WARNED, true);
    const suggestion = getInstallSuggestion();
    const action = await vscode.window.showWarningMessage(`portrm CLI not found. Install it to use all features.`, "Copy Install Command", "Dismiss");
    if (action === "Copy Install Command") {
        await vscode.env.clipboard.writeText(suggestion);
        vscode.window.showInformationMessage(`Copied: ${suggestion}`);
    }
    return false;
}
//# sourceMappingURL=installer.js.map