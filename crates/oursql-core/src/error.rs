//! Stable error codes. See docs/14-error-catalog.md.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Language,
    Bureau,
    Storage,
    Mesh,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: u16,
    pub name: &'static str,
    pub kind: ErrorKind,
    pub message: String,
    pub retry_after_ms: Option<u16>,
}

impl Error {
    pub fn new(
        code: u16,
        name: &'static str,
        kind: ErrorKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            name,
            kind,
            message: message.into(),
            retry_after_ms: None,
        }
    }

    pub fn retry(mut self, ms: u16) -> Self {
        self.retry_after_ms = Some(ms);
        self
    }

    pub fn gulag(retry_after_ms: u16) -> Self {
        Self::new(
            1905,
            "GULAG",
            ErrorKind::Bureau,
            "Too capitalist. Temporary gulag. Retry later.",
        )
        .retry(retry_after_ms)
    }

    pub fn collective_partial(retry_after_ms: u16) -> Self {
        Self::new(
            1902,
            "COLLECTIVE_PARTIAL",
            ErrorKind::Bureau,
            "Some rows are with other comrades. Retry the same plan.",
        )
        .retry(retry_after_ms)
    }

    pub fn confiskat() -> Self {
        Self::new(
            1906,
            "CONFISKAT",
            ErrorKind::Bureau,
            "Target is under CHEKA hold. Wait for OSVOBOD.",
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERROR {} ({}) {}", self.code, self.name, self.message)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gulag_code() {
        let e = Error::gulag(8000);
        assert_eq!(e.code, 1905);
        assert_eq!(e.retry_after_ms, Some(8000));
    }
}
