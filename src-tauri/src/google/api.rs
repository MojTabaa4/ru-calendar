//! The few Google Calendar API v3 calls Ruz makes, and turning events into `RawEvent`s.

use std::collections::HashMap;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use reqwest::{StatusCode, Url};
use ruz_core::events::EventSource;
use serde::Deserialize;

use super::GoogleError;
use crate::platform::RawEvent;

const API: &str = "https://www.googleapis.com/calendar/v3";
const USERINFO: &str = "https://openidconnect.googleapis.com/v1/userinfo";

async fn get<T: for<'de> Deserialize<'de>>(
    http: &reqwest::Client,
    url: Url,
    token: &str,
) -> Result<T, GoogleError> {
    let response = http.get(url).bearer_auth(token).send().await.map_err(GoogleError::network)?;
    match response.status() {
        s if s.is_success() => response.json().await.map_err(GoogleError::network),
        StatusCode::UNAUTHORIZED => Err(GoogleError::Expired),
        s => Err(GoogleError::Api(s.as_u16())),
    }
}

#[derive(Deserialize)]
struct UserInfo {
    email: String,
}

pub async fn email(http: &reqwest::Client, token: &str) -> Result<String, GoogleError> {
    Ok(get::<UserInfo>(http, Url::parse(USERINFO).expect("valid"), token).await?.email)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: String,
    pub summary: Option<String>,
    /// The name the user gave a calendar they subscribed to.
    pub summary_override: Option<String>,
    pub background_color: Option<String>,
    #[serde(default)]
    pub primary: bool,
    /// Whether it's switched on in Google Calendar itself.
    #[serde(default)]
    pub selected: bool,
}

impl Calendar {
    pub fn title(&self) -> String {
        self.summary_override.clone().or_else(|| self.summary.clone()).unwrap_or_else(|| self.id.clone())
    }

