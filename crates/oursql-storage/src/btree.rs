//! B+tree pager. Leaf / branch / overflow / freelist. Buffer pool.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use oursql_core::{Error, Result};
use oursql_crypto::digest;

use crate::page::{pack, unpack, PageType, PAGE_SIZE};

const HDR: usize = 16;
const POOL_MAX: usize = 256;

/// One in-memory page.
#[derive(Clone)]
struct Cached {
    raw: [u8; PAGE_SIZE],
    dirty: bool,
}

pub struct PagePool {
    path: PathBuf,
    key: [u8; 32],
    pages: HashMap<u32, Cached>,
    dirty: HashSet<u32>,
    pub next_id: u32,
    pub root: u32,
    pub page_size: usize,
    pub data_key: [u8; 32],
}

impl PagePool {
    pub fn create(
        path: impl AsRef<Path>,
        storage_key: &[u8; 32],
        data_key: [u8; 32],
    ) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let mut p = Self {
            path,
            key: *storage_key,
            pages: HashMap::new(),
            dirty: HashSet::new(),
            next_id: 2,
            root: 1,
            page_size: PAGE_SIZE,
            data_key,
        };
        p.put(0, PageType::Meta, &p.meta_blob()?)?;
        p.put(1, PageType::Leaf, &leaf_empty(0))?;
        p.flush()?;
        Ok(p)
    }

    pub fn open(path: impl AsRef<Path>, storage_key: &[u8; 32]) -> Result<Option<Self>> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Ok(None);
        }
        let pages = crate::page::read_checkpoint(&path, storage_key)?;
        if pages.is_empty() {
            return Ok(None);
        }
        let mut map = HashMap::new();
        let mut next_id = 1u32;
        let mut root = 1u32;
        let mut data_key = *storage_key;
        for raw in pages {
            let (ty, id, pay) = unpack(&raw)?;
            if ty == PageType::Meta && id == 0 && pay.len() >= 40 {
                root = u32::from_le_bytes(pay[0..4].try_into().unwrap());
                next_id = u32::from_le_bytes(pay[4..8].try_into().unwrap());
                if pay.len() >= 40 {
                    data_key.copy_from_slice(&pay[8..40]);
                }
            }
            next_id = next_id.max(id + 1);
            map.insert(id, Cached { raw, dirty: false });
        }
        Ok(Some(Self {
            path,
            key: *storage_key,
            pages: map,
            dirty: HashSet::new(),
            next_id,
            root,
            page_size: PAGE_SIZE,
            data_key,
        }))
    }

    fn meta_blob(&self) -> Result<Vec<u8>> {
        let mut b = Vec::with_capacity(40);
        b.extend_from_slice(&self.root.to_le_bytes());
        b.extend_from_slice(&self.next_id.to_le_bytes());
        b.extend_from_slice(&self.data_key);
        Ok(b)
    }

    fn evict_if_needed(&mut self) -> Result<()> {
        if self.pages.len() < POOL_MAX {
            return Ok(());
        }
        if let Some((&id, _)) = self.pages.iter().find(|(i, c)| !c.dirty && **i != 0) {
            self.pages.remove(&id);
            return Ok(());
        }
        self.flush()?;
        if self.pages.len() >= POOL_MAX {
            return Err(Error::pool_exhausted());
        }
        Ok(())
    }

    fn put(&mut self, id: u32, ty: PageType, payload: &[u8]) -> Result<()> {
        self.evict_if_needed()?;
        let raw = pack(ty, id, payload)?;
        self.pages.insert(id, Cached { raw, dirty: true });
        self.dirty.insert(id);
        Ok(())
    }

    fn get(&self, id: u32) -> Result<(PageType, Vec<u8>)> {
        let c = self
            .pages
            .get(&id)
            .ok_or_else(|| Error::recovery_failed(format!("missing page {id}")))?;
        let (ty, _, pay) = unpack(&c.raw)?;
        Ok((ty, pay.to_vec()))
    }

    pub fn flush(&mut self) -> Result<()> {
        if let Ok(blob) = self.meta_blob() {
            let raw = pack(PageType::Meta, 0, &blob)?;
            self.pages.insert(0, Cached { raw, dirty: true });
            self.dirty.insert(0);
        }
        let mut ordered: Vec<[u8; PAGE_SIZE]> = Vec::new();
        let mut ids: Vec<u32> = self.pages.keys().copied().collect();
        ids.sort();
        for id in ids {
            if let Some(c) = self.pages.get(&id) {
                ordered.push(c.raw);
            }
        }
        crate::page::write_checkpoint(&self.path, &self.key, &ordered)?;
        self.dirty.clear();
        for c in self.pages.values_mut() {
            c.dirty = false;
        }
        Ok(())
    }

    pub fn insert(&mut self, key: &[u8], val: &[u8]) -> Result<()> {
        let path = self.descend_path(key)?;
        let leaf = *path.last().unwrap();
        let (_, pay) = self.get(leaf)?;
        let mut pairs = decode_pairs(&pay);
        pairs.retain(|(k, _)| k.as_slice() != key);
        pairs.push((key.to_vec(), val.to_vec()));
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let encoded = encode_pairs(&pairs);
        if encoded.len() <= PAGE_SIZE - 48 && pairs.len() <= 24 {
            self.put(leaf, PageType::Leaf, &encoded)?;
            self.fix_sep(&path, leaf, &pairs[0].0)?;
            return Ok(());
        }
        self.split_leaf(&path, pairs)
    }

    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        let path = self.descend_path(key)?;
        let leaf = *path.last().unwrap();
        let (_, pay) = self.get(leaf)?;
        let mut pairs = decode_pairs(&pay);
        let n = pairs.len();
        pairs.retain(|(k, _)| k.as_slice() != key);
        if pairs.len() == n {
            return Ok(());
        }
        self.put(leaf, PageType::Leaf, &encode_pairs(&pairs))?;
        if !pairs.is_empty() {
            self.fix_sep(&path, leaf, &pairs[0].0)?;
        }
        Ok(())
    }

    pub fn get_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let path = self.descend_path(key)?;
        let leaf = *path.last().unwrap();
        let (_, pay) = self.get(leaf)?;
        for (k, v) in decode_pairs(&pay) {
            if k == key {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    fn descend_path(&self, key: &[u8]) -> Result<Vec<u32>> {
        let mut path = vec![self.root];
        loop {
            let id = *path.last().unwrap();
            let (ty, pay) = self.get(id)?;
            match ty {
                PageType::Leaf | PageType::Overflow => return Ok(path),
                PageType::Branch => {
                    let ch = decode_children_kv(&pay);
                    if ch.is_empty() {
                        return Err(Error::recovery_failed("empty branch"));
                    }
                    path.push(choose_child(&ch, key));
                }
                _ => return Err(Error::recovery_failed("bad page in descend")),
            }
        }
    }

    fn fix_sep(&mut self, path: &[u32], child: u32, min: &[u8]) -> Result<()> {
        if path.len() < 2 {
            return Ok(());
        }
        let parent = path[path.len() - 2];
        let (ty, pay) = self.get(parent)?;
        if ty != PageType::Branch {
            return Ok(());
        }
        let mut ch = decode_children_kv(&pay);
        for (id, sep) in &mut ch {
            if *id == child {
                *sep = min.to_vec();
            }
        }
        self.put(parent, PageType::Branch, &encode_children(&ch))
    }

    fn split_leaf(&mut self, path: &[u32], pairs: Vec<(Vec<u8>, Vec<u8>)>) -> Result<()> {
        let leaf = *path.last().unwrap();
        let mid = pairs.len() / 2;
        let (left, right) = pairs.split_at(mid.max(1));
        if right.is_empty() {
            self.put(leaf, PageType::Leaf, &encode_pairs(left))?;
            return Ok(());
        }
        self.put(leaf, PageType::Leaf, &encode_pairs(left))?;
        let new_id = self.next_id;
        self.next_id += 1;
        self.put(new_id, PageType::Leaf, &encode_pairs(right))?;
        self.insert_child(
            &path[..path.len() - 1],
            leaf,
            &left[0].0,
            new_id,
            &right[0].0,
        )
    }

    fn insert_child(
        &mut self,
        ancestors: &[u32],
        left_id: u32,
        left_min: &[u8],
        right_id: u32,
        right_min: &[u8],
    ) -> Result<()> {
        if ancestors.is_empty() {
            let root = self.next_id;
            self.next_id += 1;
            let kids = vec![(left_id, left_min.to_vec()), (right_id, right_min.to_vec())];
            self.put(root, PageType::Branch, &encode_children(&kids))?;
            self.root = root;
            return Ok(());
        }
        let parent = *ancestors.last().unwrap();
        let (_, pay) = self.get(parent)?;
        let mut ch = decode_children_kv(&pay);
        if let Some((_, sep)) = ch.iter_mut().find(|(id, _)| *id == left_id) {
            *sep = left_min.to_vec();
        }
        ch.push((right_id, right_min.to_vec()));
        ch.sort_by(|a, b| a.1.cmp(&b.1));
        let encoded = encode_children(&ch);
        if encoded.len() <= PAGE_SIZE - 48 && ch.len() <= 24 {
            return self.put(parent, PageType::Branch, &encoded);
        }
        let mid = ch.len() / 2;
        let (l, r) = ch.split_at(mid.max(1));
        if r.is_empty() {
            return self.put(parent, PageType::Branch, &encode_children(l));
        }
        self.put(parent, PageType::Branch, &encode_children(l))?;
        let new_id = self.next_id;
        self.next_id += 1;
        self.put(new_id, PageType::Branch, &encode_children(r))?;
        self.insert_child(
            &ancestors[..ancestors.len() - 1],
            parent,
            &l[0].1,
            new_id,
            &r[0].1,
        )
    }

    /// Walk every leaf in key order. This is the page walker OBTAN uses.
    pub fn scan_all(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut out = Vec::new();
        self.walk(self.root, &mut out)?;
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    fn walk(&self, id: u32, out: &mut Vec<(Vec<u8>, Vec<u8>)>) -> Result<()> {
        let (ty, pay) = self.get(id)?;
        match ty {
            PageType::Leaf => {
                out.extend(decode_pairs(&pay));
            }
            PageType::Branch => {
                for child in decode_children(&pay) {
                    self.walk(child, out)?;
                }
            }
            PageType::Overflow => {
                out.extend(decode_pairs(&pay));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn rebuild_pub(&mut self, pairs: &[(Vec<u8>, Vec<u8>)]) -> Result<()> {
        self.rebuild(pairs)
    }

    fn rebuild(&mut self, pairs: &[(Vec<u8>, Vec<u8>)]) -> Result<()> {
        let chunk = 24; // pairs per leaf
        self.pages.retain(|id, _| *id == 0);
        self.dirty.clear();
        self.next_id = 1;
        if pairs.is_empty() {
            self.root = 1;
            self.next_id = 2;
            self.put(1, PageType::Leaf, &leaf_empty(0))?;
            return Ok(());
        }
        let mut leaves = Vec::new();
        for part in pairs.chunks(chunk) {
            let id = self.next_id;
            self.next_id += 1;
            self.put(id, PageType::Leaf, &encode_pairs(part))?;
            leaves.push((id, part[0].0.clone()));
        }
        if leaves.len() == 1 {
            self.root = leaves[0].0;
            return Ok(());
        }
        let mut children = leaves;
        while children.len() > 1 {
            let mut next = Vec::new();
            for part in children.chunks(chunk) {
                let id = self.next_id;
                self.next_id += 1;
                self.put(id, PageType::Branch, &encode_children(part))?;
                next.push((id, part[0].1.clone()));
            }
            children = next;
        }
        self.root = children[0].0;
        Ok(())
    }
}

fn leaf_empty(_next: u32) -> Vec<u8> {
    vec![0, 0]
}

fn encode_pairs(pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&(pairs.len() as u16).to_le_bytes());
    for (k, v) in pairs {
        b.extend_from_slice(&(k.len() as u16).to_le_bytes());
        b.extend_from_slice(k);
        b.extend_from_slice(&(v.len() as u16).to_le_bytes());
        b.extend_from_slice(v);
    }
    let _ = digest(&b);
    b
}

fn decode_pairs(pay: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    if pay.len() < 2 {
        return Vec::new();
    }
    let n = u16::from_le_bytes(pay[0..2].try_into().unwrap()) as usize;
    let mut i = 2;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if i + 2 > pay.len() {
            break;
        }
        let kl = u16::from_le_bytes(pay[i..i + 2].try_into().unwrap()) as usize;
        i += 2;
        if i + kl + 2 > pay.len() {
            break;
        }
        let k = pay[i..i + kl].to_vec();
        i += kl;
        let vl = u16::from_le_bytes(pay[i..i + 2].try_into().unwrap()) as usize;
        i += 2;
        if i + vl > pay.len() {
            break;
        }
        let v = pay[i..i + vl].to_vec();
        i += vl;
        out.push((k, v));
    }
    out
}

fn encode_children(children: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&(children.len() as u16).to_le_bytes());
    for (id, key) in children {
        b.extend_from_slice(&id.to_le_bytes());
        b.extend_from_slice(&(key.len() as u16).to_le_bytes());
        b.extend_from_slice(key);
    }
    b
}

fn decode_children(pay: &[u8]) -> Vec<u32> {
    if pay.len() < 2 {
        return Vec::new();
    }
    let n = u16::from_le_bytes(pay[0..2].try_into().unwrap()) as usize;
    let mut i = 2;
    let mut out = Vec::new();
    for _ in 0..n {
        if i + 4 > pay.len() {
            break;
        }
        let id = u32::from_le_bytes(pay[i..i + 4].try_into().unwrap());
        i += 4;
        if i + 2 > pay.len() {
            break;
        }
        let kl = u16::from_le_bytes(pay[i..i + 2].try_into().unwrap()) as usize;
        i += 2 + kl;
        out.push(id);
    }
    out
}

fn decode_children_kv(pay: &[u8]) -> Vec<(u32, Vec<u8>)> {
    if pay.len() < 2 {
        return Vec::new();
    }
    let n = u16::from_le_bytes(pay[0..2].try_into().unwrap()) as usize;
    let mut i = 2;
    let mut out = Vec::new();
    for _ in 0..n {
        if i + 4 > pay.len() {
            break;
        }
        let id = u32::from_le_bytes(pay[i..i + 4].try_into().unwrap());
        i += 4;
        if i + 2 > pay.len() {
            break;
        }
        let kl = u16::from_le_bytes(pay[i..i + 2].try_into().unwrap()) as usize;
        i += 2;
        if i + kl > pay.len() {
            break;
        }
        let key = pay[i..i + kl].to_vec();
        i += kl;
        out.push((id, key));
    }
    out
}

fn choose_child(ch: &[(u32, Vec<u8>)], key: &[u8]) -> u32 {
    let mut pick = ch[0].0;
    for (id, sep) in ch {
        if sep.as_slice() <= key {
            pick = *id;
        } else {
            break;
        }
    }
    pick
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_walk() {
        let dir = std::env::temp_dir().join(format!(
            "oursql-bt-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tree.pg");
        let key = [7u8; 32];
        {
            let mut t = PagePool::create(&path, &key, [9u8; 32]).unwrap();
            for i in 0..40u8 {
                t.insert(&[i], &[i.wrapping_mul(3)]).unwrap();
            }
            t.flush().unwrap();
            assert_eq!(t.get_key(&[10]).unwrap().unwrap(), vec![30]);
            assert_eq!(t.scan_all().unwrap().len(), 40);
        }
        let t = PagePool::open(&path, &key).unwrap().unwrap();
        assert_eq!(t.scan_all().unwrap().len(), 40);
        assert_eq!(t.get_key(&[10]).unwrap().unwrap(), vec![30]);
        std::fs::remove_dir_all(&dir).ok();
    }
}
