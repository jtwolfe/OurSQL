//! Brigade CRYPTO.
//!
//! Digests (BLAKE3), comrade/node signatures (Ed25519),
//! page seal (XChaCha20-Poly1305). No files. No NashCQL. No policy.

#![deny(unsafe_code)]

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;

use oursql_core::{Error, Result};

/// BLAKE3-256 of `data`.
pub fn digest(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

/// Canonical mutation digest per docs/05.
pub fn mutation_digest(
    kollektiv: &str,
    schema_epoch: u64,
    stmt: &str,
    narodkeys: &str,
    comrade: &str,
    ts: u64,
) -> [u8; 32] {
    let mut s = String::new();
    s.push_str(kollektiv);
    s.push('\0');
    s.push_str(&schema_epoch.to_string());
    s.push('\0');
    s.push_str(stmt);
    s.push('\0');
    s.push_str(narodkeys);
    s.push('\0');
    s.push_str(comrade);
    s.push('\0');
    s.push_str(&ts.to_string());
    digest(s.as_bytes())
}

/// IEEE CRC-32 (WAL records).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Seal plaintext with XChaCha20-Poly1305. Returns nonce || ciphertext+tag.
pub fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let mut nonce = [0u8; 24];
    OsRng.fill_bytes(&mut nonce);
    let ct = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .map_err(|_| Error::wal_io("seal failed"))?;
    let mut out = Vec::with_capacity(24 + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Open a `seal` blob.
pub fn open(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>> {
    if blob.len() < 24 + 16 {
        return Err(Error::page_checksum());
    }
    let cipher = XChaCha20Poly1305::new(key.into());
    cipher
        .decrypt(XNonce::from_slice(&blob[..24]), &blob[24..])
        .map_err(|_| Error::page_checksum())
}

#[derive(Clone)]
pub struct KeyPair {
    signing: SigningKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        let signing = SigningKey::generate(&mut OsRng);
        Self { signing }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            signing: SigningKey::from_bytes(bytes),
        }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing.to_bytes()
    }

    pub fn public_hex(&self) -> String {
        hex(&self.signing.verifying_key().to_bytes())
    }

    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.signing.sign(msg).to_bytes()
    }

    pub fn verify(pub_hex: &str, msg: &[u8], sig: &[u8; 64]) -> bool {
        let Some(raw) = unhex32(pub_hex) else {
            return false;
        };
        let Ok(vk) = VerifyingKey::from_bytes(&raw) else {
            return false;
        };
        let sig = Signature::from_bytes(sig);
        vk.verify(msg, &sig).is_ok()
    }
}

/// Node identity: signing key + storage key. File format is hex text, mode 0600.
#[derive(Clone)]
pub struct NodeIdentity {
    pub keys: KeyPair,
    pub storage_key: [u8; 32],
}

impl NodeIdentity {
    pub fn generate() -> Self {
        let mut storage_key = [0u8; 32];
        OsRng.fill_bytes(&mut storage_key);
        Self {
            keys: KeyPair::generate(),
            storage_key,
        }
    }

    pub fn load_or_create(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            Self::load(path)
        } else {
            let id = Self::generate();
            id.save(path)?;
            Ok(id)
        }
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        refuse_loose_perms(path)?;
        let text = std::fs::read_to_string(path)?;
        let mut lines = text.lines();
        let magic = lines.next().unwrap_or("");
        if magic != "OURLKEY1" {
            return Err(Error::recovery_failed("bad node.key magic"));
        }
        let sk = unhex32(lines.next().unwrap_or(""))
            .ok_or_else(|| Error::recovery_failed("bad signing key"))?;
        let storage = unhex32(lines.next().unwrap_or(""))
            .ok_or_else(|| Error::recovery_failed("bad storage key"))?;
        Ok(Self {
            keys: KeyPair::from_bytes(&sk),
            storage_key: storage,
        })
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let body = format!(
            "OURLKEY1\n{}\n{}\n",
            hex(&self.keys.to_bytes()),
            hex(&self.storage_key)
        );
        std::fs::write(path, body)?;
        tighten_perms(path)?;
        refuse_loose_perms(path)?;
        Ok(())
    }
}

fn tighten_perms(path: &std::path::Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(path)?.permissions();
        p.set_mode(0o600);
        std::fs::set_permissions(path, p)?;
    }
    let _ = path;
    Ok(())
}

fn refuse_loose_perms(path: &std::path::Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)?.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(Error::wal_io(format!(
                "node.key must be mode 0600, got {:o}",
                mode & 0o777
            )));
        }
    }
    let _ = path;
    Ok(())
}

pub fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(H[(b >> 4) as usize] as char);
        out.push(H[(b & 0xf) as usize] as char);
    }
    out
}

pub fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let b = s.as_bytes();
    for i in 0..out.capacity() {
        let hi = from_hex(b[i * 2])?;
        let lo = from_hex(b[i * 2 + 1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

pub fn unhex32(s: &str) -> Option<[u8; 32]> {
    let v = unhex(s)?;
    v.try_into().ok()
}

pub fn unhex64(s: &str) -> Option<[u8; 64]> {
    let v = unhex(s)?;
    v.try_into().ok()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_known() {
        assert_ne!(crc32(b"hello"), crc32(b"world"));
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn sign_roundtrip() {
        let kp = KeyPair::generate();
        let msg = b"INZRT V bolts";
        let sig = kp.sign(msg);
        assert!(KeyPair::verify(&kp.public_hex(), msg, &sig));
        assert!(!KeyPair::verify(&kp.public_hex(), b"nope", &sig));
    }

    #[test]
    fn digest_stable() {
        assert_eq!(digest(b"a"), digest(b"a"));
        assert_ne!(digest(b"a"), digest(b"b"));
    }

    #[test]
    fn seal_open() {
        let mut key = [0u8; 32];
        key[0] = 7;
        let blob = seal(&key, b"secret row").unwrap();
        assert!(!blob.windows(6).any(|w| w == b"secret"));
        assert_eq!(open(&key, &blob).unwrap(), b"secret row");
    }
}
