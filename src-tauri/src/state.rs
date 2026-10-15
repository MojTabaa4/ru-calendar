use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use std::time::Instant;

use jiff::Zoned;
use ruz_core::clock::{self, Clock, OffsetClock, SystemClock};
use ruz_core::{CalendarContext, GregorianDate, Jdn, Settings};

pub struct AppState {
    settings: RwLock<Settings>,
    /// Derived from `settings`; rebuilt whenever they change.
    pub ctx: RwLock<CalendarContext>,
    pub settings_path: PathBuf,
    pub clock: Box<dyn Clock>,
    /// When the popover was last hidden, so the tray click that caused the blur
    /// doesn't immediately reopen it.
    pub popover_hidden_at: Mutex<Option<Instant>>,
    /// Last day and tray text shown, to update the tray only when they change.
    pub shown: Mutex<Option<(Jdn, String)>>,
}

impl AppState {
    pub fn new(settings: Settings, settings_path: PathBuf) -> Self {
        AppState {
            ctx: RwLock::new(settings.calendar_context()),
            settings: RwLock::new(settings),
            settings_path,
            clock: make_clock(),
            popover_hidden_at: Mutex::new(None),
            shown: Mutex::new(None),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn replace_settings(&self, settings: Settings) {
        if let Ok(mut ctx) = self.ctx.write() {
            *ctx = settings.calendar_context();
        }
        if let Ok(mut current) = self.settings.write() {
            *current = settings;
        }
    }

    /// The current time in the chosen time zone, read fresh so time-zone changes are picked up.
    pub fn now(&self) -> Zoned {
        let tz = self.settings.read().map(|s| s.time_zone).unwrap_or_default().time_zone();
        self.clock.now().to_zoned(tz)
    }

    pub fn today(&self) -> Jdn {
        GregorianDate::from(self.now().date()).to_jdn()
    }
}

/// Debug builds accept `RUZ_FAKE_NOW=2026-03-20T23:59:30+03:30` to test day rollover.
fn make_clock() -> Box<dyn Clock> {
    if cfg!(debug_assertions)
        && let Ok(value) = std::env::var("RUZ_FAKE_NOW")
    {
        match clock::parse_timestamp(&value) {
            Ok(fake) => return Box::new(OffsetClock::starting_at(fake)),
            Err(e) => eprintln!("ignoring RUZ_FAKE_NOW={value:?}: {e}"),
        }
    }
    Box::new(SystemClock)
}
