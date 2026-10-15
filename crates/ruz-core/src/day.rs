//! Day numbers, weekdays and the date types shared by every calendar.

use std::fmt;
use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

/// Chronological Julian Day Number: whole days counted from 1 January 4713 BC
/// (proleptic Julian). Every conversion goes through this number.
///
/// 2000-01-01 (a Saturday) is `Jdn(2451545)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Jdn(pub i32);

impl Jdn {
    pub fn weekday(self) -> Weekday {
        Weekday::from_index((self.0 + 2).rem_euclid(7) as u8)
    }
}

impl Add<i32> for Jdn {
    type Output = Jdn;
    fn add(self, days: i32) -> Jdn {
        Jdn(self.0 + days)
    }
}

impl Sub<i32> for Jdn {
    type Output = Jdn;
    fn sub(self, days: i32) -> Jdn {
        Jdn(self.0 - days)
    }
}

impl Sub<Jdn> for Jdn {
    type Output = i32;
    fn sub(self, other: Jdn) -> i32 {
        self.0 - other.0
    }
}

/// Days of the week in Persian order: the week starts on Saturday.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Weekday {
    Saturday,
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Saturday,
        Weekday::Sunday,
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
    ];

    /// Position in the Persian week, Saturday = 0.
    pub fn index(self) -> u8 {
        self as u8
    }

    pub fn from_index(index: u8) -> Weekday {
        Weekday::ALL[usize::from(index % 7)]
    }
}

/// The three calendars Ruz shows side by side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum CalendarKind {
    /// Solar Hijri (Jalali), Iran's official calendar.
    Persian,
    Gregorian,
    /// Lunar Hijri.
    Hijri,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DateError {
    #[error("year {0} is outside the supported range")]
    OutOfRange(i32),
    #[error("month {0} does not exist")]
    InvalidMonth(u8),
    #[error("day {day} does not exist in month {month}")]
    InvalidDay { month: u8, day: u8 },
}

macro_rules! ymd_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
        pub struct $name {
            pub year: i32,
            pub month: u8,
            pub day: u8,
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
            }
        }
    };
}

ymd_type!(
    /// A Solar Hijri (Jalali) date.
    JalaliDate
);
ymd_type!(
    /// A proleptic Gregorian date.
    GregorianDate
);
ymd_type!(
    /// A Lunar Hijri date, as produced by a particular [`crate::hijri::HijriCalendar`].
    HijriDate
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn y2k_is_saturday() {
        assert_eq!(Jdn(2451545).weekday(), Weekday::Saturday);
        assert_eq!(Jdn(2451551).weekday(), Weekday::Friday);
    }

    #[test]
    fn weekday_index_round_trips() {
        for w in Weekday::ALL {
            assert_eq!(Weekday::from_index(w.index()), w);
        }
    }
}
