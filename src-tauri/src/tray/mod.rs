//! The tray icon (menu-bar item on macOS) showing today's date.
//!
//! macOS shows the date as the item's text title. Windows and most Linux trays can't
//! show text, so there the icon itself is drawn with today's day number.

// Only used off macOS, but compiled (and tested) everywhere.
#[cfg_attr(target_os = "macos", allow(dead_code))]
mod raster;

use jiff::Zoned;
use ruz_core::format::{self, Digits};
use ruz_core::{CalendarKind, GregorianDate, Jdn, day_info};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;
use crate::windows;

pub const TRAY_ID: &str = "main";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "show", "نمایش تقویم", true, None::<&str>)?,
            &MenuItem::with_id(app, "converter", "تبدیل تاریخ…", true, None::<&str>)?,
            &MenuItem::with_id(app, "settings", "تنظیمات…", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "خروج از روز", true, None::<&str>)?,
        ],
    )?;

    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("روز")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => windows::show_popover(app, None),
            "converter" => {
                if let Err(e) = windows::open_converter(app, None) {
                    eprintln!("could not open the converter: {e}");
                }
            }
            "settings" => {
                if let Err(e) = windows::open_settings(app) {
                    eprintln!("could not open settings: {e}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                windows::toggle_popover(tray.app_handle(), Some(rect));
            }
        });

    // Windows and Linux need an icon to show anything; refresh() draws the real one.
    #[cfg(not(target_os = "macos"))]
    let builder = builder.icon(raster::day_icon("", Digits::Persian));

    builder.build(app)?;
    refresh(app, true);
    Ok(())
}

struct TrayView {
    title: String,
    tooltip: String,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    day_label: String,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    digits: Digits,
}

fn tray_view(state: &AppState, now: &Zoned, today: Jdn) -> Option<TrayView> {
    let tray = state.settings().tray;
    let ctx = state.ctx.read().ok()?;
    let day = day_info(today, &ctx.hijri).ok()?;
    let time = tray.show_time.then(|| (now.hour() as u8, now.minute() as u8));
    Some(TrayView {
        title: format::tray_title(&day, time, &tray.format),
        tooltip: [CalendarKind::Persian, CalendarKind::Gregorian, CalendarKind::Hijri]
            .map(|kind| format::format_long(&day, kind, ctx.locale, ctx.digits))
            .join("\n"),
        day_label: format::number(day.persian.day, tray.format.digits),
        digits: tray.format.digits,
    })
}

#[cfg(target_os = "macos")]
fn apply(tray: &TrayIcon, view: &TrayView) {
    let _ = tray.set_title(Some(&view.title));
    let _ = tray.set_tooltip(Some(&view.tooltip));
}

#[cfg(not(target_os = "macos"))]
fn apply(tray: &TrayIcon, view: &TrayView) {
    let _ = tray.set_icon(Some(raster::day_icon(&view.day_label, view.digits)));
    // Some Linux desktops show this next to the icon; Windows ignores it.
    let _ = tray.set_title(Some(&view.title));
    let _ = tray.set_tooltip(Some(&view.tooltip));
}

/// Shows today in the tray. Unless `force`d, does nothing if that's already shown.
/// Emits `day-changed` when the date rolls over.
pub fn refresh(app: &AppHandle, force: bool) {
    let state = app.state::<AppState>();
    let now = state.now();
    let today = GregorianDate::from(now.date()).to_jdn();
    let Some(view) = tray_view(&state, &now, today) else { return };
    let Ok(mut shown) = state.shown.lock() else { return };
    if !force && shown.as_ref().is_some_and(|(jdn, title)| *jdn == today && *title == view.title) {
        return;
    }
    let day_changed = shown.as_ref().is_some_and(|(jdn, _)| *jdn != today);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        apply(&tray, &view);
    }
    *shown = Some((today, view.title));
    drop(shown);
    if day_changed {
        let _ = app.emit("day-changed", today.0);
    }
}
