//! Brigade NASHCQL.
//!
//! Lexer, keyword table, parser. Produces IR.
//! Does not execute. Does not touch WAL.

#![deny(unsafe_code)]

pub mod ast;
pub mod keywords;
pub mod lex;
pub mod parse;

pub use ast::{BinOp, Expr, Join, SelectItem, Stmt, UnaryOp};
pub use keywords::{KEYWORDS, nash_for_sql, rewrite_bourgeois};
pub use parse::{Parsed, parse};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
