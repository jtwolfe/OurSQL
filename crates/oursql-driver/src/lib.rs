//! Brigade DRIVER — line protocol client.

#![deny(unsafe_code)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

use oursql_core::{Error, Result};

pub struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        Ok(Self { stream })
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
