//! User settings, saved by the app as JSON. Every field has a default, so files from
//! older versions, or edited by hand, still load.

use jiff::tz::{Offset, TimeZone};
use serde::{Deserialize, Serialize};

use crate::cities;
use crate::day::Weekday;
use crate::format::{Digits, Locale, TrayFormat};
use crate::hijri::{HijriCalendar, HijriConfig, MAX_OFFSET};
use crate::month::CalendarContext;
use crate::observances::ObservanceFilter;
use crate::prayer::Coordinates;
use crate::prayer::view::PrayerPlace;

pub const SETTINGS_VERSION: u32 = 1;

/// Which time zone decides what "today" is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum TimeZoneChoice {
    #[default]
    System,
    Tehran,
}

impl TimeZoneChoice {
    pub fn time_zone(self) -> TimeZone {
        match self {
            TimeZoneChoice::System => TimeZone::system(),
            // Iran has kept +03:30 all year since 2022, so that's a safe fallback.
            TimeZoneChoice::Tehran => TimeZone::get("Asia/Tehran").unwrap_or_else(|_| {
                TimeZone::fixed(Offset::from_seconds(12_600).expect("+03:30 is a valid offset"))
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TraySettings {
    pub format: TrayFormat,
    /// Append the time ("۱۴:۳۵") to the menu-bar date.
    pub show_time: bool,
}

/// Where prayer times are computed for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum PrayerLocation {
    /// A city from the built-in list, by id.
    City { id: String },
    #[serde(rename_all = "camelCase")]
    Custom { name: String, latitude: f64, longitude: f64, time_zone: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PrayerSettings {
    pub enabled: bool,
    pub location: PrayerLocation,
}

impl Default for PrayerSettings {
    fn default() -> Self {
        PrayerSettings { enabled: true, location: PrayerLocation::City { id: "tehran".into() } }
    }
}

/// Events from the system calendar (macOS only).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CalendarSettings {
    pub enabled: bool,
    /// Calendars turned off, by id. New calendars show up until turned off.
    pub hidden: Vec<String>,
}

impl Default for CalendarSettings {
    fn default() -> Self {
        CalendarSettings { enabled: true, hidden: Vec::new() }
    }
}

/// A connected Google account. Its sign-in token lives in the system keychain, not here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GoogleAccount {
    pub email: String,
    /// Calendars turned off, by Google calendar id.
    #[serde(default)]
    pub hidden: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GoogleSettings {
    pub enabled: bool,
    pub accounts: Vec<GoogleAccount>,
}

impl Default for GoogleSettings {
    fn default() -> Self {
        GoogleSettings { enabled: true, accounts: Vec::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Settings {
    pub version: u32,
    pub start_at_login: bool,
    /// Language of month and weekday names in the calendar.
    pub locale: Locale,
    pub digits: Digits,
    pub weekend: Vec<Weekday>,
    pub time_zone: TimeZoneChoice,
    /// Small Gregorian and Hijri day numbers in each day of the month grid.
    pub show_gregorian: bool,
    pub show_hijri: bool,
    pub tray: TraySettings,
    pub occasions: ObservanceFilter,
    pub hijri: HijriConfig,
    pub prayer: PrayerSettings,
    pub calendars: CalendarSettings,
    pub google: GoogleSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: SETTINGS_VERSION,
            start_at_login: false,
            locale: Locale::Fa,
            digits: Digits::Persian,
            weekend: vec![Weekday::Friday],
            time_zone: TimeZoneChoice::System,
            show_gregorian: true,
            show_hijri: true,
            tray: TraySettings::default(),
            occasions: ObservanceFilter::default(),
            hijri: HijriConfig::default(),
            prayer: PrayerSettings::default(),
            calendars: CalendarSettings::default(),
            google: GoogleSettings::default(),
        }
    }
}

impl Settings {
    /// Loads settings JSON, filling in anything missing and fixing out-of-range values.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Settings>(json).map(Settings::normalized)
    }

    /// Brings values back into range instead of rejecting them.
    pub fn normalized(mut self) -> Self {
        self.version = SETTINGS_VERSION;
        self.weekend.sort();
        self.weekend.dedup();
        self.hijri.offset = self.hijri.offset.clamp(-MAX_OFFSET, MAX_OFFSET);
        self.calendars.hidden.sort();
        self.calendars.hidden.dedup();
        // One entry per account, each with a clean list of hidden calendars.
        let mut seen = Vec::new();
        self.google.accounts.retain(|a| {
            let new = !seen.contains(&a.email);
            seen.push(a.email.clone());
            new
        });
        for account in &mut self.google.accounts {
            account.hidden.sort();
            account.hidden.dedup();
        }
        self
    }

    /// Rejects values that can't be fixed silently, with a message for the user.
    pub fn validate(&self) -> Result<(), String> {
        if let PrayerLocation::Custom { latitude, longitude, time_zone, .. } = &self.prayer.location {
            if !(-90.0..=90.0).contains(latitude) || !(-180.0..=180.0).contains(longitude) {
                return Err("عرض جغرافیایی باید بین ۹۰- و ۹۰ و طول بین ۱۸۰- و ۱۸۰ باشد.".into());
            }
            if TimeZone::get(time_zone).is_err() {
                return Err(format!("منطقهٔ زمانی «{time_zone}» شناخته نشد."));
            }
        }
        Ok(())
    }

    /// Where to compute prayer times. An unknown city falls back to Tehran.
    pub fn prayer_place(&self) -> PrayerPlace {
        match &self.prayer.location {
            PrayerLocation::Custom { name, latitude, longitude, time_zone }
                if TimeZone::get(time_zone).is_ok() =>
            {
                PrayerPlace {
                    name: if name.trim().is_empty() { "مکان من".into() } else { name.clone() },
                    coordinates: Coordinates { latitude: *latitude, longitude: *longitude },
                    time_zone: TimeZone::get(time_zone).expect("checked above"),
                }
            }
            location => {
                let id = match location {
                    PrayerLocation::City { id } => id.as_str(),
                    PrayerLocation::Custom { .. } => "tehran",
                };
                let city = cities::find(id).or_else(|| cities::find("tehran")).expect("Tehran is listed");
                PrayerPlace {
                    name: city.name(self.locale).to_owned(),
                    coordinates: city.coordinates(),
                    time_zone: TimeZone::get(&city.tz).unwrap_or(TimeZone::UTC),
                }
            }
        }
    }

    pub fn calendar_context(&self) -> CalendarContext {
        CalendarContext {
            hijri: HijriCalendar::new(self.hijri),
            filter: self.occasions,
            weekend: self.weekend.clone(),
            locale: self.locale,
            digits: self.digits,
            show_gregorian: self.show_gregorian,
            show_hijri: self.show_hijri,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert_eq!(Settings::from_json(&json).unwrap(), settings);
        assert!(json.contains("\"startAtLogin\""), "fields are camelCase: {json}");
    }

    #[test]
    fn missing_fields_take_defaults() {
        let s = Settings::from_json(r#"{ "digits": "latin", "tray": { "showTime": true } }"#).unwrap();
        assert_eq!(s.digits, Digits::Latin);
        assert!(s.tray.show_time);
        assert_eq!(s.tray.format, TrayFormat::default());
        assert_eq!(s.weekend, [Weekday::Friday]);
    }

    #[test]
    fn out_of_range_values_are_fixed() {
        let s = Settings::from_json(
            r#"{ "version": 0, "weekend": ["friday", "thursday", "friday"], "hijri": { "offset": 7 } }"#,
        )
        .unwrap();
        assert_eq!(s.version, SETTINGS_VERSION);
        assert_eq!(s.weekend, [Weekday::Thursday, Weekday::Friday]);
        assert_eq!(s.hijri.offset, MAX_OFFSET);
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(Settings::from_json("{ not json").is_err());
        assert!(Settings::from_json(r#"{ "digits": "roman" }"#).is_err());
    }

    #[test]
    fn context_follows_settings() {
        let s = Settings {
            digits: Digits::Latin,
            show_hijri: false,
            hijri: HijriConfig { offset: -1, ..HijriConfig::default() },
            ..Settings::default()
        };
        let ctx = s.calendar_context();
        assert_eq!(ctx.digits, Digits::Latin);
        assert!(!ctx.show_hijri);
        assert_eq!(ctx.hijri.config().offset, -1);
    }

    #[test]
    fn prayer_places() {
        let mut s = Settings::default();
        assert_eq!(s.prayer_place().name, "تهران");
        s.prayer.location = PrayerLocation::City { id: "nowhere".into() };
        assert_eq!(s.prayer_place().name, "تهران");
        s.prayer.location = PrayerLocation::Custom {
            name: "".into(),
            latitude: 35.0,
            longitude: 50.0,
            time_zone: "Asia/Tehran".into(),
        };
        assert!(s.validate().is_ok());
        assert_eq!(s.prayer_place().name, "مکان من");
        s.prayer.location = PrayerLocation::Custom {
            name: "x".into(),
            latitude: 95.0,
            longitude: 50.0,
            time_zone: "Asia/Tehran".into(),
        };
        assert!(s.validate().is_err());
        s.prayer.location = PrayerLocation::Custom {
            name: "x".into(),
            latitude: 35.0,
            longitude: 50.0,
            time_zone: "Mars/Base".into(),
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn google_accounts_are_unique() {
        let s = Settings::from_json(
            r#"{ "google": { "accounts": [
                { "email": "a@gmail.com", "hidden": ["x", "x"] },
                { "email": "a@gmail.com" },
                { "email": "b@gmail.com" } ] } }"#,
        )
        .unwrap();
        assert!(s.google.enabled);
        assert_eq!(s.google.accounts.len(), 2);
        assert_eq!(s.google.accounts[0].hidden, ["x"]);
    }

    #[test]
    fn prayer_location_json() {
        let s = Settings::from_json(r#"{ "prayer": { "location": { "kind": "city", "id": "mashhad" } } }"#)
            .unwrap();
        assert!(s.prayer.enabled);
        assert_eq!(s.prayer.location, PrayerLocation::City { id: "mashhad".into() });
    }

    #[test]
    fn tehran_is_three_and_a_half_hours_ahead() {
        let at = "2026-10-15T12:00:00Z".parse::<jiff::Timestamp>().unwrap();
        let offset = TimeZoneChoice::Tehran.time_zone().to_offset(at);
        assert_eq!(offset.seconds(), 12_600);
    }
}
