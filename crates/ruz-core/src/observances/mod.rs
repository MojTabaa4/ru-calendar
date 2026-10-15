//! Holidays and occasions, matched against a day in any of the three calendars.

mod db;
mod rules;

use serde::{Deserialize, Serialize};

pub use db::ObservanceDb;
pub use rules::DateRule;

use crate::day::CalendarKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Category {
    /// Iran's official calendar.
    IranOfficial,
    /// Ancient and historical Iranian festivals (Mehregan, Sadeh, …).
    IranHistorical,
    /// Further Shia occasions that are not in the official calendar.
    ShiaExtra,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Subcategory {
    Main,
    /// The appendix of further national days in the official calendar.
    Appendix,
    /// Days of professions (doctors, teachers, …).
    Professions,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observance {
    pub id: String,
    pub calendar: CalendarKind,
    pub category: Category,
    pub subcategory: Subcategory,
    pub rule: DateRule,
    /// Days from the rule's date to the observance, e.g. `-1` for the eve of a day.
    pub offset: i8,
    pub day_off: bool,
    pub mourning: bool,
    pub title_fa: String,
    pub title_en: String,
    /// First and last Solar Hijri years in which this applies, when limited.
    pub since: Option<i32>,
    pub until: Option<i32>,
    pub source: String,
}

/// Which categories to show. Off categories don't count towards a day's status either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ObservanceFilter {
    pub official_main: bool,
    pub official_appendix: bool,
    pub official_professions: bool,
    pub historical: bool,
    pub shia_extra: bool,
}

impl Default for ObservanceFilter {
    fn default() -> Self {
        ObservanceFilter {
            official_main: true,
            official_appendix: false,
            official_professions: false,
            historical: true,
            shia_extra: false,
        }
    }
}

impl ObservanceFilter {
    pub fn allows(&self, category: Category, subcategory: Subcategory) -> bool {
        match (category, subcategory) {
            (Category::IranOfficial, Subcategory::Main) => self.official_main,
            (Category::IranOfficial, Subcategory::Appendix) => self.official_appendix,
            (Category::IranOfficial, Subcategory::Professions) => self.official_professions,
            (Category::IranHistorical, _) => self.historical,
            (Category::ShiaExtra, _) => self.shia_extra,
        }
    }
}

/// How a day is marked in the month grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum DateStatus {
    Normal,
    Weekend,
    DayOff,
}

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("{id}: {reason}")]
    Invalid { id: String, reason: String },
}
