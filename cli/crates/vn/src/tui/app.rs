use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use std::env;
use std::io::Read;
use std::io::{self, Stdout, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy)]
enum MenuKind {
    Root,
    RunUbuntu22,
    RunWin11,
}

#[derive(Clone)]
enum Action {
    Execute(Vec<&'static str>),
    Placeholder(&'static str),
    OpenMenu(MenuKind),
    BackToRoot,
}

#[derive(Clone)]
struct CommandItem {
    label: &'static str,
    action: Action,
}

enum ProcEvent {
    Stdout(String),
    Stderr(String),
}

enum Focus {
    Dashboard,
    Input,
}

struct AppState {
    menu: MenuKind,
    commands: Vec<CommandItem>,
    selected: usize,
    logs: Vec<String>,
    child: Option<Child>,
    child_stdin: Option<ChildStdin>,
    rx: Option<Receiver<ProcEvent>>,
    input: String,
    focus: Focus,
    output_scroll: u16,
    output_view_lines: usize,
    follow_output: bool,
    last_log_count: usize,
}

impl AppState {
    fn new() -> Self {
        Self {
            menu: MenuKind::Root,
            commands: menu_items(MenuKind::Root),
            selected: 0,
            logs: vec!["vecnode CLI output panel".to_string()],
            child: None,
            child_stdin: None,
            rx: None,
            input: String::new(),
            focus: Focus::Dashboard,
            output_scroll: 0,
            output_view_lines: 0,
            follow_output: true,
            last_log_count: 1,
        }
    }

    fn max_output_scroll(&self) -> u16 {
        self.logs
            .len()
            .saturating_sub(self.output_view_lines)
            .min(u16::MAX as usize) as u16
    }

    fn output_page_up(&mut self) {
        let step = self.output_view_lines.max(1).min(u16::MAX as usize) as u16;
        self.follow_output = false;
        self.output_scroll = self.output_scroll.saturating_sub(step);
    }

    fn output_page_down(&mut self) {
        let step = self.output_view_lines.max(1).min(u16::MAX as usize) as u16;
        let max_scroll = self.max_output_scroll();
        self.output_scroll = self.output_scroll.saturating_add(step).min(max_scroll);
        if self.output_scroll >= max_scroll {
            self.follow_output = true;
            self.output_scroll = max_scroll;
        }
    }

    fn set_menu(&mut self, menu: MenuKind) {
        self.menu = menu;
        self.commands = menu_items(menu);
        self.selected = 0;
    }

    fn next(&mut self) {
        self.selected = (self.selected + 1) % self.commands.len();
    }

    fn previous(&mut self) {
        if self.selected == 0 {
            self.selected = self.commands.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    fn activate_selected(&mut self) {
        if self.commands.is_empty() {
            return;
        }

        if self.child.is_some() {
            self.logs.push("[INFO] A command is already running.".to_string());
            self.trim_logs();
            return;
        }

        let item = self.commands[self.selected].clone();

        match item.action {
            Action::OpenMenu(next_menu) => {
                if !menu_allowed_on_current_os(next_menu) {
                    self.logs.push(format!("> {}", item.label));
                    self.logs
                        .push("[WARNING] This submenu is not supported on the current OS.".to_string());
                    if cfg!(windows) {
                        self.logs
                            .push("[INFO] Windows host: use vn run win11 submenu items.".to_string());
                    } else {
                        self.logs
                            .push("[INFO] Non-Windows host: use vn run ubuntu22 submenu items.".to_string());
                    }
                    self.trim_logs();
                    return;
                }

                self.logs.push(format!("> {}", item.label));
                self.logs.push("[INFO] Opened submenu.".to_string());
                self.set_menu(next_menu);
                self.trim_logs();
                return;
            }
            Action::BackToRoot => {
                self.logs.push(format!("> {}", item.label));
                self.logs.push("[INFO] Returned to Dashboard.".to_string());
                self.set_menu(MenuKind::Root);
                self.trim_logs();
                return;
            }
            Action::Execute(args) => {
                self.spawn_process(item.label, args);
            }
            Action::Placeholder(message) => {
                self.logs.push(format!("> {}", item.label));
                self.logs.push(format!("[INFO] {}", message));
                self.trim_logs();
                return;
            }
        }
    }

    fn spawn_process(&mut self, label: &str, args: Vec<&'static str>) {
        self.logs.push(format!("> {}", label));

        let exe = match env::current_exe() {
            Ok(path) => path,
            Err(err) => {
                self.logs
                    .push(format!("[ERROR] Could not resolve current executable: {}", err));
                self.trim_logs();
                return;
            }
        };

        let mut cmd = Command::new(exe);
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(err) => {
                self.logs
                    .push(format!("[ERROR] Failed to start command: {}", err));
                self.trim_logs();
                return;
            }
        };

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (tx, rx) = mpsc::channel::<ProcEvent>();

        if let Some(mut out) = stdout {
            let tx_out = tx.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 1024];
                loop {
                    match out.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]).to_string();
                            if tx_out.send(ProcEvent::Stdout(text)).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        if let Some(mut err) = stderr {
            let tx_err = tx.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 1024];
                loop {
                    match err.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]).to_string();
                            if tx_err.send(ProcEvent::Stderr(text)).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        self.logs.push("[INFO] Process started.".to_string());
        self.child = Some(child);
        self.child_stdin = stdin;
        self.rx = Some(rx);
        self.focus = Focus::Input;
        self.trim_logs();
    }

    fn send_input_line(&mut self) {
        if self.input.is_empty() {
            return;
        }

        let line = self.input.clone();
        self.input.clear();

        self.logs.push(format!(">> {}", line));

        match self.child_stdin.as_mut() {
            Some(stdin) => {
                if writeln!(stdin, "{}", line).is_err() || stdin.flush().is_err() {
                    self.logs.push("[ERR] failed to write to process stdin".to_string());
                }
            }
            None => {
                self.logs.push("[INFO] No running process to receive input.".to_string());
            }
        }

        self.trim_logs();
    }

    fn pump_process(&mut self) {
        if let Some(rx) = &self.rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    ProcEvent::Stdout(chunk) => self.logs.extend(split_to_lines(chunk, false)),
                    ProcEvent::Stderr(chunk) => self.logs.extend(split_to_lines(chunk, true)),
                }
            }
        }

        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.logs
                        .push(format!("[INFO] Process exited with status: {}", status));
                    self.child = None;
                    self.child_stdin = None;
                    self.rx = None;
                    self.focus = Focus::Dashboard;
                }
                Ok(None) => {}
                Err(err) => {
                    self.logs
                        .push(format!("[ERROR] Failed checking process status: {}", err));
                    self.child = None;
                    self.child_stdin = None;
                    self.rx = None;
                    self.focus = Focus::Dashboard;
                }
            }
        }

        self.trim_logs();
    }

    fn shutdown(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
        }
        self.child = None;
        self.child_stdin = None;
        self.rx = None;
    }

    fn trim_logs(&mut self) {
        if self.logs.len() > 200 {
            let overflow = self.logs.len() - 200;
            self.logs.drain(0..overflow);
        }

        let max_scroll = self.max_output_scroll();

        if self.logs.len() != self.last_log_count {
            self.last_log_count = self.logs.len();
            self.follow_output = true;
        }

        if self.follow_output {
            self.output_scroll = max_scroll;
        } else if self.output_scroll > max_scroll {
            self.output_scroll = max_scroll;
        }
    }
}

