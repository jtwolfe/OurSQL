//! Recursive-descent parser. One IR for NashCQL and rewritten SQL.

use oursql_core::{Column, ColumnType, CommitKind, Error, Result, Value};

use crate::ast::{BinOp, Expr, SelectItem, Stmt, UnaryOp};
use crate::keywords::rewrite_bourgeois;
use crate::lex::{lex, Tok};

pub struct Parsed {
    pub stmts: Vec<Stmt>,
    pub bourgeois: bool,
}

pub fn parse(input: &str) -> Result<Parsed> {
    let (rewritten, bourgeois) = rewrite_bourgeois(input);
    let tokens = lex(&rewritten)?;
    let mut p = Parser { tokens, i: 0 };
    let mut stmts = Vec::new();
    while !p.done() {
        if p.at_semi() {
            p.i += 1;
            continue;
        }
        stmts.push(p.stmt()?);
        if p.at_semi() {
            p.i += 1;
        }
    }
    if stmts.is_empty() {
        return Err(Error::bad_grammar("empty statement"));
    }
    Ok(Parsed { stmts, bourgeois })
}

struct Parser {
    tokens: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn done(&self) -> bool {
        self.i >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.i)
    }

    fn at_semi(&self) -> bool {
        matches!(self.peek(), Some(Tok::Semi))
    }

    fn bump(&mut self) -> Result<Tok> {
        let t = self
            .peek()
            .cloned()
            .ok_or_else(|| Error::bad_grammar("unexpected end"))?;
        self.i += 1;
        Ok(t)
    }

    fn eat_kw(&mut self, want: &str) -> Result<()> {
        match self.peek() {
            Some(Tok::Kw(s)) if s == want => {
                self.i += 1;
                Ok(())
            }
            other => Err(Error::bad_grammar(format!(
                "expected {want}, got {other:?}"
            ))),
        }
    }

    fn eat_ident(&mut self) -> Result<String> {
        match self.bump()? {
            Tok::Ident(s) => Ok(s),
            Tok::Kw(s) => Ok(s.to_ascii_lowercase()),
            Tok::String(s) => Ok(s),
            other => Err(Error::bad_grammar(format!("expected ident, got {other:?}"))),
        }
    }

    fn try_kw(&mut self, want: &str) -> bool {
        match self.peek() {
            Some(Tok::Kw(s)) if s == want => {
                self.i += 1;
                true
            }
            _ => false,
        }
    }

    fn stmt(&mut self) -> Result<Stmt> {
        match self.peek() {
            Some(Tok::Kw(s)) => match s.as_str() {
                "ZANIM" => self.zanim(),
                "MANUFAKTUR" => self.manufaktur(),
                "UNMAK" => self.unmak(),
                "OCHISTKA" => self.ochistka(),
                "PERESTROJ" => self.perestroj(),
                "INZRT" => self.inzrt(),
                "OPDAT" => self.opdat(),
                "REMOV" => self.remov(),
                "OBTAN" => self.obtan(),
                "NACHAT" => {
                    self.i += 1;
                    Ok(Stmt::Nachat)
                }
                "ZAVERSHIT" => self.zavershit(),
                "OTMENA" => {
                    self.i += 1;
                    Ok(Stmt::Otmena)
                }
                "ACCUSE" => self.accuse(),
                "CONFISKAT" => self.confiskat(),
                "OSVOBOD" => self.osvobod(),
                "POKAZ" => self.pokaz(),
                "DOKLAD" => self.doklad(),
                "RAZBOR" => self.razbor(),
                "USTANOV" => self.ustanov(),
                "NAGRAD" => self.nagrad(),
                "OTYAT" => self.otyat(),
                "HELLO" => self.hello(),
                "PETITION" => self.petition(),
                "ZAPOR" => {
                    self.i += 1;
                    let _ = self.try_kw("TABL");
                    Ok(Stmt::Zapor {
                        table: self.eat_ident()?,
                    })
                }
                "OTPUSK" => {
                    self.i += 1;
                    let _ = self.try_kw("TABL");
                    Ok(Stmt::Otpusk {
                        table: self.eat_ident()?,
                    })
                }
                other => Err(Error::bad_keyword(format!(
                    "cannot start a statement: {other}"
                ))),
            },
            other => Err(Error::bad_grammar(format!(
                "expected statement, got {other:?}"
            ))),
        }
    }

    fn zanim(&mut self) -> Result<Stmt> {
        self.eat_kw("ZANIM")?;
        Ok(Stmt::Zanim(self.eat_ident()?))
    }

    fn manufaktur(&mut self) -> Result<Stmt> {
        self.eat_kw("MANUFAKTUR")?;
        if self.try_kw("KOLLEKTIV") {
            return Ok(Stmt::ManufakturKollektiv {
                name: self.eat_ident()?,
            });
        }
        if self.try_kw("OCHERED") {
            return Ok(Stmt::ManufakturOchered {
                name: self.eat_ident()?,
            });
        }
        if self.try_kw("VIZOR") {
            let name = self.eat_ident()?;
            let _ = self.try_kw("KAK");
            let body = match self.bump()? {
                Tok::String(s) => s,
                _ => return Err(Error::bad_grammar("VIZOR wants a string body")),
            };
            return Ok(Stmt::ManufakturVizor { name, body });
        }
        if self.try_kw("SPRAVKA") {
            let name = self.eat_ident()?;
            let _ = self.try_kw("NA");
            let _ = self.try_kw("ON");
            let table = self.eat_ident()?;
            self.expect_lparen()?;
            let col = self.eat_ident()?;
            self.expect_rparen()?;
            return Ok(Stmt::ManufakturSpravka { name, table, col });
        }
        self.eat_kw("TABL")?;
        let name = self.eat_ident()?;
        self.expect_lparen()?;
        let mut cols = Vec::new();
        loop {
            cols.push(self.column_def()?);
            if self.try_tok(&Tok::Comma) {
                continue;
            }
            break;
        }
        self.expect_rparen()?;
        Ok(Stmt::ManufakturTabl { name, cols })
    }

    fn column_def(&mut self) -> Result<Column> {
        if self.try_kw("SOLIDARITY") {
            self.expect_lparen()?;
            let local = self.eat_ident()?;
            self.expect_rparen()?;
            let _ = self.try_kw("IZ");
            let other = self.eat_ident().unwrap_or_default();
            let _ = self.try_tok(&Tok::LParen);
            let ocol = self.eat_ident().unwrap_or_default();
            let _ = self.try_tok(&Tok::RParen);
            return Ok(Column {
                name: local,
                ty: ColumnType::Tekst,
                not_pusto: false,
                narodkey: false,
                yedinstvo: false,
                obych: None,
                solidarity: Some((other, ocol)),
            });
        }
        let name = self.eat_ident()?;
        let mut col = if self.try_kw("NARODKEY") {
            Column::new(name, ColumnType::Narodkey)
        } else {
            let ty_name = match self.bump()? {
                Tok::Kw(s) | Tok::Ident(s) => s,
                other => return Err(Error::bad_grammar(format!("expected type, got {other:?}"))),
            };
            let mut c = Column::new(name, ColumnType::parse(&ty_name)?);
            if self.try_kw("NARODKEY") {
                c.narodkey = true;
                c.not_pusto = true;
            }
            c
        };
        if self.try_kw("NYET") {
            self.eat_kw("PUSTO")?;
            col.not_pusto = true;
        }
        if self.try_kw("YEDINSTVO") {
            col.yedinstvo = true;
        }
        if self.try_kw("OBYCHNO") {
            match self.bump()? {
                Tok::String(s) | Tok::Ident(s) | Tok::Kw(s) => col.obych = Some(s),
                Tok::Int(n) => col.obych = Some(n.to_string()),
                _ => {}
            }
        }
        Ok(col)
    }

    fn unmak(&mut self) -> Result<Stmt> {
        self.eat_kw("UNMAK")?;
        self.eat_kw("TABL")?;
        Ok(Stmt::UnmakTabl {
            name: self.eat_ident()?,
        })
    }

    fn ochistka(&mut self) -> Result<Stmt> {
        self.eat_kw("OCHISTKA")?;
        let _ = self.try_kw("TABL");
        Ok(Stmt::Ochistka {
            name: self.eat_ident()?,
        })
    }

    fn perestroj(&mut self) -> Result<Stmt> {
        self.eat_kw("PERESTROJ")?;
        if self.try_kw("COMRADE") {
            let comrade = self.eat_ident()?;
            let _ = self.try_kw("ROTATE");
            let _ = self.try_kw("KEY");
            return Ok(Stmt::PerestrojRotate {
                comrade,
                key: self.eat_ident()?,
            });
        }
        self.eat_kw("TABL")?;
        let table = self.eat_ident()?;
        self.eat_kw("ADD")?;
        let _ = self.try_kw("COLUMN");
        let col = self.column_def()?;
        Ok(Stmt::PerestrojAdd { table, col })
    }

    fn inzrt(&mut self) -> Result<Stmt> {
        self.eat_kw("INZRT")?;
        self.eat_kw("V")?;
        let table = self.eat_ident()?;
        let cols = if self.try_tok(&Tok::LParen) {
            let mut c = vec![self.eat_ident()?];
            while self.try_tok(&Tok::Comma) {
                c.push(self.eat_ident()?);
            }
            self.expect_rparen()?;
            Some(c)
        } else {
            None
        };
        self.eat_kw("ZNACH")?;
        let mut rows = vec![self.row_vals()?];
        while self.try_tok(&Tok::Comma) {
            rows.push(self.row_vals()?);
        }
        let samokrit = self.samokrit_opt()?;
        Ok(Stmt::Inzrt {
            table,
            cols,
            rows,
            samokrit,
        })
    }

    fn row_vals(&mut self) -> Result<Vec<Expr>> {
        self.expect_lparen()?;
        let mut v = vec![self.expr()?];
        while self.try_tok(&Tok::Comma) {
            v.push(self.expr()?);
        }
        self.expect_rparen()?;
        Ok(v)
    }

    fn opdat(&mut self) -> Result<Stmt> {
        self.eat_kw("OPDAT")?;
        let table = self.eat_ident()?;
        self.eat_kw("NA")?;
        let mut assigns = Vec::new();
        loop {
            let col = self.eat_ident()?;
            self.expect_eq()?;
            let e = self.expr()?;
            assigns.push((col, e));
            if !self.try_tok(&Tok::Comma) {
                break;
            }
        }
        let given = if self.try_kw("GIVEN") {
            Some(self.expr()?)
        } else {
            None
        };
        let samokrit = self.samokrit_opt()?;
        Ok(Stmt::Opdat {
            table,
            assigns,
            given,
            samokrit,
        })
    }

    fn remov(&mut self) -> Result<Stmt> {
        self.eat_kw("REMOV")?;
        let _ = self.try_kw("IZ");
        let table = self.eat_ident()?;
        let given = if self.try_kw("GIVEN") {
            Some(self.expr()?)
        } else {
            None
        };
        let samokrit = self.samokrit_opt()?;
        Ok(Stmt::Remov {
            table,
            given,
            samokrit,
        })
    }

    fn obtan(&mut self) -> Result<Stmt> {
        self.eat_kw("OBTAN")?;
        let distinct = self.try_kw("OTLICH");
        let mut proj = Vec::new();
        if self.try_tok(&Tok::Star) {
            proj.push(SelectItem::Star);
        } else {
            loop {
                let expr = self.expr()?;
                let alias = if self.try_kw("KAK") {
                    Some(self.eat_ident()?)
                } else {
                    None
                };
                proj.push(SelectItem::Expr { expr, alias });
                if !self.try_tok(&Tok::Comma) {
                    break;
                }
            }
        }
        self.eat_kw("IZ")?;
        let from = self.eat_ident()?;
        let join = if self.try_kw("LEVSOYUZ") {
            let table = self.eat_ident()?;
            let _ = self.try_kw("NA");
            let _ = self.try_kw("ON");
            Some(crate::ast::Join {
                table,
                on: self.expr()?,
                left: true,
            })
        } else if self.try_kw("VNUTRSOYUZ") || self.try_kw("SOYUZ") {
            let table = self.eat_ident()?;
            let _ = self.try_kw("NA");
            let _ = self.try_kw("ON");
            Some(crate::ast::Join {
                table,
                on: self.expr()?,
                left: false,
            })
        } else {
            None
        };
        let given = if self.try_kw("GIVEN") {
            Some(self.expr()?)
        } else {
            None
        };
        let mut brigade = Vec::new();
        if self.try_kw("BRIGADE") {
            brigade = self.ident_list()?;
        }
        let priokaz = if self.try_kw("PRIOKAZ") {
            Some(self.expr()?)
        } else {
            None
        };
        let mut lineup = Vec::new();
        if self.try_kw("LINEUP") {
            loop {
                let c = self.eat_ident()?;
                let desc = matches!(self.peek(), Some(Tok::Ident(s) | Tok::Kw(s)) if s.eq_ignore_ascii_case("desc"));
                if desc {
                    self.i += 1;
                }
                lineup.push((c, !desc));
                if !self.try_tok(&Tok::Comma) {
                    break;
                }
            }
        }
        let ration = if self.try_kw("RATION") {
            Some(self.eat_int()?)
        } else {
            None
        };
        let ochered = if self.try_kw("OCHERED") {
            Some(self.eat_int()?)
        } else {
            None
        };
        Ok(Stmt::Obtan {
            distinct,
            proj,
            from,
            join,
            given,
            lineup,
            ration,
            ochered,
            brigade,
            priokaz,
        })
    }

    fn ident_list(&mut self) -> Result<Vec<String>> {
        let mut v = vec![self.eat_ident()?];
        while self.try_tok(&Tok::Comma) {
            v.push(self.eat_ident()?);
        }
        Ok(v)
    }

    fn zavershit(&mut self) -> Result<Stmt> {
        self.eat_kw("ZAVERSHIT")?;
        let kind = if let Some(Tok::Kw(s)) = self.peek() {
            if let Some(k) = CommitKind::parse(s) {
                self.i += 1;
                k
            } else {
                CommitKind::Local
            }
        } else {
            CommitKind::Local
        };
        Ok(Stmt::Zavershit(kind))
    }

    fn accuse(&mut self) -> Result<Stmt> {
        self.eat_kw("ACCUSE")?;
        let _ = self.try_kw("COMRADE");
        let comrade = match self.bump()? {
            Tok::String(s) | Tok::Ident(s) => s,
            Tok::Kw(s) => s,
            other => return Err(Error::bad_grammar(format!("expected comrade, {other:?}"))),
        };
        let _ = self.try_kw("OF");
        let _ = self.try_kw("SPY");
        let note = self.samokrit_opt()?;
        Ok(Stmt::Accuse { comrade, note })
    }

    fn confiskat(&mut self) -> Result<Stmt> {
        self.eat_kw("CONFISKAT")?;
        let _ = self.try_kw("TABL");
        let table = self.eat_ident()?;
        let note = self.samokrit_opt()?;
        Ok(Stmt::Confiskat { table, note })
    }

    fn osvobod(&mut self) -> Result<Stmt> {
        self.eat_kw("OSVOBOD")?;
        let _ = self.try_kw("TABL");
        Ok(Stmt::Osvobod {
            table: self.eat_ident()?,
        })
    }

    fn pokaz(&mut self) -> Result<Stmt> {
        self.eat_kw("POKAZ")?;
        if self.try_kw("TABL") || self.try_kw("TABLES") {
            return Ok(Stmt::PokazTabl);
        }
        if self.try_kw("USTANOV") {
            return Ok(Stmt::PokazUstanov);
        }
        if self.try_kw("AUDIT") {
            return Ok(Stmt::PokazAudit);
        }
        if self.try_kw("COMRADE") {
            return Ok(Stmt::PokazComrade);
        }
        if self.try_kw("BILET") {
            return Ok(Stmt::PokazBilet);
        }
        Ok(Stmt::PokazTabl)
    }

    fn doklad(&mut self) -> Result<Stmt> {
        self.eat_kw("DOKLAD")?;
        let _ = self.try_kw("TABL");
        Ok(Stmt::Doklad {
            table: self.eat_ident()?,
        })
    }

    fn razbor(&mut self) -> Result<Stmt> {
        self.eat_kw("RAZBOR")?;
        Ok(Stmt::Razbor(Box::new(self.stmt()?)))
    }

    fn ustanov(&mut self) -> Result<Stmt> {
        self.eat_kw("USTANOV")?;
        let key = self.eat_ident()?;
        self.expect_eq()?;
        let value = match self.bump()? {
            Tok::Int(n) => n.to_string(),
            Tok::String(s) | Tok::Ident(s) | Tok::Kw(s) => s,
            Tok::Float(f) => f.to_string(),
            other => return Err(Error::bad_grammar(format!("bad ustanov {other:?}"))),
        };
        Ok(Stmt::Ustanov { key, value })
    }

    fn hello(&mut self) -> Result<Stmt> {
        self.eat_kw("HELLO")?;
        let _ = self.try_kw("COMRADE");
        let comrade = self.eat_ident()?;
        let mut key = None;
        let mut podpis = None;
        if self.try_kw("KEY") {
            key = Some(self.eat_ident()?);
        }
        if self.try_kw("PODPIS") {
            podpis = Some(self.eat_ident()?);
        }
        Ok(Stmt::Hello {
            comrade,
            key,
            podpis,
        })
    }

    fn petition(&mut self) -> Result<Stmt> {
        self.eat_kw("PETITION")?;
        let verb = match self.bump()? {
            Tok::Kw(s) | Tok::Ident(s) => s,
            other => return Err(Error::bad_grammar(format!("expected verb, {other:?}"))),
        };
        let note = self.samokrit_opt()?;
        Ok(Stmt::Petition { verb, note })
    }

    fn nagrad(&mut self) -> Result<Stmt> {
        self.eat_kw("NAGRAD")?;
        let verb = match self.bump()? {
            Tok::Kw(s) | Tok::Ident(s) => s,
            other => return Err(Error::bad_grammar(format!("expected verb, {other:?}"))),
        };
        let _ = self.try_kw("NA");
        let _ = self.try_kw("COMRADE");
        let comrade = self.eat_ident()?;
        let mut predel = None;
        let mut ttl = None;
        loop {
            if self.try_kw("PREDEL") {
                predel = Some(self.eat_ident()?);
                continue;
            }
            if self.try_kw("SROK") {
                ttl = Some(self.eat_int()? as u64);
                continue;
            }
            if matches!(self.peek(), Some(Tok::Int(_))) {
                ttl = Some(self.eat_int()? as u64);
                continue;
            }
            break;
        }
        Ok(Stmt::Nagrad {
            verb,
            comrade,
            ttl,
            predel,
        })
    }

    fn otyat(&mut self) -> Result<Stmt> {
        self.eat_kw("OTYAT")?;
        let verb = match self.bump()? {
            Tok::Kw(s) | Tok::Ident(s) => s,
            other => return Err(Error::bad_grammar(format!("expected verb, {other:?}"))),
        };
        let _ = self.try_kw("IZ");
        let _ = self.try_kw("COMRADE");
        Ok(Stmt::Otyat {
            verb,
            comrade: self.eat_ident()?,
        })
    }

    fn samokrit_opt(&mut self) -> Result<Option<String>> {
        if self.try_kw("SAMOKRIT") {
            match self.bump()? {
                Tok::String(s) => Ok(Some(s)),
                other => Err(Error::bad_grammar(format!(
                    "SAMOKRIT wants string, {other:?}"
                ))),
            }
        } else {
            Ok(None)
        }
    }

    fn eat_int(&mut self) -> Result<i64> {
        match self.bump()? {
            Tok::Int(n) => Ok(n),
            other => Err(Error::bad_grammar(format!("expected int, {other:?}"))),
        }
    }

    fn expect_lparen(&mut self) -> Result<()> {
        match self.bump()? {
            Tok::LParen => Ok(()),
            other => Err(Error::bad_grammar(format!("expected (, got {other:?}"))),
        }
    }
    fn expect_rparen(&mut self) -> Result<()> {
        match self.bump()? {
            Tok::RParen => Ok(()),
            other => Err(Error::bad_grammar(format!("expected ), got {other:?}"))),
        }
    }
    fn expect_eq(&mut self) -> Result<()> {
        match self.bump()? {
            Tok::Eq => Ok(()),
            other => Err(Error::bad_grammar(format!("expected =, got {other:?}"))),
        }
    }
    fn try_tok(&mut self, want: &Tok) -> bool {
        if self.peek() == Some(want) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn expr(&mut self) -> Result<Expr> {
        self.expr_or()
    }

    fn expr_or(&mut self) -> Result<Expr> {
        let mut e = self.expr_and()?;
        while self.try_kw("ILI") {
            let r = self.expr_and()?;
            e = Expr::Binary {
                op: BinOp::Ili,
                left: Box::new(e),
                right: Box::new(r),
            };
        }
        Ok(e)
    }

    fn expr_and(&mut self) -> Result<Expr> {
        let mut e = self.expr_cmp()?;
        while self.try_kw("I") {
            let r = self.expr_cmp()?;
            e = Expr::Binary {
                op: BinOp::I,
                left: Box::new(e),
                right: Box::new(r),
            };
        }
        Ok(e)
    }

    fn expr_cmp(&mut self) -> Result<Expr> {
        let e = self.expr_add()?;
        if self.try_kw("PUSTO") && matches!(&e, Expr::Col(_)) {
            // rare
        }
        if self.try_kw("LI") {
            let neg = self.try_kw("NYET");
            self.eat_kw("PUSTO")?;
            return Ok(Expr::IsPusto(Box::new(e), !neg));
        }
        let op = match self.peek() {
            Some(Tok::Eq) => Some(BinOp::Eq),
            Some(Tok::Ne) => Some(BinOp::Ne),
            Some(Tok::Lt) => Some(BinOp::Lt),
            Some(Tok::Le) => Some(BinOp::Le),
            Some(Tok::Gt) => Some(BinOp::Gt),
            Some(Tok::Ge) => Some(BinOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.i += 1;
            let r = self.expr_add()?;
            return Ok(Expr::Binary {
                op,
                left: Box::new(e),
                right: Box::new(r),
            });
        }
        Ok(e)
    }

    fn expr_add(&mut self) -> Result<Expr> {
        let mut e = self.expr_mul()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Plus) => BinOp::Add,
                Some(Tok::Minus) => BinOp::Sub,
                _ => break,
            };
            self.i += 1;
            let r = self.expr_mul()?;
            e = Expr::Binary {
                op,
                left: Box::new(e),
                right: Box::new(r),
            };
        }
        Ok(e)
    }

    fn expr_mul(&mut self) -> Result<Expr> {
        let mut e = self.expr_un()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Star) => BinOp::Mul,
                Some(Tok::Slash) => BinOp::Div,
                _ => break,
            };
            self.i += 1;
            let r = self.expr_un()?;
            e = Expr::Binary {
                op,
                left: Box::new(e),
                right: Box::new(r),
            };
        }
        Ok(e)
    }

    fn expr_un(&mut self) -> Result<Expr> {
        if self.try_kw("NYET") {
            return Ok(Expr::Unary {
                op: UnaryOp::Nyet,
                inner: Box::new(self.expr_un()?),
            });
        }
        if self.try_tok(&Tok::Minus) {
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                inner: Box::new(self.expr_un()?),
            });
        }
        self.expr_prim()
    }

    fn expr_prim(&mut self) -> Result<Expr> {
        match self.bump()? {
            Tok::Int(n) => Ok(Expr::Lit(Value::Celiy(n))),
            Tok::Float(f) => Ok(Expr::Lit(Value::Drob(f))),
            Tok::String(s) => Ok(Expr::Lit(Value::Tekst(s))),
            Tok::Kw(s) if s == "DA" => Ok(Expr::Lit(Value::Daily(true))),
            Tok::Kw(s) if s == "NYETDA" => Ok(Expr::Lit(Value::Daily(false))),
            Tok::Kw(s) if s == "PUSTO" => Ok(Expr::Lit(Value::Pusto)),
            Tok::Star => Ok(Expr::Col("*".into())),
            Tok::Ident(s) | Tok::Kw(s) => {
                if self.try_tok(&Tok::LParen) {
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Tok::RParen)) {
                        if self.try_tok(&Tok::Star) {
                            args.push(Expr::Col("*".into()));
                        } else {
                            args.push(self.expr()?);
                            while self.try_tok(&Tok::Comma) {
                                args.push(self.expr()?);
                            }
                        }
                    }
                    self.expect_rparen()?;
                    Ok(Expr::Call { name: s, args })
                } else {
                    Ok(Expr::Col(s))
                }
            }
            Tok::LParen => {
                let e = self.expr()?;
                self.expect_rparen()?;
                Ok(e)
            }
            other => Err(Error::bad_grammar(format!("bad expr {other:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hello() {
        let p = parse(
            "MANUFAKTUR TABL bolts (id NARODKEY, qty CELIY NYET PUSTO); INZRT V bolts (id, qty) ZNACH ('NAR-001', 500) SAMOKRIT 'quota';",
        )
        .unwrap();
        assert_eq!(p.stmts.len(), 2);
        assert!(matches!(p.stmts[0], Stmt::ManufakturTabl { .. }));
    }

    #[test]
    fn parse_obtan_given() {
        let p = parse("OBTAN plant, qty IZ bolts GIVEN qty > 0 LINEUP plant RATION 20").unwrap();
        match &p.stmts[0] {
            Stmt::Obtan { ration, .. } => assert_eq!(*ration, Some(20)),
            _ => panic!("not obtan"),
        }
    }

    #[test]
    fn parse_bourgeois_select() {
        let p = parse("SELECT name FROM bolts WHERE qty > 0").unwrap();
        assert!(p.bourgeois);
        assert!(matches!(p.stmts[0], Stmt::Obtan { .. }));
    }

    #[test]
    fn parse_perestroj() {
        let p = parse("PERESTROJ TABL bolts ADD COLUMN note TEKST").unwrap();
        assert!(matches!(p.stmts[0], Stmt::PerestrojAdd { .. }));
    }

    #[test]
    fn parse_nagrad() {
        let p = parse("NAGRAD OBTAN NA COMRADE mill").unwrap();
        assert!(matches!(p.stmts[0], Stmt::Nagrad { .. }));
    }

    #[test]
    fn fuzz_parser_does_not_panic() {
        for i in 0u32..200 {
            let s: String = (0..32)
                .map(|j| char::from(((i * 17 + j * 13) % 95) as u8 + 32))
                .collect();
            let _ = parse(&s);
        }
    }
}
