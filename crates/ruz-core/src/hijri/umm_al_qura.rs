//! Umm al-Qura month lengths (KACST data, 1300–1600 AH) via `icu_calendar`.
//!
//! Kept to this one file so ICU API changes stay contained. Outside the data range
//! these return `None` and callers use the tabular calendar instead.

use std::ops::RangeInclusive;

use icu_calendar::Date;
use icu_calendar::cal::Hijri;
use icu_calendar::types::RataDie;

use crate::day::{HijriDate, Jdn};

/// Years backed by KACST data; ICU falls back to tabular rules outside them.
pub const YEARS: RangeInclusive<i32> = 1300..=1600;

/// Rata Die 1 is 1 January 1 (proleptic Gregorian), JDN 1721426.
const RD_TO_JDN: i64 = 1_721_425;

fn calendar() -> Hijri<icu_calendar::cal::hijri::UmmAlQura> {
    Hijri::new_umm_al_qura()
}

pub fn from_jdn(jdn: Jdn) -> Option<HijriDate> {
    let date = Date::from_rata_die(RataDie::new(i64::from(jdn.0) - RD_TO_JDN), calendar());
    let year = date.year().extended_year();
    YEARS.contains(&year).then(|| HijriDate { year, month: date.month().ordinal, day: date.day_of_month().0 })
}

pub fn month_start(year: i32, month: u8) -> Option<Jdn> {
    if !YEARS.contains(&year) {
        return None;
    }
    let date = Date::try_new_hijri_with_calendar(year, month, 1, calendar()).ok()?;
    Some(Jdn((date.to_rata_die().to_i64_date() + RD_TO_JDN) as i32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::GregorianDate;

    #[test]
    fn known_dates() {
        // 1 Ramadan 1446 began on 1 March 2025 in Saudi Arabia.
        let start = month_start(1446, 9).unwrap();
        assert_eq!(start, GregorianDate::new(2025, 3, 1).unwrap().to_jdn());
        assert_eq!(from_jdn(start), Some(HijriDate { year: 1446, month: 9, day: 1 }));
    }

    #[test]
    fn outside_the_data_range() {
        assert_eq!(month_start(1200, 1), None);
        assert_eq!(from_jdn(GregorianDate::new(1800, 1, 1).unwrap().to_jdn()), None);
    }
}
