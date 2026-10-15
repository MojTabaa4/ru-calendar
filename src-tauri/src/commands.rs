//! Commands the frontend calls with `invoke`. `ruz json <command>` in ruz-cli mirrors them.

use std::collections::HashSet;
use std::sync::Arc;

use futures_util::future::join_all;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use reqwest::Url;
use ruz_core::cities::{self, City, LocatedPlace};
use ruz_core::converter::{self, MonthsView};
use ruz_core::events::{AccessStatus, CalendarInfo, EventView, Features, GoogleStatus};
use ruz_core::format;
use ruz_core::month::{self, TodayView};
use ruz_core::prayer::view::{PrayerDayView, prayer_day};
use ruz_core::settings::{GoogleAccount, TraySettings};
use ruz_core::{
    CalendarKind, DayDetails, GregorianDate, JalaliDate, Jdn, MonthGrid, ObservanceDb, Settings, day_info,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::google::{self, Google, GoogleError};
use crate::platform::RawEvent;
use crate::state::AppState;
use crate::{platform, settings, windows};

type CommandResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn get_today(state: State<'_, AppState>) -> CommandResult<TodayView> {
    let ctx = state.ctx.read().map_err(err)?;
    month::today_view(&ctx, state.today()).map_err(err)
}

#[tauri::command]
pub fn get_month(
    state: State<'_, AppState>,
    year: Option<i32>,
    month: Option<u8>,
) -> CommandResult<MonthGrid> {
    let today = state.today();
    let current = JalaliDate::from_jdn(today).map_err(err)?;
    let ctx = state.ctx.read().map_err(err)?;
    month::month_grid(
        &ctx,
        ObservanceDb::embedded(),
        year.unwrap_or(current.year),
        month.unwrap_or(current.month),
        today,
    )
    .map_err(err)
}

#[tauri::command]
pub fn get_day(state: State<'_, AppState>, jdn: i32) -> CommandResult<DayDetails> {
    let ctx = state.ctx.read().map_err(err)?;
    month::day_details(&ctx, ObservanceDb::embedded(), Jdn(jdn)).map_err(err)
}

#[tauri::command]
pub fn convert_date(
    state: State<'_, AppState>,
    from: CalendarKind,
    year: i32,
    month: u8,
    day: u8,
) -> CommandResult<DayDetails> {
    let ctx = state.ctx.read().map_err(err)?;
    converter::convert_date(&ctx, ObservanceDb::embedded(), from, year, month, day).map_err(err)
}

#[tauri::command]
pub fn converter_months(
    state: State<'_, AppState>,
    kind: CalendarKind,
    year: i32,
) -> CommandResult<MonthsView> {
    let ctx = state.ctx.read().map_err(err)?;
    converter::months_view(&ctx, kind, year).map_err(err)
}

/// Prayer times on `jdn` (today if omitted), or `None` when they're turned off.
#[tauri::command]
pub fn get_prayer_times(
    state: State<'_, AppState>,
    jdn: Option<i32>,
) -> CommandResult<Option<PrayerDayView>> {
    let settings = state.settings();
    if !settings.prayer.enabled {
        return Ok(None);
    }
    let date = GregorianDate::from_jdn(jdn.map_or_else(|| state.today(), Jdn));
    let view =
        prayer_day(&settings.prayer_place(), date, state.clock.now(), settings.locale, settings.digits);
    view.map(Some).map_err(err)
}

#[tauri::command]
pub fn search_cities(query: String) -> Vec<City> {
    cities::search(&query).into_iter().cloned().collect()
}

#[tauri::command]
pub fn platform_features() -> Features {
    platform::FEATURES
}

#[tauri::command]
pub fn calendar_access() -> AccessStatus {
    platform::access()
}

/// Shows the system's calendar permission prompt (first time only) and waits for the answer.
#[tauri::command]
pub async fn request_calendar_access() -> CommandResult<AccessStatus> {
    tauri::async_runtime::spawn_blocking(platform::request_access).await.map_err(err)
}

#[tauri::command]
pub async fn list_calendars(state: State<'_, AppState>) -> CommandResult<Vec<CalendarInfo>> {
    let hidden = state.settings().calendars.hidden;
    let calendars = tauri::async_runtime::spawn_blocking(platform::calendars).await.map_err(err)??;
    let mut list: Vec<CalendarInfo> = calendars
        .into_iter()
        .map(|c| CalendarInfo {
            visible: !hidden.contains(&c.id),
            id: c.id,
            title: c.title,
            source: c.source,
            color: c.color,
            birthdays: c.birthdays,
        })
        .collect();
    list.sort_by(|a, b| (&a.source, &a.title).cmp(&(&b.source, &b.title)));
    Ok(list)
}

/// Events touching days `start_jdn..=end_jdn`, in the system time zone, from macOS
/// Calendar and Google Calendar.
#[tauri::command]
pub async fn get_events(
    state: State<'_, AppState>,
    google: State<'_, Arc<Google>>,
    start_jdn: i32,
    end_jdn: i32,
) -> CommandResult<Vec<EventView>> {
    let settings = state.settings();
    let tz = TimeZone::system();
    let midnight = |jdn: Jdn| -> CommandResult<Timestamp> {
        let date: jiff::civil::Date = GregorianDate::from_jdn(jdn).try_into().map_err(err)?;
        Ok(date.to_zoned(tz.clone()).map_err(err)?.timestamp())
    };
    let (start, end) = (midnight(Jdn(start_jdn))?, midnight(Jdn(end_jdn) + 1)?);

    let system = async {
        if !settings.calendars.enabled || platform::access() != AccessStatus::Authorized {
            return Vec::new();
        }
        let hidden = settings.calendars.hidden.clone();
        let (from, to) = (start.as_second() as f64, end.as_second() as f64);
        match tauri::async_runtime::spawn_blocking(move || platform::events(from, to, hidden)).await {
            Ok(Ok(events)) => events,
            Ok(Err(e)) => {
                eprintln!("could not read the system calendar: {e}");
                Vec::new()
            }
            Err(e) => {
                eprintln!("could not read the system calendar: {e}");
                Vec::new()
            }
        }
    };
    let from_google = async {
        if !settings.google.enabled {
            return Vec::new();
        }
        google.events(&settings.google.accounts, start, end).await
    };
    let (system, from_google) = futures_util::join!(system, from_google);

    let day = |seconds: f64| -> Option<(Jdn, jiff::Zoned)> {
        let at = Timestamp::from_second(seconds.floor() as i64).ok()?.to_zoned(tz.clone());
        Some((GregorianDate::from(at.date()).to_jdn(), at))
    };
    let mut events: Vec<EventView> = without_duplicates(from_google, system)
        .into_iter()
        .filter_map(|e| {
            let (start_jdn, start) = day(e.start)?;
            // The end is exclusive: an all-day event ends at the next midnight.
            let (end_jdn, _) = day((e.end - 1.0).max(e.start))?;
            Some(EventView {
                id: e.id,
                calendar_id: e.calendar_id,
                title: if e.title.trim().is_empty() { "(بدون عنوان)".into() } else { e.title },
                color: e.color,
                all_day: e.all_day,
                birthday: e.birthday,
                start_jdn,
                end_jdn,
                time: (!e.all_day).then(|| {
                    format::localize_digits(
                        &format!("{:02}:{:02}", start.hour(), start.minute()),
                        settings.digits,
                    )
                }),
                location: e.location,
                source: e.source,
                url: e.url,
            })
        })
        .collect();
    // All-day first, then by start time.
    events.sort_by(|a, b| (a.start_jdn, !a.all_day, &a.time).cmp(&(b.start_jdn, !b.all_day, &b.time)));
    Ok(events)
}

/// A Google calendar that's also added to macOS Calendar would show every event twice;
/// the Google copy is kept since it links to the event.
fn without_duplicates(google: Vec<RawEvent>, system: Vec<RawEvent>) -> Vec<RawEvent> {
    let key = |e: &RawEvent| (e.title.trim().to_owned(), e.start as i64, e.end as i64, e.all_day);
    let seen: HashSet<_> = google.iter().map(key).collect();
    google.into_iter().chain(system.into_iter().filter(|e| !seen.contains(&key(e)))).collect()
}

/// The connected Google accounts and their calendars.
#[tauri::command]
pub async fn google_status(
    state: State<'_, AppState>,
    google: State<'_, Arc<Google>>,
) -> CommandResult<GoogleStatus> {
    let accounts = state.settings().google.accounts;
    Ok(GoogleStatus {
        configured: google.client().is_some(),
        accounts: join_all(accounts.iter().map(|a| google.account_view(a))).await,
    })
}

/// Signs in to a Google account in the browser and adds it (or renews its sign-in).
/// Resolves with the account's email once the user is back from the browser.
#[tauri::command]
pub async fn google_connect(
    app: AppHandle,
    state: State<'_, AppState>,
    google: State<'_, Arc<Google>>,
) -> CommandResult<Option<String>> {
    let (email, hidden) = match google.connect().await {
        Ok(account) => account,
        Err(GoogleError::Cancelled) => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut next = state.settings();
    if !next.google.accounts.iter().any(|a| a.email == email) {
        next.google.accounts.push(GoogleAccount { email: email.clone(), hidden });
    }
    next.google.enabled = true;
    settings::apply(&app, next)?;
    // The browser has the focus now; bring the settings back.
    let _ = windows::open_settings(&app);
    Ok(Some(email))
}

#[tauri::command]
pub fn google_cancel_sign_in(google: State<'_, Arc<Google>>) {
    google.cancel_sign_in();
}

/// Signs out of a Google account and removes it.
#[tauri::command]
pub async fn google_disconnect(
    app: AppHandle,
    state: State<'_, AppState>,
    google: State<'_, Arc<Google>>,
    email: String,
) -> CommandResult<()> {
    google.disconnect(&email).await;
    let mut next = state.settings();
    next.google.accounts.retain(|a| a.email != email);
    settings::apply(&app, next)?;
    Ok(())
}

/// Opens an event in Google Calendar in the browser.
#[tauri::command]
pub fn open_event_link(app: AppHandle, url: String) -> CommandResult<()> {
    let url = Url::parse(&url).map_err(err)?;
    if !google::is_event_link(&url) {
        return Err("not a Google Calendar link".into());
    }
    windows::hide_popover(&app);
    app.opener().open_url(url.as_str(), None::<&str>).map_err(err)
}

/// The device's location and the nearest listed city.
#[tauri::command]
pub async fn locate_me(app: AppHandle) -> CommandResult<LocatedPlace> {
    let (latitude, longitude) =
        tauri::async_runtime::spawn_blocking(move || platform::locate(&app)).await.map_err(err)??;
    let nearest = cities::nearest(latitude, longitude);
    Ok(LocatedPlace {
        latitude,
        longitude,
        time_zone: TimeZone::system().iana_name().unwrap_or("UTC").to_owned(),
        distance_km: cities::distance_km(latitude, longitude, nearest),
        nearest: nearest.clone(),
    })
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: Settings,
) -> CommandResult<Settings> {
    // Google accounts come and go only by signing in and out: a window holding older
    // settings may change which calendars show, but can't drop or add an account.
    let edited = std::mem::take(&mut settings.google.accounts);
    settings.google.accounts = state
        .settings()
        .google
        .accounts
        .into_iter()
        .map(|mut account| {
            if let Some(edit) = edited.iter().find(|e| e.email == account.email) {
                account.hidden.clone_from(&edit.hidden);
            }
            account
        })
        .collect();
    settings::apply(&app, settings)
}

/// Opens the folder that holds `settings.json` (and `google-client.json`), creating it if needed.
#[tauri::command]
pub fn open_config_folder(app: AppHandle) -> CommandResult<()> {
    let dir = app.path().app_config_dir().map_err(err)?;
    std::fs::create_dir_all(&dir).map_err(err)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(err)
}

/// What the tray would show today with `tray`, for the settings preview.
#[tauri::command]
pub fn tray_preview(state: State<'_, AppState>, tray: TraySettings) -> CommandResult<String> {
    let now = state.now();
    let ctx = state.ctx.read().map_err(err)?;
    let day = day_info(state.today(), &ctx.hijri).map_err(err)?;
    let time = tray.show_time.then(|| (now.hour() as u8, now.minute() as u8));
    Ok(format::tray_title(&day, time, &tray.format))
}

/// Opens a window; the converter starts on `jdn` when given.
#[tauri::command]
pub fn open_window(app: AppHandle, name: String, jdn: Option<i32>) -> CommandResult<()> {
    match name.as_str() {
        "settings" => windows::open_settings(&app).map_err(err),
        "converter" => windows::open_converter(&app, jdn.map(Jdn)).map_err(err),
        other => Err(format!("unknown window {other:?}")),
    }
}

#[tauri::command]
pub fn hide_popover(app: AppHandle) {
    windows::hide_popover(&app);
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use ruz_core::events::EventSource;

    use super::*;

    fn event(title: &str, start: f64, source: EventSource) -> RawEvent {
        RawEvent {
            id: format!("{source:?}:{title}"),
            calendar_id: String::new(),
            title: title.into(),
            color: String::new(),
            all_day: false,
            birthday: false,
            start,
            end: start + 3600.0,
            location: None,
            source,
            url: None,
        }
    }

    #[test]
    fn events_in_both_calendars_show_once() {
        let google = vec![event("Standup", 1000.0, EventSource::Google)];
        let system = vec![
            event("Standup ", 1000.0, EventSource::System),
            event("Standup", 5000.0, EventSource::System),
        ];
        let merged = without_duplicates(google, system);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].source, EventSource::Google, "the copy that links to Google is kept");
        assert_eq!(merged[1].start, 5000.0, "a different occurrence isn't a duplicate");
    }
}
