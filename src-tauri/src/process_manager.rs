//! Owns one sing-box child. Privilege escalation and application integration are separate stages.
use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub stream: Stream,
    pub text: String,
}
#[derive(Debug)]
pub enum Status {
    Stopped,
    Running { pid: u32 },
    Exited(ExitStatus),
}

pub struct ProcessManager {
    child: Option<Child>,
    config_path: Option<PathBuf>,
    config_dir: Option<PathBuf>,
    logs_tx: Sender<LogLine>,
    logs_rx: Receiver<LogLine>,
    last_exit: Option<ExitStatus>,
    history: Vec<LogLine>,
    elevated: bool,
    cleanup_unconfirmed: bool,
    #[cfg(windows)]
    job: Option<WindowsJob>,
    #[cfg(windows)]
    adapter_name: Option<String>,
}
impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
impl ProcessManager {
    pub fn new() -> Self {
        let (logs_tx, logs_rx) = mpsc::channel();
        Self {
            child: None,
            config_path: None,
            config_dir: None,
            logs_tx,
            logs_rx,
            last_exit: None,
            history: Vec::new(),
            elevated: false,
            cleanup_unconfirmed: false,
            #[cfg(windows)]
            job: None,
            #[cfg(windows)]
            adapter_name: None,
        }
    }
    #[cfg(windows)]
    pub fn with_config_dir(dir: PathBuf) -> Self {
        let mut manager = Self::new();
        manager.config_dir = Some(dir);
        manager
    }
    /// Starts the supplied engine directly. Success means spawned, not that the TUN is ready.
    pub fn start(&mut self, binary: &Path, config: &str) -> io::Result<u32> {
        self.start_inner(binary, config, false)
    }
    #[cfg(target_os = "linux")]
    pub fn start_elevated(&mut self, binary: &Path, config: &str) -> io::Result<u32> {
        self.start_inner(binary, config, true)
    }
    fn start_inner(&mut self, binary: &Path, config: &str, elevated: bool) -> io::Result<u32> {
        if self.cleanup_unconfirmed {
            return Err(io::Error::other("Previous cleanup was not confirmed"));
        }
        if matches!(self.status()?, Status::Running { .. }) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "sing-box is already running",
            ));
        }
        self.last_exit = None;
        self.history.clear();
        while self.logs_rx.try_recv().is_ok() {}
        let binary = fs::canonicalize(binary)?;
        let supervisor = if elevated {
            Some(std::env::current_exe()?)
        } else {
            None
        };
        #[cfg(windows)]
        let adapter_name = serde_json::from_str::<serde_json::Value>(config)
            .ok()
            .and_then(|v| {
                v.get("inbounds")?
                    .as_array()?
                    .iter()
                    .find(|i| i.get("type").and_then(|v| v.as_str()) == Some("tun"))?
                    .get("interface_name")?
                    .as_str()
                    .map(str::to_owned)
            });
        let path = write_config(config, self.config_dir.as_deref())?;
        self.elevated = elevated;
        let mut command = if elevated {
            let mut c = Command::new("pkexec");
            c.arg(supervisor.as_ref().expect("supervisor path"))
                .arg("--privileged-supervisor")
                .arg(&binary)
                .arg(&path);
            c
        } else {
            let mut c = Command::new(binary);
            c.arg("run").arg("-c").arg(&path);
            c
        };
        command
            .stdin(if elevated {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;
            if let Err(error) = ensure_windows_console() {
                let _ = self.logs_tx.send(LogLine {
                    stream: Stream::Stderr,
                    text: format!(
                        "WARN: Could not prepare a console for graceful shutdown: {error}"
                    ),
                });
            }
            command.creation_flags(CREATE_NEW_PROCESS_GROUP);
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::process::CommandExt;
            let parent = std::process::id();
            unsafe {
                command.pre_exec(move || {
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                        return Err(io::Error::last_os_error());
                    }
                    if libc::getppid() as u32 != parent {
                        return Err(io::Error::other("parent exited during startup"));
                    }
                    Ok(())
                });
            }
        }
        let result = command.spawn();
        let mut child = match result {
            Ok(c) => c,
            Err(e) => {
                let _ = fs::remove_file(path);
                return Err(e);
            }
        };
        // Windows Job Object support has not yet been tested on Windows.
        #[cfg(windows)]
        {
            match WindowsJob::attach(&child) {
                Ok(job) => self.job = Some(job),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(&path);
                    return Err(error);
                }
            }
        }
        if let Some(stdout) = child.stdout.take() {
            read_lines(stdout, Stream::Stdout, self.logs_tx.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            read_lines(stderr, Stream::Stderr, self.logs_tx.clone());
        }
        let pid = child.id();
        self.child = Some(child);
        self.config_path = Some(path);
        #[cfg(windows)]
        {
            self.adapter_name = adapter_name;
        }
        Ok(pid)
    }
    pub fn status(&mut self) -> io::Result<Status> {
        if let Some(child) = self.child.as_mut() {
            if let Some(exit) = child.try_wait()? {
                self.child = None;
                #[cfg(windows)]
                {
                    self.job = None;
                }
                self.cleanup_config();
                #[cfg(windows)]
                if let Some(warning) = windows_adapter_warning(self.adapter_name.take().as_deref())
                {
                    let _ = self.logs_tx.send(LogLine {
                        stream: Stream::Stderr,
                        text: warning,
                    });
                }
                self.last_exit = Some(exit);
            } else {
                return Ok(Status::Running { pid: child.id() });
            }
        }
        Ok(match self.last_exit {
            Some(exit) => Status::Exited(exit),
            None => Status::Stopped,
        })
    }
    pub fn drain_logs(&mut self) -> Vec<LogLine> {
        let lines: Vec<_> = self.logs_rx.try_iter().collect();
        self.history.extend(lines.iter().cloned());
        if self.history.len() > 100 {
            self.history.drain(..self.history.len() - 100);
        }
        lines
    }
    pub fn mark_cleanup_unconfirmed(&mut self) {
        self.cleanup_unconfirmed = true;
    }
    pub fn recent_logs(&self) -> Vec<LogLine> {
        self.history.clone()
    }
    /// Wait for sing-box's startup message. This confirms engine startup, not proxy connectivity.
    pub fn wait_ready(&mut self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        loop {
            let lines = self.drain_logs();
            let ready = lines
                .iter()
                .any(|line| line.text.contains("sing-box started ("));
            match self.status() {
                Ok(Status::Exited(status)) => {
                    let message = if self.elevated && status.code() == Some(126) {
                        "Elevation cancelled by user".to_owned()
                    } else if self.elevated && status.code() == Some(127) {
                        "Elevation denied or authentication failed".to_owned()
                    } else {
                        format!("sing-box exited before readiness: {status}")
                    };
                    return Err(self.failure(&message));
                }
                Err(e) => return Err(self.failure(&format!("cannot check sing-box: {e}"))),
                _ => {}
            }
            if ready {
                return Ok(());
            }
            if Instant::now() >= deadline {
                if let Err(error) = self.stop(Duration::from_secs(6)) {
                    return Err(self.failure(&format!(
                        "readiness timed out; stopping sing-box failed: {error}"
                    )));
                }
                return Err(self.failure("sing-box readiness timed out"));
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
    fn failure(&mut self, message: &str) -> String {
        self.drain_logs();
        let tail = self
            .history
            .iter()
            .rev()
            .take(20)
            .rev()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if tail.is_empty() {
            message.to_owned()
        } else {
            format!("{message}\n{tail}")
        }
    }
    /// Terminate gracefully on Unix, then force termination after the timeout.
    pub fn stop(&mut self, timeout: Duration) -> io::Result<()> {
        let mut stopping_elevated = false;
        #[cfg(windows)]
        let mut stopped_child = false;
        if let Some(child) = self.child.as_mut() {
            if child.try_wait()?.is_none() {
                if self.elevated {
                    stopping_elevated = true;
                    drop(child.stdin.take());
                } else {
                    #[cfg(windows)]
                    if let Err(error) = request_termination(child) {
                        let _ = self.logs_tx.send(LogLine { stream: Stream::Stderr, text: format!("WARN: Could not send CTRL_BREAK to sing-box: {error}; will force termination if it remains running") });
                    }
                    #[cfg(not(windows))]
                    request_termination(child)?;
                }
                #[cfg(windows)]
                let timeout = timeout.min(Duration::from_secs(5));
                let deadline = Instant::now() + timeout;
                loop {
                    if let Some(status) = child.try_wait()? {
                        self.last_exit = Some(status);
                        break;
                    }
                    if Instant::now() >= deadline {
                        if self.elevated {
                            return Err(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "Elevated supervisor did not exit; cleanup not confirmed",
                            ));
                        }
                        #[cfg(windows)]
                        {
                            let _ = self.logs_tx.send(LogLine { stream: Stream::Stderr, text: "WARN: sing-box did not stop gracefully within five seconds; forcing termination. Route and DNS cleanup may require inspection.".into() });
                        }
                        child.kill()?;
                        self.last_exit = Some(child.wait()?);
                        break;
                    }
                    thread::sleep(Duration::from_millis(20));
                }
            } else {
                self.last_exit = child.try_wait()?;
            }
            self.child = None;
            #[cfg(windows)]
            {
                self.job = None;
                stopped_child = true;
            }
        }
        self.cleanup_config();
        #[cfg(windows)]
        if let Some(warning) = if stopped_child {
            windows_adapter_warning(self.adapter_name.take().as_deref())
        } else {
            None
        } {
            let _ = self.logs_tx.send(LogLine {
                stream: Stream::Stderr,
                text: warning,
            });
        }
        if stopping_elevated && self.last_exit.is_some_and(|status| !status.success()) {
            self.cleanup_unconfirmed = true;
        }
        if self.cleanup_unconfirmed {
            return Err(io::Error::other(
                "Elevated supervisor exited unsuccessfully; route and DNS cleanup not confirmed",
            ));
        }
        Ok(())
    }
    fn cleanup_config(&mut self) {
        if let Some(path) = self.config_path.take() {
            let _ = fs::remove_file(path);
        }
    }
}
impl Drop for ProcessManager {
    fn drop(&mut self) {
        let _ = self.stop(Duration::from_secs(6));
    }
}
fn read_lines<R: io::Read + Send + 'static>(reader: R, stream: Stream, tx: Sender<LogLine>) {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            match line {
                Ok(text) => {
                    if tx
                        .send(LogLine {
                            stream: stream.clone(),
                            text,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
}
fn write_config(config: &str, directory: Option<&Path>) -> io::Result<PathBuf> {
    let directory = directory
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&directory)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for attempt in 0..100 {
        let path = directory.join(format!(
            "split-tunnel-{}-{nonce}-{attempt}.json",
            std::process::id()
        ));
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        match opts.open(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(config.as_bytes()) {
                    let _ = fs::remove_file(&path);
                    return Err(e);
                }
                return Ok(path);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create temporary configuration",
    ))
}
#[cfg(unix)]
fn request_termination(child: &mut Child) -> io::Result<()> {
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    if unsafe { kill(child.id() as i32, 15) } == 0 {
        Ok(())
    } else {
        let error = io::Error::last_os_error();
        if child.try_wait()?.is_some() {
            Ok(())
        } else {
            Err(error)
        }
    }
}
#[cfg(windows)]
fn ensure_windows_console() -> io::Result<()> {
    use windows_sys::Win32::{
        System::Console::{AllocConsole, GetConsoleCP, GetConsoleWindow},
        UI::WindowsAndMessaging::{ShowWindow, SW_HIDE},
    };
    // Do not hide or detach a console inherited from a terminal. A GUI launch
    // has none, so create our own; the engine's new group shares this console.
    unsafe {
        if GetConsoleCP() != 0 {
            return Ok(());
        }
        if AllocConsole() == 0 {
            return Err(io::Error::last_os_error());
        }
        let window = GetConsoleWindow();
        if !window.is_null() {
            ShowWindow(window, SW_HIDE);
        }
    }
    Ok(())
}
#[cfg(windows)]
fn request_termination(child: &mut Child) -> io::Result<()> {
    use windows_sys::Win32::System::Console::{GenerateConsoleCtrlEvent, CTRL_BREAK_EVENT};
    if unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child.id()) } != 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if child.try_wait()?.is_some() {
        Ok(())
    } else {
        Err(error)
    }
}
#[cfg(not(any(unix, windows)))]
fn request_termination(child: &mut Child) -> io::Result<()> {
    child.kill()
}

