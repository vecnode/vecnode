use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Row, Table};
use ratatui::Terminal;
use chrono::Local;
use std::env;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::io::{self, Stdout};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

// ---- Theme: one cyan accent (the Docker header / button blue) on the
// terminal's (black) background, plus two grays. Two "blues" total: this cyan
// used as a fill (selection / focus) and as text (headers, commands). ----
const ACCENT: Color = Color::Cyan; // the single blue accent
const DIM: Color = Color::DarkGray; // de-emphasized borders/text
const MUTED: Color = Color::Gray; // secondary text / inactive titles

#[derive(Clone, Copy, PartialEq)]
enum MenuKind {
    Root,
    RunUbuntu22,
    RunUbuntu22Network,
    RunUbuntu22Dependencies,
    RunUbuntu22Github,
    RunUbuntu22Open,
    RunUbuntu22Ai,
    RunWin11,
    RunWin11Network,
    RunWin11Dependencies,
    RunWin11Github,
    RunWin11Open,
    RunWin11Ai,
    RunWin11Dotfiles,
    SelectModel,
}

/// What a typed line in the input box should do when submitted.
#[derive(Clone, Copy, PartialEq)]
enum InputPurpose {
    None,
    DownloadModel,
    Chat,
}

#[derive(Clone)]
enum Action {
    Execute(Vec<&'static str>),
    OpenMenu(MenuKind),
    BackToRoot,
    /// Query Ollama for installed models and open the model-selection menu.
    OpenModelMenu,
    /// Focus the input box and route the next submitted line to this purpose.
    ArmInput(InputPurpose),
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
    /// One row per running container: (port, image, container name).
    rows: Vec<(String, String, String)>,
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
    log_file: Option<std::fs::File>,
    selected_model: Option<String>,
    model_items: Vec<String>,
    input_purpose: InputPurpose,
}

impl AppState {
    fn new(repo_root: Option<std::path::PathBuf>) -> Self {
        let (tx, rx) = mpsc::channel::<ProcEvent>();

        let log_file = open_session_log(&repo_root);

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
                rows: Vec::new(),
            },
            log_file,
            selected_model: None,
            model_items: Vec::new(),
            input_purpose: InputPurpose::None,
        };

