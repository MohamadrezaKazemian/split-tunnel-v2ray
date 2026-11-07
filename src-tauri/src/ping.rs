//! Unprivileged reachability and proxy-delay measurements.
//!
//! The temporary sing-box owner thread lives until every delay request has
//! completed and the process has stopped. This is required on Linux because
//! PR_SET_PDEATHSIG is tied to the thread that spawned the child.
use crate::{engine, process_manager::ProcessManager};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    io,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const MAX_CONCURRENCY: usize = 8;
const TCP_TIMEOUT: Duration = Duration::from_secs(5);
const REAL_TIMEOUT: Duration = Duration::from_secs(5);
const READY_TIMEOUT: Duration = Duration::from_secs(10);
const TEST_URL: &str = "https://www.gstatic.com/generate_204";

#[derive(Clone, Debug, Deserialize)]
pub struct PingTarget {
    pub id: String,
    pub outbound: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PingMeasurement {
    Ms { ms: u64 },
    Timeout,
}

#[derive(Clone, Debug, Serialize)]
pub struct PingResult {
    pub id: String,
    pub tcp: PingMeasurement,
    pub real: PingMeasurement,
}

pub fn ping_configs(
    binary_override: Option<&str>,
    targets: Vec<PingTarget>,
) -> Result<Vec<PingResult>, String> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    let tcp = limited_map(targets.clone(), tcp_delay);
    // TCP probing deliberately remains available without the sidecar.
    let real = match engine::resolve_engine(binary_override) {
        Ok(binary) => real_delays_on_owner_thread(binary, targets.clone()),
        Err(_) => timeouts(targets.len()),
    };
    Ok(targets
        .into_iter()
        .zip(tcp)
        .zip(real)
        .map(|((target, tcp), real)| PingResult {
            id: target.id,
            tcp,
            real,
        })
        .collect())
}

fn limited_map<T: Send + 'static, R: Send + 'static>(
    values: Vec<T>,
    f: impl Fn(T) -> R + Send + Sync + 'static,
) -> Vec<R> {
    let count = values.len();
    let jobs = Arc::new(Mutex::new(
        values.into_iter().enumerate().collect::<VecDeque<_>>(),
    ));
    let results = Arc::new(Mutex::new(
        (0..count).map(|_| None).collect::<Vec<Option<R>>>(),
    ));
    let f = Arc::new(f);
    let mut workers = Vec::new();
    for _ in 0..count.min(MAX_CONCURRENCY) {
        let jobs = Arc::clone(&jobs);
        let results = Arc::clone(&results);
        let f = Arc::clone(&f);
        workers.push(thread::spawn(move || loop {
            let Some((index, value)) = jobs.lock().expect("ping jobs lock poisoned").pop_front()
            else {
                break;
            };
            results.lock().expect("ping results lock poisoned")[index] = Some(f(value));
        }));
    }
    for worker in workers {
        let _ = worker.join();
    }
    let results = match Arc::try_unwrap(results) {
        Ok(results) => results,
        Err(_) => panic!("ping workers retain results"),
    };
    results
        .into_inner()
        .expect("ping results lock poisoned")
        .into_iter()
        .map(|result| result.expect("each ping job has a result"))
        .collect()
}

fn tcp_delay(target: PingTarget) -> PingMeasurement {
    let Some((host, port)) = endpoint(&target.outbound) else {
        return PingMeasurement::Timeout;
    };
    let start = Instant::now();
    let deadline = start + TCP_TIMEOUT;
    let addresses = match (host.as_str(), port).to_socket_addrs() {
        Ok(addresses) => addresses.collect::<Vec<_>>(),
        Err(_) => return PingMeasurement::Timeout,
    };
    for address in addresses {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        if TcpStream::connect_timeout(&address, remaining).is_ok() {
            return PingMeasurement::Ms {
                ms: elapsed_ms(start),
            };
        }
    }
    PingMeasurement::Timeout
}

fn real_delays_on_owner_thread(binary: PathBuf, targets: Vec<PingTarget>) -> Vec<PingMeasurement> {
    // Keep this thread alive for the complete child lifetime; see module docs.
    let count = targets.len();
    thread::spawn(move || real_delays(&binary, targets))
        .join()
        .unwrap_or_else(|_| timeouts(count))
}

fn real_delays(binary: &std::path::Path, targets: Vec<PingTarget>) -> Vec<PingMeasurement> {
    let controller_port = match reserve_loopback_port() {
        Ok(port) => port,
        Err(_) => return timeouts(targets.len()),
    };
    let secret = match random_secret() {
        Ok(secret) => secret,
        Err(_) => return timeouts(targets.len()),
    };
    let config = temporary_config(&targets, controller_port, &secret);
    let mut manager = ProcessManager::new();
    if manager.start(binary, &config.to_string()).is_err()
        || manager.wait_ready(READY_TIMEOUT).is_err()
    {
        let _ = manager.stop(Duration::from_secs(2));
        return timeouts(targets.len());
    }
    let controller = format!("http://127.0.0.1:{controller_port}");
    let secret = Arc::new(secret);
    let delays = limited_map((0..targets.len()).collect::<Vec<_>>(), move |index| {
        clash_delay(&controller, &secret, index)
    });
    let _ = manager.stop(Duration::from_secs(3));
    delays
}

