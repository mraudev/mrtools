//! Reachability: a short TCP connect to each saved host, plus its IP addresses (so an open
//! connection that only shows the IP can still be matched to a connection saved by name).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Deserialize)]
pub struct Target {
    id: String,
    host: String,
    port: u16,
}

#[derive(Serialize, Default)]
pub struct Status {
    reachable: bool,
    ips: Vec<String>,
}

fn resolve(host: &str, port: u16) -> Vec<SocketAddr> {
    let host = crate::launch::strip_brackets(host.trim());
    (host, port).to_socket_addrs().map(Iterator::collect).unwrap_or_default()
}

fn check(target: &Target) -> Status {
    let addresses = resolve(&target.host, target.port);
    Status {
        // A host can have several addresses (IPv6 and IPv4); one answering is enough.
        reachable: addresses.iter().take(3).any(|a| TcpStream::connect_timeout(a, TIMEOUT).is_ok()),
        ips: addresses.iter().map(|a| a.ip().to_canonical().to_string()).collect(),
    }
}

/// Checks all targets at once (one thread each), so the whole run takes about one timeout.
#[tauri::command]
pub async fn probe(targets: Vec<Target>) -> Result<HashMap<String, Status>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::scope(|scope| {
            let running: Vec<_> = targets
                .iter()
                .map(|t| (t.id.clone(), scope.spawn(move || check(t))))
                .collect();
            running
                .into_iter()
                .map(|(id, handle)| (id, handle.join().unwrap_or_default()))
                .collect()
        })
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn open_and_closed_ports() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let open = check(&Target { id: "a".into(), host: "127.0.0.1".into(), port });
        assert!(open.reachable);
        assert_eq!(open.ips, ["127.0.0.1"]);
        drop(listener);
        let closed = check(&Target { id: "b".into(), host: "127.0.0.1".into(), port });
        assert!(!closed.reachable);
        assert!(!check(&Target { id: "c".into(), host: "not a host".into(), port: 1 }).reachable);
    }
}
