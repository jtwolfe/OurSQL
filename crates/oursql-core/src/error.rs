//! Stable error codes. See docs/14-error-catalog.md.

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

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

    pub fn lang(code: u16, name: &'static str, msg: impl Into<String>) -> Self {
        Self::new(code, name, ErrorKind::Language, msg)
    }

    pub fn bureau(code: u16, name: &'static str, msg: impl Into<String>) -> Self {
        Self::new(code, name, ErrorKind::Bureau, msg)
    }

    pub fn storage(code: u16, name: &'static str, msg: impl Into<String>) -> Self {
        Self::new(code, name, ErrorKind::Storage, msg)
    }

    pub fn mesh(code: u16, name: &'static str, msg: impl Into<String>) -> Self {
        Self::new(code, name, ErrorKind::Mesh, msg)
    }

    pub fn bad_token(msg: impl Into<String>) -> Self {
        Self::lang(1801, "BAD_TOKEN", msg)
    }
    pub fn bad_grammar(msg: impl Into<String>) -> Self {
        Self::lang(1802, "BAD_GRAMMAR", msg)
    }
    pub fn unknown_ident(msg: impl Into<String>) -> Self {
        Self::lang(1803, "UNKNOWN_IDENT", msg)
    }
    pub fn type_fight(msg: impl Into<String>) -> Self {
        Self::lang(1804, "TYPE_FIGHT", msg)
    }
    pub fn pusto_banned(msg: impl Into<String>) -> Self {
        Self::lang(1805, "PUSTO_WHERE_BANNED", msg)
    }
    pub fn no_narodkey() -> Self {
        Self::lang(1806, "NO_NARODKEY", "tabl needs a NARODKEY")
    }
    pub fn bad_keyword(msg: impl Into<String>) -> Self {
        Self::lang(1807, "BAD_KEYWORD", msg)
    }

    pub fn bourgeois_notice() -> Self {
        Self::bureau(
            1901,
            "BOURGEOIS_KEYWORDS",
            "bourgeois keywords tolerated at intensity 25",
        )
    }
    pub fn collective_partial(retry_after_ms: u16) -> Self {
        Self::bureau(
            1902,
            "COLLECTIVE_PARTIAL",
            "Some rows are with other comrades. Retry the same plan.",
        )
        .retry(retry_after_ms)
    }
    pub fn no_approval() -> Self {
        Self::bureau(1904, "NO_APPROVAL", "komitet has not NAGRAD APPROVAL")
    }
    pub fn gulag(retry_after_ms: u16) -> Self {
        Self::bureau(
            1905,
            "GULAG",
            "Too capitalist. Temporary gulag. Retry later.",
        )
        .retry(retry_after_ms)
    }
    pub fn confiskat() -> Self {
        Self::bureau(1906, "CONFISKAT", "Target is under CHEKA hold. Wait for OSVOBOD.")
    }
    pub fn bourgeois_dialect() -> Self {
        Self::bureau(1908, "BOURGEOIS_DIALECT", "rewrite in NashCQL")
    }
    pub fn samokrit_required() -> Self {
        Self::bureau(1909, "SAMOKRIT_REQUIRED", "add SAMOKRIT")
    }
    pub fn too_many_accusations() -> Self {
        Self::bureau(1910, "TOO_MANY_ACCUSATIONS", "ration of accusations exhausted")
    }
    pub fn intensity_denied() -> Self {
        Self::bureau(1911, "INTENSITY_DENIED", "cannot set intensity")
    }
    pub fn line_conflict() -> Self {
        Self::bureau(1912, "LINE_CONFLICT", "first certified digest wins; retry")
    }

    pub fn wal_io(msg: impl Into<String>) -> Self {
        Self::storage(2001, "WAL_IO", msg)
    }
    pub fn page_checksum() -> Self {
        Self::storage(2002, "PAGE_CHECKSUM", "page checksum mismatch")
    }
    pub fn recovery_failed(msg: impl Into<String>) -> Self {
        Self::storage(2004, "RECOVERY_FAILED", msg)
    }

    pub fn below_quorum() -> Self {
        Self::mesh(2102, "BELOW_QUORUM", "view below certification quorum")
    }
    pub fn cap_expired() -> Self {
        Self::mesh(2107, "CAP_EXPIRED", "capability expired")
    }
    pub fn node_busy() -> Self {
        Self::mesh(2108, "NODE_BUSY", "session worker queue full")
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERROR {} ({}) {}", self.code, self.name, self.message)?;
        if let Some(ms) = self.retry_after_ms {
            write!(f, " retry_after_ms={ms}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::wal_io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gulag_code_stable() {
        let e = Error::gulag(8000);
        assert_eq!(e.code, 1905);
        assert_eq!(e.retry_after_ms, Some(8000));
    }

    #[test]
    fn catalog_ranges() {
        assert!(Error::bad_token("x").code < 1900);
        assert!(Error::gulag(1).code >= 1900 && Error::gulag(1).code < 2000);
        assert!(Error::wal_io("x").code >= 2000 && Error::wal_io("x").code < 2100);
    }
}
