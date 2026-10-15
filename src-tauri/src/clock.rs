//! Wakes at every minute boundary to keep the tray date current. Once a minute is
//! also enough to catch waking from sleep and time-zone changes without OS hooks.

use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::tray;

pub fn start(app: AppHandle) {
    let spawned = std::thread::Builder::new().name("ruz-clock".into()).spawn(move || {
        loop {
            std::thread::sleep(until_next_minute(&app));
            tray::refresh(&app, false);
        }
    });
    if let Err(e) = spawned {
        eprintln!("could not start the clock thread: {e}");
    }
}

fn until_next_minute(app: &AppHandle) -> Duration {
    let now = app.state::<AppState>().clock.now();
    let millis_into_minute = now.as_millisecond().rem_euclid(60_000) as u64;
    // A little past the boundary, so the new minute has surely begun.
    Duration::from_millis(60_000 - millis_into_minute + 50)
}
