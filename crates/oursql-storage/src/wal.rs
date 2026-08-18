//! Append-only WAL. Length + CRC + JSON payload. Partial tail is ignored.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use oursql_core::{Column, Error, NarodKey, Result, Value};
use oursql_crypto::crc32;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WalRec {
    Begin {
        tx: u64,
    },
    Commit {
        tx: u64,
        #[serde(default)]
        digest: String,
        #[serde(default)]
        sig: String,
        #[serde(default)]
        signer: String,
    },
    Abort {
        tx: u64,
    },
    CreateTable {
        kollektiv: String,
        name: String,
        cols: Vec<Column>,
    },
    DropTable {
        kollektiv: String,
        name: String,
    },
    Insert {
        kollektiv: String,
        table: String,
        key: NarodKey,
        values: Vec<Value>,
    },
    Update {
        kollektiv: String,
        table: String,
        key: NarodKey,
        values: Vec<Value>,
    },
    Delete {
        kollektiv: String,
        table: String,
        key: NarodKey,
    },
    Confiskat {
        kollektiv: String,
        table: String,
        #[serde(default)]
        until: Option<u64>,
    },
    Osvobod {
        kollektiv: String,
        table: String,
    },
    CreateIndex {
        kollektiv: String,
        table: String,
        name: String,
        col: String,
    },
}

impl WalRec {
    pub fn commit(tx: u64) -> Self {
        Self::Commit {
            tx,
            digest: String::new(),
            sig: String::new(),
            signer: String::new(),
        }
    }

    pub fn commit_signed(tx: u64, digest: impl Into<String>, sig: impl Into<String>) -> Self {
        Self::commit_signed_by(tx, digest, sig, "")
    }

    pub fn commit_signed_by(
        tx: u64,
        digest: impl Into<String>,
        sig: impl Into<String>,
        signer: impl Into<String>,
    ) -> Self {
        Self::Commit {
            tx,
            digest: digest.into(),
            sig: sig.into(),
            signer: signer.into(),
        }
    }
}

pub struct Wal {
    pub path: PathBuf,
    file: File,
    pending: Vec<u8>,
}

impl Wal {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        Ok(Self {
            path,
            file,
            pending: Vec::new(),
        })
    }

    pub fn append(&mut self, rec: &WalRec) -> Result<()> {
        self.queue(rec)?;
        self.flush()
    }

    /// Buffer a record. Call `flush` once per group commit.
    pub fn queue(&mut self, rec: &WalRec) -> Result<()> {
        let payload = serde_json::to_vec(rec).map_err(|e| Error::wal_io(e.to_string()))?;
        let crc = crc32(&payload);
        let len = payload.len() as u32;
        self.pending.extend_from_slice(&len.to_le_bytes());
        self.pending.extend_from_slice(&crc.to_le_bytes());
        self.pending.extend_from_slice(&payload);
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        self.file.write_all(&self.pending)?;
        self.file.sync_all()?;
        self.pending.clear();
        Ok(())
    }

    pub fn recover(path: impl AsRef<Path>) -> Result<Vec<WalRec>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut f = File::open(path)?;
        let mut all = Vec::new();
        f.read_to_end(&mut all)?;
        let mut out = Vec::new();
        let mut i = 0;
        while i + 8 <= all.len() {
            let len = u32::from_le_bytes(all[i..i + 4].try_into().unwrap()) as usize;
            let crc = u32::from_le_bytes(all[i + 4..i + 8].try_into().unwrap());
            i += 8;
            if i + len > all.len() {
                break;
            }
            let payload = &all[i..i + len];
            if crc32(payload) != crc {
                break;
            }
            match serde_json::from_slice::<WalRec>(payload) {
                Ok(rec) => out.push(rec),
                Err(_) => break,
            }
            i += len;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oursql_core::ColumnType;

    fn tmp() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "oursql-wal-{}-{}",
            std::process::id(),
            rand_suffix()
        ));
        p
    }

    fn rand_suffix() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    #[test]
    fn append_and_recover() {
        let path = tmp();
        let _ = std::fs::remove_file(&path);
        {
            let mut w = Wal::open(&path).unwrap();
            w.append(&WalRec::Begin { tx: 1 }).unwrap();
            w.append(&WalRec::CreateTable {
                kollektiv: "sklad".into(),
                name: "bolts".into(),
                cols: vec![Column::new("id", ColumnType::Narodkey)],
            })
            .unwrap();
            w.append(&WalRec::commit(1)).unwrap();
        }
        let recs = Wal::recover(&path).unwrap();
        assert_eq!(recs.len(), 3);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn torn_tail_ignored() {
        let path = tmp();
        let _ = std::fs::remove_file(&path);
        {
            let mut w = Wal::open(&path).unwrap();
            w.append(&WalRec::Begin { tx: 1 }).unwrap();
        }
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            f.write_all(&[10, 0, 0, 0, 1, 2, 3, 4, 9]).unwrap();
        }
        let recs = Wal::recover(&path).unwrap();
        assert_eq!(recs.len(), 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn fuzz_garbage_does_not_panic() {
        let path = tmp();
        std::fs::write(&path, (0u8..=255).cycle().take(4096).collect::<Vec<_>>()).unwrap();
        let _ = Wal::recover(&path).unwrap();
        std::fs::remove_file(&path).ok();
    }
}
