//! Names, digits and date strings. The UI never formats dates itself; it shows these.

use serde::{Deserialize, Serialize};

use crate::convert::DayInfo;
use crate::day::{CalendarKind, Weekday};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Locale {
    #[default]
    Fa,
    En,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Digits {
    #[default]
    Persian,
    Latin,
}

const PERSIAN_DIGITS: [char; 10] = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];

/// Replaces ASCII digits with Persian ones when `digits` is [`Digits::Persian`].
pub fn localize_digits(s: &str, digits: Digits) -> String {
    match digits {
        Digits::Latin => s.to_owned(),
        Digits::Persian => {
            s.chars().map(|c| c.to_digit(10).map_or(c, |d| PERSIAN_DIGITS[d as usize])).collect()
        }
    }
}

pub fn number(n: impl std::fmt::Display, digits: Digits) -> String {
    localize_digits(&n.to_string(), digits)
}

const PERSIAN_MONTHS_FA: [&str; 12] =
    ["فروردین", "اردیبهشت", "خرداد", "تیر", "مرداد", "شهریور", "مهر", "آبان", "آذر", "دی", "بهمن", "اسفند"];
const PERSIAN_MONTHS_EN: [&str; 12] = [
    "Farvardin",
    "Ordibehesht",
    "Khordad",
    "Tir",
    "Mordad",
    "Shahrivar",
    "Mehr",
    "Aban",
    "Azar",
    "Dey",
    "Bahman",
    "Esfand",
];
const GREGORIAN_MONTHS_FA: [&str; 12] = [
    "ژانویه",
    "فوریه",
    "مارس",
    "آوریل",
    "مه",
    "ژوئن",
    "ژوئیه",
    "اوت",
    "سپتامبر",
    "اکتبر",
    "نوامبر",
    "دسامبر",
];
const GREGORIAN_MONTHS_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const HIJRI_MONTHS_FA: [&str; 12] = [
    "محرم",
    "صفر",
    "ربیع‌الاول",
    "ربیع‌الثانی",
    "جمادی‌الاول",
    "جمادی‌الثانی",
    "رجب",
    "شعبان",
    "رمضان",
    "شوال",
    "ذی‌القعده",
    "ذی‌الحجه",
];
const HIJRI_MONTHS_EN: [&str; 12] = [
    "Muharram",
    "Safar",
    "Rabi' al-Awwal",
    "Rabi' al-Thani",
    "Jumada al-Awwal",
    "Jumada al-Thani",
    "Rajab",
    "Sha'ban",
    "Ramadan",
    "Shawwal",
    "Dhu al-Qa'dah",
    "Dhu al-Hijjah",
];

const WEEKDAYS_FA: [&str; 7] = ["شنبه", "یکشنبه", "دوشنبه", "سه‌شنبه", "چهارشنبه", "پنجشنبه", "جمعه"];
const WEEKDAYS_SHORT_FA: [&str; 7] = ["ش", "ی", "د", "س", "چ", "پ", "ج"];
const WEEKDAYS_EN: [&str; 7] = ["Saturday", "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];
const WEEKDAYS_SHORT_EN: [&str; 7] = ["Sa", "Su", "Mo", "Tu", "We", "Th", "Fr"];

const GREGORIAN_MONTHS_SHORT_EN: [&str; 12] =
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// "Oct": the short English name of Gregorian month `month` (1–12).
pub fn gregorian_short_en(month: u8) -> &'static str {
    GREGORIAN_MONTHS_SHORT_EN[usize::from(month - 1)]
}

/// Name of month `month` (1–12). Panics on other values.
pub fn month_name(kind: CalendarKind, month: u8, locale: Locale) -> &'static str {
    let names = match (kind, locale) {
        (CalendarKind::Persian, Locale::Fa) => &PERSIAN_MONTHS_FA,
        (CalendarKind::Persian, Locale::En) => &PERSIAN_MONTHS_EN,
        (CalendarKind::Gregorian, Locale::Fa) => &GREGORIAN_MONTHS_FA,
        (CalendarKind::Gregorian, Locale::En) => &GREGORIAN_MONTHS_EN,
        (CalendarKind::Hijri, Locale::Fa) => &HIJRI_MONTHS_FA,
        (CalendarKind::Hijri, Locale::En) => &HIJRI_MONTHS_EN,
    };
    names[usize::from(month - 1)]
}

