//! The current time, behind a trait so tests and debug runs can fake it.

use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

use crate::day::{GregorianDate, Jdn};

pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// Real time shifted by a fixed amount, so a faked "now" keeps ticking.
pub struct OffsetClock {
    offset: SignedDuration,
}

impl OffsetClock {
    pub fn starting_at(fake_now: Timestamp) -> Self {
        OffsetClock { offset: fake_now.duration_since(Timestamp::now()) }
    }
}

impl Clock for OffsetClock {
    fn now(&self) -> Timestamp {
        Timestamp::now() + self.offset
    }
}

/// A clock that never moves.
pub struct FixedClock(pub Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

/// Parses `RUZ_FAKE_NOW`-style values such as `2026-03-20T23:59:30+03:30`.
pub fn parse_timestamp(s: &str) -> Result<Timestamp, jiff::Error> {
    s.parse()
}

/// Today's day number in `tz`.
pub fn today(clock: &dyn Clock, tz: &TimeZone) -> Jdn {
    day_of(clock.now(), tz)
}

pub fn day_of(at: Timestamp, tz: &TimeZone) -> Jdn {
    GregorianDate::from(at.to_zoned(tz.clone()).date()).to_jdn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_depends_on_the_time_zone() {
        let clock = FixedClock(parse_timestamp("2025-03-20T21:00:00Z").unwrap());
        let tehran = TimeZone::get("Asia/Tehran").unwrap();
        let utc = TimeZone::UTC;
        assert_eq!(today(&clock, &tehran) - today(&clock, &utc), 1);
        assert_eq!(today(&clock, &tehran), GregorianDate::new(2025, 3, 21).unwrap().to_jdn());
    }

    #[test]
    fn offset_clock_starts_near_the_fake_time() {
        let fake = parse_timestamp("2030-01-01T00:00:00Z").unwrap();
        let clock = OffsetClock::starting_at(fake);
        assert!(clock.now().duration_since(fake).abs() < SignedDuration::from_secs(5));
    }
}
