//! Brigade AUTHZ.
//!
//! Comrades, komitets, capabilities. Persisted next to node.key. No WAL.

#![deny(unsafe_code)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use oursql_core::{ComradeId, Error, Result};
use oursql_crypto::{hex, KeyPair, NodeIdentity};
use oursql_nashcql::Stmt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Verb {
    Obtan,
    Inzrt,
    Opdat,
    Remov,
    Ddl,
    Cheka,
    Accuse,
    Admin,
    Approve,
}

impl Verb {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_uppercase().as_str() {
            "OBTAN" | "SELECT" => Ok(Self::Obtan),
            "INZRT" | "INSERT" => Ok(Self::Inzrt),
            "OPDAT" | "UPDATE" => Ok(Self::Opdat),
            "REMOV" | "DELETE" => Ok(Self::Remov),
            "DDL" | "MANUFAKTUR" | "PERESTROJ" | "UNMAK" => Ok(Self::Ddl),
            "CHEKA" => Ok(Self::Cheka),
            "ACCUSE" => Ok(Self::Accuse),
            "ADMIN" => Ok(Self::Admin),
            "APPROVAL" | "APPROVE" => Ok(Self::Approve),
            other => Err(Error::bad_keyword(format!("unknown verb {other}"))),
        }
    }

    pub fn of(stmt: &Stmt) -> Self {
        match stmt {
            Stmt::Obtan { .. }
            | Stmt::PokazTabl
            | Stmt::PokazUstanov
            | Stmt::PokazAudit
            | Stmt::PokazComrade
            | Stmt::Doklad { .. }
            | Stmt::Razbor(_) => Verb::Obtan,
            Stmt::Inzrt { .. } => Verb::Inzrt,
            Stmt::Opdat { .. } => Verb::Opdat,
            Stmt::Remov { .. } | Stmt::Ochistka { .. } => Verb::Remov,
            Stmt::ManufakturTabl { .. }
            | Stmt::ManufakturSpravka { .. }
            | Stmt::UnmakTabl { .. }
            | Stmt::PerestrojAdd { .. } => Verb::Ddl,
            Stmt::Confiskat { .. } | Stmt::Osvobod { .. } => Verb::Cheka,
            Stmt::Accuse { .. } => Verb::Accuse,
            Stmt::Nagrad { .. } | Stmt::Otyat { .. } => Verb::Admin,
            _ => Verb::Obtan,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    pub holder: String,
    pub verbs: HashSet<Verb>,
    pub not_after_epoch: Option<u64>,
}

impl Capability {
    pub fn god(holder: impl Into<String>) -> Self {
        Self {
            holder: holder.into(),
            verbs: HashSet::from([
                Verb::Obtan,
                Verb::Inzrt,
                Verb::Opdat,
                Verb::Remov,
                Verb::Ddl,
                Verb::Cheka,
                Verb::Accuse,
                Verb::Admin,
                Verb::Approve,
            ]),
            not_after_epoch: None,
        }
    }

    pub fn expired(&self, now: u64) -> bool {
        match self.not_after_epoch {
            Some(t) => now >= t,
            None => false,
        }
    }

    pub fn allows(&self, v: &Verb, now: u64) -> bool {
        if self.expired(now) {
            return false;
        }
        self.verbs.contains(v) || self.verbs.contains(&Verb::Admin)
    }
}

#[derive(Serialize, Deserialize)]
struct Persist {
    comrades: HashSet<String>,
    caps: Vec<Capability>,
}

pub struct Authz {
    pub node: KeyPair,
    pub comrades: HashSet<String>,
    pub caps: Vec<Capability>,
    path: PathBuf,
}

impl Authz {
    pub fn open_in(dir: impl AsRef<Path>, identity: &NodeIdentity) -> Result<Self> {
        let path = dir.as_ref().join("authz.json");
        if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            let p: Persist =
                serde_json::from_str(&raw).map_err(|e| Error::recovery_failed(e.to_string()))?;
            return Ok(Self {
                node: identity.keys.clone(),
                comrades: p.comrades,
                caps: p.caps,
                path,
            });
        }
        let founder = "founder".to_string();
        let a = Self {
            node: identity.keys.clone(),
            comrades: HashSet::from([founder.clone()]),
            caps: vec![Capability::god(founder)],
            path,
        };
        a.save()?;
        Ok(a)
    }

    pub fn open() -> Self {
        let node = KeyPair::generate();
        let founder = "founder".to_string();
        Self {
            node,
            comrades: HashSet::from([founder.clone()]),
            caps: vec![Capability::god(founder)],
            path: PathBuf::from("authz.json"),
        }
    }

    fn save(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let p = Persist {
            comrades: self.comrades.clone(),
            caps: self.caps.clone(),
        };
        let s = serde_json::to_string_pretty(&p).map_err(|e| Error::wal_io(e.to_string()))?;
        std::fs::write(&self.path, s)?;
        Ok(())
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn check(&self, who: &ComradeId, stmt: &Stmt) -> Result<()> {
        let verb = Verb::of(stmt);
        let now = Self::now();
        let ok = self
            .caps
            .iter()
            .filter(|c| c.holder == who.0 || c.holder == "*")
            .any(|c| c.allows(&verb, now));
        if !ok {
            return Err(Error::cap_expired());
        }
        Ok(())
    }

    pub fn nagrad_god(&mut self, holder: impl Into<String>) {
        let h = holder.into();
        self.comrades.insert(h.clone());
        if !self.caps.iter().any(|c| c.holder == h) {
            self.caps.push(Capability::god(h));
        }
        let _ = self.save();
    }

    pub fn nagrad(&mut self, holder: &str, verb: Verb, ttl_secs: Option<u64>) -> Result<()> {
        self.comrades.insert(holder.to_string());
        let not_after = if matches!(verb, Verb::Cheka) {
            Some(Self::now() + ttl_secs.unwrap_or(24 * 3600).min(7 * 24 * 3600))
        } else {
            ttl_secs.map(|s| Self::now() + s)
        };
        if let Some(c) = self.caps.iter_mut().find(|c| c.holder == holder) {
            c.verbs.insert(verb);
            if not_after.is_some() {
                c.not_after_epoch = not_after;
            }
        } else {
            let mut verbs = HashSet::new();
            verbs.insert(verb);
            verbs.insert(Verb::Obtan);
            self.caps.push(Capability {
                holder: holder.to_string(),
                verbs,
                not_after_epoch: not_after,
            });
        }
        self.save()
    }

    pub fn otyat(&mut self, holder: &str, verb: Verb) -> Result<()> {
        for c in &mut self.caps {
            if c.holder == holder {
                c.verbs.remove(&verb);
            }
        }
        self.save()
    }

    pub fn hello(&self, name: &str) -> Result<ComradeId> {
        if self.comrades.contains(name) || self.caps.iter().any(|c| c.holder == name) {
            Ok(ComradeId(name.to_string()))
        } else {
            Err(Error::cap_expired())
        }
    }

    pub fn sign_mutation(&self, digest: &[u8; 32]) -> String {
        hex(&self.node.sign(digest))
    }

    pub fn verify_mutation(&self, digest: &[u8; 32], sig_hex: &str) -> bool {
        let Some(sig) = oursql_crypto::unhex64(sig_hex) else {
            return false;
        };
        KeyPair::verify(&self.node.public_hex(), digest, &sig)
    }

    pub fn list_comrades(&self) -> Vec<String> {
        let mut v: Vec<String> = self.comrades.iter().cloned().collect();
        v.sort();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oursql_nashcql::parse;

    #[test]
    fn founder_can_ddl() {
        let a = Authz::open();
        let p = parse("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
        a.check(&ComradeId("founder".into()), &p.stmts[0]).unwrap();
    }

    #[test]
    fn stranger_denied() {
        let a = Authz::open();
        let p = parse("OBTAN * IZ t").unwrap();
        assert!(a.check(&ComradeId("spy".into()), &p.stmts[0]).is_err());
    }

    #[test]
    fn cheka_expires() {
        let mut a = Authz::open();
        a.nagrad("mill", Verb::Cheka, Some(0)).unwrap();
        // ttl 0 => already expired
        let p = parse("CONFISKAT TABL t").unwrap();
        assert!(a.check(&ComradeId("mill".into()), &p.stmts[0]).is_err());
    }
}
