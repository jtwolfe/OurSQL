//! 16 KiB pages. Checksum over plaintext. Encryption is a separate pass.

use oursql_core::{Error, Result};
use oursql_crypto::digest;

pub const PAGE_SIZE: usize = 16 * 1024;
pub const PAGE_MAGIC: &[u8; 8] = b"OURLPG01";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageType {
    Meta = 1,
    Leaf = 2,
    Branch = 5,
    Overflow = 3,
    Undo = 6,
    Freelist = 4,
}

impl PageType {
    fn from_u8(v: u8) -> Result<Self> {
        match v {
            1 => Ok(Self::Meta),
            2 => Ok(Self::Leaf),
            3 => Ok(Self::Overflow),
            4 => Ok(Self::Freelist),
            5 => Ok(Self::Branch),
            6 => Ok(Self::Undo),
            _ => Err(Error::page_checksum()),
        }
    }
}

/// Plaintext page: header + payload, always PAGE_SIZE.
pub fn pack(ty: PageType, id: u32, payload: &[u8]) -> Result<[u8; PAGE_SIZE]> {
    if payload.len() > PAGE_SIZE - 48 {
        return Err(Error::wal_io("page payload too large"));
    }
    let mut page = [0u8; PAGE_SIZE];
    page[0] = ty as u8;
    page[1..5].copy_from_slice(&id.to_le_bytes());
    page[5..7].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    page[48..48 + payload.len()].copy_from_slice(payload);
    let sum = digest(&page[48..]);
    page[16..48].copy_from_slice(&sum);
    Ok(page)
}

pub fn unpack(page: &[u8]) -> Result<(PageType, u32, &[u8])> {
    if page.len() != PAGE_SIZE {
        return Err(Error::page_checksum());
    }
    let ty = PageType::from_u8(page[0])?;
    let id = u32::from_le_bytes(page[1..5].try_into().unwrap());
    let used = u16::from_le_bytes(page[5..7].try_into().unwrap()) as usize;
    if 48 + used > PAGE_SIZE {
        return Err(Error::page_checksum());
    }
    let want = digest(&page[48..]);
    if want != page[16..48] {
        return Err(Error::page_checksum());
    }
    Ok((ty, id, &page[48..48 + used]))
}

/// Encrypted checkpoint file: magic + page_count + sealed pages.
pub fn write_checkpoint(
    path: impl AsRef<std::path::Path>,
    key: &[u8; 32],
    pages: &[[u8; PAGE_SIZE]],
) -> Result<()> {
    use std::io::Write;
    let path = path.as_ref();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::File::create(path)?;
    f.write_all(PAGE_MAGIC)?;
    f.write_all(&(pages.len() as u32).to_le_bytes())?;
    for p in pages {
        let sealed = oursql_crypto::seal(key, p)?;
        let len = sealed.len() as u32;
        f.write_all(&len.to_le_bytes())?;
        f.write_all(&sealed)?;
    }
    f.sync_all()?;
    Ok(())
}

pub fn read_checkpoint(
    path: impl AsRef<std::path::Path>,
    key: &[u8; 32],
) -> Result<Vec<[u8; PAGE_SIZE]>> {
    use std::io::Read;
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut f = std::fs::File::open(path)?;
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic)?;
    if &magic != PAGE_MAGIC {
        return Err(Error::recovery_failed("bad page magic"));
    }
    let mut nbuf = [0u8; 4];
    f.read_exact(&mut nbuf)?;
    let n = u32::from_le_bytes(nbuf) as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        f.read_exact(&mut nbuf)?;
        let len = u32::from_le_bytes(nbuf) as usize;
        let mut sealed = vec![0u8; len];
        f.read_exact(&mut sealed)?;
        let plain = oursql_crypto::open(key, &sealed)?;
        if plain.len() != PAGE_SIZE {
            return Err(Error::page_checksum());
        }
        let mut page = [0u8; PAGE_SIZE];
        page.copy_from_slice(&plain);
        let _ = unpack(&page)?;
        out.push(page);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_roundtrip() {
        let p = pack(PageType::Leaf, 3, b"hello").unwrap();
        let (ty, id, pay) = unpack(&p).unwrap();
        assert_eq!(ty, PageType::Leaf);
        assert_eq!(id, 3);
        assert_eq!(pay, b"hello");
    }

    #[test]
    fn corrupt_checksum() {
        let mut p = pack(PageType::Leaf, 1, b"x").unwrap();
        p[50] ^= 1;
        assert!(unpack(&p).is_err());
    }
}
