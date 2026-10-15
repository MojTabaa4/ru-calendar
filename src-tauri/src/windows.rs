//! The popover window: created hidden at startup, shown under the tray icon,
//! hidden again when it loses focus.

use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalRect, PhysicalSize, Rect, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use ruz_core::Jdn;

use crate::state::AppState;

pub const POPOVER: &str = "popover";
pub const SETTINGS: &str = "settings";
pub const CONVERTER: &str = "converter";
const POPOVER_SIZE: (f64, f64) = (340.0, 610.0);
const SETTINGS_SIZE: (f64, f64) = (560.0, 560.0);
const CONVERTER_SIZE: (f64, f64) = (460.0, 440.0);
/// A tray click first blurs the open popover (hiding it), then arrives as a click;
/// a click this soon after hiding is that same click and must not reopen it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

pub fn create_popover(app: &AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, POPOVER, WebviewUrl::App("popover.html".into()))
        .title("روز")
        .inner_size(POPOVER_SIZE.0, POPOVER_SIZE.1)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .shadow(true)
        .build()?;

    #[cfg(target_os = "macos")]
    fade_like_a_panel(&window);

    let handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(false) => hide_popover(&handle),
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide_popover(&handle);
        }
        _ => {}
    });
    Ok(())
}

/// Borderless windows appear and vanish instantly by default; a utility-window
/// animation gives the popover the short fade of system panels (and follows the
/// Reduce Motion setting by itself).
#[cfg(target_os = "macos")]
fn fade_like_a_panel(window: &WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowAnimationBehavior};
    if let Ok(ns_window) = window.ns_window() {
        // Safety: Tauri returns the window's live NSWindow, and setup runs on the main thread.
        let ns_window = unsafe { &*ns_window.cast::<NSWindow>() };
        ns_window.setAnimationBehavior(NSWindowAnimationBehavior::UtilityWindow);
    }
}

fn popover(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(POPOVER)
}

pub fn hide_popover(app: &AppHandle) {
    let Some(window) = popover(app) else { return };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        if let Ok(mut hidden_at) = app.state::<AppState>().popover_hidden_at.lock() {
            *hidden_at = Some(Instant::now());
        }
    }
}

/// Tray left-click: open under the icon, or close if open.
pub fn toggle_popover(app: &AppHandle, anchor: Option<Rect>) {
    let Some(window) = popover(app) else { return };
    if window.is_visible().unwrap_or(false) {
        hide_popover(app);
        return;
    }
    let just_hidden = app
        .state::<AppState>()
        .popover_hidden_at
        .lock()
        .is_ok_and(|t| t.is_some_and(|t| t.elapsed() < REOPEN_GUARD));
    if !just_hidden {
        show_popover(app, anchor);
    }
}

