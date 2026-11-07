//! Manual, secret-safe verification for the temporary engine ping path.
use split_tunnel_backend::{engine, ping};
#[cfg(target_os = "linux")]
use std::{collections::BTreeSet, path::Path};

fn main() {
    let outbound = match std::env::var("SPLIT_TUNNEL_PING_OUTBOUND")
        .ok()
        .and_then(|value| serde_json::from_str(&value).ok())
    {
        Some(outbound) => outbound,
        None => {
            eprintln!("Missing or invalid ping configuration.");
            std::process::exit(2);
        }
    };
    let engine = engine::resolve_engine(None).ok();
    #[cfg(target_os = "linux")]
    let before = engine.as_deref().map(live_engine_pids).unwrap_or_default();

    let result = ping::ping_configs(
        None,
        vec![ping::PingTarget {
            id: "manual".into(),
            outbound,
        }],
    )
    .expect("ping measurement should return a result");
    let result = &result[0];
    println!("TCP: {}", display(&result.tcp));
    println!("Real delay: {}", display(&result.real));

    #[cfg(target_os = "linux")]
    if let Some(engine) = engine.as_deref() {
        let after = live_engine_pids(engine);
        let leftover = after.difference(&before).next().copied();
        if let Some(pid) = leftover {
            eprintln!("Temporary sing-box process {pid} is still running.");
            std::process::exit(1);
        }
        println!("Temporary sing-box cleanup: confirmed.");
    }
    #[cfg(not(target_os = "linux"))]
    println!("Temporary sing-box cleanup: not verified on this platform.");
}

fn display(measurement: &ping::PingMeasurement) -> String {
    match measurement {
        ping::PingMeasurement::Ms { ms } => format!("{ms} ms"),
        ping::PingMeasurement::Timeout => "timeout".into(),
    }
}

#[cfg(target_os = "linux")]
fn live_engine_pids(engine: &Path) -> BTreeSet<u32> {
    let Ok(engine) = engine.canonicalize() else {
        return BTreeSet::new();
    };
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return BTreeSet::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let pid = entry.file_name().to_string_lossy().parse::<u32>().ok()?;
            let executable = std::fs::read_link(entry.path().join("exe")).ok()?;
            (executable == engine).then_some(pid)
        })
        .collect()
}
