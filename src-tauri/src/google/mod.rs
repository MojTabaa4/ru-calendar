//! Google Calendar: sign-in, calendars and events (read-only), for any number of accounts.
//!
//! Refresh tokens live in the system keychain; access tokens, calendar lists and
//! events are kept in memory. Cached events are shown at once; when they're a few
//! minutes old they're fetched again in the background, and the popover is told to
//! reload if anything changed. Without a network the last events fetched stay shown.

pub mod api;
pub mod config;
pub mod oauth;
mod tokens;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::future::join_all;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use reqwest::Url;
use ruz_core::events::{CalendarInfo, GoogleAccountView};
use ruz_core::settings::GoogleAccount;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use self::api::Calendar;
use self::config::ClientConfig;
use crate::platform::RawEvent;

const EVENTS_FRESH_FOR: Duration = Duration::from_secs(5 * 60);
const CALENDARS_FRESH_FOR: Duration = Duration::from_secs(15 * 60);
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
/// Google's colour for calendars that don't have one.
const DEFAULT_COLOR: &str = "#4285f4";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoogleError {
    NotConfigured,
    /// The user said no on Google's consent page.
    Denied,
    /// The user stopped waiting for the browser.
    Cancelled,
    /// The saved sign-in is missing or was revoked; connect again.
    SignedOut,
    /// The access token was rejected (it's refreshed and retried once).
    Expired,
    Network(String),
    Api(u16),
    Keychain(String),
    Sign(String),
}

impl GoogleError {
    pub fn network(e: reqwest::Error) -> Self {
        GoogleError::Network(e.to_string())
    }

    pub fn io(e: std::io::Error) -> Self {
        GoogleError::Sign(e.to_string())
    }
}

/// Messages for the user, in Persian like the rest of the app.
impl fmt::Display for GoogleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GoogleError::NotConfigured => write!(f, "ورود با گوگل در این نسخه تنظیم نشده است."),
            GoogleError::Denied => write!(f, "اجازهٔ دسترسی به تقویم گوگل داده نشد."),
            GoogleError::Cancelled => write!(f, "ورود لغو شد."),
            GoogleError::SignedOut | GoogleError::Expired => {
                write!(f, "ورود به این حساب منقضی شده است؛ دوباره وصل شوید.")
            }
            GoogleError::Network(_) => write!(f, "اتصال به گوگل برقرار نشد."),
            GoogleError::Api(code) => write!(f, "گوگل پاسخ نداد (خطای {code})."),
            GoogleError::Keychain(_) => write!(f, "دسترسی به کلیدساز سیستم ممکن نشد."),
            GoogleError::Sign(detail) => write!(f, "ورود به گوگل انجام نشد ({detail})."),
        }
    }
}

/// Whether `url` is a Google Calendar page, so it's safe to open from an event.
pub fn is_event_link(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str().is_some_and(|host| host == "google.com" || host.ends_with(".google.com"))
}

/// One calendar's events over one range: the unit that's fetched and cached.
#[derive(Clone)]
struct Job {
    email: String,
    calendar: Calendar,
    palette: HashMap<String, String>,
    range: (Timestamp, Timestamp),
}

type JobKey = (String, String, i64, i64);

impl Job {
    fn key(&self) -> JobKey {
        (self.email.clone(), self.calendar.id.clone(), self.range.0.as_second(), self.range.1.as_second())
    }
}

pub struct Google {
    app: AppHandle,
    http: reqwest::Client,
    config_dir: PathBuf,
    /// email → (access token, when it expires)
    access: Mutex<HashMap<String, (String, Instant)>>,
    calendars: Mutex<HashMap<String, (Instant, Vec<Calendar>)>>,
    palettes: Mutex<HashMap<String, HashMap<String, String>>>,
    events: Mutex<HashMap<JobKey, (Instant, Vec<RawEvent>)>>,
    refreshing: Mutex<HashSet<JobKey>>,
    signing_in: AtomicBool,
    cancel_sign_in: Arc<AtomicBool>,
}

