use anyhow::Result;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
pub fn run(repo_root: Option<PathBuf>) -> Result<()> {
    windows::run(repo_root)
}

#[cfg(not(target_os = "windows"))]
pub fn run(_repo_root: Option<PathBuf>) -> Result<()> {
    anyhow::bail!("tray mode is currently supported only on Windows")
}

#[cfg(target_os = "windows")]
mod windows {
    use anyhow::{Context, Result};
    use std::ffi::OsStr;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Write};
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use sysinfo::{Pid, ProcessesToUpdate, System};
    use tray_item::TrayItem;
    use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleWindow};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        LoadIconW, MessageBoxW, ShowWindow, IDI_SHIELD, MB_ICONERROR, MB_OK, SW_HIDE,
        SW_SHOWNORMAL,
    };

    struct InstanceGuard {
        _file: File,
        path: PathBuf,
    }

    impl Drop for InstanceGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    pub fn run(repo_root: Option<PathBuf>) -> Result<()> {
        let _instance = match acquire_single_instance("vecnode.vn.tray")? {
            Some(guard) => guard,
            None => return Ok(()),
        };

        hide_console_window();

        let repo_root = resolve_repo_root(repo_root)?;
        let vn_bin = std::env::current_exe().context("failed to resolve current executable")?;
        let quit = Arc::new(AtomicBool::new(false));

        let raw_icon = choose_windows_tray_icon()?;
        let mut tray = TrayItem::new("vecnode", tray_item::IconSource::RawIcon(raw_icon))
            .context("failed to initialize tray icon")?;

        // Some Windows setups only render after an explicit set_icon call.
        tray.set_icon(tray_item::IconSource::RawIcon(raw_icon))
            .context("failed to set tray icon")?;

        {
            let vn_bin = vn_bin.clone();
            let repo_root = repo_root.clone();
            tray.add_menu_item("Open Admin TUI Terminal", move || {
                let _ = open_admin_tui_terminal(&vn_bin, &repo_root);
            })
            .context("failed to add tray menu item: Open Admin TUI Terminal")?;
        }

        {
            let quit = quit.clone();
            tray.add_menu_item("Quit", move || {
                quit.store(true, Ordering::SeqCst);
            })
            .context("failed to add tray menu item: Quit")?;
        }

        while !quit.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(200));
        }

        Ok(())
    }

    fn hide_console_window() {
        unsafe {
            let hwnd = GetConsoleWindow();
            if hwnd != 0 {
                ShowWindow(hwnd, SW_HIDE);
                let _ = FreeConsole();
            }
        }
    }

    fn choose_windows_tray_icon() -> Result<isize> {
        let shield_icon = unsafe { LoadIconW(0, IDI_SHIELD) };
        if shield_icon != 0 {
            return Ok(shield_icon);
        }

        anyhow::bail!("failed to load IDI_SHIELD icon handle")
    }

    fn acquire_single_instance(name: &str) -> Result<Option<InstanceGuard>> {
        let lock_path = std::env::temp_dir().join(format!("{}.lock", name));
        let mut retries = 0;

        loop {
            let open_result = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path);

            match open_result {
                Ok(mut file) => {
                    let pid = std::process::id();
                    file.write_all(pid.to_string().as_bytes())
                        .context("failed to write tray lock file")?;

                    return Ok(Some(InstanceGuard {
                        _file: file,
                        path: lock_path,
                    }));
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    if let Some(lock_pid) = read_lock_pid(&lock_path) {
                        if process_is_running(lock_pid) {
                            return Ok(None);
                        }
                    }

                    // Stale lock: remove and retry a limited number of times.
                    let _ = std::fs::remove_file(&lock_path);
                    retries += 1;
                    if retries >= 2 {
                        return Ok(None);
                    }
                }
                Err(err) => return Err(err).context("failed to create tray lock file"),
            }
        }
    }

    fn read_lock_pid(lock_path: &Path) -> Option<u32> {
        let mut content = String::new();
        let mut file = File::open(lock_path).ok()?;
        file.read_to_string(&mut content).ok()?;
        content.trim().parse::<u32>().ok()
    }

    fn process_is_running(pid: u32) -> bool {
        let mut system = System::new_all();
        system.refresh_processes(ProcessesToUpdate::All);
        system.process(Pid::from_u32(pid)).is_some()
    }

    fn open_admin_tui_terminal(vn_bin: &Path, repo_root: &Path) -> Result<()> {
        let cwd = repo_root
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid repo root path"))?;
        let cwd = cwd.trim_end_matches(['\\', '/']);
        let vn = vn_bin
            .canonicalize()
            .with_context(|| format!("failed to resolve vn path: {}", vn_bin.display()))?;
        let vn = vn
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid vn executable path"))?;

        let temp_cmd = std::env::temp_dir().join("vecnode-open-admin-tui.cmd");
        let temp_log = std::env::temp_dir().join("vecnode-open-admin-tui.log");
        let mut script = File::create(&temp_cmd)
            .with_context(|| format!("failed to create temp launcher: {}", temp_cmd.display()))?;

        writeln!(script, "@echo off")?;
        writeln!(script, "setlocal EnableExtensions")?;
        writeln!(script, "set \"VECNODE_LOG={}\"", temp_log.display())?;
        writeln!(script, "echo [vecnode] admin launcher starting > \"%VECNODE_LOG%\"")?;
        writeln!(script, "set \"VECNODE_REPO_ROOT={}\"", cwd)?;
        writeln!(script, "echo [vecnode] repo_root=%VECNODE_REPO_ROOT% >> \"%VECNODE_LOG%\"")?;
        writeln!(script, "echo [vecnode] vn={} >> \"%VECNODE_LOG%\"", vn)?;
        writeln!(script, "if not exist \"%VECNODE_REPO_ROOT%\" (")?;
        writeln!(script, "  echo [vecnode] ERROR: repo_root does not exist >> \"%VECNODE_LOG%\"")?;
        writeln!(script, "  pause")?;
        writeln!(script, "  exit /b 1")?;
        writeln!(script, ")")?;
        writeln!(script, "if not exist \"{}\" (", vn)?;
        writeln!(script, "  echo [vecnode] ERROR: vn.exe does not exist >> \"%VECNODE_LOG%\"")?;
        writeln!(script, "  pause")?;
        writeln!(script, "  exit /b 1")?;
        writeln!(script, ")")?;
        writeln!(script, "pushd \"{}\"", cwd)?;
        writeln!(script, "echo [vecnode] launching TUI... >> \"%VECNODE_LOG%\"")?;
        writeln!(script, "call \"{}\" >> \"%VECNODE_LOG%\" 2>&1", vn)?;
        writeln!(script, "echo [vecnode] vecnode exited with code %ERRORLEVEL% >> \"%VECNODE_LOG%\"")?;
        writeln!(script, "type \"%VECNODE_LOG%\"")?;
        writeln!(script, "pause")?;
        writeln!(script, "popd")?;
        script.flush()?;
        drop(script);

        let operation_w = to_wide("runas");
        let file_w = to_wide(
            temp_cmd
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("invalid temp launcher path"))?,
        );
        let params_w = to_wide("");
        let cwd_w = to_wide(cwd);

        let result = unsafe {
            ShellExecuteW(0, operation_w.as_ptr(), file_w.as_ptr(), params_w.as_ptr(), cwd_w.as_ptr(), SW_SHOWNORMAL)
        };

        if result <= 32 {
            let title = to_wide("vecnode");
            let message = to_wide("Failed to open the administrator TUI terminal.");
            unsafe {
                MessageBoxW(0, message.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
            }
            anyhow::bail!(
                "failed to open elevated TUI terminal (ShellExecuteW code: {})",
                result
            );
        }

        Ok(())
    }
    fn resolve_repo_root(repo_root: Option<PathBuf>) -> Result<PathBuf> {
        if let Some(path) = repo_root {
            return Ok(path);
        }

        let exe = std::env::current_exe().context("failed to resolve current executable")?;
        let parent = exe
            .parent()
            .ok_or_else(|| anyhow::anyhow!("missing executable parent"))?;

        // vn.exe lives under cli/target/<target>/<profile>/, so repo root is 4 levels up.
        let mut root = parent.to_path_buf();
        for _ in 0..4 {
            root = root
                .parent()
                .ok_or_else(|| anyhow::anyhow!("failed to resolve repository root"))?
                .to_path_buf();
        }

        Ok(root)
    }

    fn to_wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
}
