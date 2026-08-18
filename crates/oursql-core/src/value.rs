//! Values, column types, rows. Owned by CORE so WAL and the planner share one shape.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnType {
    Celiy,
    Drob,
    Tekst,
    Daily,
    Narodkey,
}

impl ColumnType {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_uppercase().as_str() {
            "CELIY" | "INT" | "INTEGER" | "BIGINT" => Ok(Self::Celiy),
            "DROB" | "FLOAT" | "DOUBLE" | "REAL" => Ok(Self::Drob),
            "TEKST" | "TEXT" | "VARCHAR" | "STRING" | "BAIT" | "BYTES" | "DOSYE" | "PODPIS" => {
                Ok(Self::Tekst)
            }
            "DAILY" | "BOOL" | "BOOLEAN" => Ok(Self::Daily),
            "NARODKEY" => Ok(Self::Narodkey),
            "MGN" | "TIMESTAMP" => Ok(Self::Celiy),
            other => Err(Error::bad_keyword(format!("unknown type {other}"))),
        }
    }

    pub fn storage(self) -> Self {
        match self {
            ColumnType::Narodkey => ColumnType::Tekst,
            t => t,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub ty: ColumnType,
    pub not_pusto: bool,
    pub narodkey: bool,
    #[serde(default)]
    pub yedinstvo: bool,
    #[serde(default)]
    pub obych: Option<String>,
    #[serde(default)]
    pub solidarity: Option<(String, String)>,
}

impl Column {
    pub fn new(name: impl Into<String>, ty: ColumnType) -> Self {
        let narodkey = matches!(ty, ColumnType::Narodkey);
        Self {
            name: name.into(),
            ty,
            not_pusto: narodkey,
            narodkey,
            yedinstvo: false,
            obych: None,
            solidarity: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Pusto,
    Celiy(i64),
    Drob(f64),
    Tekst(String),
    Daily(bool),
}

impl Value {
    pub fn type_of(&self) -> Option<ColumnType> {
        match self {
            Value::Pusto => None,
            Value::Celiy(_) => Some(ColumnType::Celiy),
            Value::Drob(_) => Some(ColumnType::Drob),
            Value::Tekst(_) => Some(ColumnType::Tekst),
            Value::Daily(_) => Some(ColumnType::Daily),
        }
    }

    pub fn is_pusto(&self) -> bool {
        matches!(self, Value::Pusto)
    }

    pub fn as_celiy(&self) -> Result<i64> {
        match self {
            Value::Celiy(n) => Ok(*n),
            Value::Drob(f) if f.fract() == 0.0 => Ok(*f as i64),
            Value::Tekst(s) => s
                .parse()
                .map_err(|_| Error::type_fight(format!("not celiy: {s}"))),
            _ => Err(Error::type_fight("expected CELIY")),
        }
    }

    pub fn coerce(&self, ty: ColumnType) -> Result<Value> {
        let ty = ty.storage();
        if matches!(self, Value::Pusto) {
            return Ok(Value::Pusto);
        }
        match ty {
            ColumnType::Celiy => Ok(Value::Celiy(self.as_celiy()?)),
            ColumnType::Drob => match self {
                Value::Drob(f) => Ok(Value::Drob(*f)),
                Value::Celiy(n) => Ok(Value::Drob(*n as f64)),
                Value::Tekst(s) => s
                    .parse()
                    .map(Value::Drob)
                    .map_err(|_| Error::type_fight("not drob")),
                _ => Err(Error::type_fight("not drob")),
            },
            ColumnType::Tekst | ColumnType::Narodkey => Ok(Value::Tekst(self.to_plain())),
            ColumnType::Daily => match self {
                Value::Daily(b) => Ok(Value::Daily(*b)),
                Value::Celiy(n) => Ok(Value::Daily(*n != 0)),
                Value::Tekst(s) => match s.to_ascii_uppercase().as_str() {
                    "DA" | "TRUE" | "1" => Ok(Value::Daily(true)),
                    "NYETDA" | "FALSE" | "0" => Ok(Value::Daily(false)),
                    _ => Err(Error::type_fight("not daily")),
                },
                _ => Err(Error::type_fight("not daily")),
            },
        }
    }

    pub fn to_plain(&self) -> String {
        match self {
            Value::Pusto => "PUSTO".into(),
            Value::Celiy(n) => n.to_string(),
            Value::Drob(f) => f.to_string(),
            Value::Tekst(s) => s.clone(),
            Value::Daily(true) => "DA".into(),
            Value::Daily(false) => "NYETDA".into(),
        }
    }

    pub fn cmp_nash(&self, other: &Value) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::*;
        match (self, other) {
            (Value::Pusto, Value::Pusto) => Some(Equal),
            (Value::Pusto, _) => Some(Less),
            (_, Value::Pusto) => Some(Greater),
            (Value::Celiy(a), Value::Celiy(b)) => Some(a.cmp(b)),
            (Value::Celiy(a), Value::Drob(b)) => (*a as f64).partial_cmp(b),
            (Value::Drob(a), Value::Celiy(b)) => a.partial_cmp(&(*b as f64)),
            (Value::Drob(a), Value::Drob(b)) => a.partial_cmp(b),
            (Value::Tekst(a), Value::Tekst(b)) => Some(a.cmp(b)),
            (Value::Daily(a), Value::Daily(b)) => Some(a.cmp(b)),
            (Value::Tekst(a), b) => Some(a.cmp(&b.to_plain())),
            (a, Value::Tekst(b)) => Some(a.to_plain().cmp(b)),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Pusto => write!(f, "PUSTO"),
            Value::Celiy(n) => write!(f, "{n}"),
            Value::Drob(x) => write!(f, "{x}"),
            Value::Tekst(s) => write!(f, "'{}'", s.replace('\'', "''")),
            Value::Daily(true) => write!(f, "DA"),
            Value::Daily(false) => write!(f, "NYETDA"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct NarodKey(pub String);

impl NarodKey {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for NarodKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub key: NarodKey,
    pub values: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coerce_celiy_from_tekst() {
        let v = Value::Tekst("42".into()).coerce(ColumnType::Celiy).unwrap();
        assert_eq!(v, Value::Celiy(42));
    }

    #[test]
    fn pusto_cmp_is_less() {
        assert_eq!(
            Value::Pusto.cmp_nash(&Value::Celiy(0)),
            Some(std::cmp::Ordering::Less)
        );
    }
}
