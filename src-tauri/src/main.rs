use serde::Serialize;
use split_tunnel_backend::service::{Request, Service};
use std::{path::PathBuf, sync::mpsc};
use tauri::{Emitter, Manager, State};

#[derive(Clone, Serialize)]
struct Event {
    status: String,
    logs: Vec<String>,
}
struct AppState {
    service: Service,
    data_file: PathBuf,
    installed_cache: std::sync::Arc<
        std::sync::OnceLock<Vec<split_tunnel_backend::installed_apps::InstalledApp>>,
    >,
}

fn reply(
    service: &Service,
    f: impl FnOnce(mpsc::Sender<Result<(), String>>) -> Request,
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    service.request(f(tx))?;
    rx.recv().map_err(|e| e.to_string())?
}
// Always put actual engine and GUI executables ahead of user-controlled matchers.
fn protect_processes(
    config: &mut serde_json::Value,
    engine: &std::path::Path,
    app: &std::path::Path,
) -> Result<(), String> {
    let mut expressions = Vec::new();
    let mut names = Vec::new();
    for path in [engine, app] {
        let raw = path.to_str().ok_or("Executable path is not valid UTF-8")?;
        let normalized = if cfg!(windows) {
            if let Some(unc) = raw.strip_prefix(r"\\?\UNC\") {
                format!(r"\\{unc}")
            } else {
                raw.strip_prefix(r"\\?\").unwrap_or(raw).to_owned()
            }
        } else {
            raw.to_owned()
        };
        let raw = normalized.as_str();
        let escaped: String = raw
            .chars()
            .flat_map(|c| {
                if "\\.+*?()|[]{}^$".contains(c) {
                    vec!['\\', c]
                } else {
                    vec![c]
                }
            })
            .collect();
        expressions.push(format!(
            "{}^{}$",
            if cfg!(windows) { "(?i)" } else { "" },
            escaped
        ));
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            names.push(name.to_owned());
        }
    }
    let route = config
        .pointer_mut("/route/rules")
        .and_then(|v| v.as_array_mut())
        .ok_or("Missing route rules")?;
    route.insert(0, serde_json::json!({"process_path_regex": expressions, "action":"route", "outbound":"direct"}));
    route.insert(
        0,
        serde_json::json!({"process_name": names, "action":"route", "outbound":"direct"}),
    );
    let dns = config
        .pointer_mut("/dns/rules")
        .and_then(|v| v.as_array_mut())
        .ok_or("Missing DNS rules")?;
    dns.insert(
        0,
        serde_json::json!({"process_path_regex": expressions, "action":"route", "server":"local"}),
    );
    dns.insert(
        0,
        serde_json::json!({"process_name": names, "action":"route", "server":"local"}),
    );
    Ok(())
}

