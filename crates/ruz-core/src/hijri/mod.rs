//! Lunar Hijri dates, in three layers:
//!
//! 1. a base calendar: Umm al-Qura (with the `icu` feature) or the tabular calendar,
//! 2. Iran's official month starts from [`IranHijriTable`], when enabled and covered,
//! 3. a user offset of up to ±2 days, for when a sighting differs from the table.

pub mod iran_table;
pub mod tabular;
#[cfg(feature = "icu")]
pub mod umm_al_qura;

use serde::{Deserialize, Serialize};

pub use iran_table::IranHijriTable;

use crate::day::{DateError, HijriDate, Jdn};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum HijriBase {
    UmmAlQura,
    Tabular,
}

/// Where a Hijri date came from. Anything but `IranTable` may be off by a day or two
/// from the official Iranian date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum HijriSource {
    IranTable,
    UmmAlQura,
    Tabular,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct HijriConfig {
    pub base: HijriBase,
    /// Prefer Iran's official month starts where the table covers them.
    pub iran_adjustment: bool,
    /// Days added before conversion: the date shown for day `d` is the computed date of `d + offset`.
    /// A month that started one day later than computed needs `-1`.
    pub offset: i8,
}

impl Default for HijriConfig {
    fn default() -> Self {
        HijriConfig {
            base: if cfg!(feature = "icu") { HijriBase::UmmAlQura } else { HijriBase::Tabular },
            iran_adjustment: true,
            offset: 0,
        }
    }
}

pub const MAX_OFFSET: i8 = 2;

#[derive(Clone, Debug)]
pub struct HijriCalendar {
    config: HijriConfig,
    table: &'static IranHijriTable,
}

impl Default for HijriCalendar {
    fn default() -> Self {
        HijriCalendar::new(HijriConfig::default())
    }
}

impl HijriCalendar {
    pub fn new(config: HijriConfig) -> Self {
        Self::with_table(config, IranHijriTable::embedded())
    }

    pub fn with_table(mut config: HijriConfig, table: &'static IranHijriTable) -> Self {
        config.offset = config.offset.clamp(-MAX_OFFSET, MAX_OFFSET);
        HijriCalendar { config, table }
    }

    pub fn config(&self) -> HijriConfig {
        self.config
    }

    pub fn from_jdn(&self, jdn: Jdn) -> (HijriDate, HijriSource) {
        let jdn = jdn + i32::from(self.config.offset);
        if self.config.iran_adjustment
            && let Some(date) = self.table.lookup(jdn)
        {
            return (date, HijriSource::IranTable);
        }
        self.base_from_jdn(jdn)
    }

    pub fn to_jdn(&self, date: HijriDate) -> Result<Jdn, DateError> {
        if !(1..=12).contains(&date.month) {
            return Err(DateError::InvalidMonth(date.month));
        }
        if date.day == 0 || date.day > self.month_len(date.year, date.month) {
            return Err(DateError::InvalidDay { month: date.month, day: date.day });
        }
        let start = self
            .table_start(date.year, date.month)
            .unwrap_or_else(|| self.base_month_start(date.year, date.month));
        Ok(start + i32::from(date.day) - 1 - i32::from(self.config.offset))
    }

    pub fn month_len(&self, year: i32, month: u8) -> u8 {
        if self.config.iran_adjustment
            && let Some(len) = self.table.month_len(year, month)
        {
            return len;
        }
        let start = self.base_month_start(year, month);
        let next = if month == 12 {
            self.base_month_start(year + 1, 1)
        } else {
            self.base_month_start(year, month + 1)
        };
        (next - start) as u8
    }

    fn table_start(&self, year: i32, month: u8) -> Option<Jdn> {
        self.config.iran_adjustment.then(|| self.table.month_start(year, month)).flatten()
    }

