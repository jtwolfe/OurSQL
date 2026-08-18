//! Brigade AUTHZ.
//!
//! Comrades, komitets, capabilities. No WAL.

#![deny(unsafe_code)]

use std::collections::HashSet;

use oursql_core::{ComradeId, Error, Result};
use oursql_crypto::KeyPair;
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
}

impl Verb {
    pub fn of(stmt: &Stmt) -> Self {
        match stmt {
            Stmt::Obtan { .. } | Stmt::PokazTabl | Stmt::PokazUstanov | Stmt::Doklad { .. } | Stmt::Razbor(_) => {
                Verb::Obtan
            }
            Stmt::Inzrt { .. } => Verb::Inzrt,
            Stmt::Opdat { .. } => Verb::Opdat,
            Stmt::Remov { .. } | Stmt::Ochistka { .. } => Verb::Remov,
            Stmt::ManufakturTabl { .. }
            | Stmt::UnmakTabl { .. }
            | Stmt::PerestrojAdd { .. } => Verb::Ddl,
            Stmt::Confiskat { .. } | Stmt::Osvobod { .. } => Verb::Cheka,
            Stmt::Accuse { .. } => Verb::Accuse,
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
            ]),
            not_after_epoch: None,
        }
    }

    pub fn allows(&self, v: &Verb) -> bool {
        self.verbs.contains(v) || self.verbs.contains(&Verb::Admin)
    }
}

pub struct Authz {
    pub node: KeyPair,
    pub comrades: HashSet<String>,
    pub caps: Vec<Capability>,
}

impl Authz {
    pub fn open() -> Self {
        let node = KeyPair::generate();
        let founder = "founder".to_string();
        Self {
            node,
            comrades: HashSet::from([founder.clone()]),
            caps: vec![Capability::god(founder)],
        }
    }

    pub fn check(&self, who: &ComradeId, stmt: &Stmt) -> Result<()> {
        let verb = Verb::of(stmt);
        let ok = self
            .caps
            .iter()
            .filter(|c| c.holder == who.0 || c.holder == "*" )
            .any(|c| c.allows(&verb));
        if !ok {
            return Err(Error::cap_expired());
        }
        Ok(())
    }

    pub fn nagrad_god(&mut self, holder: impl Into<String>) {
        let h = holder.into();
        self.comrades.insert(h.clone());
        self.caps.push(Capability::god(h));
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
}
