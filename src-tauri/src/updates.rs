use semver::Version;
use serde::{Deserialize, Serialize};

const RELEASES: &str =
    "https://api.github.com/repos/MohamadrezaKazemian/split-tunnel-v2ray/releases";
const DOWNLOADS: &str =
    "https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/releases/download";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    draft: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdate {
    pub version: String,
    pub notes: String,
    pub kind: String,
    pub command: Option<String>,
}

fn version(value: &str) -> Option<Version> {
    Version::parse(value.strip_prefix('v').unwrap_or(value)).ok()
}

fn newest(releases: Vec<Release>, current: &str) -> Option<Release> {
    let current = version(current)?;
    releases
        .into_iter()
        .filter(|r| {
            !r.draft
                && r.tag_name.starts_with('v')
                && version(&r.tag_name).is_some_and(|v| v > current)
        })
        .max_by_key(|r| version(&r.tag_name))
}

pub async fn check() -> Result<Option<AvailableUpdate>, String> {
    let response = reqwest::Client::builder()
        .user_agent("split-tunnel-v2ray")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?
        .get(format!("{RELEASES}?per_page=100"))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("Could not check releases: {e}"))?;
    let releases: Vec<Release> = response
        .json()
        .await
        .map_err(|e| format!("Invalid release response: {e}"))?;
    let Some(release) = newest(releases, env!("CARGO_PKG_VERSION")) else {
        return Ok(None);
    };
    let parsed = version(&release.tag_name).ok_or("Invalid release version")?;
    let kind = install_kind();
    let command = match kind.as_str() {
        "deb" => Some(format!("wget -O split-tunnel-v2ray_{parsed}_linux_amd64.deb {DOWNLOADS}/{}/split-tunnel-v2ray_{parsed}_linux_amd64.deb && sudo apt install ./split-tunnel-v2ray_{parsed}_linux_amd64.deb", release.tag_name)),
        "arch" => Some(format!("sudo pacman -U {DOWNLOADS}/{}/split-tunnel-v2ray_{}_linux_x86_64.pkg.tar.zst", release.tag_name, parsed)),
        _ => None,
    };
    Ok(Some(AvailableUpdate {
        version: parsed.to_string(),
        notes: release.body.unwrap_or_default(),
        kind,
        command,
    }))
}

pub fn endpoint(version: &str) -> Result<reqwest::Url, String> {
    let parsed = Version::parse(version).map_err(|_| "Invalid update version")?;
    format!("{DOWNLOADS}/v{parsed}/latest.json")
        .parse()
        .map_err(|_| "Invalid update endpoint".into())
}

pub fn install_kind() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(exe) = std::env::current_exe() {
            for (program, args, kind) in [
                ("dpkg-query", vec!["-S"], "deb"),
                ("pacman", vec!["-Qo"], "arch"),
            ] {
                if std::process::Command::new(program)
                    .args(args)
                    .arg(&exe)
                    .output()
                    .is_ok_and(|o| o.status.success())
                {
                    return kind.into();
                }
            }
            if let (Some(image), Some(dir)) =
                (std::env::var_os("APPIMAGE"), std::env::var_os("APPDIR"))
            {
                if std::path::Path::new(&image).is_file()
                    && std::path::Path::new(&dir)
                        .canonicalize()
                        .is_ok_and(|dir| exe.starts_with(dir))
                {
                    return "appimage".into();
                }
            }
        }
    }
    #[cfg(windows)]
    {
        if windows_nsis_install() {
            return "nsis".into();
        }
    }
    "unsupported".into()
}

#[cfg(windows)]
fn windows_nsis_install() -> bool {
    // Verify the current executable is the registered NSIS uninstall location.
    let Ok(exe) = std::env::current_exe().and_then(|p| p.canonicalize()) else {
        return false;
    };
    for hive in ["HKCU", "HKLM"] {
        let key =
            format!(r"{hive}\Software\Microsoft\Windows\CurrentVersion\Uninstall\Split Tunnel");
        if let Ok(output) = std::process::Command::new("reg")
            .args(["query", &key, "/v", "InstallLocation"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(dir) = text.lines().find_map(|line| {
                line.split_once("REG_SZ")
                    .map(|(_, value)| value.trim().trim_matches('"'))
            }) {
                if std::path::Path::new(dir)
                    .join("split-tunnel-backend.exe")
                    .canonicalize()
                    .is_ok_and(|p| p == exe)
                {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares_prereleases_and_ignores_drafts_and_invalid_tags() {
        let releases: Vec<Release> = serde_json::from_str(r#"[{"tag_name":"v1.0.0-beta.1","body":"beta"},{"tag_name":"v1.0.0","draft":true},{"tag_name":"garbage"},{"tag_name":"v0.9.9"}]"#).unwrap();
        let result = newest(releases, "0.9.0").unwrap();
        assert_eq!(result.tag_name, "v1.0.0-beta.1");
        assert_eq!(result.body.as_deref(), Some("beta"));
        assert!(newest(vec![result], "1.0.0").is_none());
    }
    #[test]
    fn endpoint_accepts_only_semver() {
        assert!(endpoint("../../bad").is_err());
        assert!(endpoint("1.2.3-beta.1")
            .unwrap()
            .as_str()
            .ends_with("/v1.2.3-beta.1/latest.json"));
    }
}