#[tauri::command]
async fn connect(
    state: State<'_, AppState>,
    mut config: serde_json::Value,
    binary: Option<String>,
) -> Result<(), String> {
    let binary = split_tunnel_backend::engine::resolve_engine(binary.as_deref())?;
    let own_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    protect_processes(&mut config, &binary, &own_exe)?;
    #[cfg(windows)]
    if let Some(inbounds) = config.get_mut("inbounds").and_then(|v| v.as_array_mut()) {
        for inbound in inbounds {
            if inbound.get("type").and_then(|v| v.as_str()) == Some("tun") {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                inbound["interface_name"] =
                    format!("SplitTunnel-{}-{timestamp}", std::process::id()).into();
            }
        }
    }
    let level = config
        .pointer("/log/level")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    let engine_level = if level == "trace" || level == "debug" {
        level
    } else {
        "info"
    }
    .to_string();
    config["log"]["level"] = engine_level.into();
    let controller_port = split_tunnel_backend::ping::reserve_loopback_port()
        .map_err(|_| "Could not reserve the local ping controller port")?;
    let controller_secret = split_tunnel_backend::ping::random_secret()
        .map_err(|_| "Could not create the local ping controller secret")?;
    config["experimental"]["clash_api"] = serde_json::json!({
        "external_controller": format!("127.0.0.1:{controller_port}"),
        "secret": controller_secret,
    });
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        reply(&service, |reply| Request::Connect {
            binary,
            config: config.to_string(),
            reply,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        reply(&service, |reply| Request::Disconnect { reply })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn status(state: State<'_, AppState>) -> Result<String, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (tx, rx) = mpsc::channel();
        service.request(Request::Status { reply: tx })?;
        rx.recv().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn ping_configs(
    binary: Option<String>,
    targets: Vec<split_tunnel_backend::ping::PingTarget>,
) -> Result<Vec<split_tunnel_backend::ping::PingResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        split_tunnel_backend::ping::ping_configs(binary.as_deref(), targets)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn active_ping(
    state: State<'_, AppState>,
) -> Result<split_tunnel_backend::ping::PingMeasurement, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (tx, rx) = mpsc::channel();
        service.request(Request::Ping { reply: tx })?;
        rx.recv().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn fetch_subscription(url: String) -> Result<String, String> {
    const LIMIT: usize = 2 * 1024 * 1024;
    let parsed = reqwest::Url::parse(&url).map_err(|_| "Invalid subscription URL")?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("Subscription URL must use HTTP or HTTPS".into());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| "Could not initialize HTTP client")?;
    let mut response = client
        .get(parsed)
        .send()
        .await
        .map_err(|_| "Could not fetch subscription")?
        .error_for_status()
        .map_err(|e| {
            format!(
                "Subscription server returned {}",
                e.status().map(|s| s.to_string()).unwrap_or_default()
            )
        })?;
    if response.content_length().is_some_and(|n| n > LIMIT as u64) {
        return Err("Subscription is too large".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read subscription")?
    {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("Subscription is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| "Subscription is not UTF-8".into())
}
#[derive(Serialize)]
struct RunningProcess {
    #[serde(flatten)]
    app: split_tunnel_backend::installed_apps::InstalledApp,
    protected: bool,
}
#[tauri::command]
async fn running_apps(binary: Option<String>) -> Result<Vec<RunningProcess>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = split_tunnel_backend::engine::resolve_engine(binary.as_deref()).ok();
        let own = std::env::current_exe()
            .ok()
            .and_then(|p| p.canonicalize().ok());
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        let mut apps: Vec<_> = sys
            .processes()
            .values()
            .filter_map(|p| {
                p.exe().map(|e| {
                    let canonical = e.canonicalize().ok();
                    let protected = canonical.as_ref().is_some_and(|path| {
                        Some(path) == engine.as_ref() || Some(path) == own.as_ref()
                    });
                    RunningProcess {
                        app: split_tunnel_backend::installed_apps::app_from_path(
                            p.name().to_string_lossy().into_owned(),
                            e.to_path_buf(),
                        ),
                        protected,
                    }
                })
            })
            .collect();
        apps.sort_by_key(|a| a.app.name.to_lowercase());
        apps
    })
    .await
    .map_err(|e| e.to_string())
}
#[tauri::command]
async fn installed_apps(
    state: State<'_, AppState>,
) -> Result<Vec<split_tunnel_backend::installed_apps::InstalledApp>, String> {
    let cache = state.installed_cache.clone();
    tauri::async_runtime::spawn_blocking(move || {
        cache
            .get_or_init(split_tunnel_backend::installed_apps::scan)
            .clone()
    })
    .await
    .map_err(|e| e.to_string())
}
#[tauri::command]
async fn resolve_app(
    path: String,
) -> Result<split_tunnel_backend::installed_apps::InstalledApp, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let file = PathBuf::from(&path);
        #[cfg(windows)]
        if file
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
        {
            return split_tunnel_backend::installed_apps::resolve_shortcut(&file);
        }
        if file.extension().and_then(|v| v.to_str()) == Some("desktop") {
            return split_tunnel_backend::installed_apps::resolve_desktop(&file);
        }
        let path = file.canonicalize().map_err(|e| e.to_string())?;
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        Ok(split_tunnel_backend::installed_apps::app_from_path(
            name, path,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn load_data(state: State<'_, AppState>) -> Result<Option<serde_json::Value>, String> {
    match std::fs::read(&state.data_file) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
#[tauri::command]
fn save_data(state: State<'_, AppState>, data: serde_json::Value) -> Result<(), String> {
    let path = &state.data_file;
    std::fs::create_dir_all(path.parent().ok_or("Invalid data path")?)
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    file.write_all(&serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn main() {
    #[cfg(target_os = "linux")]
    {
        let args: Vec<_> = std::env::args_os().collect();
        if args.get(1).is_some_and(|x| x == "--privileged-supervisor") {
            if args.len() != 4 {
                eprintln!("Invalid supervisor arguments");
                std::process::exit(2);
            }
            match split_tunnel_backend::privileged_helper::run(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
            ) {
                Ok(code) => std::process::exit(code),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let handle = app.handle().clone();
            let service = Service::new(dir.join("sing-box.pid"), move |status, logs| {
                let _ = handle.emit(
                    "tunnel-status",
                    Event {
                        status,
                        logs: logs.into_iter().map(|l| l.text).collect(),
                    },
                );
            });
            app.manage(AppState {
                service,
                data_file: dir.join("data.json"),
                installed_cache: Default::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            connect,
            disconnect,
            status,
            ping_configs,
            active_ping,
            running_apps,
            installed_apps,
            fetch_subscription,
            resolve_app,
            load_data,
            save_data
        ])
        .build(tauri::generate_context!())
        .expect("failed to build app")
        .run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = handle.state::<AppState>();
                let _ = state.service.request(Request::Shutdown);
            }
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn safety_rules_precede_broad_user_rules_for_route_and_dns() {
        let mut config = serde_json::json!({"route":{"rules":[{"process_path_regex":["(?i).*"],"action":"route","outbound":"proxy"}]},"dns":{"rules":[{"process_path_regex":["(?i).*"],"action":"route","server":"remote"}]}});
        let engine = std::path::Path::new("/opt/example/sing+box");
        let app = std::path::Path::new("/opt/example/app");
        super::protect_processes(&mut config, engine, app).unwrap();
        assert_eq!(config["route"]["rules"][0]["outbound"], "direct");
        assert_eq!(config["route"]["rules"][1]["outbound"], "direct");
        assert_eq!(config["dns"]["rules"][0]["server"], "local");
        assert_eq!(config["dns"]["rules"][1]["server"], "local");
        assert!(config["route"]["rules"][1]["process_path_regex"][0]
            .as_str()
            .unwrap()
            .contains("sing\\+box"));
        assert_eq!(config["route"]["rules"][2]["outbound"], "proxy");
    }
    use super::*;
    use std::io::{Read, Write};
    #[test]
    fn fetches_subscription_body_from_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 2048];
            let _ = stream.read(&mut buffer).unwrap();
            let body = "trojan://secret@example.com:443\n";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let body = tauri::async_runtime::block_on(fetch_subscription(format!(
            "http://{address}/subscription"
        )))
        .unwrap();
        assert_eq!(body, "trojan://secret@example.com:443\n");
        server.join().unwrap();
    }
}