    /// Google's "Holidays in Iran" (in any language), which Ruz's own occasions already cover.
    pub fn is_iran_holidays(&self) -> bool {
        self.id.ends_with(".ir#holiday@group.v.calendar.google.com")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarList {
    #[serde(default)]
    items: Vec<Calendar>,
    next_page_token: Option<String>,
}

pub async fn calendars(http: &reqwest::Client, token: &str) -> Result<Vec<Calendar>, GoogleError> {
    let mut all = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut url = Url::parse(&format!("{API}/users/me/calendarList")).expect("valid");
        url.query_pairs_mut().append_pair("maxResults", "250").append_pair(
            "fields",
            "items(id,summary,summaryOverride,backgroundColor,primary,selected),nextPageToken",
        );
        if let Some(p) = &page {
            url.query_pairs_mut().append_pair("pageToken", p);
        }
        let list: CalendarList = get(http, url, token).await?;
        all.extend(list.items);
        match list.next_page_token {
            Some(next) => page = Some(next),
            None => break,
        }
    }
    // The primary calendar first, then by name.
    all.sort_by_key(|c| (!c.primary, c.title()));
    Ok(all)
}

#[derive(Deserialize)]
struct Colors {
    #[serde(default)]
    event: HashMap<String, Color>,
}

#[derive(Deserialize)]
struct Color {
    background: String,
}

/// Event colour ids ("1"–"11") to "#rrggbb".
pub async fn event_colors(
    http: &reqwest::Client,
    token: &str,
) -> Result<HashMap<String, String>, GoogleError> {
    let mut url = Url::parse(&format!("{API}/colors")).expect("valid");
    url.query_pairs_mut().append_pair("fields", "event");
    let colors: Colors = get(http, url, token).await?;
    Ok(colors.event.into_iter().map(|(id, c)| (id, c.background)).collect())
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub location: Option<String>,
    pub html_link: Option<String>,
    pub color_id: Option<String>,
    pub start: Time,
    pub end: Time,
    #[serde(default)]
    pub attendees: Vec<Attendee>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Time {
    /// All-day events: "2026-10-15".
    pub date: Option<String>,
    /// Timed events: RFC 3339 with an offset.
    pub date_time: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attendee {
    #[serde(rename = "self", default)]
    pub is_self: bool,
    pub response_status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventList {
    #[serde(default)]
    items: Vec<Event>,
    next_page_token: Option<String>,
}

/// Events overlapping `[start, end)`, with repeating events expanded into occurrences.
pub async fn events(
    http: &reqwest::Client,
    token: &str,
    calendar_id: &str,
    start: Timestamp,
    end: Timestamp,
) -> Result<Vec<Event>, GoogleError> {
    let mut all = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut url = Url::parse(API).expect("valid");
        // Calendar ids contain "@" and "#", so they go in as an encoded path segment.
        url.path_segments_mut().expect("base URL").extend(["calendars", calendar_id, "events"]);
        url.query_pairs_mut()
            .append_pair("timeMin", &start.to_string())
            .append_pair("timeMax", &end.to_string())
            .append_pair("singleEvents", "true")
            .append_pair("orderBy", "startTime")
            .append_pair("maxResults", "250")
            .append_pair(
                "fields",
                "items(id,status,summary,location,htmlLink,colorId,start,end,attendees(self,responseStatus)),nextPageToken",
            );
        if let Some(p) = &page {
            url.query_pairs_mut().append_pair("pageToken", p);
        }
        let list: EventList = get(http, url, token).await?;
        all.extend(list.items);
        match list.next_page_token {
            Some(next) => page = Some(next),
            None => break,
        }
    }
    Ok(all)
}

fn instant(time: &Time, tz: &TimeZone) -> Option<Timestamp> {
    if let Some(dt) = &time.date_time {
        return dt.parse().ok();
    }
    // All-day dates are calendar days, so they start at local midnight wherever the user is.
    let date: jiff::civil::Date = time.date.as_deref()?.parse().ok()?;
    Some(date.to_zoned(tz.clone()).ok()?.timestamp())
}

/// What Ruz shows for an event, or `None` for cancelled events and ones the user declined.
pub fn to_raw(
    event: Event,
    account: &str,
    calendar: &Calendar,
    palette: &HashMap<String, String>,
    tz: &TimeZone,
) -> Option<RawEvent> {
    if event.status.as_deref() == Some("cancelled") {
        return None;
    }
    if event.attendees.iter().any(|a| a.is_self && a.response_status.as_deref() == Some("declined")) {
        return None;
    }
    let start = instant(&event.start, tz)?;
    let end = instant(&event.end, tz)?;
    let color = event
        .color_id
        .as_ref()
        .and_then(|id| palette.get(id))
        .or(calendar.background_color.as_ref())
        .cloned()
        .unwrap_or_else(|| "#4285f4".into());
    Some(RawEvent {
        id: format!("google:{account}:{}", event.id),
        calendar_id: format!("google:{account}:{}", calendar.id),
        title: event.summary.unwrap_or_default(),
        color,
        all_day: event.start.date.is_some(),
        birthday: false,
        start: start.as_second() as f64,
        end: end.as_second() as f64,
        location: event.location.filter(|l| !l.trim().is_empty()),
        source: EventSource::Google,
        url: event.html_link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calendar() -> Calendar {
        Calendar {
            id: "me@gmail.com".into(),
            summary: Some("Me".into()),
            summary_override: None,
            background_color: Some("#9fe1e7".into()),
            primary: true,
            selected: true,
        }
    }

    fn parse(json: &str) -> Event {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn timed_event() {
        let tz = TimeZone::get("Asia/Tehran").unwrap();
        let e = parse(
            r#"{"id":"e1","status":"confirmed","summary":"جلسه","htmlLink":"https://www.google.com/calendar/event?eid=x",
                "start":{"dateTime":"2026-10-15T10:00:00+03:30"},"end":{"dateTime":"2026-10-15T11:30:00+03:30"}}"#,
        );
        let raw = to_raw(e, "me@gmail.com", &calendar(), &HashMap::new(), &tz).unwrap();
        assert!(!raw.all_day);
        assert_eq!(raw.end - raw.start, 5400.0);
        assert_eq!(raw.color, "#9fe1e7", "falls back to the calendar's colour");
        assert_eq!(raw.source, EventSource::Google);
        assert!(raw.url.unwrap().starts_with("https://www.google.com/calendar/"));
    }

    #[test]
    fn all_day_event_spans_local_days() {
        let tz = TimeZone::get("America/Toronto").unwrap();
        let e = parse(
            r#"{"id":"e2","summary":"Trip","colorId":"11","start":{"date":"2026-10-15"},"end":{"date":"2026-10-17"}}"#,
        );
        let palette = HashMap::from([("11".to_string(), "#dc2127".to_string())]);
        let raw = to_raw(e, "me@gmail.com", &calendar(), &palette, &tz).unwrap();
        assert!(raw.all_day);
        assert_eq!(raw.color, "#dc2127", "an event's own colour wins");
        let start = Timestamp::from_second(raw.start as i64).unwrap().to_zoned(tz.clone());
        assert_eq!((start.day(), start.hour()), (15, 0));
        assert_eq!(raw.end - raw.start, 2.0 * 86_400.0);
    }

    #[test]
    fn skips_cancelled_and_declined() {
        let tz = TimeZone::UTC;
        let cancelled = parse(
            r#"{"id":"e3","status":"cancelled","start":{"date":"2026-10-15"},"end":{"date":"2026-10-16"}}"#,
        );
        assert!(to_raw(cancelled, "a", &calendar(), &HashMap::new(), &tz).is_none());
        let declined = parse(
            r#"{"id":"e4","start":{"date":"2026-10-15"},"end":{"date":"2026-10-16"},
                "attendees":[{"self":true,"responseStatus":"declined"},{"responseStatus":"accepted"}]}"#,
        );
        assert!(to_raw(declined, "a", &calendar(), &HashMap::new(), &tz).is_none());
    }

    #[test]
    fn iran_holidays_calendar() {
        let mut c = calendar();
        assert!(!c.is_iran_holidays());
        c.id = "fa.ir#holiday@group.v.calendar.google.com".into();
        assert!(c.is_iran_holidays());
        c.id = "en.canadian#holiday@group.v.calendar.google.com".into();
        assert!(!c.is_iran_holidays());
    }

    #[test]
    fn calendar_titles() {
        let mut c = calendar();
        c.summary_override = Some("Family".into());
        assert_eq!(c.title(), "Family");
    }
}
