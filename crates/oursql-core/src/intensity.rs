//! Bureaucracy intensity: 0 (engine only) ..= 100 (demo oppression).

use serde::{Deserialize, Serialize};

/// Tunable oppression. Default is 25.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Intensity(u8);

impl Intensity {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 100;
    pub const DEFAULT: u8 = 25;

    pub fn new(raw: u8) -> std::result::Result<Self, u8> {
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

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    pub const fn bureau_active(self) -> bool {
        self.0 > 0
    }

    pub const fn allows_partial(self) -> bool {
        self.0 >= 20
    }

    pub const fn allows_gulag(self) -> bool {
        self.0 >= 10
    }

    pub const fn allows_accuse(self) -> bool {
        self.0 >= 25
    }

    pub const fn review_on_ddl(self) -> bool {
        self.0 >= 15
    }

    pub const fn review_on_large(self) -> bool {
        self.0 >= 25
    }

    pub const fn requires_approval(self) -> bool {
        self.0 >= 60
    }

    pub const fn requires_samokrit(self) -> bool {
        self.0 >= 50
    }

    pub const fn allows_bourgeois_sql(self) -> bool {
        self.0 <= 40
    }

    pub const fn confiskat_exists(self) -> bool {
        self.0 >= 25
    }
}

impl Default for Intensity {
    fn default() -> Self {
        Self::default_25()
    }
}

impl std::fmt::Display for Intensity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
        assert!(!Intensity::saturating(50).allows_bourgeois_sql());
    }

    #[test]
    fn zero_has_no_gulag() {
        assert!(!Intensity::zero().allows_gulag());
        assert!(!Intensity::zero().allows_partial());
    }
}
