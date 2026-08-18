//! Bureaucracy intensity: 0 (engine only) ..= 100 (demo oppression).

/// Tunable oppression. Default is 25.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Intensity(u8);

impl Intensity {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 100;
    pub const DEFAULT: u8 = 25;

    pub fn new(raw: u8) -> Result<Self, u8> {
        if raw <= Self::MAX {
            Ok(Self(raw))
        } else {
            Err(raw)
        }
    }

    pub fn saturating(raw: u16) -> Self {
        Self(raw.min(Self::MAX as u16) as u8)
    }

    pub const fn default_25() -> Self {
        Self(Self::DEFAULT)
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    pub const fn bureau_active(self) -> bool {
        self.0 > 0
    }

    /// Partial results (B1) start here.
    pub const fn allows_partial(self) -> bool {
        self.0 >= 20
    }

    /// Gulag (B3) starts here.
    pub const fn allows_gulag(self) -> bool {
        self.0 >= 10
    }

    /// Accuse verb accepted.
    pub const fn allows_accuse(self) -> bool {
        self.0 >= 25
    }

    /// Decadent SQL still rewritten instead of refused.
    pub const fn allows_bourgeois_sql(self) -> bool {
        self.0 <= 40
    }
}

impl Default for Intensity {
    fn default() -> Self {
        Self::default_25()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_25() {
        assert_eq!(Intensity::default().get(), 25);
    }

    #[test]
    fn rejects_101() {
        assert!(Intensity::new(101).is_err());
    }

    #[test]
    fn bourgeois_sql_at_25() {
        assert!(Intensity::default_25().allows_bourgeois_sql());
    }
}
