//! Brigade BUREAU.
//!
//! Policy overlay. Does not write pages. See docs/08-bureaucracy.md.

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use oursql_core::{ComradeId, Error, Intensity, Result};
use oursql_nashcql::Stmt;

/// Per-comrade ration bucket.
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

    pub fn take(&mut self, intensity: Intensity) -> Result<()> {
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

#[derive(Clone, Debug)]
struct AccuseRec {
    day: u64,
    count: u32,
}

pub struct Bureau {
    pub intensity: Intensity,
    pub ration_qps: f64,
    pub ration_burst: f64,
    pub partial_pct: u8,
    pub review_delay_ms: (u64, u64),
    pub accuse_per_day: u32,
    pub skip_sleep: bool,
    rations: HashMap<String, Ration>,
    accuses: HashMap<String, AccuseRec>,
    demotions: HashMap<String, u64>,
    last_tick: SystemTime,
    counter: u64,
}

impl Default for Bureau {
    fn default() -> Self {
        Self::new(Intensity::default_25())
    }
}

impl Bureau {
    pub fn new(intensity: Intensity) -> Self {
        Self {
            intensity,
            ration_qps: 40.0,
            ration_burst: 80.0,
            partial_pct: 8,
            review_delay_ms: (40, 180),
            accuse_per_day: 3,
            skip_sleep: std::env::var("OURL_NO_SLEEP").ok().as_deref() == Some("1"),
            rations: HashMap::new(),
            accuses: HashMap::new(),
            demotions: HashMap::new(),
            last_tick: SystemTime::now(),
            counter: 1,
        }
    }

    pub fn tick(&mut self) {
        let now = SystemTime::now();
        let dt = now
            .duration_since(self.last_tick)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64();
        self.last_tick = now;
        for r in self.rations.values_mut() {
            r.refill(dt);
        }
    }

    pub fn check_ration(&mut self, who: &ComradeId) -> Result<()> {
        self.tick();
        let qps = self.ration_qps;
        let burst = self.ration_burst;
        let r = self
            .rations
            .entry(who.0.clone())
            .or_insert_with(|| Ration::new(qps, burst));
        r.take(self.intensity)
    }

    pub fn review_delay(&mut self, stmt: &Stmt) -> Option<Duration> {
        let need = if stmt.is_ddl() {
            self.intensity.review_on_ddl()
        } else if stmt.is_mutation() {
            self.intensity.review_on_large()
        } else {
            false
        };
        if !need {
            return None;
        }
        let (lo, hi) = self.review_delay_ms;
        self.counter = self.counter.wrapping_add(1);
        let span = hi.saturating_sub(lo).max(1);
        let ms = lo + (self.counter % span);
        Some(Duration::from_millis(ms))
    }

    pub fn maybe_sleep(&self, d: Duration) {
        if !self.skip_sleep && !d.is_zero() {
            std::thread::sleep(d);
        }
    }

    pub fn require_samokrit(&self, stmt: &Stmt) -> Result<()> {
        if self.intensity.requires_samokrit() && stmt.is_mutation() && stmt.samokrit().is_none() {
            return Err(Error::samokrit_required());
        }
        Ok(())
    }

    pub fn should_partial(&mut self, stmt: &Stmt) -> bool {
        if !self.intensity.allows_partial() {
            return false;
        }
        if !matches!(stmt, Stmt::Obtan { .. }) {
            return false;
        }
        self.counter = self.counter.wrapping_add(1);
        (self.counter % 100) < self.partial_pct as u64
    }

    pub fn accuse(&mut self, accuser: &ComradeId, accused: &str) -> Result<String> {
        if !self.intensity.allows_accuse() {
            return Err(Error::bad_keyword("ACCUSE requires intensity >= 25"));
        }
        let day = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 86_400;
        let rec = self.accuses.entry(accuser.0.clone()).or_insert(AccuseRec {
            day,
            count: 0,
        });
        if rec.day != day {
            rec.day = day;
            rec.count = 0;
        }
        if rec.count >= self.accuse_per_day {
            return Err(Error::too_many_accusations());
        }
        rec.count += 1;
        let until = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + 30;
        self.demotions.insert(accused.to_string(), until);
        Ok(format!("ACCUSED {accused} (priority demotion 30s)"))
    }

    pub fn is_demoted(&self, who: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.demotions.get(who).copied().unwrap_or(0) > now
    }

    pub fn bourgeois_notice(&self, bourgeois: bool) -> Option<String> {
        if bourgeois && self.intensity.allows_bourgeois_sql() {
            Some("NOTICE 1901: bourgeois keywords tolerated at intensity 25".into())
        } else {
            None
        }
    }

    pub fn reject_bourgeois(&self, bourgeois: bool) -> Result<()> {
        if bourgeois && !self.intensity.allows_bourgeois_sql() {
            return Err(Error::bourgeois_dialect());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intensity_zero_never_gulags() {
        let mut r = Ration::new(1.0, 1.0);
        r.tokens = 0.0;
        assert!(r.take(Intensity::zero()).is_ok());
    }

    #[test]
    fn intensity_25_gulags_when_empty() {
        let mut r = Ration::new(1.0, 1.0);
        r.tokens = 0.0;
        let e = r.take(Intensity::default_25()).unwrap_err();
        assert_eq!(e.code, 1905);
    }

    #[test]
    fn accuse_ration() {
        let mut b = Bureau::new(Intensity::default_25());
        b.accuse_per_day = 1;
        let c = ComradeId("a".into());
        b.accuse(&c, "mill").unwrap();
        assert!(b.accuse(&c, "mill").is_err());
    }
}
