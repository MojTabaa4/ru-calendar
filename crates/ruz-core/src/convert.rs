//! One day seen in all three calendars, and conversion between them.

use serde::Serialize;

use crate::day::{CalendarKind, DateError, GregorianDate, HijriDate, JalaliDate, Jdn, Weekday};
use crate::hijri::{HijriCalendar, HijriSource};
use crate::{gregorian, jalali};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DayInfo {
    pub jdn: Jdn,
    pub weekday: Weekday,
    pub persian: JalaliDate,
    pub gregorian: GregorianDate,
    pub hijri: HijriDate,
    pub hijri_source: HijriSource,
}

impl DayInfo {
    pub fn ymd(&self, kind: CalendarKind) -> (i32, u8, u8) {
        match kind {
            CalendarKind::Persian => (self.persian.year, self.persian.month, self.persian.day),
            CalendarKind::Gregorian => (self.gregorian.year, self.gregorian.month, self.gregorian.day),
            CalendarKind::Hijri => (self.hijri.year, self.hijri.month, self.hijri.day),
        }
    }
}

pub fn day_info(jdn: Jdn, hijri: &HijriCalendar) -> Result<DayInfo, DateError> {
    let (hijri_date, hijri_source) = hijri.from_jdn(jdn);
    Ok(DayInfo {
        jdn,
        weekday: jdn.weekday(),
        persian: JalaliDate::from_jdn(jdn)?,
        gregorian: GregorianDate::from_jdn(jdn),
        hijri: hijri_date,
        hijri_source,
    })
}

pub fn to_jdn(
    kind: CalendarKind,
    year: i32,
    month: u8,
    day: u8,
    hijri: &HijriCalendar,
) -> Result<Jdn, DateError> {
    match kind {
        CalendarKind::Persian => Ok(JalaliDate::new(year, month, day)?.to_jdn()),
        CalendarKind::Gregorian => Ok(GregorianDate::new(year, month, day)?.to_jdn()),
        CalendarKind::Hijri => hijri.to_jdn(HijriDate { year, month, day }),
    }
}

pub fn convert(
    kind: CalendarKind,
    year: i32,
    month: u8,
    day: u8,
    hijri: &HijriCalendar,
) -> Result<DayInfo, DateError> {
    day_info(to_jdn(kind, year, month, day, hijri)?, hijri)
}

pub fn month_len(kind: CalendarKind, year: i32, month: u8, hijri: &HijriCalendar) -> Result<u8, DateError> {
    if !(1..=12).contains(&month) {
        return Err(DateError::InvalidMonth(month));
    }
    Ok(match kind {
        CalendarKind::Persian => jalali::month_len(year, month),
        CalendarKind::Gregorian => gregorian::month_len(year, month),
        CalendarKind::Hijri => hijri.month_len(year, month),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_between_calendars() {
        let h = HijriCalendar::default();
        let day = convert(CalendarKind::Persian, 1404, 1, 1, &h).unwrap();
        assert_eq!(day.gregorian, GregorianDate { year: 2025, month: 3, day: 21 });
        assert_eq!(day.weekday, Weekday::Friday);
        let back = convert(CalendarKind::Gregorian, 2025, 3, 21, &h).unwrap();
        assert_eq!(back.persian, JalaliDate { year: 1404, month: 1, day: 1 });
        let via_hijri =
            convert(CalendarKind::Hijri, day.hijri.year, day.hijri.month, day.hijri.day, &h).unwrap();
        assert_eq!(via_hijri.jdn, day.jdn);
    }

    #[test]
    fn rejects_invalid_input() {
        let h = HijriCalendar::default();
        assert!(convert(CalendarKind::Persian, 1404, 13, 1, &h).is_err());
        assert!(convert(CalendarKind::Gregorian, 2025, 2, 30, &h).is_err());
        assert!(convert(CalendarKind::Hijri, 1447, 1, 31, &h).is_err());
        assert!(month_len(CalendarKind::Persian, 1404, 0, &h).is_err());
    }
}
