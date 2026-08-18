//! Brigade CORE.
//!
//! Shared land. Every other brigade may depend on this crate.
//! This crate depends on nothing but `serde` (for WAL / wire shapes).
//! It does not open files, parse NashCQL, or apply policy.

#![deny(unsafe_code)]

pub mod error;
pub mod intensity;
pub mod value;

pub use error::{Error, ErrorKind, Result};
pub use intensity::Intensity;
pub use value::{Column, ColumnType, NarodKey, Row, Value};

/// Crate / protocol version string.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Session dossier, always `DOS-` plus zero-padded digits.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Dossier(pub String);

impl Dossier {
    pub fn new(n: u64) -> Self {
        Self(format!("DOS-{n:06}"))
    }
}

impl std::fmt::Display for Dossier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Public identifier for a comrade.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ComradeId(pub String);

impl ComradeId {
    pub fn anonymous() -> Self {
        Self("comrade-anon".into())
    }
}

impl std::fmt::Display for ComradeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Public identifier for a node.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn local() -> Self {
        Self("node-local".into())
    }
}

/// Named database (kollektiv).
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Kollektiv(pub String);

impl Default for Kollektiv {
    fn default() -> Self {
        Self("sklad".into())
    }
}

/// How durable a ZAVERSHIT is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CommitKind {
    Local,
    Soyuz,
    Cheka,
    Inherit,
}

impl CommitKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "LOCAL" => Some(Self::Local),
            "SOYUZ" => Some(Self::Soyuz),
            "CHEKA" => Some(Self::Cheka),
            "INHERIT" | "DEFAULT" => Some(Self::Inherit),
            _ => None,
        }
    }
}

/// Outcome of a statement.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Empty {
        notice: Option<String>,
    },
    Rows {
        columns: Vec<String>,
        rows: Vec<Vec<Value>>,
        partial: bool,
        notice: Option<String>,
    },
    Count {
        n: u64,
        notice: Option<String>,
    },
    Razbor {
        text: String,
    },
}

impl Outcome {
    pub fn empty() -> Self {
        Self::Empty { notice: None }
    }

    pub fn with_notice(mut self, n: impl Into<String>) -> Self {
        let s = n.into();
        match &mut self {
            Outcome::Empty { notice }
            | Outcome::Rows { notice, .. }
            | Outcome::Count { notice, .. } => *notice = Some(s),
            Outcome::Razbor { .. } => {}
        }
        self
    }

    pub fn row_count(&self) -> usize {
        match self {
            Outcome::Rows { rows, .. } => rows.len(),
            Outcome::Count { n, .. } => *n as usize,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dossier_pads() {
        assert_eq!(Dossier::new(7).0, "DOS-000007");
    }
}
