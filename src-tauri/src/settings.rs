//! Loading, saving and applying [`Settings`]. Rust owns them: the tray needs them
//! before any window exists, and every change goes through [`apply`].

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ruz_core::Settings;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::state::AppState;
use crate::tray;

pub fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("settings.json"))
}

/// Reads settings, falling back to defaults. A file that can't be parsed is kept
/// next to it as `settings.json.invalid` rather than silently overwritten.
pub fn load(path: &Path) -> Settings {
    match fs::read_to_string(path) {
        Ok(json) => Settings::from_json(&json).unwrap_or_else(|e| {
            eprintln!("{} is not valid ({e}); starting from defaults", path.display());
            let _ = fs::rename(path, path.with_extension("json.invalid"));
            Settings::default()
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Settings::default(),
        Err(e) => {
            eprintln!("could not read {}: {e}", path.display());
            Settings::default()
        }
    }
}

/// Writes atomically, so a crash mid-write can't leave a truncated file.
pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(&tmp, path)
}

/// Saves `next` and makes every part of the app follow it.
pub fn apply(app: &AppHandle, next: Settings) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let next = next.normalized();
    next.validate()?;
    let previous = state.settings();

    if next.start_at_login != previous.start_at_login {
        set_start_at_login(app, next.start_at_login)?;
    }
    save(&state.settings_path, &next).map_err(|e| format!("could not save settings: {e}"))?;
    state.replace_settings(next.clone());
    tray::refresh(app, true);
    let _ = app.emit("settings-changed", &next);
    Ok(next)
}

fn set_start_at_login(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    let result = if enabled { autolaunch.enable() } else { autolaunch.disable() };
    result.map_err(|e| format!("could not change start at login: {e}"))
}

/// The OS is the source of truth for start at login (the user may remove the login
/// item in system settings), so the saved setting follows it at launch.
pub fn sync_start_at_login(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok(enabled) = app.autolaunch().is_enabled() else { return };
    let mut settings = state.settings();
    if settings.start_at_login != enabled {
        settings.start_at_login = enabled;
        if let Err(e) = save(&state.settings_path, &settings) {
            eprintln!("could not save settings: {e}");
        }
        state.replace_settings(settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruz_core::format::Digits;

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ruz-settings-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("nested").join("settings.json")
    }

    #[test]
    fn save_then_load() {
        let path = temp_path("roundtrip");
        let settings = Settings { digits: Digits::Latin, ..Settings::default() };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path), settings);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn missing_file_gives_defaults() {
        assert_eq!(load(&temp_path("missing")), Settings::default());
    }

    #[test]
    fn invalid_file_is_set_aside() {
        let path = temp_path("invalid");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(load(&path), Settings::default());
        assert!(!path.exists());
        assert!(path.with_extension("json.invalid").exists());
    }
}
