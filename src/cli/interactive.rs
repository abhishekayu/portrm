use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState};
use ratatui::Terminal;

use crate::classifier::ServiceClassifier;
use crate::engine::{FixEngine, Strategy};
use crate::models::{PortInfo, ServiceKind};
use crate::platform::PlatformAdapter;
use crate::scanner::PortScanner;

// ── App state ─────────────────────────────────────────────────────────

#[derive(PartialEq, Eq)]
enum View {
    List,
    Detail,
    ActionMenu,
}

#[derive(PartialEq, Eq)]
enum ActionChoice {
    Kill,
    Fix,
    Back,
}

const ACTIONS: &[ActionChoice] = &[ActionChoice::Fix, ActionChoice::Kill, ActionChoice::Back];

impl ActionChoice {
    fn label(&self) -> &'static str {
        match self {
            Self::Fix => "Fix (graceful)",
            Self::Kill => "Kill (force)",
            Self::Back => "Back",
        }
    }
}

struct App {
    ports: Vec<PortInfo>,
    table_state: TableState,
    view: View,
    action_idx: usize,
    status_msg: Option<(String, Color)>,
    should_quit: bool,
}

impl App {
    fn new(ports: Vec<PortInfo>) -> Self {
        let mut table_state = TableState::default();
        if !ports.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            ports,
            table_state,
            view: View::List,
            action_idx: 0,
            status_msg: None,
            should_quit: false,
        }
    }

    fn selected_port(&self) -> Option<&PortInfo> {
        self.table_state.selected().and_then(|i| self.ports.get(i))
    }

    fn move_down(&mut self) {
        if self.ports.is_empty() {
            return;
        }
        let i = self
            .table_state
            .selected()
            .map(|i| if i >= self.ports.len() - 1 { 0 } else { i + 1 })
            .unwrap_or(0);
        self.table_state.select(Some(i));
    }

    fn move_up(&mut self) {
        if self.ports.is_empty() {
            return;
        }
        let i = self
            .table_state
            .selected()
            .map(|i| if i == 0 { self.ports.len() - 1 } else { i - 1 })
            .unwrap_or(0);
        self.table_state.select(Some(i));
    }
}

// ── Entry point ───────────────────────────────────────────────────────

pub fn run_interactive(adapter: &dyn PlatformAdapter) -> anyhow::Result<()> {
    // Initial scan.
    let scanner = PortScanner::new(adapter);
    let ports = scan_and_classify(&scanner)?;

    // Terminal setup.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(ports);

    // Main loop.
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Global quit: q, Esc, Ctrl+C.
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                break;
            }

            match app.view {
                View::List => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                    KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                    KeyCode::Enter => {
                        if app.selected_port().is_some() {
                            app.view = View::Detail;
                        }
                    }
                    KeyCode::Char('r') => {
                        // Refresh.
                        let scanner = PortScanner::new(adapter);
                        if let Ok(ports) = scan_and_classify(&scanner) {
                            app.ports = ports;
                            if app.ports.is_empty() {
                                app.table_state.select(None);
                            } else {
                                let sel = app.table_state.selected().unwrap_or(0);
                                app.table_state
                                    .select(Some(sel.min(app.ports.len() - 1)));
                            }
                            app.status_msg =
                                Some(("Refreshed".into(), Color::Green));
                        }
                    }
                    KeyCode::Char('f') | KeyCode::Char('x') => {
                        if app.selected_port().is_some() {
                            app.action_idx = 0;
                            app.view = View::ActionMenu;
                        }
                    }
                    _ => {}
                },
                View::Detail => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => {
                        app.view = View::List;
                    }
                    KeyCode::Char('f') | KeyCode::Char('x') | KeyCode::Enter => {
                        app.action_idx = 0;
                        app.view = View::ActionMenu;
                    }
                    _ => {}
                },
                View::ActionMenu => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => {
                        app.view = View::Detail;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.action_idx = (app.action_idx + 1) % ACTIONS.len();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.action_idx = if app.action_idx == 0 {
                            ACTIONS.len() - 1
                        } else {
                            app.action_idx - 1
                        };
                    }
                    KeyCode::Enter => {
                        match &ACTIONS[app.action_idx] {
                            ActionChoice::Back => {
                                app.view = View::Detail;
                            }
                            action => {
                                if let Some(info) = app.selected_port().cloned() {
                                    let msg = execute_action(adapter, &info, action);
                                    app.status_msg = Some(msg);

                                    // Refresh after action.
                                    let scanner = PortScanner::new(adapter);
                                    if let Ok(ports) = scan_and_classify(&scanner) {
                                        app.ports = ports;
                                        if app.ports.is_empty() {
                                            app.table_state.select(None);
                                        } else {
                                            let sel =
                                                app.table_state.selected().unwrap_or(0);
                                            app.table_state.select(Some(
                                                sel.min(app.ports.len().saturating_sub(1)),
                                            ));
                                        }
                                    }
                                }
                                app.view = View::List;
                            }
                        }
                    }
                    _ => {}
                },
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Cleanup.
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

