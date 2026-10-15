//! Official start dates of Lunar Hijri months as used in Iran.
//!
//! Iran starts each lunar month from moon sighting, so the dates can't be
//! computed. They come from `data/hijri_iran.toml`, which we maintain from
//! Iran's published official calendar.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::day::{GregorianDate, HijriDate, Jdn};
use crate::observances::DataError;

#[derive(Debug, Default)]
pub struct IranHijriTable {
    /// Sorted by start day: (first day of the month, year, month).
    starts: Vec<(Jdn, i32, u8)>,
    /// First day the table no longer covers. The last month may be cut short by it,
    /// in which case its length is unknown.
    end: Option<Jdn>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    year: Vec<YearRow>,
    end: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct YearRow {
    year: i32,
    /// Month that `starts[0]` begins (default 1), for years only partly covered.
    first_month: Option<u8>,
    /// Gregorian dates (YYYY-MM-DD) on which consecutive months begin.
    starts: Vec<String>,
    #[allow(dead_code)]
    source: String,
}

fn parse_date(s: &str) -> Result<Jdn, DataError> {
    let invalid = || DataError::Invalid { id: s.to_owned(), reason: "expected YYYY-MM-DD".into() };
    let mut parts = s.split('-');
    let (Some(y), Some(m), Some(d), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(invalid());
    };
    let (y, m, d) = (
        y.parse().map_err(|_| invalid())?,
        m.parse().map_err(|_| invalid())?,
        d.parse().map_err(|_| invalid())?,
    );
    Ok(GregorianDate::new(y, m, d).map_err(|_| invalid())?.to_jdn())
}

impl IranHijriTable {
    pub fn embedded() -> &'static IranHijriTable {
        static TABLE: OnceLock<IranHijriTable> = OnceLock::new();
        TABLE.get_or_init(|| {
            IranHijriTable::from_toml(include_str!("../../data/hijri_iran.toml"))
                .expect("embedded data/hijri_iran.toml is valid (checked by tests)")
        })
    }

    pub fn from_toml(source: &str) -> Result<Self, DataError> {
        let file: File = toml::from_str(source)?;
        let mut starts = Vec::new();
        for row in &file.year {
            let first_month = row.first_month.unwrap_or(1);
            if first_month == 0 || row.starts.is_empty() || usize::from(first_month) + row.starts.len() > 13 {
                return Err(DataError::Invalid {
                    id: format!("hijri {}", row.year),
                    reason: format!("{} starts from month {first_month} run past month 12", row.starts.len()),
                });
            }
            for (i, s) in row.starts.iter().enumerate() {
                starts.push((parse_date(s)?, row.year, first_month + i as u8));
            }
        }
        starts.sort();
        let end = file.end.as_deref().map(parse_date).transpose()?;
        if !starts.is_empty() && end.is_none() {
            return Err(DataError::Invalid { id: "hijri table".into(), reason: "missing `end`".into() });
        }

        let table = IranHijriTable { starts, end };
        for (i, &(start, year, month)) in table.starts.iter().enumerate() {
            let is_last = i + 1 == table.starts.len();
            let len = table.month_end(i) - start;
            let allowed = if is_last { 1..=30 } else { 29..=30 };
            if !allowed.contains(&len) {
                return Err(DataError::Invalid {
                    id: format!("hijri {year}-{month}"),
                    reason: format!("month is {len} days long"),
                });
            }
            if let Some(&(_, next_year, next_month)) = table.starts.get(i + 1) {
                let expected = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
                if (next_year, next_month) != expected {
                    return Err(DataError::Invalid {
                        id: format!("hijri {year}-{month}"),
                        reason: "months are not consecutive".into(),
                    });
                }
            }
        }
        Ok(table)
    }

    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
    }

    fn month_end(&self, index: usize) -> Jdn {
        self.starts.get(index + 1).map_or_else(|| self.end.expect("validated"), |n| n.0)
    }

    pub fn lookup(&self, jdn: Jdn) -> Option<HijriDate> {
        let first = self.starts.first()?.0;
        if jdn < first || jdn >= self.end? {
            return None;
        }
        let index = self.starts.partition_point(|s| s.0 <= jdn) - 1;
        let (start, year, month) = self.starts[index];
        Some(HijriDate { year, month, day: (jdn - start + 1) as u8 })
    }

    fn index_of(&self, year: i32, month: u8) -> Option<usize> {
        self.starts.iter().position(|&(_, y, m)| y == year && m == month)
    }

    pub fn month_start(&self, year: i32, month: u8) -> Option<Jdn> {
        self.index_of(year, month).map(|i| self.starts[i].0)
    }

    /// The month's length, or `None` if it isn't covered or the table ends partway through it.
    pub fn month_len(&self, year: i32, month: u8) -> Option<u8> {
        let i = self.index_of(year, month)?;
        let len = self.month_end(i) - self.starts[i].0;
        (29..=30).contains(&len).then_some(len as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
        end = "2025-06-26"
        [[year]]
        year = 1446
        starts = ["2024-07-07", "2024-08-06", "2024-09-04", "2024-10-04", "2024-11-03", "2024-12-02",
                  "2025-01-01", "2025-01-31", "2025-03-01", "2025-03-31", "2025-04-29", "2025-05-28"]
        source = "test"
    "#;

    #[test]
    fn lookups() {
        let t = IranHijriTable::from_toml(SAMPLE).unwrap();
        let jdn = |y, m, d| GregorianDate::new(y, m, d).unwrap().to_jdn();
        assert_eq!(t.lookup(jdn(2025, 3, 31)), Some(HijriDate { year: 1446, month: 10, day: 1 }));
        assert_eq!(t.lookup(jdn(2025, 3, 30)), Some(HijriDate { year: 1446, month: 9, day: 30 }));
        assert_eq!(t.lookup(jdn(2025, 6, 25)), Some(HijriDate { year: 1446, month: 12, day: 29 }));
        assert_eq!(t.lookup(jdn(2025, 6, 26)), None);
        assert_eq!(t.lookup(jdn(2024, 7, 6)), None);
        assert_eq!(t.month_len(1446, 9), Some(30));
        assert_eq!(t.month_start(1446, 10), Some(jdn(2025, 3, 31)));
    }

    #[test]
    fn partial_years_and_a_partial_last_month() {
        let t = IranHijriTable::from_toml(
            r#"
            end = "2025-04-10"
            [[year]]
            year = 1446
            first_month = 9
            starts = ["2025-03-02", "2025-03-31"]
            source = "test"
            "#,
        )
        .unwrap();
        let jdn = |y, m, d| GregorianDate::new(y, m, d).unwrap().to_jdn();
        assert_eq!(t.lookup(jdn(2025, 3, 2)), Some(HijriDate { year: 1446, month: 9, day: 1 }));
        assert_eq!(t.lookup(jdn(2025, 4, 9)), Some(HijriDate { year: 1446, month: 10, day: 10 }));
        assert_eq!(t.lookup(jdn(2025, 4, 10)), None);
        assert_eq!(t.month_len(1446, 9), Some(29));
        assert_eq!(t.month_len(1446, 10), None, "Shawwal is only partly covered");
    }

    #[test]
    fn rejects_bad_month_lengths() {
        let bad = SAMPLE.replace("2025-03-31", "2025-04-02");
        assert!(IranHijriTable::from_toml(&bad).is_err());
    }

    #[test]
    fn embedded_table_is_valid() {
        IranHijriTable::from_toml(include_str!("../../data/hijri_iran.toml")).unwrap();
    }
}