    fn base_from_jdn(&self, jdn: Jdn) -> (HijriDate, HijriSource) {
        #[cfg(feature = "icu")]
        if self.config.base == HijriBase::UmmAlQura
            && let Some(date) = umm_al_qura::from_jdn(jdn)
        {
            return (date, HijriSource::UmmAlQura);
        }
        (tabular::from_jdn(jdn), HijriSource::Tabular)
    }

    fn base_month_start(&self, year: i32, month: u8) -> Jdn {
        #[cfg(feature = "icu")]
        if self.config.base == HijriBase::UmmAlQura
            && let Some(start) = umm_al_qura::month_start(year, month)
        {
            return start;
        }
        tabular::to_jdn(year, month, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::GregorianDate;

    fn jdn(y: i32, m: u8, d: u8) -> Jdn {
        GregorianDate::new(y, m, d).unwrap().to_jdn()
    }

    fn table() -> &'static IranHijriTable {
        static T: std::sync::OnceLock<IranHijriTable> = std::sync::OnceLock::new();
        T.get_or_init(|| {
            IranHijriTable::from_toml(
                r#"
                end = "2026-06-17"
                [[year]]
                year = 1446
                starts = ["2024-07-07", "2024-08-06", "2024-09-04", "2024-10-04", "2024-11-03",
                          "2024-12-02", "2025-01-01", "2025-01-31", "2025-03-01", "2025-03-31",
                          "2025-04-29", "2025-05-28"]
                source = "test"
                [[year]]
                year = 1447
                starts = ["2025-06-27", "2025-07-26", "2025-08-24", "2025-09-23", "2025-10-23",
                          "2025-11-21", "2025-12-21", "2026-01-20", "2026-02-19", "2026-03-21",
                          "2026-04-19", "2026-05-19"]
                source = "test"
                "#,
            )
            .unwrap()
        })
    }

    #[test]
    fn table_wins_where_it_covers() {
        let cal = HijriCalendar::with_table(HijriConfig::default(), table());
        let (date, source) = cal.from_jdn(jdn(2025, 3, 31));
        assert_eq!((date, source), (HijriDate { year: 1446, month: 10, day: 1 }, HijriSource::IranTable));
        assert_eq!(cal.to_jdn(date).unwrap(), jdn(2025, 3, 31));
        assert_ne!(cal.from_jdn(jdn(2030, 1, 1)).1, HijriSource::IranTable);
    }

    #[test]
    fn adjustment_can_be_turned_off() {
        let cal = HijriCalendar::with_table(
            HijriConfig { iran_adjustment: false, ..HijriConfig::default() },
            table(),
        );
        assert_ne!(cal.from_jdn(jdn(2025, 3, 31)).1, HijriSource::IranTable);
    }

    #[test]
    fn offset_shifts_both_directions() {
        let late = HijriCalendar::with_table(HijriConfig { offset: -1, ..HijriConfig::default() }, table());
        let (date, _) = late.from_jdn(jdn(2025, 4, 1));
        assert_eq!(date, HijriDate { year: 1446, month: 10, day: 1 });
        assert_eq!(late.to_jdn(date).unwrap(), jdn(2025, 4, 1));

        let clamped = HijriCalendar::with_table(HijriConfig { offset: 9, ..HijriConfig::default() }, table());
        assert_eq!(clamped.config().offset, MAX_OFFSET);
    }

    #[test]
    fn month_lengths_are_lunar() {
        let cal = HijriCalendar::default();
        for year in 1440..1460 {
            for month in 1..=12 {
                assert!((29..=30).contains(&cal.month_len(year, month)), "{year}-{month}");
            }
        }
    }

    #[test]
    fn base_round_trips() {
        for base in [HijriBase::UmmAlQura, HijriBase::Tabular] {
            let cal = HijriCalendar::new(HijriConfig { base, iran_adjustment: false, offset: 0 });
            for day in (jdn(1990, 1, 1).0..jdn(2060, 1, 1).0).step_by(5) {
                let (date, _) = cal.from_jdn(Jdn(day));
                assert_eq!(cal.to_jdn(date).unwrap(), Jdn(day), "{base:?} {date}");
            }
        }
    }
}
