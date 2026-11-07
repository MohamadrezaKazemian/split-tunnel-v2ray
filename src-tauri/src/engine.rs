use std::path::{Path, PathBuf};

pub const MISSING_ENGINE: &str = "Engine missing, run pnpm setup:engine";

fn executable(path: &Path) -> Option<PathBuf> {
    let meta = path.metadata().ok()?;
    if !meta.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    path.canonicalize().ok()
}

fn find_engine(
    override_path: Option<&Path>,
    sidecars: &[PathBuf],
    project_dir: &Path,
    path_dirs: &[PathBuf],
) -> Result<PathBuf, String> {
    if let Some(found) = override_path.and_then(executable) {
        return Ok(found);
    }
    let name = if cfg!(windows) {
        "sing-box.exe"
    } else {
        "sing-box"
    };
    for candidate in sidecars
        .iter()
        .cloned()
        .chain(std::iter::once(project_dir.join("bin").join(name)))
        .chain(path_dirs.iter().map(|dir| dir.join(name)))
    {
        if let Some(found) = executable(&candidate) {
            return Ok(found);
        }
    }
    Err(MISSING_ENGINE.into())
}

pub fn resolve_engine(override_path: Option<&str>) -> Result<PathBuf, String> {
    let name = if cfg!(windows) {
        "sing-box.exe"
    } else {
        "sing-box"
    };
    let sidecars: Vec<PathBuf> = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join(name)))
        .into_iter()
        .collect();
    #[cfg(debug_assertions)]
    let sidecars = {
        let mut sidecars = sidecars;
        let suffix = if cfg!(windows) { ".exe" } else { "" };
        sidecars.push(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("binaries")
                .join(format!("sing-box-{}{suffix}", env!("ENGINE_TARGET_TRIPLE"))),
        );
        sidecars
    };
    #[cfg(debug_assertions)]
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    #[cfg(not(debug_assertions))]
    let cwd = std::env::current_dir().unwrap_or_default();
    let path_dirs: Vec<_> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    find_engine(
        override_path
            .filter(|s| !s.trim().is_empty())
            .map(Path::new),
        &sidecars,
        &cwd,
        &path_dirs,
    )
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };
    fn make(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "test").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    #[test]
    fn resolves_in_order_and_honors_override() {
        let root = std::env::temp_dir().join(format!(
            "engine-search-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sidecar = root.join("sidecar/sing-box");
        let project = root.join("project");
        let project_bin = project.join("bin/sing-box");
        let path_dir = root.join("path");
        let path_bin = path_dir.join("sing-box");
        let other = root.join("custom");
        for p in [&sidecar, &project_bin, &path_bin, &other] {
            make(p);
        }
        let find = |override_path| {
            find_engine(
                override_path,
                std::slice::from_ref(&sidecar),
                &project,
                std::slice::from_ref(&path_dir),
            )
        };
        assert_eq!(find(Some(&other)).unwrap(), other.canonicalize().unwrap());
        assert_eq!(find(None).unwrap(), sidecar.canonicalize().unwrap());
        fs::set_permissions(&other, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(find(Some(&other)).unwrap(), sidecar.canonicalize().unwrap());
        fs::remove_file(&sidecar).unwrap();
        assert_eq!(find(None).unwrap(), project_bin.canonicalize().unwrap());
        assert_eq!(
            find(Some(&sidecar)).unwrap(),
            project_bin.canonicalize().unwrap()
        );
        fs::remove_file(&project_bin).unwrap();
        assert_eq!(find(None).unwrap(), path_bin.canonicalize().unwrap());
        fs::remove_file(&path_bin).unwrap();
        assert_eq!(find(None).unwrap_err(), MISSING_ENGINE);
        assert_eq!(find(Some(&sidecar)).unwrap_err(), MISSING_ENGINE);
        fs::remove_file(other).unwrap();
        fs::remove_dir(root.join("sidecar")).unwrap();
        fs::remove_dir(project.join("bin")).unwrap();
        fs::remove_dir(project).unwrap();
        fs::remove_dir(path_dir).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