impl Google {
    pub fn new(app: &AppHandle) -> tauri::Result<Self> {
        Ok(Google {
            app: app.clone(),
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(20))
                .user_agent(concat!("Ruz/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("a default HTTP client builds"),
            config_dir: app.path().app_config_dir()?,
            access: Mutex::default(),
            calendars: Mutex::default(),
            palettes: Mutex::default(),
            events: Mutex::default(),
            refreshing: Mutex::default(),
            signing_in: AtomicBool::new(false),
            cancel_sign_in: Arc::new(AtomicBool::new(false)),
        })
    }

    /// The OAuth client to sign in with, if this install has one.
    pub fn client(&self) -> Option<ClientConfig> {
        config::load(&self.config_dir)
    }

    /// A valid access token, refreshed from the keychain's refresh token when needed.
    async fn token(&self, email: &str) -> Result<String, GoogleError> {
        if let Some((token, expires)) = self.access.lock().unwrap().get(email)
            && Instant::now() + Duration::from_secs(60) < *expires
        {
            return Ok(token.clone());
        }
        let client = self.client().ok_or(GoogleError::NotConfigured)?;
        let refresh_token = tokens::load(email)?;
        let fresh = oauth::refresh(&self.http, &client, &refresh_token).await?;
        let expires = Instant::now() + Duration::from_secs(fresh.expires_in);
        self.access.lock().unwrap().insert(email.to_owned(), (fresh.access_token.clone(), expires));
        Ok(fresh.access_token)
    }

    /// Runs `call` with a token, retrying once with a fresh one if Google rejects it.
    async fn with_token<T, F, Fut>(&self, email: &str, call: F) -> Result<T, GoogleError>
    where
        F: Fn(reqwest::Client, String) -> Fut,
        Fut: Future<Output = Result<T, GoogleError>>,
    {
        match call(self.http.clone(), self.token(email).await?).await {
            Err(GoogleError::Expired) => {
                self.access.lock().unwrap().remove(email);
                call(self.http.clone(), self.token(email).await?).await
            }
            result => result,
        }
    }

    /// Signs in through the browser. Returns the account's email and the calendars to
    /// start hidden: those switched off in Google Calendar, and Google's Iranian holidays
    /// since Ruz shows the official ones itself.
    pub async fn connect(&self) -> Result<(String, Vec<String>), GoogleError> {
        let client = self.client().ok_or(GoogleError::NotConfigured)?;
        if self.signing_in.swap(true, Ordering::SeqCst) {
            return Err(GoogleError::Sign("a sign-in is already open in the browser".into()));
        }
        self.cancel_sign_in.store(false, Ordering::SeqCst);
        let result = self.sign_in(&client).await;
        self.signing_in.store(false, Ordering::SeqCst);
        let (email, access) = result?;
        let calendars = api::calendars(&self.http, &access).await.unwrap_or_default();
        let hidden =
            calendars.iter().filter(|c| !c.selected || c.is_iran_holidays()).map(|c| c.id.clone()).collect();
        self.calendars.lock().unwrap().insert(email.clone(), (Instant::now(), calendars));
        Ok((email, hidden))
    }

    /// Stops waiting for a sign-in the user abandoned in the browser.
    pub fn cancel_sign_in(&self) {
        self.cancel_sign_in.store(true, Ordering::SeqCst);
    }

    async fn sign_in(&self, client: &ClientConfig) -> Result<(String, String), GoogleError> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(GoogleError::io)?;
        let redirect_uri =
            format!("http://127.0.0.1:{}", listener.local_addr().map_err(GoogleError::io)?.port());
        let (verifier, challenge) = oauth::pkce();
        let state = oauth::new_state();
        let url = oauth::auth_url(client, &redirect_uri, &challenge, &state);
        self.app
            .opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|e| GoogleError::Sign(e.to_string()))?;

