//! Shared types for OurSQL (NashCQL).
//!
//! This crate is the contract the rest of the workspace is not allowed
//! to violate: intensity range, stable error codes, identifiers.

#![deny(unsafe_code)]

pub mod error;
pub mod intensity;

pub use error::{Error, ErrorKind};
pub use intensity::Intensity;

/// Crate / protocol version string.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Session dossier, always `DOS-` plus decimal digits.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Dossier(pub String);

impl Dossier {
    pub fn new(n: u64) -> Self {
        Self(format!("DOS-{n:06}"))
    }
}

/// Public identifier for a comrade (hex of the Ed25519 key, later).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComradeId(pub String);

/// Public identifier for a node.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);
