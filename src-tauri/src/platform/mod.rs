//! System calendar and location. Only macOS has them; elsewhere these report
//! `Unsupported` and the UI hides the features.

#[cfg(target_os = "macos")]
mod macos_calendar;
#[cfg(target_os = "macos")]
mod macos_location;

use ruz_core::events::{EventSource, Features};

pub struct RawCalendar {
    pub id: String,
    pub title: String,
    pub source: String,
    pub color: String,
    pub birthdays: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawEvent {
    pub id: String,
    pub calendar_id: String,
    pub title: String,
    pub color: String,
    pub all_day: bool,
    pub birthday: bool,
    /// Unix seconds; `end` is exclusive.
    pub start: f64,
    pub end: f64,
    pub location: Option<String>,
    pub source: EventSource,
    /// Where to open the event, for sources that have a web page for it.
    pub url: Option<String>,
}

pub const FEATURES: Features =
    Features { calendars: cfg!(target_os = "macos"), location: cfg!(target_os = "macos") };

#[cfg(target_os = "macos")]
pub use macos_calendar::{access, calendars, events, request_access, watch};
#[cfg(target_os = "macos")]
pub use macos_location::locate;

#[cfg(not(target_os = "macos"))]
mod unsupported {
    use ruz_core::events::AccessStatus;
    use tauri::AppHandle;

    use super::{RawCalendar, RawEvent};

    pub fn access() -> AccessStatus {
        AccessStatus::Unsupported
    }

    pub fn request_access() -> AccessStatus {
        AccessStatus::Unsupported
    }

    pub fn calendars() -> Result<Vec<RawCalendar>, String> {
        Ok(Vec::new())
    }

    pub fn events(_start: f64, _end: f64, _hidden: Vec<String>) -> Result<Vec<RawEvent>, String> {
        Ok(Vec::new())
    }

    pub fn watch(_app: AppHandle) {}

    pub fn locate(_app: &AppHandle) -> Result<(f64, f64), String> {
        Err("unsupported".into())
    }
}

#[cfg(not(target_os = "macos"))]
pub use unsupported::{access, calendars, events, locate, request_access, watch};
