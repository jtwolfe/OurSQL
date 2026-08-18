//! SKLAD — in-memory tabls with durable WAL + encrypted 16KiB checkpoint.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use oursql_core::{Column, CommitKind, Error, Kollektiv, NarodKey, Result, Row, Value};
use oursql_crypto::NodeIdentity;
use serde::{Deserialize, Serialize};

use crate::btree::PagePool;
use crate::page::{pack, read_checkpoint, unpack, PageType, PAGE_SIZE};
use crate::wal::{Wal, WalRec};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub cols: Vec<Column>,
    pub rows: BTreeMap<NarodKey, Vec<Value>>,
    #[serde(default)]
    pub indexes: Vec<(String, String)>,
}

impl Table {
    pub fn pk_index(&self) -> Result<usize> {
        self.cols
            .iter()
            .position(|c| c.narodkey)
            .ok_or_else(Error::no_narodkey)
    }

    pub fn col_index(&self, name: &str) -> Result<usize> {
        self.cols
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| Error::unknown_ident(format!("column {name}")))
    }
}

#[derive(Clone, Debug, Default)]
struct Scratch {
    tables_created: Vec<(String, String, Vec<Column>)>,
    tables_dropped: Vec<(String, String)>,
    inserts: Vec<(String, String, NarodKey, Vec<Value>)>,
    updates: Vec<(String, String, NarodKey, Vec<Value>)>,
    deletes: Vec<(String, String, NarodKey)>,
    indexes: Vec<(String, String, String, String)>,
}

#[derive(Serialize, Deserialize)]
struct Snap {
    tables: Vec<((String, String), Table)>,
    holds: Vec<((String, String), Option<u64>)>,
    next_tx: u64,
    next_narod: u64,
    last_seq: u64,
}

pub struct Sklad {
    dir: PathBuf,
    wal: Wal,
    pub kollektiv: String,
    tables: BTreeMap<(String, String), Table>,
    holds: BTreeMap<(String, String), Option<u64>>,
    pub last_sig: Option<(String, String)>,
    pub schema_epoch: u64,
    pub perestroj_wait: bool,
    pub audit_path: PathBuf,
    locks: BTreeSet<String>,
    sequences: BTreeMap<String, u64>,
    views: BTreeMap<String, String>,
    pool: Option<PagePool>,
    next_tx: u64,
    next_narod: u64,
    in_tx: bool,
    scratch: Scratch,
    pub identity: NodeIdentity,
    pub last_commit_recs: Vec<WalRec>,
    pub last_seq: u64,
}

