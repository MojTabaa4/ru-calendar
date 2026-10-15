//! Ruz: a Persian calendar in the menu bar (macOS) or system tray (Windows, Linux).

mod clock;
mod commands;
mod google;
mod platform;
mod settings;
mod state;
mod tray;
mod windows;

use std::sync::Arc;

use tauri::{Manager, RunEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::google::Google;
use crate::state::AppState;

pub fn run() {
    tauri::Builder::default()
        // Must come first: a second launch just opens the running app's calendar.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_popover(app, None);
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_today,
            commands::get_month,
            commands::get_day,
            commands::convert_date,
            commands::converter_months,
            commands::get_prayer_times,
            commands::search_cities,
            commands::platform_features,
            commands::calendar_access,
            commands::request_calendar_access,
            commands::list_calendars,
            commands::get_events,
            commands::google_status,
            commands::google_connect,
            commands::google_cancel_sign_in,
            commands::google_disconnect,
            commands::open_event_link,
            commands::locate_me,
            commands::get_settings,
            commands::update_settings,
            commands::open_config_folder,
            commands::tray_preview,
            commands::open_window,
            commands::hide_popover,
            commands::quit,
        ])
        .setup(|app| {
            // Menu-bar only: no Dock icon or app menu.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let settings_path = settings::path(app.handle())?;
            app.manage(AppState::new(settings::load(&settings_path), settings_path));
            app.manage(Arc::new(Google::new(app.handle())?));
            settings::sync_start_at_login(app.handle());

            windows::create_popover(app.handle())?;
            tray::create(app.handle())?;
            clock::start(app.handle().clone());
            platform::watch(app.handle().clone());
            if cfg!(debug_assertions) && std::env::var_os("RUZ_OPEN_POPOVER").is_some() {
                windows::show_popover(app.handle(), None);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Ruz")
        .run(|_app, event| {
            // Hiding the last window must not quit a tray app; only Quit does (exit code set).
            if let RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
            }
        });
}
