//! Ready-to-render view models for the month popover.

use serde::Serialize;

use crate::convert::{DayInfo, day_info};
use crate::day::{CalendarKind, DateError, GregorianDate, HijriDate, JalaliDate, Jdn, Weekday};
use crate::format::{self, Digits, Locale};
use crate::hijri::{HijriCalendar, HijriSource};
use crate::jalali;
use crate::observances::{Category, DateStatus, ObservanceDb, ObservanceFilter, Subcategory};

/// User choices that affect how days are computed and shown.
#[derive(Clone, Debug)]
pub struct CalendarContext {
    pub hijri: HijriCalendar,
    pub filter: ObservanceFilter,
    pub weekend: Vec<Weekday>,
    pub locale: Locale,
    pub digits: Digits,
    pub show_gregorian: bool,
    pub show_hijri: bool,
}

impl Default for CalendarContext {
    fn default() -> Self {
        CalendarContext {
            hijri: HijriCalendar::default(),
            filter: ObservanceFilter::default(),
            weekend: vec![Weekday::Friday],
            locale: Locale::Fa,
            digits: Digits::Persian,
            show_gregorian: true,
            show_hijri: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct YearMonth {
    pub year: i32,
    pub month: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct WeekdayHeader {
    pub name: String,
    pub short: String,
    pub weekend: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DayCell {
    pub jdn: Jdn,
    pub label: String,
    pub gregorian_label: Option<String>,
    pub hijri_label: Option<String>,
    /// On the 1st of a Gregorian month (when shown): that month, e.g. "نوامبر · November".
    pub gregorian_month_start: Option<String>,
    pub in_month: bool,
    pub is_today: bool,
    pub status: DateStatus,
    pub mourning: bool,
    pub has_observance: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MonthGrid {
    pub year: i32,
    pub month: u8,
    /// "مهر ۱۴۰۴"
    pub title: String,
    /// "مهر"
    pub month_name: String,
    /// "۱۴۰۴"
    pub year_label: String,
    /// "سپتامبر – اکتبر ۲۰۲۵"
    pub gregorian_span: String,
    /// "Sep – Oct", the English names beside the Persian ones; `None` in English.
    pub gregorian_span_en: Option<String>,
    /// "ربیع‌الاول – ربیع‌الثانی ۱۴۴۷"
    pub hijri_span: String,
    pub weekdays: Vec<WeekdayHeader>,
    /// Always six weeks (42 days), starting on the Saturday on or before the 1st.
    pub cells: Vec<DayCell>,
    pub prev: YearMonth,
    pub next: YearMonth,
    pub contains_today: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ObservanceView {
    pub id: String,
    pub title: String,
    pub calendar: CalendarKind,
    pub category: Category,
    pub subcategory: Subcategory,
    pub day_off: bool,
    pub mourning: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DayDetails {
    pub jdn: Jdn,
    pub weekday: String,
    pub persian: String,
    pub gregorian: String,
    /// "October 15, 2026", beside the Persian form; `None` in English.
    pub gregorian_en: Option<String>,
    pub hijri: String,
    pub hijri_approximate: bool,
    pub status: DateStatus,
    pub mourning: bool,
    pub observances: Vec<ObservanceView>,
    pub persian_date: JalaliDate,
    pub gregorian_date: GregorianDate,
    pub hijri_date: HijriDate,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TodayView {
    pub jdn: Jdn,
    pub persian: JalaliDate,
    /// "چهارشنبه ۲۳ مهر ۱۴۰۴"
    pub title: String,
}

pub fn today_view(ctx: &CalendarContext, today: Jdn) -> Result<TodayView, DateError> {
    let info = day_info(today, &ctx.hijri)?;
    Ok(TodayView {
        jdn: today,
        persian: info.persian,
        title: format::format_long(&info, CalendarKind::Persian, ctx.locale, ctx.digits),
    })
}

fn shift_month(year: i32, month: u8, delta: i32) -> YearMonth {
    let index = year * 12 + i32::from(month) - 1 + delta;
    YearMonth { year: index.div_euclid(12), month: (index.rem_euclid(12) + 1) as u8 }
}

/// "اکتبر ۲۰۲۵", "سپتامبر – اکتبر ۲۰۲۵" or "دسامبر ۲۰۲۵ – ژانویه ۲۰۲۶".
fn span(kind: CalendarKind, first: (i32, u8), last: (i32, u8), locale: Locale, digits: Digits) -> String {
    let name = |m| format::month_name(kind, m, locale);
    let year = |y| format::number(y, digits);
    if first == last {
        format!("{} {}", name(first.1), year(first.0))
    } else if first.0 == last.0 {
        format!("{} – {} {}", name(first.1), name(last.1), year(last.0))
    } else {
        format!("{} {} – {} {}", name(first.1), year(first.0), name(last.1), year(last.0))
    }
}

pub fn month_grid(
    ctx: &CalendarContext,
    db: &ObservanceDb,
    year: i32,
    month: u8,
    today: Jdn,
) -> Result<MonthGrid, DateError> {
    let first = JalaliDate::new(year, month, 1)?.to_jdn();
    let last = first + i32::from(jalali::month_len(year, month)) - 1;
    let grid_start = first - i32::from(first.weekday().index());

    let mut cells = Vec::with_capacity(42);
    for offset in 0..42 {
        let info = day_info(grid_start + offset, &ctx.hijri)?;
        let found = db.on(&info, &ctx.hijri, &ctx.filter);
        let (status, mourning) = ObservanceDb::summarize(&found, info.weekday, &ctx.weekend);
        cells.push(DayCell {
            jdn: info.jdn,
            label: format::number(info.persian.day, ctx.digits),
            gregorian_label: ctx.show_gregorian.then(|| info.gregorian.day.to_string()),
            hijri_label: ctx.show_hijri.then(|| format::number(info.hijri.day, ctx.digits)),
            gregorian_month_start: (ctx.show_gregorian && info.gregorian.day == 1).then(|| {
                let name = format::month_name(CalendarKind::Gregorian, info.gregorian.month, ctx.locale);
                match ctx.locale {
                    Locale::Fa => {
                        format!(
                            "{name} · {}",
                            format::month_name(CalendarKind::Gregorian, info.gregorian.month, Locale::En)
                        )
                    }
                    Locale::En => name.to_owned(),
                }
            }),
            in_month: info.persian.month == month,
            is_today: info.jdn == today,
            status,
            mourning,
            has_observance: !found.is_empty(),
        });
    }

    let (first_info, last_info) = (day_info(first, &ctx.hijri)?, day_info(last, &ctx.hijri)?);
    let ym = |d: &DayInfo, kind| {
        let (y, m, _) = d.ymd(kind);
        (y, m)
    };
    Ok(MonthGrid {
        year,
        month,
        title: format::month_title(CalendarKind::Persian, year, month, ctx.locale, ctx.digits),
        month_name: format::month_name(CalendarKind::Persian, month, ctx.locale).to_owned(),
        year_label: format::number(year, ctx.digits),
        gregorian_span_en: (ctx.locale == Locale::Fa).then(|| {
            let (first, last) = (first_info.gregorian.month, last_info.gregorian.month);
            if first == last {
                format::gregorian_short_en(first).to_owned()
            } else {
                format!("{} – {}", format::gregorian_short_en(first), format::gregorian_short_en(last))
            }
        }),
        gregorian_span: span(
            CalendarKind::Gregorian,
            ym(&first_info, CalendarKind::Gregorian),
            ym(&last_info, CalendarKind::Gregorian),
            ctx.locale,
            ctx.digits,
        ),
        hijri_span: span(
            CalendarKind::Hijri,
            ym(&first_info, CalendarKind::Hijri),
            ym(&last_info, CalendarKind::Hijri),
            ctx.locale,
            ctx.digits,
        ),
        weekdays: Weekday::ALL
            .iter()
            .map(|&w| WeekdayHeader {
                name: format::weekday_name(w, ctx.locale).to_owned(),
                short: format::weekday_short(w, ctx.locale).to_owned(),
                weekend: ctx.weekend.contains(&w),
            })
            .collect(),
        cells,
        prev: shift_month(year, month, -1),
        next: shift_month(year, month, 1),
        contains_today: (first..=last).contains(&today),
    })
}

pub fn day_details(ctx: &CalendarContext, db: &ObservanceDb, jdn: Jdn) -> Result<DayDetails, DateError> {
    let info = day_info(jdn, &ctx.hijri)?;
    let found = db.on(&info, &ctx.hijri, &ctx.filter);
    let (status, mourning) = ObservanceDb::summarize(&found, info.weekday, &ctx.weekend);
    let date = |kind| {
        let (y, m, d) = info.ymd(kind);
        format::format_date(kind, y, m, d, ctx.locale, ctx.digits)
    };
    Ok(DayDetails {
        jdn,
        weekday: format::weekday_name(info.weekday, ctx.locale).to_owned(),
        persian: date(CalendarKind::Persian),
        gregorian: date(CalendarKind::Gregorian),
        gregorian_en: (ctx.locale == Locale::Fa).then(|| {
            let g = info.gregorian;
            format!(
                "{} {}, {}",
                format::month_name(CalendarKind::Gregorian, g.month, Locale::En),
                g.day,
                g.year
            )
        }),
        hijri: date(CalendarKind::Hijri),
        hijri_approximate: info.hijri_source != HijriSource::IranTable,
        status,
        mourning,
        observances: found
            .into_iter()
            .map(|o| ObservanceView {
                id: o.id.clone(),
                title: if ctx.locale == Locale::Fa { o.title_fa.clone() } else { o.title_en.clone() },
                calendar: o.calendar,
                category: o.category,
                subcategory: o.subcategory,
                day_off: o.day_off,
                mourning: o.mourning,
            })
            .collect(),
        persian_date: info.persian,
        gregorian_date: info.gregorian,
        hijri_date: info.hijri,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_layout() {
        let ctx = CalendarContext::default();
        let db = ObservanceDb::embedded();
        let today = JalaliDate::new(1404, 7, 23).unwrap().to_jdn();
        let grid = month_grid(&ctx, db, 1404, 7, today).unwrap();
        assert_eq!(grid.cells.len(), 42);
        assert_eq!(grid.title, "مهر ۱۴۰۴");
        assert_eq!(grid.gregorian_span, "سپتامبر – اکتبر ۲۰۲۵");
        assert_eq!(grid.gregorian_span_en.as_deref(), Some("Sep – Oct"));
        assert_eq!((grid.month_name.as_str(), grid.year_label.as_str()), ("مهر", "۱۴۰۴"));
        // 1 Mehr 1404 is a Tuesday, so the grid starts three days earlier.
        assert!(!grid.cells[2].in_month);
        assert!(grid.cells[3].in_month && grid.cells[3].label == "۱");
        assert_eq!(grid.cells.iter().filter(|c| c.in_month).count(), 30);
        assert_eq!(grid.cells.iter().filter(|c| c.is_today).count(), 1);
        assert!(grid.contains_today);
        assert_eq!(
            (grid.prev, grid.next),
            (YearMonth { year: 1404, month: 6 }, YearMonth { year: 1404, month: 8 })
        );
        assert_eq!(grid.weekdays[6].short, "ج");
        // 9 Mehr 1404 is 1 October 2025.
        let first_of = |label: &str| grid.cells.iter().find(|c| c.in_month && c.label == label).unwrap();
        assert_eq!(first_of("۹").gregorian_month_start.as_deref(), Some("اکتبر · October"));
        assert_eq!(grid.cells.iter().filter(|c| c.in_month && c.gregorian_month_start.is_some()).count(), 1);
        assert!(grid.weekdays[6].weekend);
    }

    #[test]
    fn year_boundaries() {
        assert_eq!(shift_month(1404, 12, 1), YearMonth { year: 1405, month: 1 });
        assert_eq!(shift_month(1404, 1, -1), YearMonth { year: 1403, month: 12 });
    }

    #[test]
    fn day_details_of_nowruz() {
        let ctx = CalendarContext::default();
        let jdn = JalaliDate::new(1404, 1, 1).unwrap().to_jdn();
        let d = day_details(&ctx, ObservanceDb::embedded(), jdn).unwrap();
        assert_eq!(d.persian, "۱ فروردین ۱۴۰۴");
        assert_eq!(d.gregorian, "۲۱ مارس ۲۰۲۵");
        assert_eq!(d.gregorian_en.as_deref(), Some("March 21, 2025"));
        assert_eq!(d.weekday, "جمعه");
        assert_eq!(d.status, DateStatus::DayOff);
        assert!(!d.observances.is_empty());
    }
}
