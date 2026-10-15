use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::{Category, DataError, DateRule, DateStatus, Observance, ObservanceFilter, Subcategory};
use crate::convert::{DayInfo, day_info, month_len};
use crate::day::{CalendarKind, GregorianDate, Jdn, Weekday};
use crate::hijri::HijriCalendar;

#[derive(Debug, Default)]
pub struct ObservanceDb {
    rows: Vec<Observance>,
    /// (row index, year in the row's own calendar) → the day it was actually observed.
    moved: HashMap<(usize, i32), Jdn>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    /// Used by groups and rows without their own `source`.
    default_source: String,
    #[serde(default)]
    group: Vec<Group>,
    #[serde(default)]
    moved: Vec<Moved>,
}

/// Rows sharing a calendar and category; `day_off`, `mourning` and `source` are
/// defaults the rows can override.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    calendar: CalendarKind,
    category: Category,
    subcategory: Subcategory,
    #[serde(default)]
    day_off: bool,
    #[serde(default)]
    mourning: bool,
    source: Option<String>,
    observance: Vec<Row>,
}

/// A year in which the official calendar puts an observance somewhere other than its
/// rule says, e.g. because the calendar was printed before a moon sighting.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Moved {
    id: String,
    /// The year in the observance's own calendar.
    year: i32,
    /// Gregorian YYYY-MM-DD.
    date: String,
    #[allow(dead_code)]
    source: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    id: String,
    month: u8,
    day: Option<DaySpec>,
    weekday: Option<Weekday>,
    nth: Option<i8>,
    offset: Option<i8>,
    day_off: Option<bool>,
    mourning: Option<bool>,
    fa: String,
    en: String,
    since: Option<i32>,
    until: Option<i32>,
    source: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum DaySpec {
    Number(u8),
    Word(DayWord),
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DayWord {
    Last,
}

fn parse_gregorian(s: &str) -> Option<Jdn> {
    let mut parts = s.split('-').map(str::parse::<i32>);
    let (Some(Ok(y)), Some(Ok(m)), Some(Ok(d)), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    Some(GregorianDate::new(y, u8::try_from(m).ok()?, u8::try_from(d).ok()?).ok()?.to_jdn())
}

impl Row {
    fn into_observance(self, group: &Group, default_source: &str) -> Result<Observance, DataError> {
        let invalid = |reason: &str| DataError::Invalid { id: self.id.clone(), reason: reason.into() };
        if !(1..=12).contains(&self.month) {
            return Err(invalid("month must be 1–12"));
        }
        let max_day = match group.calendar {
            CalendarKind::Persian => 31,
            CalendarKind::Gregorian => 31,
            CalendarKind::Hijri => 30,
        };
        let rule = match (&self.day, self.weekday, self.nth) {
            (Some(DaySpec::Number(day)), None, None) if (1..=max_day).contains(day) => {
                DateRule::Fixed { month: self.month, day: *day }
            }
            (Some(DaySpec::Word(DayWord::Last)), None, None) => {
                DateRule::LastDayOfMonth { month: self.month }
            }
            (None, Some(weekday), Some(n)) if (1..=5).contains(&n) || n == -1 => {
                DateRule::NthWeekday { month: self.month, weekday, n }
            }
            _ => return Err(invalid("needs `day` (1–31 or \"last\"), or `weekday` with `nth` (1–5 or -1)")),
        };
        if self.fa.trim().is_empty() || self.en.trim().is_empty() {
            return Err(invalid("needs both `fa` and `en` titles"));
        }
        let source = self.source.as_deref().or(group.source.as_deref()).unwrap_or(default_source).to_owned();
        if source.trim().is_empty() {
            return Err(invalid("needs a source"));
        }
        Ok(Observance {
            id: self.id,
            calendar: group.calendar,
            category: group.category,
            subcategory: group.subcategory,
            rule,
            offset: self.offset.unwrap_or(0),
            day_off: self.day_off.unwrap_or(group.day_off),
            mourning: self.mourning.unwrap_or(group.mourning),
            title_fa: self.fa,
            title_en: self.en,
            since: self.since,
            until: self.until,
            source,
        })
    }
}

impl ObservanceDb {
    /// The observances shipped with Ruz (`data/observances.toml`).
    pub fn embedded() -> &'static ObservanceDb {
        static DB: OnceLock<ObservanceDb> = OnceLock::new();
        DB.get_or_init(|| {
            ObservanceDb::from_toml(include_str!("../../data/observances.toml"))
                .expect("embedded data/observances.toml is valid (checked by tests)")
        })
    }

    pub fn from_toml(source: &str) -> Result<Self, DataError> {
        let file: File = toml::from_str(source)?;
        let mut rows: Vec<Observance> = Vec::new();
        for group in &file.group {
            for row in &group.observance {
                if rows.iter().any(|r| r.id == row.id) {
                    return Err(DataError::Invalid { id: row.id.clone(), reason: "duplicate id".into() });
                }
            }
            for row in group.observance.iter().cloned() {
                rows.push(row.into_observance(group, &file.default_source)?);
            }
        }
        let mut moved = HashMap::new();
        for m in file.moved {
            let invalid = |reason: &str| DataError::Invalid { id: m.id.clone(), reason: reason.into() };
            let index = rows.iter().position(|r| r.id == m.id).ok_or_else(|| invalid("moved: unknown id"))?;
            let date = parse_gregorian(&m.date).ok_or_else(|| invalid("moved: date must be YYYY-MM-DD"))?;
            if moved.insert((index, m.year), date).is_some() {
                return Err(invalid("moved twice in the same year"));
            }
        }
        Ok(ObservanceDb { rows, moved })
    }

    pub fn all(&self) -> &[Observance] {
        &self.rows
    }

    /// Observances on `day` that `filter` lets through, in data-file order.
    pub fn on<'a>(
        &'a self,
        day: &DayInfo,
        hijri: &HijriCalendar,
        filter: &ObservanceFilter,
    ) -> Vec<&'a Observance> {
        let year = day.persian.year;
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, o)| filter.allows(o.category, o.subcategory))
            .filter(|(_, o)| o.since.is_none_or(|s| year >= s) && o.until.is_none_or(|u| year <= u))
            .filter(|&(i, o)| self.falls_on(i, o, day, hijri))
            .map(|(_, o)| o)
            .collect()
    }

    fn falls_on(&self, index: usize, o: &Observance, day: &DayInfo, hijri: &HijriCalendar) -> bool {
        // The rule describes the day `offset` days before the observance.
        let rule_day = if o.offset == 0 {
            *day
        } else {
            match day_info(day.jdn - i32::from(o.offset), hijri) {
                Ok(d) => d,
                Err(_) => return false,
            }
        };
        let (y, m, d) = rule_day.ymd(o.calendar);
        if let Some(&moved_to) = self.moved.get(&(index, y)) {
            return moved_to == day.jdn;
        }
        let len = if o.rule.needs_month_len() { month_len(o.calendar, y, m, hijri).unwrap_or(31) } else { 0 };
        o.rule.matches(m, d, len, rule_day.weekday)
    }

    /// The day's status and whether it is a day of mourning.
    pub fn status(
        &self,
        day: &DayInfo,
        hijri: &HijriCalendar,
        filter: &ObservanceFilter,
        weekend: &[Weekday],
    ) -> (DateStatus, bool) {
        Self::summarize(&self.on(day, hijri, filter), day.weekday, weekend)
    }

    /// [`ObservanceDb::status`] for observances already looked up with [`ObservanceDb::on`].
    pub fn summarize(found: &[&Observance], weekday: Weekday, weekend: &[Weekday]) -> (DateStatus, bool) {
        let mourning = found.iter().any(|o| o.mourning);
        let status = if found.iter().any(|o| o.day_off) {
            DateStatus::DayOff
        } else if weekend.contains(&weekday) {
            DateStatus::Weekend
        } else {
            DateStatus::Normal
        };
        (status, mourning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::convert;

    const SAMPLE: &str = r#"
        default_source = "test"

        [[group]]
        calendar = "persian"
        category = "iranOfficial"
        subcategory = "main"
        day_off = true
        observance = [
          { id = "nowruz-1", month = 1, day = 1, fa = "نوروز", en = "Nowruz" },
        ]

        [[group]]
        calendar = "hijri"
        category = "iranOfficial"
        subcategory = "main"
        day_off = true
        mourning = true
        observance = [
          { id = "end-of-safar", month = 2, day = "last", fa = "پایان صفر", en = "End of Safar" },
        ]

        [[group]]
        calendar = "persian"
        category = "iranHistorical"
        subcategory = "main"
        observance = [
          { id = "esfand-30", month = 12, day = 30, fa = "سی اسفند", en = "30 Esfand", since = 1400 },
          { id = "suri", month = 12, weekday = "wednesday", nth = -1, offset = -1, fa = "چهارشنبه‌سوری", en = "Chaharshanbe Suri" },
        ]

        [[moved]]
        id = "nowruz-1"
        year = 1410
        date = "2031-03-22"
        source = "test"
    "#;

    fn day(kind: CalendarKind, y: i32, m: u8, d: u8) -> DayInfo {
        convert(kind, y, m, d, &HijriCalendar::default()).unwrap()
    }

    #[test]
    fn matches_and_filters() {
        let db = ObservanceDb::from_toml(SAMPLE).unwrap();
        let h = HijriCalendar::default();
        let nowruz = day(CalendarKind::Persian, 1404, 1, 1);
        assert_eq!(db.on(&nowruz, &h, &ObservanceFilter::default())[0].id, "nowruz-1");
        assert_eq!(db.status(&nowruz, &h, &ObservanceFilter::default(), &[]), (DateStatus::DayOff, false));
        let off = ObservanceFilter { official_main: false, ..Default::default() };
        assert!(db.on(&nowruz, &h, &off).is_empty());
        let ordinary = day(CalendarKind::Persian, 1404, 1, 16);
        assert_eq!(db.status(&ordinary, &h, &off, &[Weekday::Friday]).0, DateStatus::Normal);
    }

    #[test]
    fn leap_only_and_year_limited_rows() {
        let db = ObservanceDb::from_toml(SAMPLE).unwrap();
        let h = HijriCalendar::default();
        let f = ObservanceFilter::default();
        assert_eq!(db.on(&day(CalendarKind::Persian, 1403, 12, 30), &h, &f).len(), 1);
        assert!(db.on(&day(CalendarKind::Persian, 1399, 12, 30), &h, &f).is_empty());
    }

    #[test]
    fn last_day_of_hijri_month() {
        let db = ObservanceDb::from_toml(SAMPLE).unwrap();
        let h = HijriCalendar::default();
        let len = h.month_len(1447, 2);
        let last = day(CalendarKind::Hijri, 1447, 2, len);
        let (status, mourning) = db.status(&last, &h, &ObservanceFilter::default(), &[]);
        assert_eq!((status, mourning), (DateStatus::DayOff, true));
    }

    #[test]
    fn rejects_bad_rows() {
        assert!(ObservanceDb::from_toml(&SAMPLE.replace("day = 1,", "day = 40,")).is_err());
        assert!(ObservanceDb::from_toml(&SAMPLE.replace("en = \"Nowruz\"", "en = \"\"")).is_err());
        assert!(ObservanceDb::from_toml(&SAMPLE.replace("end-of-safar", "nowruz-1")).is_err());
    }

    #[test]
    fn offsets_and_moved_dates() {
        let db = ObservanceDb::from_toml(SAMPLE).unwrap();
        let h = HijriCalendar::default();
        let f = ObservanceFilter::default();
        let ids = |kind, y, m, d| -> Vec<String> {
            db.on(&day(kind, y, m, d), &h, &f).iter().map(|o| o.id.clone()).collect()
        };
        // The last Wednesday of 1404 is 27 Esfand, so Suri is on Tuesday 26 Esfand.
        assert_eq!(ids(CalendarKind::Persian, 1404, 12, 26), ["suri"]);
        assert!(ids(CalendarKind::Persian, 1404, 12, 27).is_empty());
        // 1410's Nowruz was "moved" a day in the sample; other years are unaffected.
        assert!(ids(CalendarKind::Persian, 1410, 1, 1).is_empty());
        assert_eq!(ids(CalendarKind::Persian, 1410, 1, 2), ["nowruz-1"]);
        assert_eq!(ids(CalendarKind::Persian, 1411, 1, 1), ["nowruz-1"]);
    }

    #[test]
    fn embedded_data_is_valid() {
        let db = ObservanceDb::from_toml(include_str!("../../data/observances.toml")).unwrap();
        assert!(!db.all().is_empty());
    }
}