// The app sets a distinctive TUN name in its Windows config. Never remove a
// network adapter automatically; it may belong to another installation.
#[cfg(windows)]
fn windows_adapter_warning(name: Option<&str>) -> Option<String> {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;
    let Some(name) = name else {
        return Some("WARN: The TUN adapter name is unknown, so cleanup could not be verified. Check Get-NetAdapter -IncludeHidden, ipconfig /all, and route print.".into());
    };
    let script = "$ErrorActionPreference='Stop'; $a=Get-NetAdapter -IncludeHidden | Where-Object { $_.Name -eq $env:SPLIT_TUNNEL_ADAPTER }; if ($null -ne $a) { Write-Output 'present' }";
    let result = (|| -> io::Result<(ExitStatus, String)> {
        let mut child = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env("SPLIT_TUNNEL_ADAPTER", name)
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child.try_wait()? {
                let mut text = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    use std::io::Read;
                    stdout.read_to_string(&mut text)?;
                }
                return Ok((status, text));
            }
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "adapter check timed out",
                ));
            }
            thread::sleep(Duration::from_millis(30));
        }
    })();
    match result {
        Ok((status, output)) if status.success() && output.lines().any(|line| line.trim() == "present") => Some(format!("WARN: The {name} adapter still exists. Inspect it with Get-NetAdapter -IncludeHidden. If it belongs to this app and is stale, remove it in Device Manager after checking routes and DNS.")),
        Ok((status, _)) if status.success() => None,
        _ => Some(format!("WARN: Could not verify whether the {name} adapter was removed. Check Get-NetAdapter -IncludeHidden, ipconfig /all, and route print.")),
    }
}

