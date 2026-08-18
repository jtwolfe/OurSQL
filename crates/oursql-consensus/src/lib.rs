//! Brigade MESH.
//!
//! Certification of mutation digests. In-process hub for tests;
//! TCP APPLY/REPAIR for live nodes.

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use oursql_core::{CommitKind, Error, Result};
use oursql_crypto::digest;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Cert {
    pub digest: [u8; 32],
    pub node: String,
}

#[derive(Clone, Default)]
pub struct View {
    pub members: HashSet<String>,
    pub epoch: u64,
}

impl View {
    pub fn quorum(&self) -> usize {
        let n = self.members.len().max(1);
        n / 2 + 1
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyMsg {
    pub from: String,
    pub seq: u64,
    pub recs_json: String,
    pub digest: String,
}

/// In-process collective. Tests use this instead of sockets.
#[derive(Clone, Default)]
pub struct LocalMesh {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    views: HashMap<String, View>,
    certified: HashSet<[u8; 32]>,
    log: Vec<(String, String)>,
    inboxes: HashMap<String, Vec<ApplyMsg>>,
}

impl LocalMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn join(&self, node: &str) {
        let mut g = self.inner.lock().expect("mesh");
        let v = g.views.entry("default".into()).or_default();
        v.members.insert(node.to_string());
        g.inboxes.entry(node.to_string()).or_default();
    }

    pub fn certify(&self, node: &str, stmt: &str, kind: CommitKind) -> Result<[u8; 32]> {
        let d = digest(stmt.as_bytes());
        let mut g = self.inner.lock().expect("mesh");
        if matches!(kind, CommitKind::Soyuz | CommitKind::Cheka) {
            let v = g.views.entry("default".into()).or_default();
            if v.members.is_empty() {
                v.members.insert(node.to_string());
            }
            if !v.members.contains(node) {
                return Err(Error::mesh(2101, "NOT_IN_VIEW", "node not in view"));
            }
        }
        g.certified.insert(d);
        g.log.push((node.to_string(), stmt.to_string()));
        Ok(d)
    }

    pub fn publish(&self, from: &str, msg: ApplyMsg) -> usize {
        let mut g = self.inner.lock().expect("mesh");
        let mut n = 0;
        let names: Vec<String> = g.inboxes.keys().cloned().collect();
        for name in names {
            if name != from {
                g.inboxes.get_mut(&name).unwrap().push(msg.clone());
                n += 1;
            }
        }
        n
    }

    pub fn drain(&self, node: &str) -> Vec<ApplyMsg> {
        let mut g = self.inner.lock().expect("mesh");
        g.inboxes
            .entry(node.to_string())
            .or_default()
            .drain(..)
            .collect()
    }

    pub fn apply_log(&self) -> Vec<(String, String)> {
        self.inner.lock().expect("mesh").log.clone()
    }

    pub fn is_certified(&self, d: &[u8; 32]) -> bool {
        self.inner.lock().expect("mesh").certified.contains(d)
    }

    pub fn members(&self) -> Vec<String> {
        let g = self.inner.lock().expect("mesh");
        g.views
            .get("default")
            .map(|v| v.members.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Placement: hash(narodkey) % n, plus R-1 successors. RF=2 on 2 nodes = everyone.
    pub fn owners(&self, narodkey: &str, rf: usize) -> Vec<String> {
        let mut m = self.members();
        m.sort();
        if m.is_empty() {
            return m;
        }
        let h = narodkey
            .bytes()
            .fold(0usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize));
        let start = h % m.len();
        let take = rf.min(m.len()).max(1);
        let mut out = Vec::new();
        for i in 0..take {
            out.push(m[(start + i) % m.len()].clone());
        }
        out
    }
}

/// Ask a peer for a snapshot of certified state.
pub fn request_repair(addr: &str) -> Result<ApplyMsg> {
    let mut s = TcpStream::connect(addr)?;
    s.set_read_timeout(Some(Duration::from_secs(5))).ok();
    s.set_write_timeout(Some(Duration::from_secs(5))).ok();
    writeln!(s, "NEED")?;
    s.flush()?;
    let mut reader = BufReader::new(s);
    let mut resp = String::new();
    reader.read_line(&mut resp)?;
    let rest = resp
        .strip_prefix("SNAPSHOT ")
        .ok_or_else(|| Error::mesh(2108, "NODE_BUSY", "no snapshot"))?;
    serde_json::from_str(rest.trim()).map_err(|e| Error::mesh(2108, "NODE_BUSY", e.to_string()))
}

/// Push APPLY json lines to a peer. Returns true on ACK.
pub fn push_peer(addr: &str, msg: &ApplyMsg) -> Result<bool> {
    let mut s = TcpStream::connect(addr)?;
    s.set_read_timeout(Some(Duration::from_secs(3))).ok();
    s.set_write_timeout(Some(Duration::from_secs(3))).ok();
    let line =
        serde_json::to_string(msg).map_err(|e| Error::mesh(2108, "NODE_BUSY", e.to_string()))?;
    writeln!(s, "APPLY {line}")?;
    s.flush()?;
    let mut reader = BufReader::new(s);
    let mut resp = String::new();
    reader.read_line(&mut resp)?;
    Ok(resp.starts_with("ACK"))
}

/// Listen for APPLY / NEED on `addr`.
pub fn serve_mesh(
    addr: &str,
    on_apply: Arc<dyn Fn(ApplyMsg) -> Result<()> + Send + Sync>,
    on_need: Arc<dyn Fn() -> Result<ApplyMsg> + Send + Sync>,
) -> Result<()> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(false)?;
    thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(stream) = conn else { continue };
            let apply = Arc::clone(&on_apply);
            let need = Arc::clone(&on_need);
            thread::spawn(move || {
                let _ = handle_mesh(stream, apply, need);
            });
        }
    });
    Ok(())
}

fn handle_mesh(
    stream: TcpStream,
    on_apply: Arc<dyn Fn(ApplyMsg) -> Result<()> + Send + Sync>,
    on_need: Arc<dyn Fn() -> Result<ApplyMsg> + Send + Sync>,
) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if let Some(rest) = line.strip_prefix("APPLY ") {
        let msg: ApplyMsg = serde_json::from_str(rest.trim())
            .map_err(|e| Error::mesh(2108, "NODE_BUSY", e.to_string()))?;
        on_apply(msg)?;
        writeln!(writer, "ACK")?;
    } else if line.starts_with("NEED") {
        let msg = on_need()?;
        let body = serde_json::to_string(&msg)
            .map_err(|e| Error::mesh(2108, "NODE_BUSY", e.to_string()))?;
        writeln!(writer, "SNAPSHOT {body}")?;
    } else {
        writeln!(writer, "ERR")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soyuz_certifies() {
        let m = LocalMesh::new();
        m.join("a");
        m.join("b");
        let d = m
            .certify("a", "INZRT V t ZNACH (1)", CommitKind::Soyuz)
            .unwrap();
        assert!(m.is_certified(&d));
    }

    #[test]
    fn hub_delivers() {
        let m = LocalMesh::new();
        m.join("a");
        m.join("b");
        m.publish(
            "a",
            ApplyMsg {
                from: "a".into(),
                seq: 1,
                recs_json: "[]".into(),
                digest: "d".into(),
            },
        );
        assert_eq!(m.drain("b").len(), 1);
        assert!(m.drain("a").is_empty());
    }

    #[test]
    fn rf2_two_nodes_both_own() {
        let m = LocalMesh::new();
        m.join("a");
        m.join("b");
        let o = m.owners("NAR-001", 2);
        assert_eq!(o.len(), 2);
    }
}
