//! Brigade MESH.
//!
//! Certification of mutation digests. In-process mesh for tests;
//! TCP broadcast for live nodes.

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use oursql_core::{CommitKind, Error, Result};
use oursql_crypto::digest;

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

/// In-process collective. Tests use this instead of sockets.
#[derive(Clone, Default)]
pub struct LocalMesh {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    views: HashMap<String, View>,
    certified: HashSet<[u8; 32]>,
    log: Vec<(String, String)>, // (node, stmt)
}

impl LocalMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn join(&self, node: &str) {
        let mut g = self.inner.lock().expect("mesh");
        let v = g.views.entry("default".into()).or_default();
        v.members.insert(node.to_string());
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

    pub fn apply_log(&self) -> Vec<(String, String)> {
        self.inner.lock().expect("mesh").log.clone()
    }

    pub fn is_certified(&self, d: &[u8; 32]) -> bool {
        self.inner.lock().expect("mesh").certified.contains(d)
    }
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
}