pub fn weekday_name(weekday: Weekday, locale: Locale) -> &'static str {
    let names = if locale == Locale::Fa { &WEEKDAYS_FA } else { &WEEKDAYS_EN };
    names[usize::from(weekday.index())]
}

pub fn weekday_short(weekday: Weekday, locale: Locale) -> &'static str {
    let names = if locale == Locale::Fa { &WEEKDAYS_SHORT_FA } else { &WEEKDAYS_SHORT_EN };
    names[usize::from(weekday.index())]
}

/// "۲۳ مهر ۱۴۰۴", or "23 Mehr 1404" in English.
pub fn format_date(
    kind: CalendarKind,
    year: i32,
    month: u8,
    day: u8,
    locale: Locale,
    digits: Digits,
) -> String {
    format!("{} {} {}", number(day, digits), month_name(kind, month, locale), number(year, digits))
}

/// The date in `kind` with its weekday, e.g. "چهارشنبه ۲۳ مهر ۱۴۰۴".
pub fn format_long(day: &DayInfo, kind: CalendarKind, locale: Locale, digits: Digits) -> String {
    let (y, m, d) = day.ymd(kind);
    let weekday = weekday_name(day.weekday, locale);
    let date = format_date(kind, y, m, d, locale, digits);
    if locale == Locale::Fa { format!("{weekday} {date}") } else { format!("{weekday}, {date}") }
}

/// "مهر ۱۴۰۴".
pub fn month_title(kind: CalendarKind, year: i32, month: u8, locale: Locale, digits: Digits) -> String {
    format!("{} {}", month_name(kind, month, locale), number(year, digits))
}

/// Text of the menu-bar / tray title.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TrayFormat {
    pub show_weekday: bool,
    pub show_month: bool,
    pub show_year: bool,
    pub locale: Locale,
    pub digits: Digits,
}

impl Default for TrayFormat {
    fn default() -> Self {
        TrayFormat {
            show_weekday: true,
            show_month: true,
            show_year: false,
            locale: Locale::Fa,
            digits: Digits::Persian,
        }
    }
}

/// e.g. "چهارشنبه ۲۳ مهر", with an optional "۱۴:۳۵" clock (hour, minute) appended.
pub fn tray_title(day: &DayInfo, clock: Option<(u8, u8)>, f: &TrayFormat) -> String {
    let p = day.persian;
    let mut parts = Vec::new();
    if f.show_weekday {
        parts.push(weekday_name(day.weekday, f.locale).to_owned());
    }
    parts.push(number(p.day, f.digits));
    if f.show_month {
        parts.push(month_name(CalendarKind::Persian, p.month, f.locale).to_owned());
    }
    if f.show_year {
        parts.push(number(p.year, f.digits));
    }
    if let Some((h, m)) = clock {
        parts.push(localize_digits(&format!("{h:02}:{m:02}"), f.digits));
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits() {
        assert_eq!(localize_digits("1404-07-23", Digits::Persian), "۱۴۰۴-۰۷-۲۳");
        assert_eq!(localize_digits("1404", Digits::Latin), "1404");
        assert_eq!(number(9, Digits::Persian), "۹");
    }

    #[test]
    fn names() {
        assert_eq!(month_name(CalendarKind::Persian, 7, Locale::Fa), "مهر");
        assert_eq!(month_name(CalendarKind::Hijri, 9, Locale::En), "Ramadan");
        assert_eq!(weekday_name(Weekday::Friday, Locale::Fa), "جمعه");
        assert_eq!(weekday_short(Weekday::Saturday, Locale::Fa), "ش");
    }
}