fn temporary_config(targets: &[PingTarget], controller_port: u16, secret: &str) -> Value {
    let outbounds = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let mut outbound = target.outbound.clone();
            outbound["tag"] = format!("ping-{index}").into();
            outbound
        })
        .chain(std::iter::once(json!({"type":"direct", "tag":"direct"})))
        .collect::<Vec<_>>();
    json!({
        "log": {"level":"error", "timestamp": false},
        "inbounds": [],
        "outbounds": outbounds,
        "route": {"final":"direct"},
        "experimental": {
            "clash_api": {
                "external_controller": format!("127.0.0.1:{controller_port}"),
                "secret": secret
            }
        }
    })
}

fn clash_delay(controller: &str, secret: &str, index: usize) -> PingMeasurement {
    clash_delay_for_tag(controller, secret, &format!("ping-{index}"))
}

pub fn running_delay(controller: &str, secret: &str) -> PingMeasurement {
    clash_delay_for_tag(controller, secret, "proxy")
}

fn clash_delay_for_tag(controller: &str, secret: &str, tag: &str) -> PingMeasurement {
    let encoded_url = url_encode(TEST_URL);
    let url = format!("{controller}/proxies/{tag}/delay?url={encoded_url}&timeout=5000");
    let client = match reqwest::blocking::Client::builder()
        .timeout(REAL_TIMEOUT + Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(_) => return PingMeasurement::Timeout,
    };
    let response = client.get(url).bearer_auth(secret).send();
    let Ok(response) = response.and_then(reqwest::blocking::Response::error_for_status) else {
        return PingMeasurement::Timeout;
    };
    match response.json::<Value>() {
        Ok(value) => value
            .get("delay")
            .and_then(Value::as_u64)
            .map(|ms| PingMeasurement::Ms { ms })
            .unwrap_or(PingMeasurement::Timeout),
        Err(_) => PingMeasurement::Timeout,
    }
}

fn endpoint(outbound: &Value) -> Option<(String, u16)> {
    let host = outbound.get("server")?.as_str()?.to_owned();
    let port = outbound.get("server_port")?.as_u64()?.try_into().ok()?;
    Some((host, port))
}

pub fn reserve_loopback_port() -> io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.local_addr().map(|address| address.port())
}

pub fn random_secret() -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn timeouts(count: usize) -> Vec<PingMeasurement> {
    (0..count).map(|_| PingMeasurement::Timeout).collect()
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn tcp_ping_reports_connect_time() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let accepted = thread::spawn(move || listener.accept().unwrap());
        let measurement = tcp_delay(PingTarget {
            id: "test".into(),
            outbound: json!({"server":"127.0.0.1", "server_port":port}),
        });
        assert!(matches!(measurement, PingMeasurement::Ms { .. }));
        accepted.join().unwrap();
    }

    #[test]
    fn tcp_ping_rejects_missing_or_invalid_endpoint() {
        assert_eq!(
            tcp_delay(PingTarget {
                id: "bad".into(),
                outbound: json!({"server_port":443})
            }),
            PingMeasurement::Timeout
        );
        assert_eq!(
            tcp_delay(PingTarget {
                id: "bad".into(),
                outbound: json!({"server":"example.com", "server_port":70000})
            }),
            PingMeasurement::Timeout
        );
    }

    #[test]
    fn temporary_config_has_no_tun_and_unique_safe_tags() {
        let targets = vec![
            PingTarget {
                id: "one".into(),
                outbound: json!({"type":"trojan", "tag":"proxy", "server":"one.example", "server_port":443, "password":"secret"}),
            },
            PingTarget {
                id: "two".into(),
                outbound: json!({"type":"vless", "tag":"proxy", "server":"two.example", "server_port":443, "uuid":"uuid"}),
            },
        ];
        let config = temporary_config(&targets, 12345, "temporary-secret");
        assert_eq!(config["inbounds"], json!([]));
        assert_eq!(config["outbounds"][0]["tag"], "ping-0");
        assert_eq!(config["outbounds"][1]["tag"], "ping-1");
        assert_eq!(
            config["experimental"]["clash_api"]["external_controller"],
            "127.0.0.1:12345"
        );
    }

    #[test]
    fn limiter_runs_every_job() {
        let (tx, rx) = mpsc::channel();
        let results = limited_map((0..20).collect(), move |value| {
            tx.send(value).unwrap();
            value * 2
        });
        assert_eq!(results, (0..20).map(|value| value * 2).collect::<Vec<_>>());
        assert_eq!(rx.iter().count(), 20);
    }
}
