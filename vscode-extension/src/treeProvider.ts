import * as vscode from "vscode";
import {
  ServiceStatus,
  PortInfo,
  runPtrmJson,
  hasConfig,
  getProjectName,
  formatServiceLabel,
  formatPort,
  formatMemory,
  formatUptime,
} from "./utils";

// ── Tree item types ────────────────────────────────────────────────

type NodeKind = "section" | "header" | "service" | "port" | "action" | "info";

interface NodeData {
  kind: NodeKind;
  label: string;
  description?: string;
  tooltip?: string;
  icon?: vscode.ThemeIcon;
  contextValue?: string;
  command?: vscode.Command;
  collapsible?: vscode.TreeItemCollapsibleState;
  /** For service items */
  serviceName?: string;
  /** For port items */
  port?: number;
  children?: NodeData[];
}

// ── Provider ───────────────────────────────────────────────────────

export class PtrmTreeProvider implements vscode.TreeDataProvider<NodeData> {
  private _onDidChangeTreeData = new vscode.EventEmitter<NodeData | undefined | void>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private _services: ServiceStatus[] = [];
  private _ports: PortInfo[] = [];
  private _projectMode = false;
  private _projectName: string | undefined;
  private _error: string | undefined;
  private _scanError: string | undefined;
  private _loading = false;

