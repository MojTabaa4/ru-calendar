//! Proleptic Gregorian calendar.

use crate::day::{DateError, GregorianDate, Jdn};

pub fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn month_len(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        _ => 28,
    }
}

/// Years accepted by [`GregorianDate::new`], matching `jiff::civil::Date`.
pub const SUPPORTED_YEARS: std::ops::RangeInclusive<i32> = -4700..=9999;

impl GregorianDate {
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, DateError> {
        if !SUPPORTED_YEARS.contains(&year) {
            return Err(DateError::OutOfRange(year));
        }
        if !(1..=12).contains(&month) {
            return Err(DateError::InvalidMonth(month));
        }
        if day == 0 || day > month_len(year, month) {
            return Err(DateError::InvalidDay { month, day });
        }
        Ok(GregorianDate { year, month, day })
    }

    pub fn to_jdn(self) -> Jdn {
        Jdn(to_jdn(self.year, self.month, self.day))
    }

    pub fn from_jdn(jdn: Jdn) -> Self {
        let (year, month, day) = from_jdn(jdn.0);
        GregorianDate { year, month, day }
    }
}

/// Fliegel–Van Flandern: Gregorian date to Julian Day Number.
pub(crate) fn to_jdn(year: i32, month: u8, day: u8) -> i32 {
    let (y, m, d) = (i64::from(year), i64::from(month), i64::from(day));
    let a = (14 - m) / 12;
    let y = y + 4800 - a;
    let m = m + 12 * a - 3;
    let jdn =
        d + (153 * m + 2) / 5 + 365 * y + y.div_euclid(4) - y.div_euclid(100) + y.div_euclid(400) - 32045;
    jdn as i32
}

/// Richards' algorithm: Julian Day Number to Gregorian date.
pub(crate) fn from_jdn(jdn: i32) -> (i32, u8, u8) {
    let a = i64::from(jdn) + 32044;
    let b = (4 * a + 3).div_euclid(146_097);
    let c = a - (146_097 * b).div_euclid(4);
    let d = (4 * c + 3).div_euclid(1461);
    let e = c - (1461 * d).div_euclid(4);
    let m = (5 * e + 2).div_euclid(153);
    let day = e - (153 * m + 2).div_euclid(5) + 1;
    let month = m + 3 - 12 * m.div_euclid(10);
    let year = 100 * b + d - 4800 + m.div_euclid(10);
    (year as i32, month as u8, day as u8)
}

impl From<jiff::civil::Date> for GregorianDate {
    fn from(d: jiff::civil::Date) -> Self {
        GregorianDate { year: i32::from(d.year()), month: d.month() as u8, day: d.day() as u8 }
    }
}

impl TryFrom<GregorianDate> for jiff::civil::Date {
    type Error = jiff::Error;
    fn try_from(d: GregorianDate) -> Result<Self, Self::Error> {
        jiff::civil::Date::new(d.year as i16, d.month as i8, d.day as i8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_day_numbers() {
        assert_eq!(GregorianDate::new(2000, 1, 1).unwrap().to_jdn(), Jdn(2451545));
        assert_eq!(GregorianDate::new(1858, 11, 17).unwrap().to_jdn(), Jdn(2400001));
        assert_eq!(GregorianDate::new(622, 7, 19).unwrap().to_jdn(), Jdn(1948440));
    }

    #[test]
    fn round_trips() {
        for jdn in (1_721_426..2_816_788).step_by(7) {
            let g = GregorianDate::from_jdn(Jdn(jdn));
            assert_eq!(GregorianDate::new(g.year, g.month, g.day).unwrap().to_jdn(), Jdn(jdn));
        }
    }

    #[test]
    fn leap_years() {
        assert!(is_leap(2000));
        assert!(is_leap(2024));
        assert!(!is_leap(1900));
        assert!(!is_leap(2025));
        assert_eq!(month_len(2024, 2), 29);
        assert!(GregorianDate::new(2025, 2, 29).is_err());
    }
}