fn split_to_lines(chunk: String, is_stderr: bool) -> Vec<String> {
    let normalized = chunk.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = Vec::new();

    for part in normalized.split('\n') {
        if part.is_empty() {
            continue;
        }

        if is_stderr {
            out.push(format!("[ERR] {}", part));
        } else {
            out.push(part.to_string());
        }
    }

    out
}

fn menu_allowed_on_current_os(menu: MenuKind) -> bool {
    match menu {
        MenuKind::RunWin11 => cfg!(windows),
        MenuKind::RunUbuntu22 => !cfg!(windows),
        MenuKind::Root => true,
    }
}

fn menu_items(menu: MenuKind) -> Vec<CommandItem> {
    match menu {
        MenuKind::Root => vec![
            CommandItem {
                label: "vn ai \"prompt\"",
                action: Action::Placeholder("Ongoing local AI API"),
            },
            CommandItem {
                label: "vn sys info",
                action: Action::Execute(vec!["sys", "info"]),
            },
            CommandItem {
                label: "vn run ubuntu22",
                action: Action::OpenMenu(MenuKind::RunUbuntu22),
            },
            CommandItem {
                label: "vn run win11",
                action: Action::OpenMenu(MenuKind::RunWin11),
            },
        ],
        MenuKind::RunUbuntu22 => vec![
            CommandItem {
                label: "vn run ubuntu22-check-internet",
                action: Action::Execute(vec!["run", "ubuntu22-check-internet"]),
            },
            CommandItem {
                label: "vn run ubuntu22-check-dependencies",
                action: Action::Execute(vec!["run", "ubuntu22-check-dependencies"]),
            },
            CommandItem {
                label: "vn run ubuntu22-download-all-repos",
                action: Action::Execute(vec!["run", "ubuntu22-download-all-repos"]),
            },
            CommandItem {
                label: "vn run ubuntu22-download-all-orgs",
                action: Action::Execute(vec!["run", "ubuntu22-download-all-orgs"]),
            },
            CommandItem {
                label: "vn run ubuntu22-run-cli-container",
                action: Action::Execute(vec!["run", "ubuntu22-run-cli-container"]),
            },
            CommandItem {
                label: "vn run ubuntu22-run-silverbullet",
                action: Action::Execute(vec!["run", "ubuntu22-run-silverbullet"]),
            },
            CommandItem {
                label: "< Back to Dashboard",
                action: Action::BackToRoot,
            },
        ],
        MenuKind::RunWin11 => vec![
            CommandItem {
                label: "vn run win11-check-internet",
                action: Action::Execute(vec!["run", "win11-check-internet"]),
            },
            CommandItem {
                label: "vn run win11-check-dependencies",
                action: Action::Execute(vec!["run", "win11-check-dependencies"]),
            },
            CommandItem {
                label: "vn run win11-download-all-repos",
                action: Action::Execute(vec!["run", "win11-download-all-repos"]),
            },
            CommandItem {
                label: "vn run win11-open-docker",
                action: Action::Execute(vec!["run", "win11-open-docker"]),
            },
            CommandItem {
                label: "vn run win11-check-docker",
                action: Action::Execute(vec!["run", "win11-check-docker"]),
            },
            CommandItem {
                label: "vn run win11-run-silverbullet",
                action: Action::Execute(vec!["run", "win11-run-silverbullet"]),
            },
            CommandItem {
                label: "< Back to Dashboard",
                action: Action::BackToRoot,
            },
        ],
    }
}