/// Shows the popover near `anchor` (the tray icon), or near the mouse pointer.
pub fn show_popover(app: &AppHandle, anchor: Option<Rect>) {
    let Some(window) = popover(app) else { return };
    match popover_position(app, &window, anchor) {
        Ok(position) => {
            let _ = window.set_position(position);
        }
        Err(e) => eprintln!("could not position the popover: {e}"),
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("popover-shown", ());
}

/// Opens the settings window, or brings it forward if it's already open.
pub fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    hide_popover(app);
    if let Some(window) = app.get_webview_window(SETTINGS) {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    let window = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings.html".into()))
        .title("تنظیمات روز")
        .inner_size(SETTINGS_SIZE.0, SETTINGS_SIZE.1)
        .resizable(false)
        .maximizable(false)
        .center()
        .build()?;
    window.set_focus()
}

/// Opens the date converter on `jdn` (today if `None`). If it's already open, it
/// comes forward and moves to `jdn`.
pub fn open_converter(app: &AppHandle, jdn: Option<Jdn>) -> tauri::Result<()> {
    hide_popover(app);
    if let Some(window) = app.get_webview_window(CONVERTER) {
        if let Some(jdn) = jdn {
            window.emit("converter-show-day", jdn.0)?;
        }
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    let page = match jdn {
        Some(jdn) => format!("converter.html?jdn={}", jdn.0),
        None => "converter.html".to_owned(),
    };
    let window = WebviewWindowBuilder::new(app, CONVERTER, WebviewUrl::App(page.into()))
        .title("تبدیل تاریخ")
        .inner_size(CONVERTER_SIZE.0, CONVERTER_SIZE.1)
        .resizable(false)
        .maximizable(false)
        .center()
        .build()?;
    window.set_focus()
}

fn popover_position(
    app: &AppHandle,
    window: &WebviewWindow,
    anchor: Option<Rect>,
) -> tauri::Result<PhysicalPosition<i32>> {
    let size = window.outer_size()?;
    let anchor = match anchor {
        Some(rect) => {
            let scale = window.scale_factor()?;
            let position = rect.position.to_physical::<f64>(scale);
            let size = rect.size.to_physical::<f64>(scale);
            Anchor { x: position.x, y: position.y, width: size.width, height: size.height }
        }
        None => {
            let cursor = app.cursor_position()?;
            Anchor { x: cursor.x, y: cursor.y, width: 0.0, height: 0.0 }
        }
    };
    let (cx, cy) = (anchor.x + anchor.width / 2.0, anchor.y + anchor.height / 2.0);
    let monitor = match app.monitor_from_point(cx, cy)? {
        Some(m) => Some(m),
        None => app.primary_monitor()?,
    };
    let Some(monitor) = monitor else {
        return Ok(PhysicalPosition::new(anchor.x as i32, (anchor.y + anchor.height) as i32));
    };
    let gap = (6.0 * monitor.scale_factor()).round();
    Ok(place(anchor, size, *monitor.work_area(), gap))
}

#[derive(Clone, Copy, Debug)]
struct Anchor {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Centres the popover on the anchor, below it when the anchor is in the top half of
/// the screen (macOS menu bar, top panels) and above it otherwise (Windows taskbar),
/// then keeps it inside the work area.
fn place(
    anchor: Anchor,
    size: PhysicalSize<u32>,
    area: PhysicalRect<i32, u32>,
    gap: f64,
) -> PhysicalPosition<i32> {
    let (width, height) = (f64::from(size.width), f64::from(size.height));
    let (left, top) = (f64::from(area.position.x), f64::from(area.position.y));
    let (right, bottom) = (left + f64::from(area.size.width), top + f64::from(area.size.height));

    let x = anchor.x + anchor.width / 2.0 - width / 2.0;
    let y = if anchor.y + anchor.height / 2.0 < (top + bottom) / 2.0 {
        anchor.y + anchor.height + gap
    } else {
        anchor.y - height - gap
    };
    let x = x.clamp(left + gap, (right - width - gap).max(left + gap));
    let y = y.clamp(top, (bottom - height).max(top));
    PhysicalPosition::new(x.round() as i32, y.round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(x: i32, y: i32, w: u32, h: u32) -> PhysicalRect<i32, u32> {
        PhysicalRect { position: PhysicalPosition::new(x, y), size: PhysicalSize::new(w, h) }
    }

    const POPOVER: PhysicalSize<u32> = PhysicalSize::new(680, 1060);

    #[test]
    fn below_a_menu_bar_icon() {
        // Retina MacBook: 2x, menu bar 0..48 px, icon near the right edge.
        let icon = Anchor { x: 2600.0, y: 0.0, width: 120.0, height: 48.0 };
        let p = place(icon, POPOVER, area(0, 48, 3024, 1916), 12.0);
        assert_eq!(p.y, 60);
        assert_eq!(p.x, 2600 + 60 - 340);
    }

    #[test]
    fn clamped_at_the_screen_edge() {
        let icon = Anchor { x: 2990.0, y: 0.0, width: 30.0, height: 48.0 };
        let p = place(icon, POPOVER, area(0, 48, 3024, 1916), 12.0);
        assert_eq!(p.x, 3024 - 680 - 12);
    }

    #[test]
    fn above_a_bottom_taskbar() {
        // Windows: taskbar 1040..1080, work area above it.
        let icon = Anchor { x: 1700.0, y: 1044.0, width: 32.0, height: 32.0 };
        let small = PhysicalSize::new(340, 530);
        let p = place(icon, small, area(0, 0, 1920, 1040), 6.0);
        assert_eq!(p.y, 1044 - 530 - 6);
        assert!(p.x + 340 <= 1920);
    }

    #[test]
    fn on_a_secondary_monitor() {
        let icon = Anchor { x: 3500.0, y: 0.0, width: 40.0, height: 30.0 };
        let small = PhysicalSize::new(340, 530);
        let p = place(icon, small, area(1920, 30, 1920, 1050), 6.0);
        assert!(p.x >= 1920 && p.x + 340 <= 3840);
        assert_eq!(p.y, 36);
    }
}
