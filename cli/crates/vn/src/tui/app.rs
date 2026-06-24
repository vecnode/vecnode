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
use std::io::{self, Stdout};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy)]
enum MenuKind {
    Root,
    RunUbuntu22,
    RunUbuntu22Network,
    RunUbuntu22Dependencies,
    RunUbuntu22Github,
    RunUbuntu22Docker,
    RunUbuntu22Open,
    RunUbuntu22Ai,
    RunWin11,
    RunWin11Network,
    RunWin11Dependencies,
    RunWin11Github,
    RunWin11Docker,
    RunWin11Open,
    RunWin11Ai,
    RunWin11Dotfiles,
}

#[derive(Clone)]
enum Action {
    Execute(Vec<&'static str>),
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

enum LogEntry {
    Command(String),
    Info(String),
    Error(String),
    Stdout(String),
    Stderr(String),
}

enum Focus {
    Dashboard,
    Input,
}

struct DockerPanelData {
    available: bool,
    images: Vec<String>,
    ports: Vec<String>,
}

struct RunningProcess {
    label: String,
    child: Child,
}

struct AppState {
    menu: MenuKind,
    commands: Vec<CommandItem>,
    selected: usize,
    repo_root: Option<std::path::PathBuf>,
    logs: Vec<LogEntry>,
    running: Vec<RunningProcess>,
    tx: Sender<ProcEvent>,
    rx: Receiver<ProcEvent>,
    input: String,
    focus: Focus,
    output_scroll: u16,
    output_view_lines: usize,
    follow_output: bool,
    last_log_count: usize,
    docker_panel: DockerPanelData,
}

impl AppState {
    fn new(repo_root: Option<std::path::PathBuf>) -> Self {
        let (tx, rx) = mpsc::channel::<ProcEvent>();

        let mut app = Self {
            menu: MenuKind::Root,
            commands: menu_items(MenuKind::Root),
            selected: 0,
            repo_root,
            logs: vec![],
            running: Vec::new(),
            tx,
            rx,
            input: String::new(),
            focus: Focus::Dashboard,
            output_scroll: 0,
            output_view_lines: 0,
            follow_output: true,
            last_log_count: 1,
            docker_panel: DockerPanelData {
                available: false,
                images: Vec::new(),
                ports: Vec::new(),
            },
        };

        app.refresh_ui();
        app
    }

    fn refresh_ui(&mut self) {
        self.refresh_docker_panel();
    }

    fn refresh_docker_panel(&mut self) {
        let images_out = Command::new("docker")
            .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
            .output();

        let images_out = match images_out {
            Ok(out) if out.status.success() => out,
            _ => {
                self.docker_panel.available = false;
                self.docker_panel.images.clear();
                self.docker_panel.ports.clear();
                return;
            }
        };

        let ports_out = Command::new("docker")
            .args(["ps", "--format", "{{.Ports}}"])
            .output();

        let ports_out = match ports_out {
            Ok(out) if out.status.success() => out,
            _ => {
                self.docker_panel.available = false;
                self.docker_panel.images.clear();
                self.docker_panel.ports.clear();
                return;
            }
        };

        let images_text = String::from_utf8_lossy(&images_out.stdout);
        let mut images: Vec<String> = images_text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && *line != "<none>:<none>")
            .take(6)
            .map(ToOwned::to_owned)
            .collect();
        images.sort();
        images.dedup();

        let ports_text = String::from_utf8_lossy(&ports_out.stdout);
        let mut ports: Vec<String> = ports_text
            .lines()
            .flat_map(extract_host_ports)
            .collect();
        ports.sort();
        ports.dedup();

        self.docker_panel.available = true;
        self.docker_panel.images = images;
        self.docker_panel.ports = ports;
    }

    fn docker_panel_lines(&self) -> Vec<Line<'static>> {
        if !self.docker_panel.available {
            return vec![Line::from("Docker is not available")];
        }

        let mut lines = Vec::new();

        lines.push(Line::from("Docker Images:"));
        if self.docker_panel.images.is_empty() {
            lines.push(Line::from("- none"));
        } else {
            for image in self.docker_panel.images.iter().take(3) {
                lines.push(Line::from(format!("- {}", image)));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from("Docker Ports:"));
        if self.docker_panel.ports.is_empty() {
            lines.push(Line::from("- none"));
        } else {
            for port in &self.docker_panel.ports {
                lines.push(Line::from(format!("- {}", port)));
            }
        }

        lines
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

        let item = self.commands[self.selected].clone();

        match item.action {
            Action::OpenMenu(next_menu) => {
                if !menu_allowed_on_current_os(next_menu) {
                    self.logs.push(LogEntry::Command(item.label.to_string()));
                    self.logs.push(LogEntry::Error(
                        "[WARNING] This submenu is not supported on the current OS.".to_string(),
                    ));
                    if cfg!(windows) {
                        self.logs.push(LogEntry::Info(
                            "[INFO] Windows host: use vn run win11 submenu items.".to_string(),
                        ));
                    } else {
                        self.logs.push(LogEntry::Info(
                            "[INFO] Non-Windows host: use vn run ubuntu22 submenu items.".to_string(),
                        ));
                    }
                    self.trim_logs();
                    return;
                }

                self.logs.push(LogEntry::Command(item.label.to_string()));
                self.logs.push(LogEntry::Info("[INFO] Opened submenu.".to_string()));
                self.set_menu(next_menu);
                self.trim_logs();
                return;
            }
            Action::BackToRoot => {
                self.logs.push(LogEntry::Command(item.label.to_string()));
                self.logs.push(LogEntry::Info("[INFO] Returned to Dashboard.".to_string()));
                self.set_menu(MenuKind::Root);
                self.trim_logs();
                return;
            }
            Action::Execute(args) => {
                self.spawn_process(item.label, args);
            }
        }
    }

    fn spawn_process(&mut self, label: &str, args: Vec<&'static str>) {
        self.logs.push(LogEntry::Command(label.to_string()));

        let exe = match env::current_exe() {
            Ok(path) => path,
            Err(err) => {
                self.logs.push(LogEntry::Error(format!(
                    "[ERROR] Could not resolve current executable: {}",
                    err
                )));
                self.trim_logs();
                return;
            }
        };

        let mut cmd = Command::new(exe);
        if let Some(repo_root) = &self.repo_root {
            cmd.arg("--repo-root").arg(repo_root);
            cmd.current_dir(repo_root);
            cmd.env("VECNODE_REPO_ROOT", repo_root);
        }

        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(err) => {
                self.logs.push(LogEntry::Error(format!(
                    "[ERROR] Failed to start command: {}",
                    err
                )));
                self.trim_logs();
                return;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        if let Some(mut out) = stdout {
            let tx_out = self.tx.clone();
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
            let tx_err = self.tx.clone();
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

        self.logs
            .push(LogEntry::Info("[INFO] Process started in background.".to_string()));
        self.running.push(RunningProcess {
            label: label.to_string(),
            child,
        });
        self.trim_logs();
    }

    fn send_input_line(&mut self) {
        if self.input.trim().is_empty() {
            self.input.clear();
            return;
        }

        self.input.clear();
        self.logs.push(LogEntry::Info(
            "[INFO] Input forwarding is disabled. Commands run in background mode.".to_string(),
        ));

        self.trim_logs();
    }

    fn pump_process(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                ProcEvent::Stdout(chunk) => self
                    .logs
                    .extend(split_to_entries(chunk, false).into_iter().map(LogEntry::Stdout)),
                ProcEvent::Stderr(chunk) => self
                    .logs
                    .extend(split_to_entries(chunk, true).into_iter().map(LogEntry::Stderr)),
            }
        }

        let mut idx = 0;
        while idx < self.running.len() {
            let (remove_current, message) = {
                let proc = &mut self.running[idx];
                match proc.child.try_wait() {
                    Ok(Some(status)) => (
                        true,
                        Some(LogEntry::Info(format!(
                            "[INFO] Process '{}' exited with status: {}",
                            proc.label, status
                        ))),
                    ),
                    Ok(None) => (false, None),
                    Err(err) => (
                        true,
                        Some(LogEntry::Error(format!(
                            "[ERROR] Failed checking process '{}' status: {}",
                            proc.label, err
                        ))),
                    ),
                }
            };

            if let Some(message) = message {
                self.logs.push(message);
            }

            if remove_current {
                self.running.remove(idx);
            } else {
                idx += 1;
            }
        }

        self.trim_logs();
    }

    fn shutdown(&mut self) {
        for proc in &mut self.running {
            let _ = proc.child.kill();
        }
        self.running.clear();
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

fn split_to_entries(chunk: String, is_stderr: bool) -> Vec<String> {
    let normalized = chunk.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = Vec::new();

    for part in normalized.split('\n') {
        if part.is_empty() {
            continue;
        }

        if is_stderr {
            if is_non_error_stderr_line(part) {
                out.push(part.to_string());
            } else {
                out.push(format!("[ERR] {}", part));
            }
        } else {
            out.push(part.to_string());
        }
    }

    out
}

fn is_non_error_stderr_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }

    // Docker BuildKit and CLI tools often print normal progress to stderr.
    if trimmed.starts_with('#') {
        return true;
    }

    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("sending build context")
        || lower.starts_with("step ")
        || lower.starts_with(" --->")
        || lower.starts_with("successfully built")
        || lower.starts_with("successfully tagged")
        || lower.starts_with("naming to ")
        || lower.starts_with("exporting ")
        || lower.starts_with("transferring ")
        || lower.starts_with("unpacking ")
        || lower.starts_with("load build definition")
        || lower.starts_with("load metadata")
        || lower.starts_with("load .dockerignore")
        || lower.starts_with("build context")
}

fn extract_host_ports(line: &str) -> Vec<String> {
    line.split(',')
        .filter_map(|entry| {
            let part = entry.trim();
            if part.is_empty() {
                return None;
            }

            let mapped = part.split("->").next().unwrap_or(part).trim();
            let host_segment = mapped.rsplit(':').next().unwrap_or("").trim();
            let host_port = host_segment.split('/').next().unwrap_or("").trim();

            if !host_port.is_empty()
                && host_port
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '-')
            {
                Some(host_port.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn menu_allowed_on_current_os(menu: MenuKind) -> bool {
    match menu {
        MenuKind::RunUbuntu22
        | MenuKind::RunUbuntu22Network
        | MenuKind::RunUbuntu22Dependencies
        | MenuKind::RunUbuntu22Github
        | MenuKind::RunUbuntu22Docker
        | MenuKind::RunUbuntu22Open
        | MenuKind::RunUbuntu22Ai => !cfg!(windows),
        MenuKind::RunWin11
        | MenuKind::RunWin11Network
        | MenuKind::RunWin11Dependencies
        | MenuKind::RunWin11Github
        | MenuKind::RunWin11Docker
        | MenuKind::RunWin11Open
        | MenuKind::RunWin11Ai
        | MenuKind::RunWin11Dotfiles => cfg!(windows),
        MenuKind::Root => true,
    }
}

fn menu_items(menu: MenuKind) -> Vec<CommandItem> {
    match menu {
        MenuKind::Root => vec![
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
                label: "vn run ubuntu22-ai",
                action: Action::OpenMenu(MenuKind::RunUbuntu22Ai),
            },
            CommandItem {
                label: "vn run ubuntu22-network",
                action: Action::OpenMenu(MenuKind::RunUbuntu22Network),
            },
            CommandItem {
                label: "vn run ubuntu22-dependencies",
                action: Action::OpenMenu(MenuKind::RunUbuntu22Dependencies),
            },
            CommandItem {
                label: "vn run ubuntu22-github",
                action: Action::OpenMenu(MenuKind::RunUbuntu22Github),
            },
            CommandItem {
                label: "vn run ubuntu22-docker",
                action: Action::OpenMenu(MenuKind::RunUbuntu22Docker),
            },
            CommandItem {
                label: "vn run ubuntu22-open",
                action: Action::OpenMenu(MenuKind::RunUbuntu22Open),
            },
            CommandItem {
                label: "< Back to Dashboard",
                action: Action::BackToRoot,
            },
        ],
        MenuKind::RunUbuntu22Network => vec![
            CommandItem {
                label: "vn net scan (rustscan open ports, local /24)",
                action: Action::Execute(vec!["net", "scan"]),
            },
            CommandItem {
                label: "vn run ubuntu22-check-internet",
                action: Action::Execute(vec!["run", "ubuntu22-check-internet"]),
            },
            CommandItem {
                label: "< Back to ubuntu22",
                action: Action::OpenMenu(MenuKind::RunUbuntu22),
            },
        ],
        MenuKind::RunUbuntu22Dependencies => vec![
            CommandItem {
                label: "vn run ubuntu22-check-dependencies",
                action: Action::Execute(vec!["run", "ubuntu22-check-dependencies"]),
            },
            CommandItem {
                label: "< Back to ubuntu22",
                action: Action::OpenMenu(MenuKind::RunUbuntu22),
            },
        ],
        MenuKind::RunUbuntu22Github => vec![
            CommandItem {
                label: "vn run ubuntu22-download-all-repos",
                action: Action::Execute(vec!["run", "ubuntu22-download-all-repos"]),
            },
            CommandItem {
                label: "vn run ubuntu22-download-all-orgs",
                action: Action::Execute(vec!["run", "ubuntu22-download-all-orgs"]),
            },
            CommandItem {
                label: "< Back to ubuntu22",
                action: Action::OpenMenu(MenuKind::RunUbuntu22),
            },
        ],
        MenuKind::RunUbuntu22Docker => vec![
            CommandItem {
                label: "vn run ubuntu22-open-docker",
                action: Action::Execute(vec!["run", "ubuntu22-open-docker"]),
            },
            CommandItem {
                label: "vn run ubuntu22-check-docker",
                action: Action::Execute(vec!["run", "ubuntu22-check-docker"]),
            },
            CommandItem {
                label: "vn run ubuntu22-remove-containers",
                action: Action::Execute(vec!["run", "ubuntu22-remove-containers"]),
            },
            CommandItem {
                label: "vn run ubuntu22-remove-images",
                action: Action::Execute(vec!["run", "ubuntu22-remove-images"]),
            },
            CommandItem {
                label: "< Back to ubuntu22",
                action: Action::OpenMenu(MenuKind::RunUbuntu22),
            },
        ],
        MenuKind::RunUbuntu22Open => vec![
            CommandItem {
                label: "vn run ubuntu22-open-docs",
                action: Action::Execute(vec!["run", "ubuntu22-open-docs"]),
            },
            CommandItem {
                label: "vn run ubuntu22-open-silverbullet",
                action: Action::Execute(vec!["run", "ubuntu22-open-silverbullet"]),
            },
            CommandItem {
                label: "vn run ubuntu22-open-media-processor",
                action: Action::Execute(vec!["run", "ubuntu22-open-media-processor"]),
            },
            CommandItem {
                label: "< Back to ubuntu22",
                action: Action::OpenMenu(MenuKind::RunUbuntu22),
            },
        ],
        MenuKind::RunUbuntu22Ai => vec![
            CommandItem {
                label: "vn run ubuntu22-check-ollama",
                action: Action::Execute(vec!["run", "ubuntu22-check-ollama"]),
            },
            CommandItem {
                label: "vn run ubuntu22-open-ollama",
                action: Action::Execute(vec!["run", "ubuntu22-open-ollama"]),
            },
            CommandItem {
                label: "< Back to ubuntu22",
                action: Action::OpenMenu(MenuKind::RunUbuntu22),
            },
        ],
        MenuKind::RunWin11 => vec![
            CommandItem {
                label: "vn run win11-ai",
                action: Action::OpenMenu(MenuKind::RunWin11Ai),
            },
            CommandItem {
                label: "vn run win11-dotfiles",
                action: Action::OpenMenu(MenuKind::RunWin11Dotfiles),
            },
            CommandItem {
                label: "vn run win11-network",
                action: Action::OpenMenu(MenuKind::RunWin11Network),
            },
            CommandItem {
                label: "vn run win11-dependencies",
                action: Action::OpenMenu(MenuKind::RunWin11Dependencies),
            },
            CommandItem {
                label: "vn run win11-github",
                action: Action::OpenMenu(MenuKind::RunWin11Github),
            },
            CommandItem {
                label: "vn run win11-docker",
                action: Action::OpenMenu(MenuKind::RunWin11Docker),
            },
            CommandItem {
                label: "vn run win11-open",
                action: Action::OpenMenu(MenuKind::RunWin11Open),
            },
            CommandItem {
                label: "< Back to Dashboard",
                action: Action::BackToRoot,
            },
        ],
        MenuKind::RunWin11Network => vec![
            CommandItem {
                label: "vn run win11-check-peripherals",
                action: Action::Execute(vec!["run", "win11-check-peripherals"]),
            },
            CommandItem {
                label: "vn net scan (rustscan open ports, local /24)",
                action: Action::Execute(vec!["net", "scan"]),
            },
            CommandItem {
                label: "vn run win11-check-internet",
                action: Action::Execute(vec!["run", "win11-check-internet"]),
            },
            CommandItem {
                label: "< Back to win11",
                action: Action::OpenMenu(MenuKind::RunWin11),
            },
        ],
        MenuKind::RunWin11Dependencies => vec![
            CommandItem {
                label: "vn run win11-check-dependencies",
                action: Action::Execute(vec!["run", "win11-check-dependencies"]),
            },
            CommandItem {
                label: "< Back to win11",
                action: Action::OpenMenu(MenuKind::RunWin11),
            },
        ],
        MenuKind::RunWin11Github => vec![
            CommandItem {
                label: "vn run win11-download-all-repos",
                action: Action::Execute(vec!["run", "win11-download-all-repos"]),
            },
            CommandItem {
                label: "< Back to win11",
                action: Action::OpenMenu(MenuKind::RunWin11),
            },
        ],
        MenuKind::RunWin11Docker => vec![
            CommandItem {
                label: "vn run win11-open-docker",
                action: Action::Execute(vec!["run", "win11-open-docker"]),
            },
            CommandItem {
                label: "vn run win11-check-docker",
                action: Action::Execute(vec!["run", "win11-check-docker"]),
            },
            CommandItem {
                label: "vn run win11-remove-containers",
                action: Action::Execute(vec!["run", "win11-remove-containers"]),
            },
            CommandItem {
                label: "vn run win11-remove-images",
                action: Action::Execute(vec!["run", "win11-remove-images"]),
            },
            CommandItem {
                label: "< Back to win11",
                action: Action::OpenMenu(MenuKind::RunWin11),
            },
        ],
        MenuKind::RunWin11Open => vec![
            CommandItem {
                label: "vn run win11-open-docs",
                action: Action::Execute(vec!["run", "win11-open-docs"]),
            },
            CommandItem {
                label: "vn run win11-open-silverbullet",
                action: Action::Execute(vec!["run", "win11-open-silverbullet"]),
            },
            CommandItem {
                label: "vn run win11-open-media-processor",
                action: Action::Execute(vec!["run", "win11-open-media-processor"]),
            },
            CommandItem {
                label: "vn run win11-open-media-processor-dev",
                action: Action::Execute(vec!["run", "win11-open-media-processor-dev"]),
            },
            CommandItem {
                label: "< Back to win11",
                action: Action::OpenMenu(MenuKind::RunWin11),
            },
        ],
        MenuKind::RunWin11Ai => vec![
            CommandItem {
                label: "vn run win11-check-ollama",
                action: Action::Execute(vec!["run", "win11-check-ollama"]),
            },
            CommandItem {
                label: "vn run win11-open-ollama",
                action: Action::Execute(vec!["run", "win11-open-ollama"]),
            },
            CommandItem {
                label: "< Back to win11",
                action: Action::OpenMenu(MenuKind::RunWin11),
            },
        ],
        MenuKind::RunWin11Dotfiles => vec![
            CommandItem {
                label: "vn run win11-setup-dotfiles",
                action: Action::Execute(vec!["run", "win11-setup-dotfiles"]),
            },
            CommandItem {
                label: "< Back to win11",
                action: Action::OpenMenu(MenuKind::RunWin11),
            },
        ],
    }
}

pub fn run(repo_root: Option<std::path::PathBuf>) -> Result<()> {
    if let Some(repo_root) = &repo_root {
        env::set_var("VECNODE_REPO_ROOT", repo_root);
        env::set_current_dir(repo_root)?;
    }

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, repo_root);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    repo_root: Option<std::path::PathBuf>,
) -> Result<()> {
    let mut app = AppState::new(repo_root);

    loop {
        app.pump_process();

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
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
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        app.refresh_ui();
                        terminal.clear()?;
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
                },
                Event::Resize(_, _) => {
                    terminal.clear()?;
                }
                Event::FocusGained | Event::FocusLost => {}
                _ => {}
            }
        }

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
                .style(Style::default().add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL).title("CLI"));

