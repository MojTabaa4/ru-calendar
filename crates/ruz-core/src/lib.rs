//! Calendar engine for Ruz: Solar Hijri (Persian), Gregorian and Lunar Hijri dates,
//! Iranian holidays, and ready-to-render month views.
//!
//! Every conversion goes through a [`Jdn`] (Julian Day Number). Nothing here depends
//! on Tauri, so the same code backs the app, the CLI and the tests.

pub mod cities;
pub mod clock;
pub mod convert;
pub mod converter;
pub mod day;
pub mod events;
pub mod format;
pub mod gregorian;
pub mod hijri;
pub mod jalali;
pub mod month;
pub mod observances;
pub mod prayer;
pub mod settings;

pub use convert::{DayInfo, convert, day_info};
pub use day::{CalendarKind, DateError, GregorianDate, HijriDate, JalaliDate, Jdn, Weekday};
pub use hijri::{HijriCalendar, HijriConfig};
pub use month::{CalendarContext, DayDetails, MonthGrid, day_details, month_grid};
pub use observances::{ObservanceDb, ObservanceFilter};
pub use settings::Settings;
