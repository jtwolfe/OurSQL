//! Policy overlay. Does not write pages. See docs/08-bureaucracy.md.

#![deny(unsafe_code)]

use oursql_core::{Error, Intensity};

/// Per-comrade ration bucket (in-memory; a node will persist later).
#[derive(Clone, Debug)]
pub struct Ration {
    pub tokens: f64,
    pub burst: f64,
    pub qps: f64,
}

impl Ration {
    pub fn new(qps: f64, burst: f64) -> Self {
        Self {
            tokens: burst,
            burst,
            qps,
        }
    }

    /// Spend one request. On empty, return GULAG.
    pub fn take(&mut self, intensity: Intensity) -> Result<(), Error> {
        if !intensity.allows_gulag() {
            return Ok(());
        }
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            Err(Error::gulag(8000))
        }
    }

    pub fn refill(&mut self, seconds: f64) {
        self.tokens = (self.tokens + self.qps * seconds).min(self.burst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intensity_zero_never_gulags() {
        let mut r = Ration::new(1.0, 1.0);
        r.tokens = 0.0;
        let i = Intensity::new(0).unwrap();
        assert!(r.take(i).is_ok());
    }

    #[test]
    fn intensity_25_gulags_when_empty() {
        let mut r = Ration::new(1.0, 1.0);
        r.tokens = 0.0;
        let i = Intensity::default_25();
        let e = r.take(i).unwrap_err();
        assert_eq!(e.code, 1905);
    }
}
