//! Brigade WIRE — OCHERED/1.
//!
//! Length-prefixed frames. Also a line protocol for humans.

#![deny(unsafe_code)]

use std::io::{Read, Write};

use oursql_core::{Error, Outcome, Result, Value};

pub const ALPN: &str = "oursql/1";
pub const MAX_FRAME: usize = 16 * 1024 * 1024;

pub const T_HELLO: u8 = 0x01;
pub const T_WELCOME: u8 = 0x02;
pub const T_STMT: u8 = 0x03;
pub const T_BIND: u8 = 0x04;
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

    pub fn write_to(&self, w: &mut impl Write) -> Result<()> {
        let enc = self.encode()?;
        w.write_all(&enc)?;
        w.flush()?;
        Ok(())
    }

    pub fn read_from(r: &mut impl Read) -> Result<Self> {
        let mut hdr = [0u8; 4];
        r.read_exact(&mut hdr)?;
        let len = u32::from_be_bytes(hdr) as usize;
        if len < 4 || len > MAX_FRAME {
            return Err(Error::bad_token("bad frame len"));
        }
        let mut rest = vec![0u8; len];
        r.read_exact(&mut rest)?;
        let mut full = hdr.to_vec();
        full.extend_from_slice(&rest);
        Self::decode(&full)
    }
}

pub fn hello_payload(comrade: &str, nonce: &str, client: &str) -> Vec<u8> {
    format!("{comrade}\n{nonce}\n{client}").into_bytes()
}

pub fn parse_hello(p: &[u8]) -> Result<(String, String, String)> {
    let s = String::from_utf8_lossy(p);
    let mut it = s.lines();
    Ok((
        it.next().unwrap_or("founder").into(),
        it.next().unwrap_or("").into(),
        it.next().unwrap_or("oursql").into(),
    ))
}

pub fn welcome_payload(dossier: &str, intensity: u8, node: &str, epoch: u64) -> Vec<u8> {
    format!("{dossier}\n{intensity}\n{node}\n{epoch}").into_bytes()
}

pub fn error_payload(code: u16, retry_after_ms: u16, msg: &str) -> Vec<u8> {
    let mut o = Vec::new();
    o.extend_from_slice(&code.to_be_bytes());
    o.extend_from_slice(&retry_after_ms.to_be_bytes());
    o.extend_from_slice(msg.as_bytes());
    o
}

pub fn outcome_frames(out: &Outcome) -> Vec<Frame> {
    let mut v = Vec::new();
    match out {
        Outcome::Rows {
            columns,
            rows,
            partial,
            notice,
        } => {
            let mut p = String::new();
            p.push_str(&columns.join("\t"));
            p.push('\n');
            for r in rows {
                let line: Vec<String> = r.iter().map(Value::to_plain).collect();
                p.push_str(&line.join("\t"));
                p.push('\n');
            }
            if *partial {
                p.push_str("PARTIAL\n");
            }
            v.push(Frame {
                typ: T_ROWS,
                payload: p.into_bytes(),
            });
            if let Some(n) = notice {
                v.push(Frame {
                    typ: T_NOTICE,
                    payload: n.as_bytes().to_vec(),
                });
            }
            v.push(Frame {
                typ: T_DONE,
                payload: Vec::new(),
            });
        }
        other => {
            let text = match other {
                Outcome::Empty { notice } => notice.clone().unwrap_or_else(|| "ok".into()),
                Outcome::Count { n, notice } => {
                    let mut s = format!("{n}");
                    if let Some(n) = notice {
                        s.push('\n');
                        s.push_str(n);
                    }
                    s
                }
                Outcome::Razbor { text } => text.clone(),
                Outcome::Rows { .. } => unreachable!(),
            };
            v.push(Frame {
                typ: T_DONE,
                payload: text.into_bytes(),
            });
        }
    }
    v
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

    #[test]
    fn hello_welcome() {
        let p = hello_payload("founder", "n1", "cli");
        let (c, n, cl) = parse_hello(&p).unwrap();
        assert_eq!(c, "founder");
        assert_eq!(n, "n1");
        assert_eq!(cl, "cli");
    }
}
