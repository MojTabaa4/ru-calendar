//! Arithmetic (tabular) Islamic calendar with the civil (Friday) epoch.
//!
//! Leap years are 2, 5, 7, 10, 13, 16, 18, 21, 24, 26 and 29 of each 30-year cycle.
//! This is the fallback when no observational data covers a date.

use crate::day::{HijriDate, Jdn};

/// 1 Muharram 1 AH (civil epoch): Friday, 16 July 622 Julian.
const EPOCH: i32 = 1_948_440;

pub fn is_leap(year: i32) -> bool {
    (14 + 11 * year).rem_euclid(30) < 11
}

pub fn month_len(year: i32, month: u8) -> u8 {
    if month % 2 == 1 || (month == 12 && is_leap(year)) { 30 } else { 29 }
}

pub fn to_jdn(year: i32, month: u8, day: u8) -> Jdn {
    let (m, d) = (i32::from(month), i32::from(day));
    Jdn(d + (59 * (m - 1) + 1) / 2 + 354 * (year - 1) + (3 + 11 * year).div_euclid(30) + EPOCH - 1)
}

pub fn from_jdn(jdn: Jdn) -> HijriDate {
    let mut year = (30 * (jdn.0 - EPOCH) + 10646).div_euclid(10631);
    while to_jdn(year + 1, 1, 1) <= jdn {
        year += 1;
    }
    while to_jdn(year, 1, 1) > jdn {
        year -= 1;
    }
    let month = (1..=12).rev().find(|&m| to_jdn(year, m, 1) <= jdn).unwrap_or(1);
    let day = (jdn - to_jdn(year, month, 1) + 1) as u8;
    HijriDate { year, month, day }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch() {
        assert_eq!(to_jdn(1, 1, 1), Jdn(EPOCH));
        assert_eq!(from_jdn(Jdn(EPOCH)), HijriDate { year: 1, month: 1, day: 1 });
    }

    #[test]
    fn year_lengths_follow_the_leap_cycle() {
        for year in 1..2000 {
            let len = to_jdn(year + 1, 1, 1) - to_jdn(year, 1, 1);
            assert_eq!(len, if is_leap(year) { 355 } else { 354 }, "year {year}");
        }
    }

    #[test]
    fn round_trips() {
        for jdn in (EPOCH..EPOCH + 600_000).step_by(3) {
            let d = from_jdn(Jdn(jdn));
            assert!(d.day >= 1 && d.day <= month_len(d.year, d.month));
            assert_eq!(to_jdn(d.year, d.month, d.day), Jdn(jdn));
        }
    }
}
