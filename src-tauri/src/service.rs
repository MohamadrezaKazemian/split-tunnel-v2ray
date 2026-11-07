//! Dedicated owner thread keeps Linux PR_SET_PDEATHSIG's parent thread alive
//! from child spawn through disconnect (Linux ties the signal to the spawning thread).
use crate::process_manager::{LogLine, ProcessManager, Status};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

pub enum Request {
    Connect {
        binary: PathBuf,
        config: String,
        reply: Sender<Result<(), String>>,
    },
    Disconnect {
        reply: Sender<Result<(), String>>,
    },
    Status {
        reply: Sender<String>,
    },
    Shutdown,
}
#[derive(Clone)]
pub struct Service {
    tx: Sender<Request>,
}
impl Service {
    pub fn new(pidfile: PathBuf, report: impl Fn(String, Vec<LogLine>) + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            #[cfg(windows)]
            let mut manager = ProcessManager::with_config_dir(
                pidfile
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf(),
            );
            #[cfg(not(windows))]
            let mut manager = ProcessManager::new();
            let mut state = String::from("disconnected");
            let mut owns_pidfile = false;
            #[cfg(any(target_os = "linux", windows))]
            let lock = acquire_lock(&pidfile);
            #[cfg(any(target_os = "linux", windows))]
            if let Err(error) = &lock {
                state = format!("failed: {error}");
                report(state.clone(), vec![]);
            }
            #[cfg(any(target_os = "linux", windows))]
            if lock.is_ok() {
                if let Ok(data) = fs::read_to_string(&pidfile) {
                    if let Some(path) = data.lines().nth(1) {
                        if let Err(error) = recover_stale(&pidfile, Path::new(path)) {
                            state = format!("failed: stale process recovery: {error}");
                            report(state.clone(), vec![]);
                        }
                    }
                }
            }
            loop {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Request::Connect {
                        binary,
                        config,
                        reply,
                    }) => {
                        #[cfg(any(target_os = "linux", windows))]
                        if let Err(error) = &lock {
                            let _ = reply.send(Err(format!("{error}")));
                            continue;
                        }
                        if state == "connected" || state == "connecting" {
                            let _ = reply.send(Err("Already connected".into()));
                            continue;
                        }
                        state = "connecting".into();
                        report(state.clone(), vec![]);
                        let mut launched = false;
                        let result = (|| -> Result<(), String> {
                            recover_stale(&pidfile, &binary).map_err(|e| e.to_string())?;
                            #[cfg(target_os = "linux")]
                            let pid = manager
                                .start_elevated(&binary, &config)
                                .map_err(|e| format!("Could not start elevation: {e}"))?;
                            #[cfg(not(target_os = "linux"))]
                            let pid = manager.start(&binary, &config).map_err(|e| e.to_string())?;
                            launched = true;
                            if let Err(e) = save_pid(&pidfile, pid, &binary) {
                                let _ = manager.stop(Duration::from_secs(6));
                                return Err(e.to_string());
                            }
                            owns_pidfile = true;
                            manager.wait_ready(Duration::from_secs(120))?;
                            Ok(())
                        })();
                        match &result {
                            Ok(()) => state = "connected".into(),
                            Err(e) => {
                                state = format!("failed: {e}");
                                let stopped = manager.stop(Duration::from_secs(6)).is_ok();
                                if stopped && launched && owns_pidfile {
                                    let _ = fs::remove_file(&pidfile);
                                    owns_pidfile = false;
                                } else if !stopped && owns_pidfile {
                                    mark_unconfirmed(&pidfile);
                                }
                            }
                        }
                        let mut logs = manager.recent_logs();
                        logs.extend(manager.drain_logs());
                        report(state.clone(), logs);
                        let _ = reply.send(result);
                    }
                    Ok(Request::Disconnect { reply }) => {
                        let result = manager
                            .stop(Duration::from_secs(6))
                            .map_err(|e| e.to_string());
                        if result.is_ok() {
                            if owns_pidfile {
                                let _ = fs::remove_file(&pidfile);
                                owns_pidfile = false;
                            }
                            state = "disconnected".into();
                        } else if let Err(e) = &result {
                            if owns_pidfile {
                                mark_unconfirmed(&pidfile);
                            }
                            state = format!("failed: {e}");
                        }
                        report(state.clone(), manager.drain_logs());
                        let _ = reply.send(result);
                    }
                    Ok(Request::Status { reply }) => {
                        let _ = reply.send(state.clone());
                    }
                    Ok(Request::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                        if owns_pidfile {
                            if manager.stop(Duration::from_secs(6)).is_ok() {
                                let _ = fs::remove_file(&pidfile);
                            } else {
                                mark_unconfirmed(&pidfile);
                            }
                        }
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let mut state_changed = false;
                        if state == "connected" {
                            match manager.status() {
                                Ok(Status::Exited(exit)) => {
                                    state = format!("failed: sing-box exited: {exit}");
                                    state_changed = true;
                                    if owns_pidfile {
                                        if exit.success() {
                                            let _ = fs::remove_file(&pidfile);
                                            owns_pidfile = false;
                                        } else {
                                            manager.mark_cleanup_unconfirmed();
                                            mark_unconfirmed(&pidfile);
                                        }
                                    }
                                }
                                Err(e) => {
                                    state = format!("failed: {e}");
                                    state_changed = true;
                                }
                                _ => {}
                            }
                        }
                        let logs = manager.drain_logs();
                        if !logs.is_empty() || state_changed {
                            report(state.clone(), logs);
                        }
                    }
                }
            }
        });
        Self { tx }
    }
    pub fn request(&self, request: Request) -> Result<(), String> {
        self.tx
            .send(request)
            .map_err(|_| "process service stopped".into())
    }
}
#[cfg(target_os = "linux")]
fn acquire_lock(pidfile: &Path) -> io::Result<std::fs::File> {
    use std::os::fd::AsRawFd;
    if let Some(dir) = pidfile.parent() {
        fs::create_dir_all(dir)?;
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(pidfile.with_extension("lock"))?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "another app instance owns the tunnel",
        ));
    }
    Ok(file)
}
#[cfg(windows)]
struct InstanceLock(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Drop for InstanceLock {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
#[cfg(windows)]
fn acquire_lock(pidfile: &Path) -> io::Result<InstanceLock> {
    use std::{
        hash::{Hash, Hasher},
        os::windows::ffi::OsStrExt,
    };
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_ALREADY_EXISTS},
        System::Threading::CreateMutexW,
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pidfile.to_string_lossy().to_lowercase().hash(&mut hasher);
    let name: Vec<u16> = std::ffi::OsStr::new(&format!("Local\\SplitTunnel-{:x}", hasher.finish()))
        .encode_wide()
        .chain(Some(0))
        .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    let lock = InstanceLock(handle);
    if already {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "another app instance owns the tunnel",
        ));
    }
    Ok(lock)
}
fn mark_unconfirmed(file: &Path) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(file) {
        let _ = writeln!(f, "cleanup-unconfirmed");
    }
}
fn save_pid(file: &Path, pid: u32, binary: &Path) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let exe = fs::canonicalize(binary)?;
    // Paths containing newlines cannot be represented safely in this pidfile format.
    let exe = exe
        .to_str()
        .filter(|p| !p.contains('\n'))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid executable path"))?;
    #[cfg(target_os = "linux")]
    {
        let _ = exe;
        fs::write(
            file,
            format!(
                "{pid}\n{}\nelevated-supervisor\n",
                std::env::current_exe()?.display()
            ),
        )
    }
    #[cfg(windows)]
    {
        return fs::write(
            file,
            format!("{pid}\n{exe}\n{}\n", windows_process_start(pid)?),
        );
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    fs::write(file, format!("{pid}\n{exe}\n"))
}
#[cfg(target_os = "linux")]
fn recover_stale(file: &Path, binary: &Path) -> io::Result<()> {
    let data = match fs::read_to_string(file) {
        Ok(v) => v,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if data.lines().any(|line| line == "cleanup-unconfirmed") {
        return Err(io::Error::other("Previous route and DNS cleanup could not be confirmed; investigate before removing the pidfile"));
    }
    if data.lines().nth(2) == Some("elevated-supervisor") {
        let pid: i32 = data
            .lines()
            .next()
            .and_then(|v| v.parse().ok())
            .filter(|p| *p > 1)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid supervisor pidfile")
            })?;
        // The supervisor owns the root child and observes EOF when the GUI dies.
        // Never signal an unverifiable root-owned PID from the unprivileged GUI.
        for _ in 0..60 {
            let result = unsafe { libc::kill(pid, 0) };
            if result != 0 && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                fs::remove_file(file)?;
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        return Err(io::Error::other(
            "Previous elevated supervisor is still running; cleanup cannot be confirmed",
        ));
    }
    let mut parts = data.lines();
    let pid: u32 = parts
        .next()
        .and_then(|v| v.parse().ok())
        .filter(|p| *p > 1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid pidfile"))?;
    let recorded = PathBuf::from(
        parts
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid pidfile"))?,
    );
    let expected = fs::canonicalize(binary)?;
    if recorded != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "pidfile executable does not match configured binary",
        ));
    }
    // A pidfd prevents signalling a reused PID between identity verification and kill.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0) as i32 };
    if fd < 0 {
        let e = io::Error::last_os_error();
        if e.raw_os_error() == Some(libc::ESRCH) {
            fs::remove_file(file)?;
            return Ok(());
        }
        return Err(e);
    }
    let actual = fs::read_link(format!("/proc/{pid}/exe"));
    let result = match actual {
        Ok(path) if path == expected => {
            let rc = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd,
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                )
            };
            if rc == 0 {
                Ok(())
            } else {
                let e = io::Error::last_os_error();
                if e.raw_os_error() == Some(libc::ESRCH) {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "stale PID executable mismatch; process was not killed",
        )),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    };
    unsafe {
        libc::close(fd);
    }
    result?;
    fs::remove_file(file)?;
    Ok(())
}
#[cfg(windows)]
fn windows_process_start_handle(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<u64> {
    use windows_sys::Win32::{Foundation::FILETIME, System::Threading::GetProcessTimes};
    let mut created = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = created;
    let mut kernel = created;
    let mut user = created;
    if unsafe { GetProcessTimes(handle, &mut created, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64)
}
#[cfg(windows)]
fn windows_process_start(pid: u32) -> io::Result<u64> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let result = windows_process_start_handle(handle);
    unsafe {
        CloseHandle(handle);
    }
    result
}
// Windows implementation is compiled for Windows only and requires real-machine validation.
#[cfg(windows)]
fn recover_stale(file: &Path, binary: &Path) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_INVALID_PARAMETER},
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
            PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
        },
    };
    let data = match fs::read_to_string(file) {
        Ok(v) => v,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if data.lines().any(|line| line == "cleanup-unconfirmed") {
        return Err(io::Error::other("Previous route and DNS cleanup could not be confirmed; investigate before removing the pidfile"));
    }
    let mut lines = data.lines();
    let pid: u32 = lines
        .next()
        .and_then(|v| v.parse().ok())
        .filter(|v| *v > 0)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid pidfile"))?;
    let recorded = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid pidfile"))?;
    let started: u64 = lines.next().and_then(|v| v.parse().ok()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "pidfile is missing process identity",
        )
    })?;
    fn normalize(s: &str) -> String {
        let value = if let Some(unc) = s.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else {
            s.strip_prefix(r"\\?\").unwrap_or(s).to_string()
        };
        value.replace('/', "\\").to_lowercase()
    }
    if normalize(recorded) != normalize(&fs::canonicalize(binary)?.to_string_lossy()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "pidfile executable does not match configured binary",
        ));
    }
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    if handle.is_null() {
        let e = io::Error::last_os_error();
        if e.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            fs::remove_file(file)?;
            return Ok(());
        }
        return Err(e);
    }
    let result = (|| {
        let mut name = vec![0u16; 32768];
        let mut len = name.len() as u32;
        if unsafe { QueryFullProcessImageNameW(handle, 0, name.as_mut_ptr(), &mut len) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let actual = String::from_utf16(&name[..len as usize]).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid process image path")
        })?;
        if windows_process_start_handle(handle)? != started {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stale PID creation time mismatch; process was not killed",
            ));
        }
        if normalize(&actual) != normalize(recorded) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stale PID executable mismatch; process was not killed",
            ));
        }
        if unsafe { TerminateProcess(handle, 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { WaitForSingleObject(handle, 5000) } != 0 {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "stale engine did not exit",
            ));
        }
        Ok(())
    })();
    unsafe {
        CloseHandle(handle);
    }
    result?;
    fs::remove_file(file)
}
#[cfg(not(any(target_os = "linux", windows)))]
fn recover_stale(_file: &Path, _binary: &Path) -> io::Result<()> {
    Ok(())
}
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn mismatch_does_not_kill_process() {
        let file = std::env::temp_dir().join(format!("split-test-pid-{}", std::process::id()));
        let exe = std::env::current_exe().unwrap();
        let other = Path::new("/bin/sh");
        fs::write(
            &file,
            format!(
                "{}\n{}\n",
                std::process::id(),
                fs::canonicalize(other).unwrap().display()
            ),
        )
        .unwrap();
        assert!(recover_stale(&file, other)
            .unwrap_err()
            .to_string()
            .contains("mismatch"));
        assert!(file.exists());
        assert_ne!(fs::canonicalize(other).unwrap(), exe);
        fs::remove_file(file).unwrap();
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    #[test]
    fn mismatch_does_not_terminate_current_process() {
        let file = std::env::temp_dir().join(format!("split-win-test-{}.pid", std::process::id()));
        let system = PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"))
            .join("System32")
            .join("cmd.exe");
        save_pid(&file, std::process::id(), &system).unwrap();
        assert!(recover_stale(&file, &system)
            .unwrap_err()
            .to_string()
            .contains("mismatch"));
        assert!(file.exists());
        fs::remove_file(file).unwrap();
    }
}