// Untested on Windows: closing this handle terminates any remaining job members.
#[cfg(windows)]
struct WindowsJob(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
unsafe impl Send for WindowsJob {}
#[cfg(windows)]
impl WindowsJob {
    fn attach(child: &Child) -> io::Result<Self> {
        use std::{
            mem::{size_of, zeroed},
            os::windows::io::AsRawHandle,
        };
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let job = Self(handle);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle() as _) == 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(job)
        }
    }
}
#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    fn fixture(body: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "split-fixture-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    #[test]
    fn starts_collects_logs_rejects_duplicate_and_stops() {
        let binary = fixture("echo started; echo error >&2; exec sleep 30");
        let mut manager = ProcessManager::new();
        manager.start(&binary, "{}").unwrap();
        let config = manager.config_path.clone().unwrap();
        assert_eq!(
            fs::metadata(&config).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(
            matches!(manager.start(&binary, "{}"), Err(e) if e.kind() == io::ErrorKind::AlreadyExists)
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut logs = Vec::new();
        while logs.len() < 2 && Instant::now() < deadline {
            logs.extend(manager.drain_logs());
            thread::sleep(Duration::from_millis(10));
        }
        assert!(logs
            .iter()
            .any(|l| l.text == "started" && l.stream == Stream::Stdout));
        assert!(logs
            .iter()
            .any(|l| l.text == "error" && l.stream == Stream::Stderr));
        manager.stop(Duration::from_secs(1)).unwrap();
        assert!(!config.exists());
        assert!(matches!(manager.status().unwrap(), Status::Exited(_)));
        fs::remove_file(binary).unwrap();
    }
    #[test]
    fn detects_unexpected_exit_and_cleans_config() {
        let binary = fixture("exit 42");
        let mut manager = ProcessManager::new();
        manager.start(&binary, "{}").unwrap();
        let config = manager.config_path.clone().unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Status::Exited(status) = manager.status().unwrap() {
                assert_eq!(status.code(), Some(42));
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!config.exists());
        fs::remove_file(binary).unwrap();
    }
    #[test]
    fn reports_spawn_errors() {
        let mut m = ProcessManager::new();
        assert!(m.start(Path::new("/nonexistent/sing-box"), "{}").is_err());
        assert!(m.config_path.is_none());
    }
    #[test]
    fn readiness_reports_early_exit_with_logs() {
        let binary = fixture("echo startup-failed >&2; sleep 0.05; exit 42");
        let mut manager = ProcessManager::new();
        manager.start(&binary, "{}").unwrap();
        let error = manager.wait_ready(Duration::from_secs(2)).unwrap_err();
        assert!(error.contains("exited before readiness"));
        assert!(error.contains("startup-failed"));
        fs::remove_file(binary).unwrap();
    }
    #[test]
    fn readiness_timeout_stops_process_and_reports_logs() {
        let binary = fixture("echo waiting >&2; exec sleep 30");
        let mut manager = ProcessManager::new();
        manager.start(&binary, "{}").unwrap();
        let error = manager.wait_ready(Duration::from_millis(150)).unwrap_err();
        assert!(error.contains("timed out"));
        assert!(error.contains("waiting"));
        assert!(!matches!(manager.status().unwrap(), Status::Running { .. }));
        fs::remove_file(binary).unwrap();
    }
    #[test]
    fn readiness_accepts_startup_log() {
        let binary = fixture("echo 'INFO sing-box started (0.01s)'; exec sleep 30");
        let mut manager = ProcessManager::new();
        manager.start(&binary, "{}").unwrap();
        manager.wait_ready(Duration::from_secs(2)).unwrap();
        manager.stop(Duration::from_secs(1)).unwrap();
        fs::remove_file(binary).unwrap();
    }
    #[test]
    fn reports_polkit_cancellation_and_denial() {
        for (code, expected) in [(126, "cancelled"), (127, "denied")] {
            let binary = fixture(&format!("exit {code}"));
            let mut manager = ProcessManager::new();
            manager.start(&binary, "{}").unwrap();
            manager.elevated = true;
            let error = manager.wait_ready(Duration::from_secs(2)).unwrap_err();
            assert!(error.contains(expected), "{error}");
            fs::remove_file(binary).unwrap();
        }
    }
}
