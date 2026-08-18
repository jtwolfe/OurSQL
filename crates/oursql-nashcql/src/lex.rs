//! Lexer. ASCII. Case-insensitive keywords.

use oursql_core::{Error, Result};

use crate::keywords::is_keyword;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    Kw(String),
    String(String),
    Int(i64),
    Float(f64),
    Star,
    Comma,
    LParen,
    RParen,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Slash,
    Semi,
}

pub fn lex(input: &str) -> Result<Vec<Tok>> {
    let b = input.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'-' && i + 1 < b.len() && b[i + 1] == b'-' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(b.len());
            continue;
        }
        match c {
            b',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            b'(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            b';' => {
                out.push(Tok::Semi);
                i += 1;
            }
            b'*' => {
                out.push(Tok::Star);
                i += 1;
            }
            b'+' => {
                out.push(Tok::Plus);
                i += 1;
            }
            b'/' => {
                out.push(Tok::Slash);
                i += 1;
            }
            b'=' => {
                out.push(Tok::Eq);
                i += 1;
            }
            b'!' if i + 1 < b.len() && b[i + 1] == b'=' => {
                out.push(Tok::Ne);
                i += 2;
            }
            b'<' => {
                if i + 1 < b.len() && b[i + 1] == b'>' {
                    out.push(Tok::Ne);
                    i += 2;
                } else if i + 1 < b.len() && b[i + 1] == b'=' {
                    out.push(Tok::Le);
                    i += 2;
                } else {
                    out.push(Tok::Lt);
                    i += 1;
                }
            }
            b'>' => {
                if i + 1 < b.len() && b[i + 1] == b'=' {
                    out.push(Tok::Ge);
                    i += 2;
                } else {
                    out.push(Tok::Gt);
                    i += 1;
                }
            }
            b'\'' => {
                i += 1;
                let mut s = String::new();
                while i < b.len() {
                    if b[i] == b'\'' {
                        if i + 1 < b.len() && b[i + 1] == b'\'' {
                            s.push('\'');
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    s.push(b[i] as char);
                    i += 1;
                }
                out.push(Tok::String(s));
            }
            b'"' => {
                i += 1;
                let start = i;
                while i < b.len() && b[i] != b'"' {
                    i += 1;
                }
                let ident = input[start..i].to_string();
                if i < b.len() {
                    i += 1;
                }
                out.push(Tok::Ident(ident));
            }
            b'-' if i + 1 < b.len() && b[i + 1].is_ascii_digit() => {
                let (tok, n) = number(&input[i..])?;
                out.push(tok);
                i += n;
            }
            c if c.is_ascii_digit() => {
                let (tok, n) = number(&input[i..])?;
                out.push(tok);
                i += n;
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                i += 1;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let word = &input[start..i];
                let up = word.to_ascii_uppercase();
                if is_keyword(&up) {
                    out.push(Tok::Kw(up));
                } else {
                    out.push(Tok::Ident(word.to_string()));
                }
            }
            other => {
                return Err(Error::bad_token(format!(
                    "unexpected byte 0x{other:02x}"
                )));
            }
        }
    }
    Ok(out)
}

fn number(s: &str) -> Result<(Tok, usize)> {
    let b = s.as_bytes();
    let mut i = 0;
    if b[0] == b'-' {
        i = 1;
    }
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        let f: f64 = s[..i]
            .parse()
            .map_err(|_| Error::bad_token("bad drob"))?;
        return Ok((Tok::Float(f), i));
    }
    let n: i64 = s[..i]
        .parse()
        .map_err(|_| Error::bad_token("bad celiy"))?;
    Ok((Tok::Int(n), i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_obtan() {
        let t = lex("OBTAN name IZ bolts -- hi").unwrap();
        assert!(matches!(&t[0], Tok::Kw(s) if s == "OBTAN"));
        assert!(matches!(&t[1], Tok::Ident(s) if s == "name"));
        assert!(matches!(&t[2], Tok::Kw(s) if s == "IZ"));
    }

    #[test]
    fn lex_string_escape() {
        let t = lex("'it''s'").unwrap();
        assert_eq!(t, vec![Tok::String("it's".into())]);
    }
}