  /** Expose state for the status bar */
  get services(): ServiceStatus[] {
    return this._services;
  }
  get ports(): PortInfo[] {
    return this._ports;
  }
  get projectMode(): boolean {
    return this._projectMode;
  }

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  async loadData(): Promise<void> {
    this._loading = true;
    this._error = undefined;
    this._scanError = undefined;

    try {
      this._projectMode = hasConfig();
      this._projectName = getProjectName();

      if (this._projectMode) {
        this._services = await runPtrmJson<ServiceStatus[]>("status --json");
        try {
          this._ports = await runPtrmJson<PortInfo[]>("scan --json");
        } catch (e: unknown) {
          const msg = e instanceof Error ? e.message : String(e);
          console.error("[Ptrm] scan error (project mode):", msg);
          this._scanError = msg;
          this._ports = [];
        }
      } else {
        this._services = [];
        try {
          this._ports = await runPtrmJson<PortInfo[]>("scan --json");
        } catch (e: unknown) {
          const msg = e instanceof Error ? e.message : String(e);
          console.error("[Ptrm] scan error (global mode):", msg);
          this._scanError = msg;
          this._ports = [];
        }
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      this._error = msg;
      console.error("[Ptrm] loadData error:", msg);
      this._services = [];
      this._ports = [];
    } finally {
      this._loading = false;
    }
  }

  // ── TreeDataProvider implementation ──────────────────────────────

  getTreeItem(element: NodeData): vscode.TreeItem {
    const item = new vscode.TreeItem(
      element.label,
      element.collapsible ?? vscode.TreeItemCollapsibleState.None
    );
    item.description = element.description;
    item.iconPath = element.icon;
    item.contextValue = element.contextValue;
    item.command = element.command;

    // Use markdown for rich tooltips on port items
    if (element.kind === "port" && element.tooltip) {
      item.tooltip = new vscode.MarkdownString(element.tooltip, true);
    } else {
      item.tooltip = element.tooltip;
    }

    return item;
  }

  getChildren(element?: NodeData): NodeData[] {
    // Root level: return sections
    if (!element) {
      return this.buildRoot();
    }
    // Section children
    return element.children ?? [];
  }

  // ── Build tree structure ─────────────────────────────────────────

  private buildRoot(): NodeData[] {
    const sections: NodeData[] = [];

    // 1. Header
    sections.push(this.buildHeader());

    // 2. Services (project mode)
    if (this._projectMode && this._services.length > 0) {
      sections.push(this.buildServicesSection());
    }

    // 3. Ports (always show - with error or empty state)
    sections.push(this.buildPortsSection());

    // 4. Actions
    sections.push(this.buildActionsSection());

    return sections;
  }

  private buildHeader(): NodeData {
    if (this._error) {
      return {
        kind: "info",
        label: "Error loading data",
        description: this._error,
        icon: new vscode.ThemeIcon("error", new vscode.ThemeColor("errorForeground")),
        tooltip: this._error,
      };
    }
    if (this._projectMode) {
      return {
        kind: "header",
        label: `Project: ${this._projectName ?? "unnamed"}`,
        icon: new vscode.ThemeIcon("project"),
        tooltip: "Loaded from .ptrm.toml",
      };
    }
    return {
      kind: "header",
      label: "No project found",
      description: "Click to initialize",
      icon: new vscode.ThemeIcon("info"),
      tooltip: "Run ptrm init to create a project config",
      command: {
        command: "ptrm.init",
        title: "Init Project",
      },
    };
  }

  private buildServicesSection(): NodeData {
    const children = this._services.map((svc) => this.buildServiceItem(svc));
    return {
      kind: "section",
      label: "Services",
      icon: new vscode.ThemeIcon("server-process"),
      collapsible: vscode.TreeItemCollapsibleState.Expanded,
      children,
    };
  }

  private buildServiceItem(svc: ServiceStatus): NodeData {
    const statusIcon = this.getStatusIcon(svc.status);
    const statusLabel = svc.status === "running" ? "Running" : svc.status === "stopped" ? "Stopped" : "Conflict";
    let description = `${formatPort(svc.port)} - ${statusLabel}`;
    if (svc.process) {
      description += ` (${svc.process})`;
    }
    if (svc.docker) {
      description += ` [docker:${svc.docker}]`;
    }

    return {
      kind: "service",
      label: svc.name,
      description,
      icon: statusIcon,
      contextValue: "service",
      serviceName: svc.name,
      port: svc.port,
      tooltip: this.buildServiceTooltip(svc),
      command: {
        command: "ptrm.logs",
        title: "View Logs",
        arguments: [{ port: svc.port, name: svc.name }],
      },
    };
  }

  private buildPortsSection(): NodeData {
    const children: NodeData[] = [];

    if (this._scanError) {
      children.push({
        kind: "info",
        label: "Scan failed",
        description: this._scanError.length > 60
          ? this._scanError.substring(0, 60) + "..."
          : this._scanError,
        icon: new vscode.ThemeIcon("warning", new vscode.ThemeColor("list.warningForeground")),
        tooltip: `ptrm scan --json failed:\n${this._scanError}`,
      });
    } else if (this._ports.length === 0) {
      children.push({
        kind: "info",
        label: "No active ports detected",
        icon: new vscode.ThemeIcon("info"),
      });
    } else {
      for (const p of this._ports) {
        children.push(this.buildPortItem(p));
      }
    }

    return {
      kind: "section",
      label: `Ports${this._ports.length > 0 ? ` (${this._ports.length})` : ""}`,
      icon: new vscode.ThemeIcon("plug"),
      collapsible: vscode.TreeItemCollapsibleState.Expanded,
      children,
    };
  }

  private buildPortItem(info: PortInfo): NodeData {
    const service = formatServiceLabel(info);
    const pid = info.process?.pid;
    const mem = formatMemory(info.process?.memory_bytes);
    const uptime = formatUptime(info.process?.runtime);

    // Build concise description: "Node.js  ·  PID 1234  ·  48.2 MB  ·  2h 15m"
    const parts: string[] = [service];
    if (pid) { parts.push(`PID ${pid}`); }
    if (mem) { parts.push(mem); }
    if (uptime) { parts.push(uptime); }
    const description = parts.join("  \u00b7  ");

    // Markdown tooltip with full details
    const tooltip = this.buildPortTooltip(info);

    return {
      kind: "port",
      label: `${info.port}`,
      description,
      icon: this.getPortIcon(info),
      contextValue: "port",
      port: info.port,
      tooltip,
      command: {
        command: "ptrm.info",
        title: "Inspect Port",
        arguments: [{ port: info.port }],
      },
    };
  }

  private getPortIcon(info: PortInfo): vscode.ThemeIcon {
    if (info.docker_container) {
      return new vscode.ThemeIcon("package", new vscode.ThemeColor("charts.blue"));
    }
    const kind = info.service?.kind;
    switch (kind) {
      case "NextJs":
      case "Vite":
      case "CreateReactApp":
        return new vscode.ThemeIcon("globe", new vscode.ThemeColor("charts.green"));
      case "Django":
      case "Flask":
      case "NodeGeneric":
      case "Python":
      case "Java":
      case "DotNet":
      case "Go":
      case "Rust":
      case "Ruby":
        return new vscode.ThemeIcon("server", new vscode.ThemeColor("charts.green"));
      case "PostgreSQL":
      case "MySQL":
      case "Redis":
      case "SQLServer":
      case "MongoDB":
        return new vscode.ThemeIcon("database", new vscode.ThemeColor("charts.yellow"));
      case "Nginx":
      case "Apache":
      case "IIS":
        return new vscode.ThemeIcon("cloud", new vscode.ThemeColor("charts.blue"));
      case "Docker":
        return new vscode.ThemeIcon("package", new vscode.ThemeColor("charts.blue"));
      default:
        return new vscode.ThemeIcon("radio-tower");
    }
  }

  private buildPortTooltip(info: PortInfo): string {
    const lines: string[] = [];
    lines.push(`**Port ${info.port}** (${info.protocol})`);
    lines.push("");

    if (info.process) {
      const p = info.process;
      lines.push(`| | |`);
      lines.push(`|---|---|`);
      lines.push(`| **Process** | ${p.name} |`);
      lines.push(`| **PID** | ${p.pid} |`);
      lines.push(`| **Command** | \`${p.command.length > 80 ? p.command.substring(0, 77) + "..." : p.command}\` |`);
      if (p.user) { lines.push(`| **User** | ${p.user} |`); }
      if (p.memory_bytes) { lines.push(`| **Memory** | ${formatMemory(p.memory_bytes)} |`); }
      if (p.cpu_usage !== undefined) { lines.push(`| **CPU** | ${p.cpu_usage.toFixed(1)}% |`); }
      if (p.runtime) { lines.push(`| **Uptime** | ${formatUptime(p.runtime)} |`); }
      if (p.working_dir) { lines.push(`| **CWD** | ${p.working_dir} |`); }
    }

    if (info.service && info.service.kind !== "Unknown") {
      lines.push("");
      lines.push(`Service: **${info.service.kind}** (${Math.round(info.service.confidence * 100)}% confidence)`);
    }

    if (info.docker_container) {
      lines.push("");
      lines.push(`Docker: **${info.docker_container.name}** (${info.docker_container.image})`);
    }

    return lines.join("\n");
  }

  private buildActionsSection(): NodeData {
    const children: NodeData[] = [];

    if (this._projectMode) {
      children.push({
        kind: "action",
        label: "Start All",
        icon: new vscode.ThemeIcon("run-all"),
        command: { command: "ptrm.up", title: "Start All" },
      });
      children.push({
        kind: "action",
        label: "Stop All",
        icon: new vscode.ThemeIcon("debug-stop"),
        command: { command: "ptrm.down", title: "Stop All" },
      });
      children.push({
        kind: "action",
        label: "Preflight Check",
        icon: new vscode.ThemeIcon("checklist"),
        command: { command: "ptrm.preflight", title: "Preflight" },
      });
      children.push({
        kind: "action",
        label: "Switch Profile",
        icon: new vscode.ThemeIcon("arrow-swap"),
        command: { command: "ptrm.useProfile", title: "Switch Profile" },
      });
      children.push({
        kind: "action",
        label: "Reset to Default",
        icon: new vscode.ThemeIcon("home"),
        command: { command: "ptrm.resetProfile", title: "Reset to Default" },
      });
      children.push({
        kind: "action",
        label: "Registry Check",
        icon: new vscode.ThemeIcon("database"),
        command: { command: "ptrm.registry", title: "Registry" },
      });
    }

    children.push({
      kind: "action",
      label: "Fix Port Conflicts",
      icon: new vscode.ThemeIcon("wrench"),
      command: { command: "ptrm.fix", title: "Fix" },
    });
    children.push({
      kind: "action",
      label: "Doctor",
      icon: new vscode.ThemeIcon("heart"),
      command: { command: "ptrm.doctor", title: "Doctor" },
    });
    children.push({
      kind: "action",
      label: "Scan Dev Ports",
      icon: new vscode.ThemeIcon("search"),
      command: { command: "ptrm.scanDev", title: "Scan Dev" },
    });
    children.push({
      kind: "action",
      label: "Group by Role",
      icon: new vscode.ThemeIcon("group-by-ref-type"),
      command: { command: "ptrm.group", title: "Group" },
    });
    children.push({
      kind: "action",
      label: "Interactive TUI",
      icon: new vscode.ThemeIcon("terminal"),
      command: { command: "ptrm.interactive", title: "Interactive" },
    });
    children.push({
      kind: "action",
      label: "History",
      icon: new vscode.ThemeIcon("history"),
      command: { command: "ptrm.history", title: "History" },
    });
    children.push({
      kind: "action",
      label: "CI Check",
      icon: new vscode.ThemeIcon("github-action"),
      command: { command: "ptrm.ci", title: "CI" },
    });
    children.push({
      kind: "action",
      label: "Update CLI",
      icon: new vscode.ThemeIcon("cloud-download"),
      command: { command: "ptrm.update", title: "Update" },
    });

    return {
      kind: "section",
      label: "Actions",
      icon: new vscode.ThemeIcon("zap"),
      collapsible: vscode.TreeItemCollapsibleState.Collapsed,
      children,
    };
  }

  // ── Helpers ──────────────────────────────────────────────────────

  private getStatusIcon(status: string): vscode.ThemeIcon {
    switch (status) {
      case "running":
        return new vscode.ThemeIcon("circle-filled", new vscode.ThemeColor("testing.iconPassed"));
      case "stopped":
        return new vscode.ThemeIcon("circle-filled", new vscode.ThemeColor("testing.iconFailed"));
      case "conflict":
        return new vscode.ThemeIcon("circle-filled", new vscode.ThemeColor("testing.iconQueued"));
      default:
        return new vscode.ThemeIcon("circle-outline");
    }
  }

  private buildServiceTooltip(svc: ServiceStatus): string {
    let tip = `${svc.name}\nPort: ${svc.port}\nStatus: ${svc.status}`;
    if (svc.process) {
      tip += `\nProcess: ${svc.process}`;
    }
    if (svc.pid) {
      tip += `\nPID: ${svc.pid}`;
    }
    if (svc.docker) {
      tip += `\nDocker: ${svc.docker}`;
    }
    return tip;
  }
}
