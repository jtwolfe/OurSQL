//! Brigade SOYUZ — the session that binds every other brigade.
//!
//! Bureau never writes pages. Storage never parses NashCQL.

#![deny(unsafe_code)]

pub mod eval;

use std::path::Path;

use oursql_authz::Authz;
use oursql_bureau::Bureau;
use oursql_consensus::LocalMesh;
use oursql_core::{
    Column, ComradeId, CommitKind, Dossier, Error, Intensity, Outcome, Result, Value,
};
use oursql_nashcql::{parse, Expr, SelectItem, Stmt};
use oursql_storage::Sklad;

use crate::eval::{eval, truthy};

pub struct Engine {
    pub sklad: Sklad,
    pub bureau: Bureau,
    pub authz: Authz,
    pub mesh: LocalMesh,
    pub comrade: ComradeId,
    pub dossier: Dossier,
    pub node_name: String,
}

impl Engine {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        Self::open_with(dir, Intensity::default_25(), "founder")
    }

    pub fn open_with(
        dir: impl AsRef<Path>,
        intensity: Intensity,
        comrade: impl Into<String>,
    ) -> Result<Self> {
        let comrade = ComradeId(comrade.into());
        let mut authz = Authz::open();
        authz.nagrad_god(comrade.0.clone());
        let mesh = LocalMesh::new();
        mesh.join("local");
        Ok(Self {
            sklad: Sklad::open(dir)?,
            bureau: Bureau::new(intensity),
            authz,
            mesh,
            comrade,
            dossier: Dossier::new(1),
            node_name: "local".into(),
        })
    }

    pub fn execute(&mut self, sql: &str) -> Result<Outcome> {
        self.bureau.check_ration(&self.comrade)?;
        let parsed = parse(sql)?;
        self.bureau.reject_bourgeois(parsed.bourgeois)?;
        let notice = self.bureau.bourgeois_notice(parsed.bourgeois);
        let mut last = Outcome::empty();
        for stmt in parsed.stmts {
            last = self.exec_stmt(stmt)?;
        }
        if let Some(n) = notice {
            last = last.with_notice(n);
        }
        Ok(last)
    }

    fn exec_stmt(&mut self, stmt: Stmt) -> Result<Outcome> {
        self.authz.check(&self.comrade, &stmt)?;
        self.bureau.require_samokrit(&stmt)?;
        if let Some(d) = self.bureau.review_delay(&stmt) {
            self.bureau.maybe_sleep(d);
        }
        if let Some(t) = stmt.table_touch() {
            if self.sklad.is_held(t)
                && !matches!(stmt, Stmt::Osvobod { .. } | Stmt::PokazTabl | Stmt::Doklad { .. })
            {
                if matches!(
                    stmt,
                    Stmt::Obtan { .. }
                        | Stmt::Inzrt { .. }
                        | Stmt::Opdat { .. }
                        | Stmt::Remov { .. }
                        | Stmt::Ochistka { .. }
                        | Stmt::PerestrojAdd { .. }
                ) {
                    return Err(Error::confiskat());
                }
            }
        }

        match stmt {
            Stmt::Zanim(name) => {
                self.sklad.zanim(&name);
                Ok(Outcome::empty())
            }
            Stmt::ManufakturTabl { name, cols } => {
                let cols: Vec<Column> = cols
                    .into_iter()
                    .filter(|c| c.name != "_solidarity")
                    .collect();
                self.sklad.create_table(&name, cols)?;
                Ok(Outcome::Count { n: 0, notice: None })
            }
            Stmt::UnmakTabl { name } => {
                self.sklad.drop_table(&name)?;
                Ok(Outcome::empty())
            }
            Stmt::Ochistka { name } => {
                let rows = self.sklad.scan(&name)?;
                for r in rows {
                    self.sklad.delete_row(&name, &r.key)?;
                }
                Ok(Outcome::empty())
            }
            Stmt::PerestrojAdd { table, col } => {
                self.sklad.add_column(&table, col)?;
                Ok(Outcome::empty())
            }
            Stmt::Inzrt {
                table,
                cols,
                rows,
                samokrit: _,
            } => {
                let schema = self.sklad.columns(&table)?;
                let mut n = 0u64;
                for row_e in rows {
                    let values = self.project_insert(&schema, cols.as_ref(), row_e)?;
                    self.sklad.insert_row(&table, values)?;
                    n += 1;
                }
                Ok(Outcome::Count { n, notice: None })
            }
            Stmt::Opdat {
                table,
                assigns,
                given,
                ..
            } => {
                let schema = self.sklad.columns(&table)?;
                let rows = self.sklad.scan(&table)?;
                let mut n = 0u64;
                for row in rows {
                    if let Some(g) = &given {
                        if !truthy(&eval(g, &schema, &row.values)?) {
                            continue;
                        }
                    }
                    let mut vals = row.values.clone();
                    for (col, expr) in &assigns {
                        let i = schema
                            .iter()
                            .position(|c| c.name.eq_ignore_ascii_case(col))
                            .ok_or_else(|| Error::unknown_ident(format!("column {col}")))?;
                        let v = eval(expr, &schema, &vals)?;
                        vals[i] = v.coerce(schema[i].ty)?;
                    }
                    self.sklad.update_row(&table, &row.key, vals)?;
                    n += 1;
                }
                Ok(Outcome::Count { n, notice: None })
            }
            Stmt::Remov { table, given, .. } => {
                let schema = self.sklad.columns(&table)?;
                let rows = self.sklad.scan(&table)?;
                let mut n = 0u64;
                for row in rows {
                    if let Some(g) = &given {
                        if !truthy(&eval(g, &schema, &row.values)?) {
                            continue;
                        }
                    }
                    self.sklad.delete_row(&table, &row.key)?;
                    n += 1;
                }
                Ok(Outcome::Count { n, notice: None })
            }
            Stmt::Obtan {
                distinct,
                proj,
                from,
                given,
                lineup,
                ration,
                ochered,
            } => {
                let partial = self.bureau.should_partial(&Stmt::Obtan {
                    distinct,
                    proj: proj.clone(),
                    from: from.clone(),
                    given: given.clone(),
                    lineup: lineup.clone(),
                    ration,
                    ochered,
                });
                let schema = self.sklad.columns(&from)?;
                let mut rows = self.sklad.scan(&from)?;
                if let Some(g) = &given {
                    rows.retain(|r| {
                        eval(g, &schema, &r.values)
                            .map(|v| truthy(&v))
                            .unwrap_or(false)
                    });
                }
                if !lineup.is_empty() {
                    rows.sort_by(|a, b| {
                        for (col, asc) in &lineup {
                            if let Some(i) = schema.iter().position(|c| c.name.eq_ignore_ascii_case(col))
                            {
                                let oa = a.values.get(i).unwrap_or(&Value::Pusto);
                                let ob = b.values.get(i).unwrap_or(&Value::Pusto);
                                if let Some(ord) = oa.cmp_nash(ob) {
                                    if ord != std::cmp::Ordering::Equal {
                                        return if *asc { ord } else { ord.reverse() };
                                    }
                                }
                            }
                        }
                        std::cmp::Ordering::Equal
                    });
                }
                let skip = ochered.unwrap_or(0).max(0) as usize;
                if skip > 0 {
                    rows = rows.into_iter().skip(skip).collect();
                }
                if let Some(lim) = ration {
                    rows.truncate(lim.max(0) as usize);
                }
                if partial && rows.len() > 1 {
                    rows.truncate(rows.len() / 2);
                }

                let mut out_cols: Vec<String> = Vec::new();
                let mut out_rows: Vec<Vec<Value>> = Vec::new();

                let has_agg = proj.iter().any(|p| match p {
                    SelectItem::Expr {
                        expr: Expr::Call { name, .. },
                        ..
                    } => is_agg(name),
                    _ => false,
                });

                if has_agg {
                    for p in &proj {
                        match p {
                            SelectItem::Star => out_cols.push("*".into()),
                            SelectItem::Expr { expr, alias } => {
                                out_cols.push(alias.clone().unwrap_or_else(|| expr_name(expr)));
                            }
                        }
                    }
                    let mut acc = vec![Value::Pusto; proj.len()];
                    let mut count = 0i64;
                    for row in &rows {
                        count += 1;
                        for (i, p) in proj.iter().enumerate() {
                            if let SelectItem::Expr { expr, .. } = p {
                                acc[i] = fold_agg(&acc[i], expr, &schema, &row.values, count)?;
                            }
                        }
                    }
                    if proj.iter().any(|p| matches!(p, SelectItem::Expr { expr: Expr::Call { name, .. }, .. } if name.eq_ignore_ascii_case("SCHET") || name.eq_ignore_ascii_case("COUNT")))
                    {
                        // schet already folded
                    }
                    if rows.is_empty() {
                        for (i, p) in proj.iter().enumerate() {
                            if let SelectItem::Expr {
                                expr: Expr::Call { name, .. },
                                ..
                            } = p
                            {
                                if name.eq_ignore_ascii_case("SCHET")
                                    || name.eq_ignore_ascii_case("COUNT")
                                {
                                    acc[i] = Value::Celiy(0);
                                }
                            }
                        }
                    }
                    out_rows.push(acc);
                } else {
                    for p in &proj {
                        match p {
                            SelectItem::Star => {
                                for c in &schema {
                                    out_cols.push(c.name.clone());
                                }
                            }
                            SelectItem::Expr { expr, alias } => {
                                out_cols.push(alias.clone().unwrap_or_else(|| expr_name(expr)));
                            }
                        }
                    }
                    for row in rows {
                        let mut out = Vec::new();
                        for p in &proj {
                            match p {
                                SelectItem::Star => out.extend(row.values.iter().cloned()),
                                SelectItem::Expr { expr, .. } => {
                                    out.push(eval(expr, &schema, &row.values)?);
                                }
                            }
                        }
                        out_rows.push(out);
                    }
                    if distinct {
                        let mut seen = std::collections::BTreeSet::new();
                        out_rows.retain(|r| {
                            let key: Vec<String> = r.iter().map(|v| v.to_plain()).collect();
                            seen.insert(key)
                        });
                    }
                }

                Ok(Outcome::Rows {
                    columns: out_cols,
                    rows: out_rows,
                    partial,
                    notice: if partial {
                        Some("NOTICE 1902: COLLECTIVE_PARTIAL — retry".into())
                    } else {
                        None
                    },
                })
            }
            Stmt::Nachat => {
                self.sklad.begin()?;
                Ok(Outcome::empty())
            }
            Stmt::Zavershit(kind) => {
                let tx = self.sklad.commit(kind)?;
                if matches!(kind, CommitKind::Soyuz | CommitKind::Cheka) {
                    let _ = self.mesh.certify(&self.node_name, &format!("tx-{tx}"), kind);
                }
                Ok(Outcome::Count {
                    n: tx,
                    notice: None,
                })
            }
            Stmt::Otmena => {
                self.sklad.rollback();
                Ok(Outcome::empty())
            }
            Stmt::Accuse { comrade, note: _ } => {
                let msg = self.bureau.accuse(&self.comrade, &comrade)?;
                Ok(Outcome::empty().with_notice(msg))
            }
            Stmt::Confiskat { table, .. } => {
                if self.bureau.intensity.get() < 25 {
                    return Err(Error::bad_keyword("CONFISKAT needs intensity >= 25"));
                }
                self.sklad.confiskat(&table)?;
                Ok(Outcome::empty().with_notice(format!("CONFISKAT {table}")))
            }
            Stmt::Osvobod { table } => {
                self.sklad.osvobod(&table)?;
                Ok(Outcome::empty())
            }
            Stmt::PokazTabl => {
                let names = self.sklad.list_tables();
                Ok(Outcome::Rows {
                    columns: vec!["tabl".into()],
                    rows: names
                        .into_iter()
                        .map(|n| vec![Value::Tekst(n)])
                        .collect(),
                    partial: false,
                    notice: None,
                })
            }
            Stmt::PokazUstanov => Ok(Outcome::Rows {
                columns: vec!["key".into(), "value".into()],
                rows: vec![
                    vec![
                        Value::Tekst("intensity".into()),
                        Value::Celiy(self.bureau.intensity.get() as i64),
                    ],
                    vec![
                        Value::Tekst("dossier".into()),
                        Value::Tekst(self.dossier.0.clone()),
                    ],
                    vec![
                        Value::Tekst("kollektiv".into()),
                        Value::Tekst(self.sklad.kollektiv.clone()),
                    ],
                ],
                partial: false,
                notice: None,
            }),
            Stmt::Doklad { table } => {
                let cols = self.sklad.columns(&table)?;
                Ok(Outcome::Rows {
                    columns: vec!["name".into(), "type".into(), "narodkey".into()],
                    rows: cols
                        .into_iter()
                        .map(|c| {
                            vec![
                                Value::Tekst(c.name),
                                Value::Tekst(format!("{:?}", c.ty)),
                                Value::Daily(c.narodkey),
                            ]
                        })
                        .collect(),
                    partial: false,
                    notice: None,
                })
            }
            Stmt::Razbor(inner) => Ok(Outcome::Razbor {
                text: format!("{inner:?}"),
            }),
            Stmt::Ustanov { key, value } => {
                if key.to_ascii_lowercase().contains("intensity") {
                    let n: u8 = value.parse().unwrap_or(25);
                    self.bureau.intensity =
                        Intensity::new(n).map_err(|_| Error::intensity_denied())?;
                }
                Ok(Outcome::empty())
            }
        }
    }

    fn project_insert(
        &self,
        schema: &[Column],
        cols: Option<&Vec<String>>,
        exprs: Vec<Expr>,
    ) -> Result<Vec<Value>> {
        let empty = vec![];
        let eval_row = empty.as_slice();
        if let Some(names) = cols {
            if names.len() != exprs.len() {
                return Err(Error::type_fight("column/value count"));
            }
            let mut out = vec![Value::Pusto; schema.len()];
            for (name, e) in names.iter().zip(exprs.into_iter()) {
                let i = schema
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| Error::unknown_ident(format!("column {name}")))?;
                out[i] = eval(&e, schema, eval_row)?;
            }
            Ok(out)
        } else {
            if exprs.len() != schema.len() {
                return Err(Error::type_fight("value count"));
            }
            exprs
                .into_iter()
                .map(|e| eval(&e, schema, eval_row))
                .collect()
        }
    }

    pub fn format_outcome(out: &Outcome) -> String {
        match out {
            Outcome::Empty { notice } => notice.clone().unwrap_or_else(|| "ok".into()),
            Outcome::Count { n, notice } => {
                let mut s = format!("{n} row(s)");
                if let Some(n) = notice {
                    s.push('\n');
                    s.push_str(n);
                }
                s
            }
            Outcome::Razbor { text } => text.clone(),
            Outcome::Rows {
                columns,
                rows,
                partial,
                notice,
            } => {
                let mut s = columns.join(" | ");
                s.push('\n');
                s.push_str(&"-".repeat(s.len().max(3)));
                s.push('\n');
                for r in rows {
                    let line: Vec<String> = r.iter().map(|v| v.to_plain()).collect();
                    s.push_str(&line.join(" | "));
                    s.push('\n');
                }
                if *partial {
                    s.push_str("(partial)\n");
                }
                if let Some(n) = notice {
                    s.push_str(n);
                    s.push('\n');
                }
                s
            }
        }
    }
}