impl Sklad {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        let identity = NodeIdentity::load_or_create(dir.join("node.key"))?;
        let wal_path = if dir.join("wal.log").exists() {
            dir.join("wal.log")
        } else {
            dir.join("wal").join("000000.log")
        };
        let audit_path = dir
            .join("kollektiv")
            .join("sklad")
            .join("audit")
            .join("audit.log");
        if let Some(d) = audit_path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tree_path = dir
            .join("kollektiv")
            .join("sklad")
            .join("pages")
            .join("tree.pg");
        let mut data_key = identity.storage_key;
        // wrapped kollektiv key lives next to pages
        let wrap_path = dir.join("kollektiv").join("sklad").join("meta");
        if wrap_path.exists() {
            if let Ok(blob) = std::fs::read(&wrap_path) {
                if let Ok(plain) = oursql_crypto::open(&identity.storage_key, &blob) {
                    if plain.len() == 32 {
                        data_key.copy_from_slice(&plain);
                    }
                }
            }
        } else {
            let _ = std::fs::create_dir_all(wrap_path.parent().unwrap());
            if let Ok(wrapped) = oursql_crypto::seal(&identity.storage_key, &data_key) {
                let _ = std::fs::write(&wrap_path, wrapped);
            }
        }
        let pool = PagePool::open(&tree_path, &identity.storage_key)?
            .or_else(|| PagePool::create(&tree_path, &identity.storage_key, data_key).ok());
        let mut s = Self {
            wal: Wal::open(wal_path)?,
            dir,
            kollektiv: Kollektiv::default().0,
            tables: BTreeMap::new(),
            holds: BTreeMap::new(),
            next_tx: 1,
            next_narod: 1,
            in_tx: false,
            scratch: Scratch::default(),
            identity,
            last_commit_recs: Vec::new(),
            last_seq: 0,
            last_sig: None,
            schema_epoch: 1,
            perestroj_wait: false,
            audit_path,
            locks: BTreeSet::new(),
            sequences: BTreeMap::new(),
            views: BTreeMap::new(),
            pool,
        };
        s.load_checkpoint()?;
        let recs = Wal::recover(&s.wal.path)?;
        s.replay(&recs)?;
        Ok(s)
    }

    pub fn data_dir(&self) -> &Path {
        &self.dir
    }

    fn checkpoint_path(&self) -> PathBuf {
        self.dir
            .join("kollektiv")
            .join("pages")
            .join("checkpoint.pg")
    }

    fn load_checkpoint(&mut self) -> Result<()> {
        let pages = read_checkpoint(self.checkpoint_path(), &self.identity.storage_key)?;
        if pages.is_empty() {
            return Ok(());
        }
        let mut blob = Vec::new();
        for p in &pages {
            let (_, _, pay) = unpack(p)?;
            blob.extend_from_slice(pay);
        }
        let snap: Snap =
            serde_json::from_slice(&blob).map_err(|e| Error::recovery_failed(e.to_string()))?;
        self.tables = snap.tables.into_iter().collect();
        self.holds = snap.holds.into_iter().collect();
        if let Some(pool) = &self.pool {
            if let Ok(pairs) = pool.scan_all() {
                if !pairs.is_empty() && self.tables.is_empty() {
                    // pager is source if snapshot json was empty
                }
            }
        }
        self.next_tx = snap.next_tx;
        self.next_narod = snap.next_narod;
        self.last_seq = snap.last_seq;
        Ok(())
    }

    pub fn checkpoint(&mut self) -> Result<()> {
        let snap = Snap {
            tables: self
                .tables
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            holds: self.holds.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            next_tx: self.next_tx,
            next_narod: self.next_narod,
            last_seq: self.last_seq,
        };
        let json = serde_json::to_vec(&snap).map_err(|e| Error::wal_io(e.to_string()))?;
        let chunk = PAGE_SIZE - 48;
        let mut pages = Vec::new();
        if json.is_empty() {
            pages.push(pack(PageType::Meta, 0, &[])?);
        } else {
            for (i, part) in json.chunks(chunk).enumerate() {
                let ty = if i == 0 {
                    PageType::Meta
                } else {
                    PageType::Overflow
                };
                pages.push(pack(ty, i as u32, part)?);
            }
        }
        // Also rebuild the live B+tree so OBTAN can walk pages.
        if let Some(pool) = self.pool.as_mut() {
            let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
            pairs.push((b"__snap".to_vec(), json.clone()));
            for ((k, n), t) in &self.tables {
                for (key, values) in &t.rows {
                    let kk = format!("r:{k}/{n}/{}", key.0).into_bytes();
                    if let Ok(vv) = serde_json::to_vec(values) {
                        pairs.push((kk, vv));
                    }
                }
            }
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            let _ = pool.rebuild_pub(&pairs);
            let _ = pool.flush();
        }
        crate::page::write_checkpoint(self.checkpoint_path(), &self.identity.storage_key, &pages)
    }

    fn replay(&mut self, recs: &[WalRec]) -> Result<()> {
        let mut open: Option<u64> = None;
        let mut buf: Vec<&WalRec> = Vec::new();
        for rec in recs {
            match rec {
                WalRec::Begin { tx } => {
                    open = Some(*tx);
                    buf.clear();
                }
                WalRec::Commit { tx, .. } => {
                    if open == Some(*tx) {
                        let apply = buf.clone();
                        for r in apply {
                            self.apply_committed(r)?;
                        }
                    }
                    open = None;
                    buf.clear();
                    self.next_tx = self.next_tx.max(*tx + 1);
                }
                WalRec::Abort { .. } => {
                    open = None;
                    buf.clear();
                }
                other => {
                    if open.is_some() {
                        buf.push(other);
                    } else {
                        self.apply_committed(other)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn apply_committed(&mut self, rec: &WalRec) -> Result<()> {
        match rec {
            WalRec::CreateTable {
                kollektiv,
                name,
                cols,
            } => {
                self.tables.insert(
                    (kollektiv.clone(), name.clone()),
                    Table {
                        name: name.clone(),
                        cols: cols.clone(),
                        rows: BTreeMap::new(),
                        indexes: Vec::new(),
                    },
                );
            }
            WalRec::DropTable { kollektiv, name } => {
                self.tables.remove(&(kollektiv.clone(), name.clone()));
                self.holds.remove(&(kollektiv.clone(), name.clone()));
            }
            WalRec::Insert {
                kollektiv,
                table,
                key,
                values,
            } => {
                if let Some(t) = self.tables.get_mut(&(kollektiv.clone(), table.clone())) {
                    if let Some(old) = t.rows.get(key) {
                        if old != values {
                            return Err(Error::line_conflict());
                        }
                    }
                    t.rows.insert(key.clone(), values.clone());
                    self.bump_narod(key);
                }
            }
            WalRec::Update {
                kollektiv,
                table,
                key,
                values,
            } => {
                if let Some(t) = self.tables.get_mut(&(kollektiv.clone(), table.clone())) {
                    t.rows.insert(key.clone(), values.clone());
                }
            }
            WalRec::Delete {
                kollektiv,
                table,
                key,
            } => {
                if let Some(t) = self.tables.get_mut(&(kollektiv.clone(), table.clone())) {
                    t.rows.remove(key);
                }
            }
            WalRec::Confiskat {
                kollektiv,
                table,
                until,
            } => {
                self.holds
                    .insert((kollektiv.clone(), table.clone()), *until);
            }
            WalRec::Osvobod { kollektiv, table } => {
                self.holds.remove(&(kollektiv.clone(), table.clone()));
            }
            WalRec::Commit { digest, sig, .. } => {
                if !digest.is_empty() {
                    self.last_sig = Some((digest.clone(), sig.clone()));
                }
            }
            WalRec::CreateIndex {
                kollektiv,
                table,
                name,
                col,
            } => {
                if let Some(t) = self.tables.get_mut(&(kollektiv.clone(), table.clone())) {
                    if !t.indexes.iter().any(|(n, _)| n == name) {
                        t.indexes.push((name.clone(), col.clone()));
                    }
                }
            }
            WalRec::Begin { .. } | WalRec::Abort { .. } => {}
        }
        Ok(())
    }

    /// Apply a remote certified batch. Idempotent.
    pub fn apply_remote(&mut self, recs: &[WalRec]) -> Result<()> {
        for r in recs {
            if matches!(
                r,
                WalRec::Begin { .. } | WalRec::Commit { .. } | WalRec::Abort { .. }
            ) {
                continue;
            }
            self.apply_committed(r)?;
            self.last_seq += 1;
        }
        self.checkpoint()?;
        Ok(())
    }

    pub fn export_snapshot(&self) -> Vec<WalRec> {
        let mut out = Vec::new();
        for ((k, n), t) in &self.tables {
            out.push(WalRec::CreateTable {
                kollektiv: k.clone(),
                name: n.clone(),
                cols: t.cols.clone(),
            });
            for (idx, col) in &t.indexes {
                out.push(WalRec::CreateIndex {
                    kollektiv: k.clone(),
                    table: n.clone(),
                    name: idx.clone(),
                    col: col.clone(),
                });
            }
            for (key, values) in &t.rows {
                out.push(WalRec::Insert {
                    kollektiv: k.clone(),
                    table: n.clone(),
                    key: key.clone(),
                    values: values.clone(),
                });
            }
        }
        for ((k, n), until) in &self.holds {
            out.push(WalRec::Confiskat {
                kollektiv: k.clone(),
                table: n.clone(),
                until: *until,
            });
        }
        out
    }

    fn bump_narod(&mut self, key: &NarodKey) {
        if let Some(rest) = key.0.strip_prefix("NAR-") {
            if let Ok(n) = rest.parse::<u64>() {
                self.next_narod = self.next_narod.max(n + 1);
            }
        }
    }

    pub fn begin(&mut self) -> Result<()> {
        if self.in_tx {
            return Err(Error::bad_grammar("already in NACHAT"));
        }
        self.in_tx = true;
        self.scratch = Scratch::default();
        Ok(())
    }

    pub fn commit(&mut self, _kind: CommitKind) -> Result<u64> {
        let tx = self.next_tx;
        self.next_tx += 1;
        let mut recs = vec![WalRec::Begin { tx }];
        let scratch = std::mem::take(&mut self.scratch);
        for (k, n, cols) in &scratch.tables_created {
            recs.push(WalRec::CreateTable {
                kollektiv: k.clone(),
                name: n.clone(),
                cols: cols.clone(),
            });
        }
        for (k, n) in &scratch.tables_dropped {
            recs.push(WalRec::DropTable {
                kollektiv: k.clone(),
                name: n.clone(),
            });
        }
        for (k, t, name, col) in &scratch.indexes {
            recs.push(WalRec::CreateIndex {
                kollektiv: k.clone(),
                table: t.clone(),
                name: name.clone(),
                col: col.clone(),
            });
        }
        for (k, t, key, values) in &scratch.inserts {
            recs.push(WalRec::Insert {
                kollektiv: k.clone(),
                table: t.clone(),
                key: key.clone(),
                values: values.clone(),
            });
        }
        for (k, t, key, values) in &scratch.updates {
            recs.push(WalRec::Update {
                kollektiv: k.clone(),
                table: t.clone(),
                key: key.clone(),
                values: values.clone(),
            });
        }
        for (k, t, key) in &scratch.deletes {
            recs.push(WalRec::Delete {
                kollektiv: k.clone(),
                table: t.clone(),
                key: key.clone(),
            });
        }
        let (digest, sig) = self
            .last_sig
            .clone()
            .unwrap_or_else(|| (String::new(), String::new()));
        recs.push(WalRec::commit_signed(tx, digest, sig));
        for r in &recs {
            self.wal.queue(r)?;
        }
        self.wal.flush()?;
        for r in recs.iter().filter(|r| {
            !matches!(
                r,
                WalRec::Begin { .. } | WalRec::Commit { .. } | WalRec::Abort { .. }
            )
        }) {
            self.apply_committed(r)?;
        }
        self.last_commit_recs = recs;
        self.last_seq += 1;
        self.in_tx = false;
        self.scratch = Scratch::default();
        if self.last_seq % 8 == 0 {
            self.checkpoint()?;
        }
        Ok(tx)
    }

    pub fn rollback(&mut self) {
        self.in_tx = false;
        self.scratch = Scratch::default();
    }

    pub fn zanim(&mut self, name: &str) {
        self.kollektiv = name.to_string();
    }

    pub fn create_table(&mut self, name: &str, cols: Vec<Column>) -> Result<()> {
        if !cols.iter().any(|c| c.narodkey) {
            return Err(Error::no_narodkey());
        }
        let key = (self.kollektiv.clone(), name.to_string());
        if self.tables.contains_key(&key)
            || self
                .scratch
                .tables_created
                .iter()
                .any(|(k, n, _)| k == &self.kollektiv && n == name)
        {
            return Err(Error::bad_grammar(format!("tabl {name} already exists")));
        }
        if self.in_tx {
            self.scratch
                .tables_created
                .push((self.kollektiv.clone(), name.to_string(), cols));
        } else {
            self.begin()?;
            self.scratch
                .tables_created
                .push((self.kollektiv.clone(), name.to_string(), cols));
            self.commit(CommitKind::Local)?;
        }
        Ok(())
    }

    pub fn create_index(&mut self, table: &str, name: &str, col: &str) -> Result<()> {
        let cols = self.columns(table)?;
        if !cols.iter().any(|c| c.name.eq_ignore_ascii_case(col)) {
            return Err(Error::unknown_ident(format!("column {col}")));
        }
        if self.in_tx {
            self.scratch.indexes.push((
                self.kollektiv.clone(),
                table.to_string(),
                name.to_string(),
                col.to_string(),
            ));
        } else {
            self.begin()?;
            self.scratch.indexes.push((
                self.kollektiv.clone(),
                table.to_string(),
                name.to_string(),
                col.to_string(),
            ));
            self.commit(CommitKind::Local)?;
        }
        Ok(())
    }

    pub fn lookup_index(
        &self,
        table: &str,
        col: &str,
        value: &Value,
    ) -> Result<Option<Vec<NarodKey>>> {
        let t = self.table(table)?;
        if !t.indexes.iter().any(|(_, c)| c.eq_ignore_ascii_case(col)) {
            return Ok(None);
        }
        let i = t.col_index(col)?;
        let mut keys = Vec::new();
        for (k, vals) in &t.rows {
            if vals.get(i) == Some(value) {
                keys.push(k.clone());
            }
        }
        Ok(Some(keys))
    }

    pub fn drop_table(&mut self, name: &str) -> Result<()> {
        self.table(name)?;
        if self.in_tx {
            self.scratch
                .tables_dropped
                .push((self.kollektiv.clone(), name.to_string()));
        } else {
            self.begin()?;
            self.scratch
                .tables_dropped
                .push((self.kollektiv.clone(), name.to_string()));
            self.commit(CommitKind::Local)?;
        }
        Ok(())
    }

    pub fn table(&self, name: &str) -> Result<&Table> {
        if self
            .scratch
            .tables_dropped
            .iter()
            .any(|(k, n)| k == &self.kollektiv && n == name)
        {
            return Err(Error::unknown_ident(format!("tabl {name}")));
        }
        self.tables
            .get(&(self.kollektiv.clone(), name.to_string()))
            .ok_or_else(|| Error::unknown_ident(format!("tabl {name}")))
    }

    fn table_mut_live(&mut self, name: &str) -> Result<&mut Table> {
        let k = self.kollektiv.clone();
        self.tables
            .get_mut(&(k, name.to_string()))
            .ok_or_else(|| Error::unknown_ident(format!("tabl {name}")))
    }

    pub fn is_held(&self, name: &str) -> bool {
        match self.holds.get(&(self.kollektiv.clone(), name.to_string())) {
            None => false,
            Some(None) => true,
            Some(Some(until)) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                now < *until
            }
        }
    }

    pub fn confiskat(&mut self, name: &str) -> Result<()> {
        self.confiskat_until(name, None)
    }

    pub fn confiskat_until(&mut self, name: &str, until: Option<u64>) -> Result<()> {
        self.table(name)?;
        self.wal.append(&WalRec::Confiskat {
            kollektiv: self.kollektiv.clone(),
            table: name.to_string(),
            until,
        })?;
        self.holds
            .insert((self.kollektiv.clone(), name.to_string()), until);
        Ok(())
    }

    pub fn osvobod(&mut self, name: &str) -> Result<()> {
        self.wal.append(&WalRec::Osvobod {
            kollektiv: self.kollektiv.clone(),
            table: name.to_string(),
        })?;
        self.holds
            .remove(&(self.kollektiv.clone(), name.to_string()));
        Ok(())
    }

    pub fn next_key(&mut self) -> NarodKey {
        let k = NarodKey(format!("NAR-{:06}", self.next_narod));
        self.next_narod += 1;
        k
    }

    pub fn insert_row(&mut self, table: &str, values: Vec<Value>) -> Result<NarodKey> {
        let cols = if let Ok(t) = self.table(table) {
            t.cols.clone()
        } else if let Some((_, _, cols)) = self
            .scratch
            .tables_created
            .iter()
            .find(|(k, n, _)| k == &self.kollektiv && n == table)
        {
            cols.clone()
        } else {
            return Err(Error::unknown_ident(format!("tabl {table}")));
        };
        if values.len() != cols.len() {
            return Err(Error::type_fight(format!(
                "expected {} values, got {}",
                cols.len(),
                values.len()
            )));
        }
        let mut coerced = Vec::with_capacity(values.len());
        let mut key: Option<NarodKey> = None;
        for (c, v) in cols.iter().zip(values.into_iter()) {
            let mut v = v.coerce(c.ty)?;
            if c.narodkey && v.is_pusto() {
                v = Value::Tekst(self.next_key().0);
            }
            if v.is_pusto() {
                if let Some(d) = &c.obych {
                    v = Value::Tekst(d.clone())
                        .coerce(c.ty)
                        .unwrap_or(Value::Tekst(d.clone()));
                }
            }
            if c.not_pusto && v.is_pusto() {
                return Err(Error::pusto_banned(format!("{} may not be PUSTO", c.name)));
            }
            if c.narodkey {
                key = Some(NarodKey(v.to_plain()));
            }
            coerced.push(v);
        }
        let key = key.ok_or_else(Error::no_narodkey)?;
        if let Ok(t) = self.table(table) {
            for (i, c) in t.cols.iter().enumerate() {
                if c.yedinstvo {
                    let v = &coerced[i];
                    for (k, vals) in &t.rows {
                        if k != &key && vals.get(i) == Some(v) {
                            return Err(Error::type_fight(format!("YEDINSTVO {} broken", c.name)));
                        }
                    }
                }
                if let Some((ot, oc)) = &c.solidarity {
                    if let Ok(other) = self.table(ot) {
                        if let Ok(oi) = other.col_index(oc) {
                            let v = &coerced[i];
                            if !v.is_pusto() && !other.rows.values().any(|r| r.get(oi) == Some(v)) {
                                return Err(Error::fk_fight(format!(
                                    "SOLIDARITY {} -> {}.{} missing",
                                    c.name, ot, oc
                                )));
                            }
                        }
                    }
                }
            }
        }
        if self.in_tx {
            self.scratch.inserts.push((
                self.kollektiv.clone(),
                table.to_string(),
                key.clone(),
                coerced,
            ));
        } else {
            self.begin()?;
            self.scratch.inserts.push((
                self.kollektiv.clone(),
                table.to_string(),
                key.clone(),
                coerced,
            ));
            self.commit(CommitKind::Local)?;
        }
        Ok(key)
    }

    pub fn scan(&self, table: &str) -> Result<Vec<Row>> {
        let t = self.table(table)?;
        let mut rows: Vec<Row> = t
            .rows
            .iter()
            .map(|(k, v)| Row {
                key: k.clone(),
                values: v.clone(),
            })
            .collect();
        for (k, tname, key, values) in &self.scratch.inserts {
            if k == &self.kollektiv && tname == table {
                rows.retain(|r| &r.key != key);
                rows.push(Row {
                    key: key.clone(),
                    values: values.clone(),
                });
            }
        }
        for (k, tname, key, values) in &self.scratch.updates {
            if k == &self.kollektiv && tname == table {
                if let Some(r) = rows.iter_mut().find(|r| &r.key == key) {
                    r.values = values.clone();
                }
            }
        }
        for (k, tname, key) in &self.scratch.deletes {
            if k == &self.kollektiv && tname == table {
                rows.retain(|r| &r.key != key);
            }
        }
        rows.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(rows)
    }

    pub fn columns(&self, table: &str) -> Result<Vec<Column>> {
        if let Some((_, _, cols)) = self
            .scratch
            .tables_created
            .iter()
            .find(|(k, n, _)| k == &self.kollektiv && n == table)
        {
            return Ok(cols.clone());
        }
        Ok(self.table(table)?.cols.clone())
    }

    pub fn update_row(&mut self, table: &str, key: &NarodKey, values: Vec<Value>) -> Result<()> {
        if self.in_tx {
            self.scratch.updates.push((
                self.kollektiv.clone(),
                table.to_string(),
                key.clone(),
                values,
            ));
        } else {
            self.begin()?;
            self.scratch.updates.push((
                self.kollektiv.clone(),
                table.to_string(),
                key.clone(),
                values,
            ));
            self.commit(CommitKind::Local)?;
        }
        Ok(())
    }

    pub fn delete_row(&mut self, table: &str, key: &NarodKey) -> Result<()> {
        if self.in_tx {
            self.scratch
                .deletes
                .push((self.kollektiv.clone(), table.to_string(), key.clone()));
        } else {
            self.begin()?;
            self.scratch
                .deletes
                .push((self.kollektiv.clone(), table.to_string(), key.clone()));
            self.commit(CommitKind::Local)?;
        }
        Ok(())
    }

    pub fn list_tables(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .tables
            .iter()
            .filter(|((k, _), _)| k == &self.kollektiv)
            .map(|((_, n), _)| n.clone())
            .collect();
        for (k, n, _) in &self.scratch.tables_created {
            if k == &self.kollektiv && !names.contains(n) {
                names.push(n.clone());
            }
        }
        names.sort();
        names
    }

    pub fn add_column(&mut self, table: &str, col: Column) -> Result<()> {
        let t = self.table_mut_live(table)?;
        if t.cols.iter().any(|c| c.name == col.name) {
            return Err(Error::bad_grammar(format!("column {} exists", col.name)));
        }
        t.cols.push(col);
        for row in t.rows.values_mut() {
            row.push(Value::Pusto);
        }
        let t = self.table(table)?.clone();
        let k = self.kollektiv.clone();
        let standalone = !self.in_tx;
        if standalone {
            self.begin()?;
        }
        self.scratch
            .tables_dropped
            .push((k.clone(), table.to_string()));
        self.scratch
            .tables_created
            .push((k.clone(), table.to_string(), t.cols.clone()));
        for (key, values) in t.rows {
            self.scratch
                .inserts
                .push((k.clone(), table.to_string(), key, values));
        }
        if standalone {
            self.commit(CommitKind::Local)?;
        }
        Ok(())
    }

    pub fn scan_pages(&self) -> Result<Vec<(String, String)>> {
        let Some(pool) = &self.pool else {
            return Ok(Vec::new());
        };
        Ok(pool
            .scan_all()?
            .into_iter()
            .map(|(k, v)| {
                (
                    String::from_utf8_lossy(&k).into_owned(),
                    String::from_utf8_lossy(&v).into_owned(),
                )
            })
            .collect())
    }

    pub fn zapor(&mut self, table: &str) -> Result<()> {
        self.table(table)?;
        self.locks.insert(table.to_string());
        Ok(())
    }

    pub fn otpusk(&mut self, table: &str) {
        self.locks.remove(table);
    }

    pub fn is_locked(&self, table: &str) -> bool {
        self.locks.contains(table)
    }

    pub fn next_ochered(&mut self, name: &str) -> u64 {
        let e = self.sequences.entry(name.to_string()).or_insert(0);
        *e += 1;
        *e
    }

    pub fn put_vizor(&mut self, name: &str, body: &str) {
        self.views.insert(name.to_string(), body.to_string());
    }

    pub fn vizor(&self, name: &str) -> Option<&str> {
        self.views.get(name).map(|s| s.as_str())
    }

    pub fn append_audit(&self, line: &str) -> Result<()> {
        use std::io::Write;
        if let Some(d) = self.audit_path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.audit_path)?;
        writeln!(f, "{line}")?;
        Ok(())
    }

    pub fn read_audit(&self) -> Vec<String> {
        std::fs::read_to_string(&self.audit_path)
            .unwrap_or_default()
            .lines()
            .map(|s| s.to_string())
            .collect()
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use oursql_core::ColumnType;

    fn tmpdir() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oursql-sklad-{}-{}",
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
    fn insert_survives_reopen() {
        let dir = tmpdir();
        {
            let mut s = Sklad::open(&dir).unwrap();
            s.create_table(
                "bolts",
                vec![Column::new("id", ColumnType::Narodkey), {
                    let mut c = Column::new("qty", ColumnType::Celiy);
                    c.not_pusto = true;
                    c
                }],
            )
            .unwrap();
            s.insert_row(
                "bolts",
                vec![Value::Tekst("NAR-001".into()), Value::Celiy(7)],
            )
            .unwrap();
            s.checkpoint().unwrap();
        }
        let s = Sklad::open(&dir).unwrap();
        let rows = s.scan("bolts").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].values[1], Value::Celiy(7));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rollback_discards() {
        let dir = tmpdir();
        let mut s = Sklad::open(&dir).unwrap();
        s.create_table("t", vec![Column::new("id", ColumnType::Narodkey)])
            .unwrap();
        s.begin().unwrap();
        s.insert_row("t", vec![Value::Tekst("x".into())]).unwrap();
        s.rollback();
        assert!(s.scan("t").unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn checkpoint_is_encrypted() {
        let dir = tmpdir();
        let mut s = Sklad::open(&dir).unwrap();
        s.create_table(
            "t",
            vec![
                Column::new("id", ColumnType::Narodkey),
                Column::new("secret", ColumnType::Tekst),
            ],
        )
        .unwrap();
        s.insert_row(
            "t",
            vec![
                Value::Tekst("k".into()),
                Value::Tekst("brisbane-se-secret".into()),
            ],
        )
        .unwrap();
        s.checkpoint().unwrap();
        let raw = std::fs::read(s.checkpoint_path()).unwrap();
        assert!(!raw.windows(10).any(|w| w == b"brisbane-s"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn node_key_is_0600() {
        let dir = tmpdir();
        let _ = Sklad::open(&dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("node.key"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
