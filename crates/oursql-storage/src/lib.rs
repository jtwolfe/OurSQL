//! Brigade SKLAD.
//!
//! Pages of truth. WAL. Encrypted checkpoint. Crash recovery.
//! Does not parse NashCQL. Does not know gulag.

#![deny(unsafe_code)]

pub mod btree;
pub mod page;
pub mod sklad;
pub mod wal;

pub use sklad::{Sklad, Table};
pub use wal::{Wal, WalRec};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