fn is_agg(name: &str) -> bool {
    matches!(
        name.to_ascii_uppercase().as_str(),
        "SCHET" | "COUNT" | "ITOG" | "SUM" | "SREDN" | "AVG" | "NAIMEN" | "MIN" | "NAIBOL" | "MAX"
    )
}

fn expr_name(e: &Expr) -> String {
    match e {
        Expr::Col(c) => c.clone(),
        Expr::Call { name, .. } => name.clone(),
        _ => "?".into(),
    }
}

fn fold_agg(
    acc: &Value,
    expr: &Expr,
    cols: &[Column],
    row: &[Value],
    count: i64,
) -> Result<Value> {
    match expr {
        Expr::Call { name, args } => {
            let up = name.to_ascii_uppercase();
            let v = if args.is_empty() {
                Value::Celiy(1)
            } else {
                eval(&args[0], cols, row)?
            };
            match up.as_str() {
                "SCHET" | "COUNT" => Ok(Value::Celiy(count)),
                "ITOG" | "SUM" => match (acc, &v) {
                    (Value::Pusto, v) => Ok(v.clone()),
                    (Value::Celiy(a), Value::Celiy(b)) => Ok(Value::Celiy(a + b)),
                    _ => Ok(v),
                },
                "NAIMEN" | "MIN" => match acc.cmp_nash(&v) {
                    Some(std::cmp::Ordering::Greater) | None if acc.is_pusto() => Ok(v),
                    Some(std::cmp::Ordering::Greater) => Ok(v),
                    _ => Ok(acc.clone()),
                },
                "NAIBOL" | "MAX" => match acc.cmp_nash(&v) {
                    Some(std::cmp::Ordering::Less) | None if acc.is_pusto() => Ok(v),
                    Some(std::cmp::Ordering::Less) => Ok(v),
                    _ => Ok(acc.clone()),
                },
                "SREDN" | "AVG" => match (acc, &v) {
                    (Value::Pusto, Value::Celiy(b)) => Ok(Value::Drob(*b as f64)),
                    (Value::Drob(a), Value::Celiy(b)) => {
                        let prev = *a * (count - 1) as f64;
                        Ok(Value::Drob((prev + *b as f64) / count as f64))
                    }
                    _ => Ok(v),
                },
                _ => eval(expr, cols, row),
            }
        }
        _ => eval(expr, cols, row),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oursql-eng-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn hello_kollektiv() {
        let dir = tmp();
        let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
        e.execute("ZANIM sklad").unwrap();
        e.execute(
            "MANUFAKTUR TABL bolts (id NARODKEY, plant TEKST NYET PUSTO, qty CELIY NYET PUSTO)",
        )
        .unwrap();
        e.execute(
            "INZRT V bolts (id, plant, qty) ZNACH ('NAR-001', 'brisbane-se', 500) SAMOKRIT 'quota'",
        )
        .unwrap();
        let out = e
            .execute("OBTAN plant, qty IZ bolts GIVEN qty > 0 LINEUP plant RATION 20")
            .unwrap();
        match out {
            Outcome::Rows { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0][1], Value::Celiy(500));
            }
            other => panic!("{other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn crash_recovery() {
        let dir = tmp();
        {
            let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
            e.execute("MANUFAKTUR TABL t (id NARODKEY, n CELIY)")
                .unwrap();
            e.execute("INZRT V t (id, n) ZNACH ('a', 1)").unwrap();
        }
        let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
        let out = e.execute("OBTAN n IZ t").unwrap();
        assert_eq!(out.row_count(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rollback() {
        let dir = tmp();
        let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
        e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
        e.execute("NACHAT").unwrap();
        e.execute("INZRT V t (id) ZNACH ('x')").unwrap();
        e.execute("OTMENA").unwrap();
        let out = e.execute("OBTAN * IZ t").unwrap();
        assert_eq!(out.row_count(), 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn confiskat_blocks() {
        let dir = tmp();
        let mut e = Engine::open_with(&dir, Intensity::default_25(), "founder").unwrap();
        e.bureau.skip_sleep = true;
        e.bureau.ration_burst = 10_000.0;
        e.bureau.ration_qps = 10_000.0;
        e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
        e.execute("CONFISKAT TABL t SAMOKRIT 'audit'").unwrap();
        let err = e.execute("OBTAN * IZ t").unwrap_err();
        assert_eq!(err.code, 1906);
        e.execute("OSVOBOD TABL t").unwrap();
        e.execute("OBTAN * IZ t").unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn gulag_on_spam() {
        let dir = tmp();
        let mut e = Engine::open_with(&dir, Intensity::default_25(), "founder").unwrap();
        e.bureau.skip_sleep = true;
        e.bureau.ration_burst = 2.0;
        e.bureau.ration_qps = 0.0;
        e.execute("POKAZ TABL").unwrap();
        e.execute("POKAZ TABL").unwrap();
        let err = e.execute("POKAZ TABL").unwrap_err();
        assert_eq!(err.code, 1905);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn opdat_and_remov() {
        let dir = tmp();
        let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
        e.execute("MANUFAKTUR TABL t (id NARODKEY, n CELIY)")
            .unwrap();
        e.execute("INZRT V t (id, n) ZNACH ('a', 1), ('b', 2)")
            .unwrap();
        e.execute("OPDAT t NA n = n + 10 GIVEN id = 'a'").unwrap();
        e.execute("REMOV IZ t GIVEN id = 'b'").unwrap();
        let out = e.execute("OBTAN n IZ t").unwrap();
        match out {
            Outcome::Rows { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0][0], Value::Celiy(11));
            }
            other => panic!("{other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