        app.refresh_ui();
        app
    }

    /// Append a single log entry to the on-disk session log, prefixed with a
    /// local timestamp. Commands get a prominent marker so a session reads as a
    /// dated history of what was run and what it printed. Mirrors what the
    /// "CLI Output" panel shows; failures to write are intentionally silent so
    /// logging never disrupts the TUI.
    fn write_log_line(&mut self, entry: &LogEntry) {
        let Some(file) = self.log_file.as_mut() else {
            return;
        };

        let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = match entry {
            LogEntry::Command(text) => format!("[{}] >>> COMMAND: {}", ts, text),
            LogEntry::Info(text) => format!("[{}] INFO  | {}", ts, text),
            LogEntry::Error(text) => format!("[{}] ERROR | {}", ts, text),
            LogEntry::Stdout(text) => format!("[{}] OUT   | {}", ts, text),
            LogEntry::Stderr(text) => format!("[{}] ERR   | {}", ts, text),
        };

        let _ = writeln!(file, "{}", line);
        let _ = file.flush();
    }

    /// Record one log entry: persist it to the session log and show it in the
    /// "CLI Output" panel. Replaces direct pushes to `self.logs` so every line
    /// is logged in exactly one place.
    fn push_log(&mut self, entry: LogEntry) {
        self.write_log_line(&entry);
        self.logs.push(entry);
    }

    /// Record many log entries (used for streamed stdout/stderr chunks).
    fn extend_log<I: IntoIterator<Item = LogEntry>>(&mut self, entries: I) {
        for entry in entries {
            self.push_log(entry);
        }
    }

    fn refresh_ui(&mut self) {
        self.refresh_docker_panel();
    }

    fn refresh_docker_panel(&mut self) {
        // One line per running container: ports, image and name.
        let out = Command::new("docker")
            .args(["ps", "--format", "{{.Ports}}\t{{.Image}}\t{{.Names}}"])
            .output();

        let out = match out {
            Ok(out) if out.status.success() => out,
            _ => {
                self.docker_panel.available = false;
                self.docker_panel.rows.clear();
                return;
            }
        };

        let text = String::from_utf8_lossy(&out.stdout);
        let mut rows: Vec<(String, String, String)> = Vec::new();
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.splitn(3, '\t');
            let ports_raw = parts.next().unwrap_or("");
            let image = parts.next().unwrap_or("").to_string();
            let name = parts.next().unwrap_or("").to_string();

            let host_ports = extract_host_ports(ports_raw);
            let port = if host_ports.is_empty() {
                "-".to_string()
            } else {
                host_ports.join(",")
            };
            rows.push((port, image, name));
        }
        rows.sort_by(|a, b| a.2.cmp(&b.2));

        self.docker_panel.available = true;
        self.docker_panel.rows = rows;
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
        // The model-selection menu is built dynamically from `model_items`;
        // every other menu uses the static menu tree.
        self.commands = if menu == MenuKind::SelectModel {
            Vec::new()
        } else {
            menu_items(menu)
        };
        self.selected = 0;
    }

    /// Number of selectable rows in the current menu. The model menu shows at
    /// least one row (a placeholder when no models are installed).
    fn item_count(&self) -> usize {
        if self.menu == MenuKind::SelectModel {
            self.model_items.len().max(1)
        } else {
            self.commands.len()
        }
    }

    /// The AI submenu to return to after the model menu, per host OS.
    fn ai_back_menu(&self) -> MenuKind {
        if cfg!(windows) {
            MenuKind::RunWin11Ai
        } else {
            MenuKind::RunUbuntu22Ai
        }
    }

    /// Run `vn ai models` and load the printed model names into `model_items`.
    /// Blocks briefly; if Ollama is down the command fails fast and the list
    /// stays empty.
    fn load_models_into_menu(&mut self) {
        self.model_items.clear();

        let exe = match env::current_exe() {
            Ok(path) => path,
            Err(_) => return,
        };

        let mut cmd = Command::new(exe);
        if let Some(repo_root) = &self.repo_root {
            cmd.arg("--repo-root").arg(repo_root);
        }
        cmd.args(["ai", "models"]).stdin(Stdio::null());

        if let Ok(output) = cmd.output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let name = line.trim();
                if !name.is_empty() {
                    self.model_items.push(name.to_string());
                }
            }
        }
    }

    fn next(&mut self) {
        let count = self.item_count();
        if count > 0 {
            self.selected = (self.selected + 1) % count;
        }
    }

    fn previous(&mut self) {
        let count = self.item_count();
        if count == 0 {
            return;
        }
        if self.selected == 0 {
            self.selected = count - 1;
        } else {
            self.selected -= 1;
        }
    }

    fn activate_selected(&mut self) {
        // The model-selection menu is dynamic and has no CommandItem entries;
        // handle picking a model (or the empty placeholder) up front.
        if self.menu == MenuKind::SelectModel {
            let back = self.ai_back_menu();
            if self.model_items.is_empty() {
                self.push_log(LogEntry::Info(
                    "[INFO] No models installed. Use Download Model to fetch one.".to_string(),
                ));
                self.set_menu(back);
                self.trim_logs();
                return;
            }
            let name = self.model_items[self.selected].clone();
            self.selected_model = Some(name.clone());
            self.push_log(LogEntry::Command(format!("select model: {}", name)));
            self.push_log(LogEntry::Info(format!(
                "[INFO] Active model set to '{}'.",
                name
            )));
            self.set_menu(back);
            self.trim_logs();
            return;
        }

        if self.commands.is_empty() {
            return;
        }

        let item = self.commands[self.selected].clone();

        match item.action {
            Action::OpenMenu(next_menu) => {
                if !menu_allowed_on_current_os(next_menu) {
                    self.push_log(LogEntry::Command(item.label.to_string()));
                    self.push_log(LogEntry::Error(
                        "[WARNING] This submenu is not supported on the current OS.".to_string(),
                    ));
                    if cfg!(windows) {
                        self.push_log(LogEntry::Info(
                            "[INFO] Windows host: use vn run win11 submenu items.".to_string(),
                        ));
                    } else {
                        self.push_log(LogEntry::Info(
                            "[INFO] Non-Windows host: use vn run ubuntu22 submenu items.".to_string(),
                        ));
                    }
                    self.trim_logs();
                    return;
                }

                self.push_log(LogEntry::Command(item.label.to_string()));
                self.push_log(LogEntry::Info("[INFO] Opened submenu.".to_string()));
                self.set_menu(next_menu);
                self.trim_logs();
                return;
            }
            Action::BackToRoot => {
                self.push_log(LogEntry::Command(item.label.to_string()));
                self.push_log(LogEntry::Info("[INFO] Returned to Dashboard.".to_string()));
                self.set_menu(MenuKind::Root);
                self.trim_logs();
                return;
            }
            Action::Execute(args) => {
                let args = args.into_iter().map(String::from).collect();
                self.spawn_process(item.label, args);
            }
            Action::OpenModelMenu => {
                self.push_log(LogEntry::Command(item.label.to_string()));
                self.push_log(LogEntry::Info("[INFO] Loading installed models...".to_string()));
                self.load_models_into_menu();
                if self.model_items.is_empty() {
                    self.push_log(LogEntry::Info(
                        "[INFO] No models found. Is Ollama running? Try Open Ollama, or use Download Model."
                            .to_string(),
                    ));
                } else {
                    self.push_log(LogEntry::Info(format!(
                        "[INFO] {} model(s) found. Select one and press Enter.",
                        self.model_items.len()
                    )));
                }
                self.set_menu(MenuKind::SelectModel);
                self.trim_logs();
            }
            Action::ArmInput(purpose) => {
                self.input_purpose = purpose;
                self.focus = Focus::Input;
                self.input.clear();
                self.push_log(LogEntry::Command(item.label.to_string()));
                match purpose {
                    InputPurpose::DownloadModel => self.push_log(LogEntry::Info(
                        "[INFO] Type a model name (e.g. llama3.2) and press Enter to download. Tab returns to the dashboard."
                            .to_string(),
                    )),
                    InputPurpose::Chat => {
                        let model = self
                            .selected_model
                            .clone()
                            .unwrap_or_else(|| "default".to_string());
                        self.push_log(LogEntry::Info(format!(
                            "[INFO] Chatting with '{}'. Type a message and press Enter. Tab returns to the dashboard.",
                            model
                        )));
                    }
                    InputPurpose::None => {}
                }
                self.trim_logs();
            }
        }
    }

    fn spawn_process(&mut self, label: &str, args: Vec<String>) {
        self.push_log(LogEntry::Command(label.to_string()));

        let exe = match env::current_exe() {
            Ok(path) => path,
            Err(err) => {
                self.push_log(LogEntry::Error(format!(
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
                self.push_log(LogEntry::Error(format!(
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

        self.push_log(LogEntry::Info(
            "[INFO] Process started in background.".to_string(),
        ));
        self.running.push(RunningProcess {
            label: label.to_string(),
            child,
        });
        self.trim_logs();
    }

    fn send_input_line(&mut self) {
        let text = self.input.trim().to_string();
        self.input.clear();
        if text.is_empty() {
            return;
        }

        match self.input_purpose {
            InputPurpose::DownloadModel => {
                self.spawn_process(
                    "vn ai pull",
                    vec!["ai".to_string(), "pull".to_string(), text],
                );
                // One-shot: return focus to the dashboard after starting.
                self.input_purpose = InputPurpose::None;
                self.focus = Focus::Dashboard;
            }
            InputPurpose::Chat => {
                let model = self.selected_model.clone();
                self.push_log(LogEntry::Command(format!("You: {}", text)));
                let mut args = vec!["ai".to_string(), "chat".to_string()];
                args.push("--session".to_string());
                args.push("tui".to_string());
                if let Some(model) = model {
                    args.push("--model".to_string());
                    args.push(model);
                }
                args.push(text);
                self.spawn_process("vn ai chat", args);
                // Stay in chat mode so the conversation can continue.
            }
            InputPurpose::None => {
                self.push_log(LogEntry::Info(
                    "[INFO] Input is not attached to an action. Use Download Model or Chat first."
                        .to_string(),
                ));
            }
        }

        self.trim_logs();
    }

    fn pump_process(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                ProcEvent::Stdout(chunk) => {
                    self.extend_log(split_to_entries(chunk, false).into_iter().map(LogEntry::Stdout))
                }
                ProcEvent::Stderr(chunk) => {
                    self.extend_log(split_to_entries(chunk, true).into_iter().map(LogEntry::Stderr))
                }
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
                self.push_log(message);
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

/// Open (creating if needed) the persistent TUI session log under
/// `<repo_root>/logs/vn-tui.log` in append mode and write a session header.
/// Falls back to the current directory if no repo root is known. The `logs/`
/// directory and `*.log` files are gitignored, so this never reaches GitHub.
/// Returns `None` if the log cannot be opened; the TUI then runs without
/// file logging rather than failing.
fn open_session_log(repo_root: &Option<std::path::PathBuf>) -> Option<std::fs::File> {
    let base = repo_root
        .clone()
        .or_else(|| env::current_dir().ok())?;
    let dir = base.join("logs");
    std::fs::create_dir_all(&dir).ok()?;

    let path = dir.join("vn-tui.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;

    let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
    let _ = writeln!(file);
    let _ = writeln!(file, "[{}] ===== vn TUI session started =====", ts);
    let _ = file.flush();

    Some(file)
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
        | MenuKind::RunUbuntu22Open
        | MenuKind::RunUbuntu22Ai => !cfg!(windows),
        MenuKind::RunWin11
        | MenuKind::RunWin11Network
        | MenuKind::RunWin11Dependencies
        | MenuKind::RunWin11Github
        | MenuKind::RunWin11Open
        | MenuKind::RunWin11Ai
        | MenuKind::RunWin11Dotfiles => cfg!(windows),
        // Model selection talks to Ollama over HTTP and works on any host.
        MenuKind::Root | MenuKind::SelectModel => true,
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
                label: "vn run ubuntu22-open (apps + docker)",
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
        MenuKind::RunUbuntu22Open => vec![
            CommandItem {
                label: "vn run ubuntu22-open-docker",
                action: Action::Execute(vec!["run", "ubuntu22-open-docker"]),
            },
            CommandItem {
                label: "vn run ubuntu22-check-docker",
                action: Action::Execute(vec!["run", "ubuntu22-check-docker"]),
            },
            CommandItem {
                label: "vn run ubuntu22-stop-all-containers",
                action: Action::Execute(vec!["run", "ubuntu22-stop-all-containers"]),
            },
            CommandItem {
                label: "vn run ubuntu22-remove-all-containers",
                action: Action::Execute(vec!["run", "ubuntu22-remove-all-containers"]),
            },
            CommandItem {
                label: "vn run ubuntu22-remove-all-images",
                action: Action::Execute(vec!["run", "ubuntu22-remove-all-images"]),
            },
            CommandItem {
                label: "vn run ubuntu22-open-docs",
                action: Action::Execute(vec!["run", "ubuntu22-open-docs"]),
            },
            CommandItem {
                label: "vn run ubuntu22-open-silverbullet",
                action: Action::Execute(vec!["run", "ubuntu22-open-silverbullet"]),
            },
            CommandItem {
                label: "vn run ubuntu22-open-stirling-pdf",
                action: Action::Execute(vec!["run", "ubuntu22-open-stirling-pdf"]),
            },
            CommandItem {
                label: "vn run ubuntu22-stop-stirling-pdf",
                action: Action::Execute(vec!["run", "ubuntu22-stop-stirling-pdf"]),
            },
            CommandItem {
                label: "vn run ubuntu22-open-library-portal",
                action: Action::Execute(vec!["run", "ubuntu22-open-library-portal"]),
            },
            CommandItem {
                label: "vn run ubuntu22-stop-library-portal",
                action: Action::Execute(vec!["run", "ubuntu22-stop-library-portal"]),
            },
            CommandItem {
                label: "vn run ubuntu22-open-doc-processor",
                action: Action::Execute(vec!["run", "ubuntu22-open-doc-processor"]),
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
                label: "Select Model",
                action: Action::OpenModelMenu,
            },
            CommandItem {
                label: "Download Model (type name)",
                action: Action::ArmInput(InputPurpose::DownloadModel),
            },
            CommandItem {
                label: "Chat (type message)",
                action: Action::ArmInput(InputPurpose::Chat),
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
                label: "vn run win11-open (apps + docker)",
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
        MenuKind::RunWin11Open => vec![
            CommandItem {
                label: "vn run win11-open-docker",
                action: Action::Execute(vec!["run", "win11-open-docker"]),
            },
            CommandItem {
                label: "vn run win11-check-docker",
                action: Action::Execute(vec!["run", "win11-check-docker"]),
            },
            CommandItem {
                label: "vn run win11-stop-all-containers",
                action: Action::Execute(vec!["run", "win11-stop-all-containers"]),
            },
            CommandItem {
                label: "vn run win11-remove-all-containers",
                action: Action::Execute(vec!["run", "win11-remove-all-containers"]),
            },
            CommandItem {
                label: "vn run win11-remove-all-images",
                action: Action::Execute(vec!["run", "win11-remove-all-images"]),
            },
            CommandItem {
                label: "vn run win11-open-docs",
                action: Action::Execute(vec!["run", "win11-open-docs"]),
            },
            CommandItem {
                label: "vn run win11-open-silverbullet",
                action: Action::Execute(vec!["run", "win11-open-silverbullet"]),
            },
            CommandItem {
                label: "vn run win11-open-stirling-pdf",
                action: Action::Execute(vec!["run", "win11-open-stirling-pdf"]),
            },
            CommandItem {
                label: "vn run win11-stop-stirling-pdf",
                action: Action::Execute(vec!["run", "win11-stop-stirling-pdf"]),
            },
            CommandItem {
                label: "vn run win11-open-library-portal",
                action: Action::Execute(vec!["run", "win11-open-library-portal"]),
            },
            CommandItem {
                label: "vn run win11-stop-library-portal",
                action: Action::Execute(vec!["run", "win11-stop-library-portal"]),
            },
            CommandItem {
                label: "vn run win11-open-doc-processor",
                action: Action::Execute(vec!["run", "win11-open-doc-processor"]),
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
                label: "Select Model",
                action: Action::OpenModelMenu,
            },
            CommandItem {
                label: "Download Model (type name)",
                action: Action::ArmInput(InputPurpose::DownloadModel),
            },
            CommandItem {
                label: "Chat (type message)",
                action: Action::ArmInput(InputPurpose::Chat),
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
        // Built dynamically from installed models in `set_menu`; never queried here.
        MenuKind::SelectModel => Vec::new(),
    }
}

/// A bordered panel. When `focused` it gets a bright accent border and a
/// highlighted (light-blue) title label; otherwise a dim border + muted title.
/// This shows which panel (Dashboard / Input) is active without flooding the
/// terminal's black background.
fn panel_block(title: &str, focused: bool) -> Block<'static> {
    let (border, title_style) = if focused {
        (
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            Style::default().fg(DIM),
            Style::default().fg(MUTED),
        )
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(Span::styled(format!(" {} ", title), title_style))
}

/// A plain (non-focusable) panel: dim border, muted title — for the static
/// panels (CLI header, Docker, CLI Output).
fn plain_block(title: String) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(format!(" {} ", title), Style::default().fg(MUTED)))
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
                    KeyCode::Esc => {
                        app.shutdown();
                        break;
                    }
                    KeyCode::Char('q') if matches!(app.focus, Focus::Dashboard) => {
                        app.shutdown();
                        break;
                    }
                    KeyCode::Tab => {
                        app.focus = match app.focus {
                            Focus::Dashboard => Focus::Input,
                            Focus::Input => Focus::Dashboard,
                        }
                    }
                    KeyCode::Down => {
                        if matches!(app.focus, Focus::Dashboard) {
                            app.next();
                        }
                    }
                    KeyCode::Up => {
                        if matches!(app.focus, Focus::Dashboard) {
                            app.previous();
                        }
                    }
                    KeyCode::Char('j') if matches!(app.focus, Focus::Dashboard) => {
                        app.next();
                    }
                    KeyCode::Char('k') if matches!(app.focus, Focus::Dashboard) => {
                        app.previous();
                    }
                    KeyCode::Char(',') if matches!(app.focus, Focus::Dashboard) => {
                        app.output_page_up();
                    }
                    KeyCode::Char('.') if matches!(app.focus, Focus::Dashboard) => {
                        app.output_page_down();
                    }
                    KeyCode::Char('r') | KeyCode::Char('R')
                        if matches!(app.focus, Focus::Dashboard) =>
                    {
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

            let active_model = app.selected_model.clone().unwrap_or_else(|| "none".to_string());
            let today = Local::now().format("%Y-%m-%d");
            let header = Paragraph::new(format!(
                "vecnode vn    |    AI model: {}    |    Date: {}",
                active_model, today
            ))
            .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            .block(plain_block("CLI".to_string()));

            let middle = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(areas[1]);

            let left_panels = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(middle[0]);

            // The model menu lists installed models (or a placeholder); every
            // other menu lists its CommandItem labels.
            let (labels, dashboard_title): (Vec<String>, &str) =
                if app.menu == MenuKind::SelectModel {
                    if app.model_items.is_empty() {
                        (
                            vec!["(no models found - use Download Model)".to_string()],
                            "Select Model",
                        )
                    } else {
                        (app.model_items.clone(), "Select Model")
                    }
                } else {
                    (
                        app.commands.iter().map(|cmd| cmd.label.to_string()).collect(),
                        "Dashboard",
                    )
                };

            let button_items: Vec<ListItem> = labels
                .iter()
                .enumerate()
                .map(|(idx, label)| {
                    if idx == app.selected {
                        ListItem::new(Line::from(vec![Span::styled(
                            format!("[ {} ]", label),
                            Style::default()
                                .fg(Color::Black)
                                .bg(ACCENT)
                                .add_modifier(Modifier::BOLD),
                        )]))
                    } else {
                        ListItem::new(Line::from(vec![Span::styled(
                            format!("[ {} ]", label),
                            Style::default().fg(Color::White),
                        )]))
                    }
                })
                .collect();

            let mut list_state = ListState::default();
            list_state.select(Some(app.selected));

            let dashboard = List::new(button_items)
                .block(panel_block(dashboard_title, matches!(app.focus, Focus::Dashboard)));

            let docker_status = if app.docker_panel.available { "ON" } else { "OFF" };
            let docker_header = Row::new(vec!["Port", "Image", "Container"])
                .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));
            let docker_rows: Vec<Row> = if app.docker_panel.available && app.docker_panel.rows.is_empty()
            {
                vec![Row::new(vec!["-", "(no running containers)", "-"])]
            } else if !app.docker_panel.available {
                vec![Row::new(vec!["-", "(docker not running)", "-"])]
            } else {
                app.docker_panel
                    .rows
                    .iter()
                    .map(|(p, i, c)| Row::new(vec![p.clone(), i.clone(), c.clone()]))
                    .collect()
            };
            let docker = Table::new(
                docker_rows,
                [
                    Constraint::Percentage(13),
                    Constraint::Percentage(59),
                    Constraint::Percentage(28),
                ],
            )
            .header(docker_header)
            .column_spacing(1)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DIM))
                    .title(Span::styled(
                        format!(" Docker: {} ", docker_status),
                        Style::default()
                            .fg(if app.docker_panel.available { ACCENT } else { DIM })
                            .add_modifier(Modifier::BOLD),
                    )),
            );

            let log_lines: Vec<Line> = app
                .logs
                .iter()
                .map(|entry| {
                    if let LogEntry::Command(command) = entry {
                        Line::from(Span::styled(
                            format!("> {}", command),
                            Style::default()
                                .fg(ACCENT)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else if let LogEntry::Error(text) = entry {
                        Line::from(Span::styled(text.clone(), Style::default().fg(Color::LightRed)))
                    } else if let LogEntry::Stderr(text) = entry {
                        Line::from(Span::styled(text.clone(), Style::default().fg(Color::Yellow)))
                    } else if let LogEntry::Stdout(text) = entry {
                        Line::from(Span::styled(text.clone(), Style::default().fg(Color::White)))
                    } else {
                        match entry {
                            LogEntry::Info(text) => {
                                Line::from(Span::styled(text.clone(), Style::default().fg(MUTED)))
                            }
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
                .block(plain_block("CLI Output".to_string()))
                .scroll((app.output_scroll, 0));

            let keys_text = match app.focus {
                Focus::Dashboard => {
                    "Tab: input  Up/Down: select  Enter: run  R: refresh ui  ,/.: output page  q/Esc: exit"
                }
                Focus::Input => {
                    "Type to enter text  Tab: dashboard  Enter: send input  Backspace: delete  Esc: exit"
                }
            };

            let input_text = if !app.input.is_empty() {
                app.input.clone()
            } else {
                match app.input_purpose {
                    InputPurpose::DownloadModel => {
                        "Type a model name to download, then Enter...".to_string()
                    }
                    InputPurpose::Chat => format!(
                        "Message {} , then Enter...",
                        app.selected_model.clone().unwrap_or_else(|| "none".to_string())
                    ),
                    InputPurpose::None => "Type input for running command...".to_string(),
                }
            };

            let input_focused = matches!(app.focus, Focus::Input);
            let input_style = if input_focused {
                Style::default().fg(Color::Black).bg(ACCENT)
            } else {
                Style::default().fg(DIM)
            };

            let footer = Paragraph::new(vec![
                Line::from(Span::styled(input_text, input_style)),
                Line::from(Span::styled(keys_text, Style::default().fg(MUTED))),
            ])
            .block(panel_block("Input", input_focused));

            frame.render_widget(header, areas[0]);
            frame.render_stateful_widget(dashboard, left_panels[0], &mut list_state);
            frame.render_widget(docker, left_panels[1]);
            frame.render_widget(output, middle[1]);
            frame.render_widget(footer, areas[2]);
        })?;
    }

    Ok(())
}
