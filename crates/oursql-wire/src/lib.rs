//! Brigade WIRE — OCHERED/1.
//!
//! Length-prefixed frames. Also a line protocol for humans.

#![deny(unsafe_code)]

use oursql_core::{Error, Result};

pub const ALPN: &str = "oursql/1";
pub const MAX_FRAME: usize = 16 * 1024 * 1024;

pub const T_HELLO: u8 = 0x01;
pub const T_WELCOME: u8 = 0x02;
pub const T_STMT: u8 = 0x03;
pub const T_ROWS: u8 = 0x05;
pub const T_DONE: u8 = 0x06;
pub const T_NOTICE: u8 = 0x07;
pub const T_ERROR: u8 = 0x08;
pub const T_PING: u8 = 0x0B;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub typ: u8,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.payload.len() > MAX_FRAME {
            return Err(Error::node_busy());
        }
        let len = (self.payload.len() + 4) as u32;
        let mut out = Vec::with_capacity(4 + 4 + self.payload.len());
        out.extend_from_slice(&len.to_be_bytes());
        out.push(0);
        out.push(self.typ);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < 8 {
            return Err(Error::bad_token("short frame"));
        }
        let len = u32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize;
        if len < 4 || 4 + len > buf.len() {
            return Err(Error::bad_token("bad frame len"));
        }
        let typ = buf[5];
        let payload = buf[8..4 + len].to_vec();
        Ok(Self { typ, payload })
    }
}

/// One NashCQL statement ending in `;`, possibly multi-line.
pub fn split_statements(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_s = false;
    for ch in input.chars() {
        if ch == '\'' {
            in_s = !in_s;
            cur.push(ch);
            continue;
        }
        if ch == ';' && !in_s {
            let t = cur.trim().to_string();
            if !t.is_empty() {
                out.push(t);
            }
            cur.clear();
            continue;
        }
        cur.push(ch);
    }
    let t = cur.trim().to_string();
    if !t.is_empty() {
        out.push(t);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let f = Frame {
            typ: T_STMT,
            payload: b"OBTAN * IZ t".to_vec(),
        };
        let enc = f.encode().unwrap();
        let dec = Frame::decode(&enc).unwrap();
        assert_eq!(f, dec);
    }

    #[test]
    fn split_two() {
        let s = split_statements("ZANIM sklad; OBTAN * IZ t;");
        assert_eq!(s.len(), 2);
    }
}
