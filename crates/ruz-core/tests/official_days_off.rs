//! Every day of the covered years is a day off exactly when Iran's official calendar says so.

use std::collections::{BTreeMap, BTreeSet};

use ruz_core::observances::DateStatus;
use ruz_core::{CalendarContext, JalaliDate, Jdn, ObservanceDb, day_details};
use serde::Deserialize;

#[derive(Deserialize)]
struct Year {
    days: Vec<String>,
}

#[test]
fn days_off_match_the_official_calendar() {
    let fixture: BTreeMap<i32, Year> =
        toml::from_str(include_str!("fixtures/official_days_off.toml")).expect("valid fixture");
    let ctx = CalendarContext { weekend: vec![], ..CalendarContext::default() };
    let db = ObservanceDb::embedded();

    for (year, expected) in fixture {
        let expected: BTreeSet<String> = expected.days.into_iter().collect();
        let first = JalaliDate::new(year, 1, 1).unwrap().to_jdn();
        let next = JalaliDate::new(year + 1, 1, 1).unwrap().to_jdn();
        let ours: BTreeSet<String> = (first.0..next.0)
            .map(Jdn)
            .filter(|&jdn| day_details(&ctx, db, jdn).unwrap().status == DateStatus::DayOff)
            .map(|jdn| {
                let d = JalaliDate::from_jdn(jdn).unwrap();
                format!("{:02}-{:02}", d.month, d.day)
            })
            .collect();
        let missing: Vec<_> = expected.difference(&ours).collect();
        let extra: Vec<_> = ours.difference(&expected).collect();
        assert!(missing.is_empty() && extra.is_empty(), "{year}: missing {missing:?}, extra {extra:?}");
    }
}
