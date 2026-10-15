//! Solar Hijri (Jalali) calendar.
//!
//! Uses Kazimierz Borkowski's "breaks" arithmetic, which reproduces the
//! astronomical (official Iranian) calendar from 1178 to 1633 SH and is
//! defined from -61 to 3177 SH.

use std::ops::RangeInclusive;

use crate::day::{DateError, JalaliDate, Jdn};
use crate::gregorian;

/// Years where the leap pattern changes; the algorithm is defined between the first and last.
const BREAKS: [i32; 20] = [
    -61, 9, 38, 199, 426, 686, 756, 818, 1111, 1181, 1210, 1635, 2060, 2097, 2192, 2262, 2324, 2394, 2456,
    3178,
];

/// Years the leap-year arithmetic is defined for.
const ALGORITHM_YEARS: RangeInclusive<i32> = BREAKS[0]..=BREAKS[BREAKS.len() - 1] - 1;

/// Years whose every day converts both ways. Converting the end of a year needs the
/// next year's data, so this stops one year short of [`ALGORITHM_YEARS`].
pub const SUPPORTED_YEARS: RangeInclusive<i32> = BREAKS[0]..=BREAKS[BREAKS.len() - 1] - 2;

struct YearInfo {
    /// Years since the last leap year; 0 means this year is leap.
    since_leap: i32,
    /// Gregorian year in which this year starts.
    gregorian_year: i32,
    /// Day of March (Gregorian) on which 1 Farvardin falls.
    march_day: i32,
}

fn year_info(year: i32) -> Result<YearInfo, DateError> {
    if !ALGORITHM_YEARS.contains(&year) {
        return Err(DateError::OutOfRange(year));
    }
    let gregorian_year = year + 621;
    let mut leaps = -14;
    let mut start = BREAKS[0];
    let mut jump = 0;
    for &next in &BREAKS[1..] {
        jump = next - start;
        if year < next {
            break;
        }
        leaps += jump / 33 * 8 + (jump % 33) / 4;
        start = next;
    }
    let mut n = year - start;
    leaps += n / 33 * 8 + (n % 33 + 3) / 4;
    if jump % 33 == 4 && jump - n == 4 {
        leaps += 1;
    }
    let gregorian_leaps = gregorian_year / 4 - (gregorian_year / 100 + 1) * 3 / 4 - 150;
    let march_day = 20 + leaps - gregorian_leaps;

    if jump - n < 6 {
        n = n - jump + (jump + 4) / 33 * 33;
    }
    let mut since_leap = ((n + 1) % 33 - 1) % 4;
    if since_leap == -1 {
        since_leap = 4;
    }
    Ok(YearInfo { since_leap, gregorian_year, march_day })
}

/// Whether `year` has 30 days in Esfand. Years the algorithm doesn't cover are reported as common.
pub fn is_leap(year: i32) -> bool {
    year_info(year).is_ok_and(|info| info.since_leap == 0)
}

pub fn month_len(year: i32, month: u8) -> u8 {
    match month {
        1..=6 => 31,
        7..=11 => 30,
        _ if is_leap(year) => 30,
        _ => 29,
    }
}

impl JalaliDate {
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
        Ok(JalaliDate { year, month, day })
    }

    pub fn to_jdn(self) -> Jdn {
        let info = year_info(self.year).expect("JalaliDate::new validated the year");
        let (month, day) = (i32::from(self.month), i32::from(self.day));
        let farvardin_1 = gregorian::to_jdn(info.gregorian_year, 3, info.march_day as u8);
        Jdn(farvardin_1 + (month - 1) * 31 - month / 7 * (month - 7) + day - 1)
    }

    pub fn from_jdn(jdn: Jdn) -> Result<Self, DateError> {
        let (gregorian_year, _, _) = gregorian::from_jdn(jdn.0);
        let mut year = gregorian_year - 621;
        let info = year_info(year)?;
        let farvardin_1 = gregorian::to_jdn(gregorian_year, 3, info.march_day as u8);
        let mut k = jdn.0 - farvardin_1;
        if k >= 0 {
            if !SUPPORTED_YEARS.contains(&year) {
                return Err(DateError::OutOfRange(year));
            }
            if k <= 185 {
                return Ok(JalaliDate { year, month: (1 + k / 31) as u8, day: (k % 31 + 1) as u8 });
            }
            k -= 186;
        } else {
            year -= 1;
            if !SUPPORTED_YEARS.contains(&year) {
                return Err(DateError::OutOfRange(year));
            }
            k += 179;
            if info.since_leap == 1 {
                k += 1;
            }
        }
        Ok(JalaliDate { year, month: (7 + k / 30) as u8, day: (k % 30 + 1) as u8 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::GregorianDate;

    fn jalali(y: i32, m: u8, d: u8) -> JalaliDate {
        JalaliDate::new(y, m, d).unwrap()
    }

    fn gregorian(y: i32, m: u8, d: u8) -> GregorianDate {
        GregorianDate::new(y, m, d).unwrap()
    }

    #[test]
    fn nowruz_anchors() {
        let cases = [
            ((1399, 1, 1), (2020, 3, 20)),
            ((1400, 1, 1), (2021, 3, 21)),
            ((1403, 1, 1), (2024, 3, 20)),
            ((1404, 1, 1), (2025, 3, 21)),
            ((1405, 1, 1), (2026, 3, 21)),
        ];
        for ((jy, jm, jd), (gy, gm, gd)) in cases {
            assert_eq!(jalali(jy, jm, jd).to_jdn(), gregorian(gy, gm, gd).to_jdn(), "{jy}-{jm}-{jd}");
        }
    }

    #[test]
    fn leap_years() {
        for y in [1395, 1399, 1403, 1408, 1412] {
            assert!(is_leap(y), "{y} should be leap");
        }
        for y in [1400, 1401, 1402, 1404, 1405, 1406, 1407] {
            assert!(!is_leap(y), "{y} should be common");
        }
        assert_eq!(jalali(1403, 12, 30).to_jdn(), gregorian(2025, 3, 20).to_jdn());
        assert!(JalaliDate::new(1404, 12, 30).is_err());
    }

    #[test]
    fn round_trips_across_the_supported_range() {
        let first = jalali(*SUPPORTED_YEARS.start(), 1, 1).to_jdn();
        let last = jalali(*SUPPORTED_YEARS.end(), 12, month_len(*SUPPORTED_YEARS.end(), 12)).to_jdn();
        let mut expected = jalali(*SUPPORTED_YEARS.start(), 1, 1);
        for jdn in first.0..=last.0 {
            let date = JalaliDate::from_jdn(Jdn(jdn)).unwrap();
            assert_eq!(date, expected, "jdn {jdn}");
            assert_eq!(date.to_jdn(), Jdn(jdn));
            expected = if date.day < month_len(date.year, date.month) {
                JalaliDate { day: date.day + 1, ..date }
            } else if date.month < 12 {
                JalaliDate { month: date.month + 1, day: 1, ..date }
            } else {
                JalaliDate { year: date.year + 1, month: 1, day: 1 }
            };
        }
    }

    #[test]
    fn out_of_range() {
        assert_eq!(JalaliDate::new(3177, 1, 1), Err(DateError::OutOfRange(3177)));
        assert!(JalaliDate::from_jdn(gregorian(3800, 6, 1).to_jdn()).is_err());
    }
}
