use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub name: String,
    pub path: String,
    pub sandbox: Option<String>,
    pub match_dir: Option<String>,
}

fn executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path
            .metadata()
            .map(|m| m.permissions().mode() & 0o111 == 0)
            .unwrap_or(true)
        {
            return false;
        }
    }
    true
}
fn find_binary(program: &str, paths: &[PathBuf]) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.is_absolute() {
        if executable(path) {
            return path.canonicalize().ok();
        }
        return None;
    }
    if path.components().count() != 1 {
        return None;
    }
    paths
        .iter()
        .map(|p| p.join(program))
        .find(|p| executable(p))
        .and_then(|p| p.canonicalize().ok())
}
fn localized_name(map: &HashMap<String, String>, locale: &str) -> Option<String> {
    let (base, modifier) = locale
        .split_once('@')
        .map_or((locale, None), |(a, b)| (a, Some(b)));
    let base = base.split('.').next().unwrap_or(base);
    let lang = base.split('_').next().unwrap_or(base);
    let mut candidates = Vec::new();
    if let Some(m) = modifier {
        candidates.push(format!("Name[{base}@{m}]"));
    }
    candidates.push(format!("Name[{base}]"));
    if let Some(m) = modifier {
        candidates.push(format!("Name[{lang}@{m}]"));
    }
    candidates.push(format!("Name[{lang}]"));
    candidates.push("Name".into());
    candidates
        .iter()
        .find_map(|k| map.get(k).filter(|s| !s.is_empty()).cloned())
}
fn strip_fields(s: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('%') => out.push('%'),
            Some('f' | 'F' | 'u' | 'U' | 'i' | 'c' | 'k' | 'd' | 'D' | 'n' | 'N' | 'v' | 'm') => {}
            _ => return Err("Invalid desktop field code".into()),
        }
    }
    Ok(out)
}
fn env_assignment(arg: &str) -> bool {
    arg.split_once('=').is_some_and(|(k, _)| {
        !k.is_empty()
            && k.chars()
                .enumerate()
                .all(|(i, c)| c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
    })
}
fn resolve_exec(exec: &str, paths: &[PathBuf]) -> Result<PathBuf, String> {
    let words = shell_words::split(exec)
        .map_err(|e| e.to_string())?
        .iter()
        .map(|s| strip_fields(s))
        .collect::<Result<Vec<_>, _>>()?;
    let mut i = 0;
    while i < words.len() && words[i].is_empty() {
        i += 1;
    }
    if words
        .get(i)
        .is_some_and(|w| Path::new(w).file_name().is_some_and(|n| n == "env"))
    {
        i += 1;
        while i < words.len() {
            if words[i] == "--" {
                i += 1;
                break;
            }
            if words[i] == "-u" || words[i] == "--unset" {
                i += 2;
                continue;
            }
            if words[i] == "-i"
                || words[i] == "--ignore-environment"
                || words[i].starts_with("--unset=")
                || env_assignment(&words[i])
            {
                i += 1;
                continue;
            }
            break;
        }
    }
    while words
        .get(i)
        .is_some_and(|w| w.is_empty() || env_assignment(w))
    {
        i += 1;
    }
    let program = words
        .get(i)
        .filter(|w| !w.is_empty())
        .ok_or("Desktop entry has no executable")?;
    find_binary(program, paths).ok_or_else(|| format!("Executable not found: {program}"))
}
fn safe_related_dir(path: &Path, name: &str) -> Option<String> {
    let base = path.file_name()?.to_string_lossy().to_lowercase();
    let label = name.to_lowercase();
    let dir = path.parent()?;
    let names = [
        "firefox", "chrome", "chromium", "electron", "msedge", "brave", "vivaldi", "discord",
        "code.exe",
    ];
    if base == "electron" || base == "electron.exe" {
        return None;
    }
    let known = names.iter().any(|n| base.contains(n) || label.contains(n))
        || dir.join("chrome-sandbox").is_file()
        || dir.join("chrome_elf.dll").is_file();
    let lower = dir.to_string_lossy().replace('\\', "/").to_lowercase();
    let shared = [
        "/",
        "/usr",
        "/usr/bin",
        "/usr/lib",
        "/usr/lib64",
        "/usr/share",
        "/usr/local",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/usr/local/bin",
        "/bin",
        "/snap/bin",
        "/opt",
        "c:/",
        "c:/windows",
        "c:/windows/system32",
    ];
    if known && !shared.contains(&lower.as_str()) {
        Some(dir.to_string_lossy().into_owned())
    } else {
        None
    }
}
// Common distro launchers are scripts in shared directories; use their real install root.
fn known_wrapper_root(path: &Path, name: &str) -> Option<String> {
    let parent = path.parent()?.to_string_lossy();
    if !matches!(parent.as_ref(), "/usr/bin" | "/usr/local/bin" | "/bin") {
        return None;
    }
    let label = format!("{} {}", name, path.file_name()?.to_string_lossy()).to_lowercase();
    let candidates: &[&str] = if label.contains("chromium") {
        &["/usr/lib/chromium", "/usr/lib/chromium-browser"]
    } else if label.contains("chrome") {
        &["/opt/google/chrome"]
    } else if label.contains("discord") {
        &["/opt/discord", "/usr/lib/discord"]
    } else if label.contains("telegram") {
        &["/opt/telegram", "/opt/Telegram"]
    } else {
        &[]
    };
    candidates
        .iter()
        .find(|p| Path::new(p).is_dir())
        .map(|p| (*p).to_owned())
}
fn desktop_entry(
    content: &str,
    paths: &[PathBuf],
    locale: &str,
    origin: &Path,
    include_hidden: bool,
) -> Result<InstalledApp, String> {
    let mut active = false;
    let mut values = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            active = line == "[Desktop Entry]";
            continue;
        }
        if active && !line.starts_with('#') {
            if let Some((key, value)) = line.split_once('=') {
                values.insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }
    }
    if values.get("Type").map(String::as_str) != Some("Application") {
        return Err("Not an application desktop entry".into());
    }
    if !include_hidden
        && ["Hidden", "NoDisplay"].iter().any(|k| {
            values
                .get(*k)
                .is_some_and(|v| v.eq_ignore_ascii_case("true"))
        })
    {
        return Err("Hidden desktop entry".into());
    }
    let name = localized_name(&values, locale).ok_or("Desktop entry has no name")?;
    if let Some(try_exec) = values.get("TryExec") {
        if find_binary(try_exec, paths).is_none() {
            return Err("TryExec is unavailable".into());
        }
    }
    let path = resolve_exec(
        values.get("Exec").ok_or("Desktop entry has no Exec")?,
        paths,
    )?;
    let origin_string = origin.to_string_lossy().to_lowercase();
    let base = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let sandbox = if origin_string.contains("flatpak") || base == "flatpak" {
        Some("Flatpak".into())
    } else if origin_string.contains("snap") || base == "snap" {
        Some("Snap".into())
    } else {
        None
    };
    let match_dir = if sandbox.as_deref() == Some("Flatpak") {
        values.get("X-Flatpak").and_then(|id| {
            if !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
            {
                return None;
            }
            let mut roots = vec![PathBuf::from("/var/lib/flatpak/app").join(id)];
            if let Some(home) = std::env::var_os("HOME") {
                roots.insert(
                    0,
                    PathBuf::from(home)
                        .join(".local/share/flatpak/app")
                        .join(id),
                );
            }
            roots
                .into_iter()
                .find(|p| p.is_dir())
                .map(|p| p.to_string_lossy().into_owned())
        })
    } else if sandbox.as_deref() == Some("Snap") {
        values.get("X-SnapInstanceName").and_then(|name| {
            if !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
            {
                return None;
            }
            let root = PathBuf::from("/snap").join(name);
            root.is_dir().then(|| root.to_string_lossy().into_owned())
        })
    } else {
        safe_related_dir(&path, &name).or_else(|| known_wrapper_root(&path, &name))
    };
    Ok(InstalledApp {
        name,
        path: path.to_string_lossy().into_owned(),
        sandbox,
        match_dir,
    })
}
fn display_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(unc) = raw.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{unc}");
        }
        return raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_owned();
    }
    #[cfg(not(windows))]
    raw.into_owned()
}
pub fn app_from_path(name: String, path: PathBuf) -> InstalledApp {
    let text = display_path(&path);
    let match_dir = safe_related_dir(Path::new(&text), &name);
    InstalledApp {
        name,
        path: text,
        sandbox: None,
        match_dir,
    }
}
fn locale() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|k| std::env::var(k).ok().filter(|s| !s.is_empty()))
        .unwrap_or_default()
}
fn path_dirs() -> Vec<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect()
}
pub fn resolve_desktop(path: &Path) -> Result<InstalledApp, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    desktop_entry(&data, &path_dirs(), &locale(), path, true)
}
#[cfg(target_os = "linux")]
pub fn scan() -> Vec<InstalledApp> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.insert(0, home.join(".local/share/applications"));
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }
    let paths = path_dirs();
    let locale = locale();
    let mut apps = Vec::new();
    let mut seen = HashSet::new();
    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|v| v.to_str()) != Some("desktop") {
                    continue;
                }
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(app) = desktop_entry(&data, &paths, &locale, &path, false) {
                        if seen.insert((app.path.clone(), app.match_dir.clone())) {
                            apps.push(app);
                        }
                    }
                }
            }
        }
    }
    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.path.cmp(&b.path))
    });
    apps
}
#[cfg(windows)]
pub fn resolve_shortcut(path: &Path) -> Result<InstalledApp, String> {
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", r#"[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); $s=New-Object -ComObject WScript.Shell; $s.CreateShortcut($env:SPLIT_TUNNEL_SHORTCUT).TargetPath"#])
        .env("SPLIT_TUNNEL_SHORTCUT", path).creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("Could not resolve shortcut".into());
    }
    let target = String::from_utf8(output.stdout).map_err(|_| "Invalid shortcut target")?;
    let target = PathBuf::from(target.trim());
    if !target
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        || !target.is_file()
    {
        return Err("Shortcut does not target an executable".into());
    }
    let path = target.canonicalize().map_err(|e| e.to_string())?;
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    Ok(app_from_path(name, path))
}
// Windows Start Menu shortcut resolution uses the system WScript.Shell COM object. Untested on Windows.
#[cfg(windows)]
pub fn scan() -> Vec<InstalledApp> {
    let script = r#"[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); $s=New-Object -ComObject WScript.Shell; $dirs=@((Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'),(Join-Path $env:PROGRAMDATA 'Microsoft\Windows\Start Menu\Programs')); $dirs | ForEach-Object { if (Test-Path -LiteralPath $_) { Get-ChildItem -LiteralPath $_ -Filter '*.lnk' -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object { try { $t=$s.CreateShortcut($_.FullName).TargetPath; if ($t -and $t.ToLower().EndsWith('.exe') -and (Test-Path -LiteralPath $t -PathType Leaf)) { @{ name=$_.BaseName; path=$t } } } catch {} } } } | ConvertTo-Json -Compress"#;
    use std::os::windows::process::CommandExt;
    let Ok(output) = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x08000000)
        .output()
    else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return vec![];
    };
    let entries = match value {
        serde_json::Value::Array(v) => v,
        serde_json::Value::Object(_) => vec![value],
        _ => vec![],
    };
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for item in entries {
        let (Some(name), Some(path)) = (
            item.get("name").and_then(|x| x.as_str()),
            item.get("path").and_then(|x| x.as_str()),
        ) else {
            continue;
        };
        let path = PathBuf::from(path);
        let Ok(path) = path.canonicalize() else {
            continue;
        };
        let raw = path.to_string_lossy();
        let text = if let Some(unc) = raw.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else {
            raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
        };
        let lower_name = name.to_lowercase();
        let lower_target = Path::new(&text)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if lower_name.contains("uninstall")
            || lower_target.contains("uninstall")
            || lower_target.starts_with("unins")
        {
            continue;
        }
        if seen.insert(text.to_lowercase()) {
            apps.push(InstalledApp {
                name: name.into(),
                match_dir: safe_related_dir(Path::new(&text), name),
                path: text,
                sandbox: None,
            });
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}
#[cfg(not(any(target_os = "linux", windows)))]
pub fn scan() -> Vec<InstalledApp> {
    vec![]
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };
    fn fixture() -> (PathBuf, Vec<PathBuf>) {
        let root = std::env::temp_dir().join(format!(
            "desktop-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        for name in ["app", "app with spaces"] {
            let p = root.join(name);
            fs::write(&p, "").unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        (root.clone(), vec![root])
    }
    #[test]
    fn parses_desktop_exec_and_hidden() {
        let (root, paths) = fixture();
        let origin = Path::new("/tmp/app.desktop");
        for exec in [
            "app %u",
            "app %F",
            "env FOO=bar app %U",
            "env -u FOO FOO=bar app %f",
            "\"app with spaces\" %F",
        ] {
            let content = format!(
                "[Desktop Entry]\nType=Application\nName=App\nName[fa]=فارسی\nExec={exec}\n"
            );
            let result = desktop_entry(&content, &paths, "fa_IR.UTF-8", origin, false).unwrap();
            assert_eq!(result.name, "فارسی");
            assert!(Path::new(&result.path).is_file());
        }
        for hidden in ["NoDisplay=true", "Hidden=true"] {
            let content =
                format!("[Desktop Entry]\nType=Application\nName=X\nExec=app\n{hidden}\n");
            assert!(desktop_entry(&content, &paths, "en_US", origin, false).is_err());
        }
        assert!(desktop_entry(
            "[Desktop Entry]\nType=Application\nName=X\nExec=missing %U",
            &paths,
            "en",
            origin,
            false
        )
        .is_err());
        assert!(desktop_entry(
            "[Desktop Entry]\nType=Link\nName=X\nExec=app",
            &paths,
            "en",
            origin,
            false
        )
        .is_err());
        fs::remove_file(root.join("app")).unwrap();
        fs::remove_file(root.join("app with spaces")).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