        let cancelled = Arc::clone(&self.cancel_sign_in);
        let code = tauri::async_runtime::spawn_blocking(move || {
            oauth::wait_for_code(listener, &state, SIGN_IN_TIMEOUT, &cancelled)
        })
        .await
        .map_err(|e| GoogleError::Sign(e.to_string()))??;
        let tokens = oauth::exchange_code(&self.http, client, &code, &verifier, &redirect_uri).await?;
        let refresh_token = tokens
            .refresh_token
            .ok_or_else(|| GoogleError::Sign("Google didn't return a refresh token".into()))?;
        let email = api::email(&self.http, &tokens.access_token).await?;
        tokens::save(&email, &refresh_token)?;
        let expires = Instant::now() + Duration::from_secs(tokens.expires_in);
        self.access.lock().unwrap().insert(email.clone(), (tokens.access_token.clone(), expires));
        Ok((email, tokens.access_token))
    }

    /// Signs out: tells Google to drop the grant and forgets everything about the account.
    pub async fn disconnect(&self, email: &str) {
        if let Ok(refresh_token) = tokens::load(email) {
            oauth::revoke(&self.http, &refresh_token).await;
        }
        tokens::delete(email);
        self.access.lock().unwrap().remove(email);
        self.calendars.lock().unwrap().remove(email);
        self.palettes.lock().unwrap().remove(email);
        self.events.lock().unwrap().retain(|(account, ..), _| account != email);
    }

    /// The account's calendars, primary first. A stale list is used when Google can't
    /// be reached, but not when the sign-in is gone.
    async fn calendars(&self, email: &str) -> Result<Vec<Calendar>, GoogleError> {
        let cached = self.calendars.lock().unwrap().get(email).cloned();
        if let Some((at, list)) = &cached
            && at.elapsed() < CALENDARS_FRESH_FOR
        {
            return Ok(list.clone());
        }
        match self.with_token(email, |http, token| async move { api::calendars(&http, &token).await }).await {
            Ok(list) => {
                self.calendars.lock().unwrap().insert(email.to_owned(), (Instant::now(), list.clone()));
                Ok(list)
            }
            Err(e) => match (e, cached) {
                (GoogleError::Network(_) | GoogleError::Api(_), Some((_, list))) => Ok(list),
                (e, _) => Err(e),
            },
        }
    }

    async fn palette(&self, email: &str) -> HashMap<String, String> {
        if let Some(palette) = self.palettes.lock().unwrap().get(email) {
            return palette.clone();
        }
        let palette = self
            .with_token(email, |http, token| async move { api::event_colors(&http, &token).await })
            .await
            .unwrap_or_default();
        if !palette.is_empty() {
            self.palettes.lock().unwrap().insert(email.to_owned(), palette.clone());
        }
        palette
    }

    /// The account and its calendars for the settings window.
    pub async fn account_view(&self, account: &GoogleAccount) -> GoogleAccountView {
        match self.calendars(&account.email).await {
            Ok(list) => GoogleAccountView {
                email: account.email.clone(),
                calendars: list
                    .iter()
                    .map(|c| CalendarInfo {
                        id: c.id.clone(),
                        title: c.title(),
                        source: account.email.clone(),
                        color: c.background_color.clone().unwrap_or_else(|| DEFAULT_COLOR.into()),
                        birthdays: false,
                        visible: !account.hidden.contains(&c.id),
                    })
                    .collect(),
                error: None,
            },
            Err(e) => GoogleAccountView {
                email: account.email.clone(),
                calendars: Vec::new(),
                error: Some(e.to_string()),
            },
        }
    }

    /// Fetches one calendar's events and caches them.
    async fn fetch(&self, job: &Job, tz: &TimeZone) -> Result<Vec<RawEvent>, GoogleError> {
        let (id, (start, end)) = (job.calendar.id.as_str(), job.range);
        let list =
            self.with_token(&job.email, |http, token| async move {
                api::events(&http, &token, id, start, end).await
            })
            .await?;
        let events: Vec<RawEvent> = list
            .into_iter()
            .filter_map(|e| api::to_raw(e, &job.email, &job.calendar, &job.palette, tz))
            .collect();
        self.events.lock().unwrap().insert(job.key(), (Instant::now(), events.clone()));
        Ok(events)
    }

    /// Fetches stale calendars again in the background and, if any changed, tells the
    /// windows to reload their events.
    fn revalidate(self: &Arc<Self>, jobs: Vec<Job>, tz: TimeZone) {
        let jobs: Vec<Job> = {
            let mut refreshing = self.refreshing.lock().unwrap();
            jobs.into_iter().filter(|job| refreshing.insert(job.key())).collect()
        };
        if jobs.is_empty() {
            return;
        }
        let this = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let changed = join_all(jobs.iter().map(|job| async {
                let before = this.events.lock().unwrap().get(&job.key()).map(|(_, events)| events.clone());
                let after = this.fetch(job, &tz).await;
                this.refreshing.lock().unwrap().remove(&job.key());
                after.is_ok_and(|after| before.as_ref() != Some(&after))
            }))
            .await;
            if changed.contains(&true) {
                let _ = this.app.emit("calendar-store-changed", ());
            }
        });
    }

    /// Events in `[start, end)` from every visible calendar of every account.
    pub async fn events(
        self: &Arc<Self>,
        accounts: &[GoogleAccount],
        start: Timestamp,
        end: Timestamp,
    ) -> Vec<RawEvent> {
        let tz = TimeZone::system();
        let mut shown = Vec::new();
        let (mut missing, mut stale) = (Vec::new(), Vec::new());
        for account in accounts {
            let Ok(calendars) = self.calendars(&account.email).await else { continue };
            let palette = self.palette(&account.email).await;
            for calendar in calendars.into_iter().filter(|c| !account.hidden.contains(&c.id)) {
                let job = Job {
                    email: account.email.clone(),
                    calendar,
                    palette: palette.clone(),
                    range: (start, end),
                };
                match self.events.lock().unwrap().get(&job.key()) {
                    Some((at, events)) => {
                        shown.extend(events.iter().cloned());
                        if at.elapsed() >= EVENTS_FRESH_FOR {
                            stale.push(job);
                        }
                    }
                    None => missing.push(job),
                }
            }
        }
        let fetched = join_all(missing.iter().map(|job| self.fetch(job, &tz))).await;
        shown.extend(fetched.into_iter().flatten().flatten());
        self.revalidate(stale, tz);
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_google_pages_open_from_events() {
        let ok = |s: &str| is_event_link(&Url::parse(s).unwrap());
        assert!(ok("https://www.google.com/calendar/event?eid=abc"));
        assert!(ok("https://calendar.google.com/calendar/r"));
        assert!(!ok("http://www.google.com/calendar/event?eid=abc"), "https only");
        assert!(!ok("https://google.com.evil.example/calendar"));
        assert!(!ok("https://notgoogle.com/"));
        assert!(!ok("file:///etc/passwd"));
    }
}