            let middle = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(areas[1]);

            let left_panels = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(middle[0]);

            let button_items: Vec<ListItem> = app
                .commands
                .iter()
                .enumerate()
                .map(|(idx, cmd)| {
                    if idx == app.selected {
                        ListItem::new(Line::from(vec![Span::styled(
                            format!("[ {} ]", cmd.label),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )]))
                    } else {
                        ListItem::new(Line::from(vec![Span::styled(
                            format!("[ {} ]", cmd.label),
                            Style::default().fg(Color::White),
                        )]))
                    }
                })
                .collect();

            let mut list_state = ListState::default();
            list_state.select(Some(app.selected));

            let dashboard = List::new(button_items)
                .block(Block::default().borders(Borders::ALL).title("Dashboard"));

            let docker = Paragraph::new(app.docker_panel_lines())
                .block(Block::default().borders(Borders::ALL).title("Docker"));

            let log_lines: Vec<Line> = app
                .logs
                .iter()
                .map(|entry| {
                    if let LogEntry::Command(command) = entry {
                        Line::from(Span::styled(
                            format!("> {}", command),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::LightGreen)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else if let LogEntry::Error(text) = entry {
                        Line::from(Span::styled(text.clone(), Style::default().fg(Color::LightRed)))
                    } else if let LogEntry::Stderr(text) = entry {
                        Line::from(Span::styled(text.clone(), Style::default().fg(Color::LightYellow)))
                    } else if let LogEntry::Stdout(text) = entry {
                        Line::from(Span::raw(text.clone()))
                    } else {
                        match entry {
                            LogEntry::Info(text) => Line::from(Span::raw(text.clone())),
                            _ => Line::from(Span::raw(String::new())),
                        }
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
                    "Tab: input  Up/Down: select  Enter: run  R: refresh ui  ,/.: output page  q/Esc: exit"
                }
                Focus::Input => {
                    "Tab: dashboard  Enter: send input  Backspace: delete  R: refresh ui  ,/.: output page  q/Esc: exit"
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
            frame.render_stateful_widget(dashboard, left_panels[0], &mut list_state);
            frame.render_widget(docker, left_panels[1]);
            frame.render_widget(output, middle[1]);
            frame.render_widget(footer, areas[2]);
        })?;
    }

    Ok(())
}
