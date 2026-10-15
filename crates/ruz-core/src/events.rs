//! Types for calendar events from macOS Calendar (EventKit) and Google Calendar. The
//! app fills them; they live here so the UI's TypeScript types come from one place.

use serde::Serialize;

use crate::day::Jdn;

/// Whether Ruz may read the system calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum AccessStatus {
    /// No system calendar on this platform.
    Unsupported,
    NotDetermined,
    Denied,
    /// Blocked by a device policy; the user can't grant it.
    Restricted,
    Authorized,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CalendarInfo {
    pub id: String,
    pub title: String,
    /// The account it belongs to, e.g. "iCloud".
    pub source: String,
    /// "#rrggbb".
    pub color: String,
    /// The system Birthdays calendar, filled from Contacts.
    pub birthdays: bool,
    pub visible: bool,
}

/// Where an event comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum EventSource {
    /// macOS Calendar.
    System,
    Google,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EventView {
    /// Unique per occurrence of a repeating event.
    pub id: String,
    pub calendar_id: String,
    pub title: String,
    pub color: String,
    pub all_day: bool,
    pub birthday: bool,
    /// First and last day the event touches, in the system time zone.
    pub start_jdn: Jdn,
    pub end_jdn: Jdn,
    /// Start time ("۱۴:۳۰"); `None` for all-day events.
    pub time: Option<String>,
    pub location: Option<String>,
    pub source: EventSource,
    /// Opens the event in its calendar (Google events link to Google Calendar).
    pub url: Option<String>,
}

/// Which platform features the app has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Features {
    pub calendars: bool,
    pub location: bool,
}

/// Google accounts connected to Ruz and their calendars.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GoogleStatus {
    /// Whether this build has a Google OAuth client to sign in with.
    pub configured: bool,
    pub accounts: Vec<GoogleAccountView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GoogleAccountView {
    pub email: String,
    pub calendars: Vec<CalendarInfo>,
    /// Why the calendars couldn't be loaded, e.g. no network or a revoked sign-in.
    pub error: Option<String>,
}