pub fn run() -> Result<()> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let mut app = AppState::new();

    loop {
        app.pump_process();

        terminal.draw(|frame| {
            let areas = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(4),
                ])
                .split(frame.area());

            let header = Paragraph::new("vecnode vn")
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL).title("CLI"));

            let middle = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(areas[1]);

            let button_items: Vec<ListItem> = app
                .commands
                .iter()
                .enumerate()
                .map(|(idx, cmd)| {
                    if idx == app.selected {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("[ {} ]", cmd.label),
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("[ {} ]", cmd.label),
                                Style::default().fg(Color::White),
                            ),
                        ]))
                    }
                })
                .collect();

            let mut list_state = ListState::default();
            list_state.select(Some(app.selected));

            let dashboard = List::new(button_items)
                .block(Block::default().borders(Borders::ALL).title("Dashboard"));

            let log_lines: Vec<Line> = app
                .logs
                .iter()
                .map(|entry| {
                    if entry.starts_with("> ") {
                        Line::from(Span::styled(
                            entry.clone(),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::LightGreen)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else if entry.starts_with("[ERR]") {
                        Line::from(Span::styled(
                            entry.clone(),
                            Style::default().fg(Color::LightRed),
                        ))
                    } else {
                        Line::from(Span::raw(entry.clone()))
                    }
                })
                .collect();

            app.output_view_lines = middle[1].height.saturating_sub(2) as usize;
            let max_scroll = app.max_output_scroll();
            if app.follow_output {
                app.output_scroll = max_scroll;
            } else if app.output_scroll > max_scroll {
                app.output_scroll = max_scroll;
            }

            let output = Paragraph::new(log_lines)
                .block(Block::default().borders(Borders::ALL).title("CLI Output"))
                .scroll((app.output_scroll, 0));

            let keys_text = match app.focus {
                Focus::Dashboard => {
                    "Tab: input  Up/Down: select  Enter: run  ,/.: output page  q/Esc: exit"
                }
                Focus::Input => {
                    "Tab: dashboard  Enter: send input  Backspace: delete  ,/.: output page  q/Esc: exit"
                }
            };

            let input_text = if app.input.is_empty() {
                "Type input for running command...".to_string()
            } else {
                app.input.clone()
            };

            let input_style = Style::default().fg(Color::White).bg(Color::Green);

            let footer = Paragraph::new(vec![
                Line::from(Span::styled(input_text, input_style)),
                Line::from(Span::styled(keys_text, Style::default().fg(Color::White))),
            ])
            .block(Block::default().borders(Borders::ALL).title("Input"));

            frame.render_widget(header, areas[0]);
            frame.render_stateful_widget(dashboard, middle[0], &mut list_state);
            frame.render_widget(output, middle[1]);
            frame.render_widget(footer, areas[2]);
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.shutdown();
                        break;
                    }
                    KeyCode::Tab => {
                        app.focus = match app.focus {
                            Focus::Dashboard => Focus::Input,
                            Focus::Input => Focus::Dashboard,
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if matches!(app.focus, Focus::Dashboard) {
                            app.next();
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if matches!(app.focus, Focus::Dashboard) {
                            app.previous();
                        }
                    }
                    KeyCode::Char(',') => {
                        app.output_page_up();
                    }
                    KeyCode::Char('.') => {
                        app.output_page_down();
                    }
                    KeyCode::Enter => {
                        if matches!(app.focus, Focus::Dashboard) {
                            app.activate_selected();
                        } else {
                            app.send_input_line();
                        }
                    }
                    KeyCode::Backspace => {
                        if matches!(app.focus, Focus::Input) {
                            app.input.pop();
                        }
                    }
                    KeyCode::Char(c) => {
                        if matches!(app.focus, Focus::Input) {
                            app.input.push(c);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