// ── UI rendering ──────────────────────────────────────────────────────

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Min(5),    // main
        Constraint::Length(2), // footer
    ])
    .split(f.area());

    render_header(f, chunks[0]);

    match app.view {
        View::List => render_list(f, chunks[1], app),
        View::Detail => render_detail(f, chunks[1], app),
        View::ActionMenu => {
            render_detail(f, chunks[1], app);
            render_action_popup(f, f.area(), app);
        }
    }

    render_footer(f, chunks[2], app);
}

fn render_header(f: &mut ratatui::Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(" ptrm ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(
            "interactive",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let header = Paragraph::new(title).block(block);
    f.render_widget(header, area);
}

fn render_list(f: &mut ratatui::Frame, area: Rect, app: &mut App) {
    if app.ports.is_empty() {
        let msg = Paragraph::new(Line::from(vec![
            Span::styled(" No listening ports found ", Style::default().fg(Color::DarkGray)),
        ]));
        f.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec![
        "PORT", "PROCESS", "PID", "SERVICE", "MEMORY", "UPTIME", "USER",
    ])
    .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD))
    .bottom_margin(0);

    let rows: Vec<Row> = app
        .ports
        .iter()
        .map(|info| {
            let (pid, name, user, mem, uptime) = match &info.process {
                Some(p) => (
                    p.pid.to_string(),
                    truncate(&p.name, 22),
                    p.user.clone().unwrap_or_default(),
                    p.memory_bytes
                        .map(crate::cli::output::format_bytes)
                        .unwrap_or_else(|| "-".into()),
                    p.runtime_display(),
                ),
                None => ("-".into(), "-".into(), "-".into(), "-".into(), "-".into()),
            };

            let svc = info
                .service
                .as_ref()
                .map(|s| s.kind.label().to_string())
                .unwrap_or_else(|| "-".into());

            let svc_color = info
                .service
                .as_ref()
                .map(|s| service_color(s.kind))
                .unwrap_or(Color::DarkGray);

            Row::new(vec![
                ratatui::text::Text::styled(
                    info.port.to_string(),
                    Style::default().fg(Color::Cyan).bold(),
                ),
                ratatui::text::Text::raw(name),
                ratatui::text::Text::styled(pid, Style::default().fg(Color::DarkGray)),
                ratatui::text::Text::styled(svc, Style::default().fg(svc_color)),
                ratatui::text::Text::styled(mem, Style::default().fg(Color::DarkGray)),
                ratatui::text::Text::styled(uptime, Style::default().fg(Color::Yellow)),
                ratatui::text::Text::styled(user, Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(7),
        Constraint::Length(24),
        Constraint::Length(8),
        Constraint::Length(14),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Fill(1),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" \u{25b6} ")
        .block(Block::default().borders(Borders::NONE));

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_detail(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let Some(info) = app.selected_port() else {
        return;
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  \u{26a1} Port ", Style::default().bold()),
            Span::styled(
                info.port.to_string(),
                Style::default().fg(Color::Cyan).bold(),
            ),
            Span::styled(" in use", Style::default().bold()),
        ]),
        Line::from(""),
    ];

    if let Some(ref proc_) = info.process {
        let svc_label = info
            .service
            .as_ref()
            .map(|s| s.kind.label().to_string())
            .unwrap_or_else(|| proc_.name.clone());

        lines.push(Line::from(vec![
            Span::raw("  \u{2192} "),
            Span::styled(svc_label.clone(), Style::default().fg(Color::Cyan)),
            Span::styled(
                format!(" (PID {})", proc_.pid),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("  \u{2192} running for "),
            Span::styled(
                proc_.runtime_display(),
                Style::default().fg(Color::Yellow),
            ),
        ]));

        if let Some(mem) = proc_.memory_bytes {
            lines.push(Line::from(vec![
                Span::raw("  \u{2192} memory "),
                Span::styled(
                    crate::cli::output::format_bytes(mem),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        lines.push(Line::from(vec![
            Span::raw("  \u{2192} "),
            Span::styled(
                proc_.command.clone(),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        if let Some(ref user) = proc_.user {
            lines.push(Line::from(vec![
                Span::raw("  \u{2192} user "),
                Span::styled(user.clone(), Style::default().fg(Color::DarkGray)),
            ]));
        }

        if let Some(ref cwd) = proc_.working_dir {
            lines.push(Line::from(vec![
                Span::raw("  \u{2192} cwd "),
                Span::styled(cwd.clone(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    if let Some(ref svc) = info.service {
        lines.push(Line::from(""));

        let svc_color = service_color(svc.kind);
        lines.push(Line::from(vec![
            Span::raw("  \u{2192} detected "),
            Span::styled(svc.kind.label(), Style::default().fg(svc_color).bold()),
            Span::styled(
                format!(" ({:.0}% confidence)", svc.confidence * 100.0),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        let safe_style = if svc.kind.safe_to_kill() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };
        let safe_text = if svc.kind.safe_to_kill() {
            "\u{1f6e1} safe to kill"
        } else {
            "\u{26a0} use caution"
        };
        lines.push(Line::from(vec![
            Span::raw("  \u{2192} "),
            Span::styled(safe_text, safe_style),
        ]));
    }

    let detail = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    f.render_widget(detail, area);
}

fn render_action_popup(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let popup_width = 30u16;
    let popup_height = (ACTIONS.len() as u16) + 4;

    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mut lines = vec![Line::from("")];

    for (i, action) in ACTIONS.iter().enumerate() {
        let marker = if i == app.action_idx { "\u{25b6} " } else { "  " };
        let style = if i == app.action_idx {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!(" {}{}", marker, action.label()),
            style,
        )));
    }

    let block = Block::default()
        .title(" Action ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let popup = Paragraph::new(lines).block(block);
    f.render_widget(popup, popup_area);
}

fn render_footer(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let status = if let Some((ref msg, color)) = app.status_msg {
        Line::from(Span::styled(
            format!("  {msg}"),
            Style::default().fg(color),
        ))
    } else {
        Line::from("")
    };

    let help = match app.view {
        View::List => {
            " \u{2191}\u{2193} navigate  Enter detail  f fix/kill  r refresh  q quit "
        }
        View::Detail => " Esc back  f fix/kill  q quit ",
        View::ActionMenu => " \u{2191}\u{2193} navigate  Enter select  Esc cancel ",
    };

    let footer_lines = vec![
        status,
        Line::from(Span::styled(help, Style::default().fg(Color::DarkGray))),
    ];

    let footer = Paragraph::new(footer_lines);
    f.render_widget(footer, area);
}

// ── Actions ───────────────────────────────────────────────────────────

fn execute_action(
    adapter: &dyn PlatformAdapter,
    info: &PortInfo,
    action: &ActionChoice,
) -> (String, Color) {
    let engine = FixEngine::new(adapter);
    let port = info.port;

    let plan = match engine.analyze(port) {
        Ok(p) => p,
        Err(e) => return (format!("Error: {e}"), Color::Red),
    };

    if plan.verdict.is_blocked() {
        return (
            format!("\u{1f6d1} BLOCKED: {}", plan.verdict.reason()),
            Color::Red,
        );
    }

    let _strategy = match action {
        ActionChoice::Kill => Strategy::Force,
        ActionChoice::Fix => plan.strategy,
        ActionChoice::Back => return ("".into(), Color::Reset),
    };

    match engine.execute(&plan, |_step| {}) {
        Ok(result) if result.success => (
            format!("\u{2714} Port {} freed (PID {})", result.port, result.pid),
            Color::Green,
        ),
        Ok(result) => (
            format!("\u{2718} Failed to free port {} (PID {})", result.port, result.pid),
            Color::Red,
        ),
        Err(e) => (format!("Error: {e}"), Color::Red),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn scan_and_classify(scanner: &PortScanner) -> anyhow::Result<Vec<PortInfo>> {
    let mut ports = scanner.scan_all()?;
    for info in &mut ports {
        if let Some(ref proc_) = info.process {
            info.service = Some(ServiceClassifier::classify_with_port(proc_, info.port));
        }
    }
    Ok(ports)
}

fn service_color(kind: ServiceKind) -> Color {
    match kind {
        ServiceKind::NextJs | ServiceKind::Vite | ServiceKind::CreateReactApp => Color::Green,
        ServiceKind::Docker | ServiceKind::Nginx => Color::Blue,
        ServiceKind::PostgreSQL | ServiceKind::MySQL | ServiceKind::Redis => Color::Yellow,
        ServiceKind::Unknown => Color::DarkGray,
        _ => Color::White,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}..", &s[..max - 2])
    }
}
