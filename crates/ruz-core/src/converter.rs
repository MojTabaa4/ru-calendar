//! View models for the date converter window.

use serde::Serialize;

use crate::convert::{self, day_info};
use crate::day::{CalendarKind, DateError};
use crate::format::{self, Locale};
use crate::month::{CalendarContext, DayDetails, day_details};
use crate::observances::ObservanceDb;

/// What the converter's month and day pickers need for one year of one calendar.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MonthsView {
    pub kind: CalendarKind,
    pub year: i32,
    pub year_label: String,
    pub names: Vec<String>,
    /// English Gregorian month names to show beside the Persian ones; empty otherwise.
    pub alt_names: Vec<String>,
    /// Days in each month; Hijri lengths follow the Iran table and offset.
    pub lengths: Vec<u8>,
    /// "۱" … "۳۱" in the chosen digits.
    pub day_labels: Vec<String>,
}

/// Errors with [`DateError::OutOfRange`] when the year can't be shown in all three calendars.
pub fn months_view(ctx: &CalendarContext, kind: CalendarKind, year: i32) -> Result<MonthsView, DateError> {
    // Both ends of the year must convert, or the pickers would offer dates that can't.
    let first = convert::to_jdn(kind, year, 1, 1, &ctx.hijri).map_err(|_| DateError::OutOfRange(year))?;
    let last_len = convert::month_len(kind, year, 12, &ctx.hijri)?;
    let last =
        convert::to_jdn(kind, year, 12, last_len, &ctx.hijri).map_err(|_| DateError::OutOfRange(year))?;
    day_info(first, &ctx.hijri).map_err(|_| DateError::OutOfRange(year))?;
    day_info(last, &ctx.hijri).map_err(|_| DateError::OutOfRange(year))?;

    Ok(MonthsView {
        kind,
        year,
        year_label: format::number(year, ctx.digits),
        names: (1..=12).map(|m| format::month_name(kind, m, ctx.locale).to_owned()).collect(),
        alt_names: if kind == CalendarKind::Gregorian && ctx.locale == Locale::Fa {
            (1..=12).map(|m| format::month_name(kind, m, Locale::En).to_owned()).collect()
        } else {
            Vec::new()
        },
        lengths: (1..=12).map(|m| convert::month_len(kind, year, m, &ctx.hijri)).collect::<Result<_, _>>()?,
        day_labels: (1..=31).map(|d| format::number(d, ctx.digits)).collect(),
    })
}

/// The day `year-month-day` in `kind`, seen in all three calendars.
pub fn convert_date(
    ctx: &CalendarContext,
    db: &ObservanceDb,
    kind: CalendarKind,
    year: i32,
    month: u8,
    day: u8,
) -> Result<DayDetails, DateError> {
    day_details(ctx, db, convert::to_jdn(kind, year, month, day, &ctx.hijri)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> CalendarContext {
        CalendarContext::default()
    }

    #[test]
    fn persian_months_follow_leap_years() {
        let leap = months_view(&ctx(), CalendarKind::Persian, 1403).unwrap();
        let common = months_view(&ctx(), CalendarKind::Persian, 1404).unwrap();
        assert_eq!(leap.lengths[11], 30);
        assert_eq!(common.lengths[11], 29);
        assert_eq!(common.lengths[0], 31);
        assert_eq!(common.names[6], "مهر");
        assert_eq!(common.year_label, "۱۴۰۴");
        assert_eq!(common.day_labels.len(), 31);
        assert_eq!(common.day_labels[30], "۳۱");
    }

    #[test]
    fn hijri_months_use_irans_lengths() {
        // From the Iran table: Safar 1447 had 30 days, Muharram 1448 had 30.
        let view = months_view(&ctx(), CalendarKind::Hijri, 1447).unwrap();
        assert_eq!(view.lengths[1], 30);
        assert_eq!(view.names[8], "رمضان");
        assert!(view.lengths.iter().all(|l| (29..=30).contains(l)));
    }

    #[test]
    fn gregorian_february() {
        let view = months_view(&ctx(), CalendarKind::Gregorian, 2024).unwrap();
        assert_eq!(view.lengths[1], 29);
        assert_eq!(view.names[1], "فوریه");
        assert_eq!(view.alt_names[1], "February");
        assert!(months_view(&ctx(), CalendarKind::Persian, 1404).unwrap().alt_names.is_empty());
    }

    #[test]
    fn years_that_cannot_convert_are_rejected() {
        assert_eq!(
            months_view(&ctx(), CalendarKind::Persian, 5000).unwrap_err(),
            DateError::OutOfRange(5000)
        );
        assert!(months_view(&ctx(), CalendarKind::Gregorian, 9000).is_err());
        assert!(months_view(&ctx(), CalendarKind::Hijri, 9000).is_err());
    }

    #[test]
    fn converts_a_date() {
        let d = convert_date(&ctx(), ObservanceDb::embedded(), CalendarKind::Gregorian, 2026, 3, 21).unwrap();
        assert_eq!(d.persian, "۱ فروردین ۱۴۰۵");
        assert_eq!(d.hijri, "۱ شوال ۱۴۴۷");
        assert!(!d.hijri_approximate);
        assert!(d.observances.iter().any(|o| o.id == "eid-al-fitr"));
        assert!(convert_date(&ctx(), ObservanceDb::embedded(), CalendarKind::Persian, 1404, 12, 30).is_err());
    }
}
