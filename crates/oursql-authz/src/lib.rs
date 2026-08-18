//! Brigade AUTHZ.
//!
//! Comrades, komitets, bilets (capabilities). Persisted next to node.key.

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
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

    pub fn as_nash(&self) -> &'static str {
        match self {
            Verb::Obtan => "OBTAN",
            Verb::Inzrt => "INZRT",
            Verb::Opdat => "OPDAT",
            Verb::Remov => "REMOV",
            Verb::Ddl => "MANUFAKTUR",
            Verb::Cheka => "CHEKA",
            Verb::Accuse => "ACCUSE",
            Verb::Admin => "ADMIN",
            Verb::Approve => "APPROVAL",
        }
    }

    pub fn of(stmt: &Stmt) -> Self {
        match stmt {
            Stmt::Obtan { .. }
            | Stmt::PokazTabl
            | Stmt::PokazUstanov
            | Stmt::PokazAudit
            | Stmt::PokazComrade
            | Stmt::PokazBilet
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

/// Extra leash on a bilet. All optional; empty uslov means "no extra leash".
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Uslov {
    pub ration: Option<u32>,
    pub max_rows: Option<u64>,
    pub samokrit: bool,
}

/// A NAGRAD ticket. Field names are NashCQL-shaped; old JSON aliases still load.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    #[serde(default)]
    pub bilet: String,
    #[serde(alias = "holder")]
    pub comrade: String,
    #[serde(alias = "verbs")]
    pub deystv: HashSet<Verb>,
    /// Scope: None = whole kollektiv, Some(tabl) = that tabl only.
    #[serde(default)]
    pub predel: Option<String>,
    #[serde(default)]
    pub nachat: Option<u64>,
    #[serde(alias = "not_after_epoch", default)]
    pub srok: Option<u64>,
    #[serde(default = "founders")]
    pub komitet: String,
    #[serde(default)]
    pub uslov: Uslov,
}

fn founders() -> String {
    "FOUNDERS".into()
}

impl Capability {
    pub fn god(comrade: impl AsRef<str>) -> Self {
        let c = comrade.as_ref().to_string();
        Self {
            bilet: format!("BIL-GOD-{}", comrade_slug(&c)),
            comrade: c,
            deystv: HashSet::from([
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
            predel: None,
            nachat: None,
            srok: None,
            komitet: "FOUNDERS".into(),
            uslov: Uslov::default(),
        }
    }

    pub fn live(&self, now: u64) -> bool {
        if let Some(s) = self.nachat {
            if now < s {
                return false;
            }
        }
        match self.srok {
            Some(t) => now < t,
            None => true,
        }
    }

    pub fn covers_tabl(&self, tabl: Option<&str>) -> bool {
        match (&self.predel, tabl) {
            (None, _) => true,
            (Some(_), None) => true,
            (Some(p), Some(t)) => p.eq_ignore_ascii_case(t),
        }
    }

    pub fn allows(&self, v: &Verb, now: u64, tabl: Option<&str>) -> bool {
        if !self.live(now) {
            return false;
        }
        if !self.covers_tabl(tabl) {
            return false;
        }
        self.deystv.contains(v) || self.deystv.contains(&Verb::Admin)
    }
}

fn comrade_slug(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[derive(Serialize, Deserialize)]
struct Persist {
    comrades: HashSet<String>,
    caps: Vec<Capability>,
    #[serde(default)]
    next_bilet: u64,
    #[serde(default)]
    pubkeys: HashMap<String, String>,
}

pub struct Authz {
    pub node: KeyPair,
    pub comrades: HashSet<String>,
    pub caps: Vec<Capability>,
    next_bilet: u64,
    path: PathBuf,
    pubkeys: HashMap<String, String>,
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
                next_bilet: p.next_bilet.max(1),
                path,
                pubkeys: p.pubkeys,
            });
        }
        let founder = "founder".to_string();
        let a = Self {
            node: identity.keys.clone(),
            comrades: HashSet::from([founder.clone()]),
            caps: vec![Capability::god(&founder)],
            next_bilet: 1,
            path,
            pubkeys: HashMap::new(),
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
            caps: vec![Capability::god(&founder)],
            next_bilet: 1,
            path: PathBuf::from("authz.json"),
            pubkeys: HashMap::new(),
        }
    }

    fn save(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let p = Persist {
            comrades: self.comrades.clone(),
            caps: self.caps.clone(),
            next_bilet: self.next_bilet,
            pubkeys: self.pubkeys.clone(),
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

    fn mint_bilet(&mut self) -> String {
        let n = self.next_bilet;
        self.next_bilet += 1;
        format!("BIL-{n:06}")
    }

    pub fn check(&self, who: &ComradeId, stmt: &Stmt) -> Result<()> {
        let verb = Verb::of(stmt);
        let now = Self::now();
        let tabl = stmt.table_touch();
        let ok = self
            .caps
            .iter()
            .filter(|c| c.comrade == who.0 || c.comrade == "*")
            .any(|c| c.allows(&verb, now, tabl));
        if !ok {
            return Err(Error::cap_expired());
        }
        for c in self
            .caps
            .iter()
            .filter(|c| c.comrade == who.0 || c.comrade == "*")
        {
            if c.uslov.samokrit && stmt.is_mutation() && stmt.samokrit().is_none() {
                return Err(Error::samokrit_required());
            }
        }
        Ok(())
    }

    pub fn nagrad_god(&mut self, comrade: impl Into<String>) {
        let h = comrade.into();
        self.comrades.insert(h.clone());
        if !self
            .caps
            .iter()
            .any(|c| c.comrade == h && c.predel.is_none())
        {
            self.caps.push(Capability::god(&h));
        }
        let _ = self.save();
    }

    pub fn nagrad(
        &mut self,
        comrade: &str,
        verb: Verb,
        ttl_secs: Option<u64>,
        predel: Option<String>,
    ) -> Result<String> {
        self.comrades.insert(comrade.to_string());
        let srok = if matches!(verb, Verb::Cheka) {
            Some(Self::now() + ttl_secs.unwrap_or(24 * 3600).min(7 * 24 * 3600))
        } else {
            ttl_secs.map(|s| Self::now() + s)
        };
        if let Some(c) = self
            .caps
            .iter_mut()
            .find(|c| c.comrade == comrade && c.predel == predel)
        {
            c.deystv.insert(verb);
            if srok.is_some() {
                c.srok = srok;
            }
            let id = c.bilet.clone();
            self.save()?;
            return Ok(id);
        }
        let bilet = self.mint_bilet();
        let mut deystv = HashSet::new();
        deystv.insert(verb);
        deystv.insert(Verb::Obtan);
        self.caps.push(Capability {
            bilet: bilet.clone(),
            comrade: comrade.to_string(),
            deystv,
            predel,
            nachat: None,
            srok,
            komitet: "FOUNDERS".into(),
            uslov: Uslov::default(),
        });
        self.save()?;
        Ok(bilet)
    }

    pub fn otyat(&mut self, comrade: &str, verb: Verb) -> Result<()> {
        for c in &mut self.caps {
            if c.comrade == comrade {
                c.deystv.remove(&verb);
            }
        }
        self.save()
    }

    pub fn hello(&self, name: &str) -> Result<ComradeId> {
        if self.pubkeys.contains_key(name) {
            return Err(Error::bad_hello());
        }
        if self.comrades.contains(name) || self.caps.iter().any(|c| c.comrade == name) {
            Ok(ComradeId(name.to_string()))
        } else {
            Err(Error::cap_expired())
        }
    }

    pub fn hello_signed(
        &mut self,
        name: &str,
        key: &str,
        podpis: &str,
        nonce: &str,
    ) -> Result<ComradeId> {
        let msg = format!("HELLO|{nonce}|{name}");
        let Some(sig) = oursql_crypto::unhex64(podpis) else {
            return Err(Error::bad_hello());
        };
        if !KeyPair::verify(key, msg.as_bytes(), &sig) {
            return Err(Error::bad_hello());
        }
        self.comrades.insert(name.to_string());
        self.pubkeys.insert(name.to_string(), key.to_string());
        let _ = self.save();
        Ok(ComradeId(name.to_string()))
    }

    pub fn rotate_key(&mut self, comrade: &str, key: &str) -> Result<()> {
        if !self.comrades.contains(comrade) {
            return Err(Error::cap_expired());
        }
        self.pubkeys.insert(comrade.to_string(), key.to_string());
        self.save()
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

    pub fn list_bilets(&self) -> Vec<Capability> {
        self.caps.clone()
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
        a.nagrad("mill", Verb::Cheka, Some(0), None).unwrap();
        let p = parse("CONFISKAT TABL t").unwrap();
        assert!(a.check(&ComradeId("mill".into()), &p.stmts[0]).is_err());
    }

    #[test]
    fn predel_scopes_tabl() {
        let mut a = Authz::open();
        a.nagrad("mill", Verb::Inzrt, None, Some("bolts".into()))
            .unwrap();
        let ok = parse("INZRT V bolts (id) ZNACH ('x')").unwrap();
        a.check(&ComradeId("mill".into()), &ok.stmts[0]).unwrap();
        let no = parse("INZRT V secrets (id) ZNACH ('x')").unwrap();
        assert!(a.check(&ComradeId("mill".into()), &no.stmts[0]).is_err());
    }

    #[test]
    fn bilet_json_uses_nash_names() {
        let mut a = Authz::open();
        let id = a
            .nagrad("mill", Verb::Obtan, None, Some("parts".into()))
            .unwrap();
        assert!(id.starts_with("BIL-"));
        let raw = serde_json::to_string(&a.caps.last().unwrap()).unwrap();
        assert!(raw.contains("\"comrade\""));
        assert!(raw.contains("\"deystv\""));
        assert!(raw.contains("\"predel\""));
        assert!(raw.contains("\"srok\""));
        assert!(raw.contains("\"komitet\""));
        assert!(!raw.contains("\"holder\""));
        assert!(!raw.contains("\"not_after\""));
    }
}
