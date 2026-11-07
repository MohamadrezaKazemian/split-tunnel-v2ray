//! Root-side supervisor. The GUI owns the other end of stdin; EOF requests shutdown.
#[cfg(target_os = "linux")]
pub fn run(binary: &std::path::Path, config: &std::path::Path) -> Result<i32, String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("The privileged supervisor must run as root".into());
    }
    run_with_control(binary, config, std::io::stdin())
}
#[cfg(target_os = "linux")]
fn run_with_control<R: std::io::Read + Send + 'static>(
    binary: &std::path::Path,
    config: &std::path::Path,
    mut input: R,
) -> Result<i32, String> {
    use std::{
        os::unix::process::CommandExt,
        process::{Command, Stdio},
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };
    let (tx, rx) = mpsc::channel();
    // stdin is not inherited by sing-box. EOF survives a GUI crash or forced exit.
    thread::spawn(move || {
        let mut bytes = [0u8; 32];
        while let Ok(n) = input.read(&mut bytes) {
            if n == 0 {
                break;
            }
        }
        let _ = tx.send(());
    });
    let mut command = Command::new(binary);
    command
        .arg("run")
        .arg("-c")
        .arg(config)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let parent = std::process::id();
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() as u32 != parent {
                return Err(std::io::Error::other("supervisor exited during startup"));
            }
            Ok(())
        });
    }
    if rx.try_recv().is_ok() {
        return Ok(0);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start sing-box: {e}"))?;
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(status.code().unwrap_or(1));
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGTERM);
                }
                let deadline = Instant::now() + Duration::from_secs(4);
                loop {
                    if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                        return if status.success() {
                            Ok(0)
                        } else {
                            Err(format!("sing-box exited during shutdown ({status}); route and DNS cleanup not confirmed"))
                        };
                    }
                    if Instant::now() >= deadline {
                        child.kill().map_err(|e| e.to_string())?;
                        child.wait().map_err(|e| e.to_string())?;
                        return Err("sing-box did not exit gracefully and was killed; route and DNS cleanup not confirmed".into());
                    }
                    thread::sleep(Duration::from_millis(30));
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::{fs::PermissionsExt, net::UnixStream},
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    #[test]
    fn closed_control_pipe_stops_child() {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let script = std::env::temp_dir().join(format!("split-supervisor-{id}"));
        let marker = std::env::temp_dir().join(format!("split-supervisor-started-{id}"));
        fs::write(
            &script,
            format!(
                "#!/bin/sh\ntrap 'exit 0' TERM\ntouch '{}'\nwhile :; do sleep 0.1; done\n",
                marker.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
        let (reader, writer) = UnixStream::pair().unwrap();
        let binary = script.clone();
        let worker = thread::spawn(move || {
            run_with_control(&binary, std::path::Path::new("/dev/null"), reader)
        });
        let deadline = Instant::now() + Duration::from_secs(3);
        while !marker.exists() {
            assert!(Instant::now() < deadline, "fixture did not start");
            thread::sleep(Duration::from_millis(10));
        }
        drop(writer);
        assert_eq!(worker.join().unwrap().unwrap(), 0);
        fs::remove_file(marker).unwrap();
        fs::remove_file(script).unwrap();
    }
}
