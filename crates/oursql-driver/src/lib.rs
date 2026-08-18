//! Brigade DRIVER — line protocol + OCHERED/1 binary client.

#![deny(unsafe_code)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

use oursql_core::{Error, Result};
use oursql_wire::{Frame, T_BIND, T_DONE, T_ERROR, T_HELLO, T_NOTICE, T_ROWS, T_STMT, T_WELCOME};

pub struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        Ok(Self { stream })
    }

    pub fn hello(&mut self, comrade: &str) -> Result<String> {
        Frame {
            typ: T_HELLO,
            payload: oursql_wire::hello_payload(comrade, "n", "driver"),
        }
        .write_to(&mut self.stream)?;
        let f = Frame::read_from(&mut self.stream)?;
        if f.typ != T_WELCOME {
            return Err(Error::bad_token("expected WELCOME"));
        }
        Ok(String::from_utf8_lossy(&f.payload).to_string())
    }

    pub fn bind(&mut self, values: &[&str]) -> Result<()> {
        Frame {
            typ: T_BIND,
            payload: values.join("\n").into_bytes(),
        }
        .write_to(&mut self.stream)?;
        let f = Frame::read_from(&mut self.stream)?;
        if f.typ != T_DONE {
            return Err(Error::bad_token("expected DONE after BIND"));
        }
        Ok(())
    }

    pub fn exec_binary(&mut self, sql: &str) -> Result<String> {
        Frame {
            typ: T_STMT,
            payload: sql.as_bytes().to_vec(),
        }
        .write_to(&mut self.stream)?;
        let mut out = String::new();
        loop {
            let f = Frame::read_from(&mut self.stream)?;
            match f.typ {
                T_ROWS | T_DONE | T_NOTICE => {
                    out.push_str(&String::from_utf8_lossy(&f.payload));
                    if f.typ == T_DONE {
                        break;
                    }
                }
                T_ERROR => {
                    return Err(Error::bad_grammar(
                        String::from_utf8_lossy(&f.payload[4.min(f.payload.len())..]).to_string(),
                    ));
                }
                _ => break,
            }
        }
        Ok(out)
    }

    pub fn exec(&mut self, sql: &str) -> Result<String> {
        writeln!(self.stream, "{sql};")?;
        self.stream.flush()?;
        let mut reader = BufReader::new(self.stream.try_clone()?);
        let mut out = String::new();
        loop {
            let mut line = String::new();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            if line.trim() == "." {
                break;
            }
            if let Some(rest) = line.strip_prefix("ERR ") {
                return Err(Error::bad_grammar(rest.trim()));
            }
            out.push_str(&line);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn crate_loads() {
        assert!(!oursql_wire::ALPN.is_empty());
    }
}
