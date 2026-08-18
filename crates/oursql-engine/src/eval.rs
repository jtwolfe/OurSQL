//! Expression evaluator. Lives with the soyuz so storage stays type-dumb.

use oursql_core::{Column, Error, Result, Value};
use oursql_nashcql::{BinOp, Expr, UnaryOp};

pub fn eval(expr: &Expr, cols: &[Column], row: &[Value]) -> Result<Value> {
    match expr {
        Expr::Lit(v) => Ok(v.clone()),
        Expr::Col(name) => {
            if name == "*" {
                return Ok(Value::Celiy(1));
            }
            let i = cols
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| Error::unknown_ident(format!("column {name}")))?;
            Ok(row.get(i).cloned().unwrap_or(Value::Pusto))
        }
        Expr::Unary { op, inner } => {
            let v = eval(inner, cols, row)?;
            match op {
                UnaryOp::Nyet => Ok(Value::Daily(!truthy(&v))),
                UnaryOp::Neg => match v {
                    Value::Celiy(n) => Ok(Value::Celiy(-n)),
                    Value::Drob(f) => Ok(Value::Drob(-f)),
                    _ => Err(Error::type_fight("negation")),
                },
            }
        }
        Expr::Binary { op, left, right } => match op {
            BinOp::I => {
                let l = eval(left, cols, row)?;
                if !truthy(&l) {
                    return Ok(Value::Daily(false));
                }
                let r = eval(right, cols, row)?;
                Ok(Value::Daily(truthy(&r)))
            }
            BinOp::Ili => {
                let l = eval(left, cols, row)?;
                if truthy(&l) {
                    return Ok(Value::Daily(true));
                }
                let r = eval(right, cols, row)?;
                Ok(Value::Daily(truthy(&r)))
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                arith(*op, &eval(left, cols, row)?, &eval(right, cols, row)?)
            }
            cmp => {
                let l = eval(left, cols, row)?;
                let r = eval(right, cols, row)?;
                let ord = l.cmp_nash(&r);
                let b = match (cmp, ord) {
                    (BinOp::Eq, Some(std::cmp::Ordering::Equal)) => true,
                    (BinOp::Ne, Some(o)) => o != std::cmp::Ordering::Equal,
                    (BinOp::Lt, Some(std::cmp::Ordering::Less)) => true,
                    (BinOp::Le, Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)) => true,
                    (BinOp::Gt, Some(std::cmp::Ordering::Greater)) => true,
                    (BinOp::Ge, Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)) => {
                        true
                    }
                    _ => false,
                };
                Ok(Value::Daily(b))
            }
        },
        Expr::IsPusto(inner, yes) => {
            let v = eval(inner, cols, row)?;
            Ok(Value::Daily(v.is_pusto() == *yes))
        }
        Expr::Call { name, args } => {
            let up = name.to_ascii_uppercase();
            match up.as_str() {
                "SCHET" | "COUNT" => Ok(Value::Celiy(1)),
                "ITOG" | "SUM" | "SREDN" | "AVG" | "NAIMEN" | "MIN" | "NAIBOL" | "MAX" => {
                    if args.is_empty() {
                        Ok(Value::Pusto)
                    } else {
                        eval(&args[0], cols, row)
                    }
                }
                _ => Err(Error::unknown_ident(format!("fn {name}"))),
            }
        }
        Expr::Param(_) => Err(Error::bad_grammar("unbound PARAM")),
    }
}

pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Daily(b) => *b,
        Value::Pusto => false,
        Value::Celiy(0) => false,
        Value::Drob(f) if *f == 0.0 => false,
        Value::Tekst(s) if s.is_empty() => false,
        _ => true,
    }
}

fn arith(op: BinOp, l: &Value, r: &Value) -> Result<Value> {
    match (l, r) {
        (Value::Celiy(a), Value::Celiy(b)) => Ok(Value::Celiy(match op {
            BinOp::Add => a.saturating_add(*b),
            BinOp::Sub => a.saturating_sub(*b),
            BinOp::Mul => a.saturating_mul(*b),
            BinOp::Div if *b != 0 => a / b,
            BinOp::Div => return Err(Error::type_fight("division by zero")),
            _ => return Err(Error::type_fight("arith")),
        })),
        (Value::Tekst(a), Value::Tekst(b)) if op == BinOp::Add => {
            Ok(Value::Tekst(format!("{a}{b}")))
        }
        _ => {
            let af = match l {
                Value::Celiy(n) => *n as f64,
                Value::Drob(f) => *f,
                _ => return Err(Error::type_fight("arith")),
            };
            let bf = match r {
                Value::Celiy(n) => *n as f64,
                Value::Drob(f) => *f,
                _ => return Err(Error::type_fight("arith")),
            };
            let f = match op {
                BinOp::Add => af + bf,
                BinOp::Sub => af - bf,
                BinOp::Mul => af * bf,
                BinOp::Div if bf != 0.0 => af / bf,
                BinOp::Div => return Err(Error::type_fight("division by zero")),
                _ => return Err(Error::type_fight("arith")),
            };
            Ok(Value::Drob(f))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oursql_core::ColumnType;

    #[test]
    fn cmp_qty() {
        let cols = vec![Column::new("qty", ColumnType::Celiy)];
        let row = vec![Value::Celiy(5)];
        let e = Expr::Binary {
            op: BinOp::Gt,
            left: Box::new(Expr::Col("qty".into())),
            right: Box::new(Expr::Lit(Value::Celiy(0))),
        };
        assert_eq!(eval(&e, &cols, &row).unwrap(), Value::Daily(true));
    }
}
