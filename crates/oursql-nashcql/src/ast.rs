//! NashCQL IR. Shared by planner and executor.

use oursql_core::{Column, CommitKind, Value};

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Zanim(String),
    ManufakturTabl {
        name: String,
        cols: Vec<Column>,
    },
    UnmakTabl {
        name: String,
    },
    ManufakturSpravka {
        name: String,
        table: String,
        col: String,
    },
    Ochistka {
        name: String,
    },
    PerestrojAdd {
        table: String,
        col: Column,
    },
    Inzrt {
        table: String,
        cols: Option<Vec<String>>,
        rows: Vec<Vec<Expr>>,
        samokrit: Option<String>,
    },
    Opdat {
        table: String,
        assigns: Vec<(String, Expr)>,
        given: Option<Expr>,
        samokrit: Option<String>,
    },
    Remov {
        table: String,
        given: Option<Expr>,
        samokrit: Option<String>,
    },
    Obtan {
        distinct: bool,
        proj: Vec<SelectItem>,
        from: String,
        join: Option<Join>,
        given: Option<Expr>,
        lineup: Vec<(String, bool)>,
        ration: Option<i64>,
        ochered: Option<i64>,
    },
    Nachat,
    Zavershit(CommitKind),
    Otmena,
    Accuse {
        comrade: String,
        note: Option<String>,
    },
    Confiskat {
        table: String,
        note: Option<String>,
    },
    Osvobod {
        table: String,
    },
    PokazTabl,
    PokazUstanov,
    PokazAudit,
    PokazComrade,
    PokazBilet,
    Doklad {
        table: String,
    },
    Razbor(Box<Stmt>),
    Ustanov {
        key: String,
        value: String,
    },
    Hello {
        comrade: String,
    },
    Nagrad {
        verb: String,
        comrade: String,
        ttl: Option<u64>,
        predel: Option<String>,
    },
    Otyat {
        verb: String,
        comrade: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Join {
    pub table: String,
    pub on: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectItem {
    Star,
    Expr { expr: Expr, alias: Option<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Lit(Value),
    Col(String),
    Unary {
        op: UnaryOp,
        inner: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    IsPusto(Box<Expr>, bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Nyet,
    Neg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    I,
    Ili,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
}

impl Stmt {
    pub fn table_touch(&self) -> Option<&str> {
        match self {
            Stmt::ManufakturTabl { name, .. }
            | Stmt::UnmakTabl { name }
            | Stmt::Ochistka { name }
            | Stmt::PerestrojAdd { table: name, .. }
            | Stmt::Inzrt { table: name, .. }
            | Stmt::Opdat { table: name, .. }
            | Stmt::Remov { table: name, .. }
            | Stmt::Obtan { from: name, .. }
            | Stmt::ManufakturSpravka { table: name, .. }
            | Stmt::Confiskat { table: name, .. }
            | Stmt::Osvobod { table: name }
            | Stmt::Doklad { table: name } => Some(name),
            _ => None,
        }
    }

    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Stmt::ManufakturTabl { .. }
                | Stmt::UnmakTabl { .. }
                | Stmt::Ochistka { .. }
                | Stmt::PerestrojAdd { .. }
                | Stmt::Inzrt { .. }
                | Stmt::Opdat { .. }
                | Stmt::Remov { .. }
                | Stmt::ManufakturSpravka { .. }
                | Stmt::Nagrad { .. }
                | Stmt::Otyat { .. }
        )
    }

    pub fn is_ddl(&self) -> bool {
        matches!(
            self,
            Stmt::ManufakturTabl { .. }
                | Stmt::UnmakTabl { .. }
                | Stmt::PerestrojAdd { .. }
                | Stmt::Ochistka { .. }
                | Stmt::ManufakturSpravka { .. }
        )
    }

    pub fn samokrit(&self) -> Option<&str> {
        match self {
            Stmt::Inzrt { samokrit, .. }
            | Stmt::Opdat { samokrit, .. }
            | Stmt::Remov { samokrit, .. } => samokrit.as_deref(),
            _ => None,
        }
    }
}
