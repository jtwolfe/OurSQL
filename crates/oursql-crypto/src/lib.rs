//! Brigade CRYPTO.
//!
//! Digests (BLAKE3) and comrade/node signatures (Ed25519).
//! No files. No NashCQL. No policy.

#![deny(unsafe_code)]

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

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

pub fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(H[(b >> 4) as usize] as char);
        out.push(H[(b & 0xf) as usize] as char);
    }
    out
}

fn unhex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = from_hex(s.as_bytes()[i * 2])?;
        let lo = from_hex(s.as_bytes()[i * 2 + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
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
}
