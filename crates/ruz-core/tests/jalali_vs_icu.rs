//! Cross-checks our Jalali arithmetic against ICU4X's astronomical Persian calendar.

use icu_calendar::Date;
use icu_calendar::cal::Persian;
use ruz_core::{JalaliDate, Jdn};

/// Rata Die 1 is 1 January 1 (proleptic Gregorian), JDN 1721426.
const RD_TO_JDN: i64 = 1_721_425;

/// The two methods disagree on whether 1502 or 1503 SH (2123/24) is the leap year.
/// It's a century away, so we keep Borkowski's answer and only check that this is
/// the sole disagreement.
const KNOWN_DIVERGENCE: std::ops::RangeInclusive<i32> = 1502..=1503;

#[test]
fn every_day_from_1200_to_1600_matches_icu() {
    let first = JalaliDate::new(1200, 1, 1).unwrap().to_jdn();
    let last = JalaliDate::new(1601, 1, 1).unwrap().to_jdn();
    let mut mismatches = Vec::new();
    for jdn in first.0..last.0 {
        let ours = JalaliDate::from_jdn(Jdn(jdn)).unwrap();
        let icu = Date::from_rata_die(icu_calendar::types::RataDie::new(i64::from(jdn) - RD_TO_JDN), Persian);
        let theirs = (icu.year().extended_year(), icu.month().ordinal, icu.day_of_month().0);
        if (ours.year, ours.month, ours.day) != theirs {
            mismatches.push((ours, theirs));
        }
    }
    let unexpected: Vec<_> = mismatches.iter().filter(|(d, _)| !KNOWN_DIVERGENCE.contains(&d.year)).collect();
    assert!(
        unexpected.is_empty(),
        "{} unexpected mismatches, first: {:?}",
        unexpected.len(),
        unexpected.first()
    );
    assert_eq!(mismatches.len(), 366, "the 1502/1503 divergence should shift exactly one year");
}

#[test]
fn icu_agrees_on_recent_nowruz_days() {
    for year in 1300..1500 {
        let icu = Date::try_new_persian(year, 1, 1).unwrap();
        let jdn = Jdn((icu.to_rata_die().to_i64_date() + RD_TO_JDN) as i32);
        assert_eq!(JalaliDate::new(year, 1, 1).unwrap().to_jdn(), jdn, "Nowruz {year}");
    }
}
