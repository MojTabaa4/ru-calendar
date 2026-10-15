import { invoke, isTauri } from "@tauri-apps/api/core";
import type { AccessStatus } from "./bindings/AccessStatus";
import type { CalendarInfo } from "./bindings/CalendarInfo";
import type { CalendarKind } from "./bindings/CalendarKind";
import type { DayDetails } from "./bindings/DayDetails";
import type { EventView } from "./bindings/EventView";
import type { Features } from "./bindings/Features";
import type { GoogleStatus } from "./bindings/GoogleStatus";
import type { LocatedPlace } from "./bindings/LocatedPlace";
import type { MonthGrid } from "./bindings/MonthGrid";
import type { City } from "./bindings/City";
import type { MonthsView } from "./bindings/MonthsView";
import type { PrayerDayView } from "./bindings/PrayerDayView";
import type { Settings } from "./bindings/Settings";
import type { TodayView } from "./bindings/TodayView";
import type { TraySettings } from "./bindings/TraySettings";

/** Calls a Tauri command, or the dev server's CLI bridge when running in a plain browser. */
async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isTauri()) {
    return invoke<T>(command, args);
  }
  const response = await fetch(`/__ruz/${command}`, { method: "POST", body: JSON.stringify(args) });
  if (!response.ok) {
    throw new Error(await response.text());
  }
  return response.json() as Promise<T>;
}

export const api = {
  today: () => call<TodayView>("get_today"),
  month: (year?: number, month?: number) => call<MonthGrid>("get_month", { year, month }),
  day: (jdn: number) => call<DayDetails>("get_day", { jdn }),
  convertDate: (from: CalendarKind, year: number, month: number, day: number) =>
    call<DayDetails>("convert_date", { from, year, month, day }),
  converterMonths: (kind: CalendarKind, year: number) => call<MonthsView>("converter_months", { kind, year }),
  /** Prayer times on `jdn` (today if omitted), or null when turned off. */
  prayerTimes: (jdn?: number) => call<PrayerDayView | null>("get_prayer_times", { jdn }),
  searchCities: (query: string) => call<City[]>("search_cities", { query }),
  features: () => call<Features>("platform_features"),
  calendarAccess: () => call<AccessStatus>("calendar_access"),
  requestCalendarAccess: () => call<AccessStatus>("request_calendar_access"),
  calendars: () => call<CalendarInfo[]>("list_calendars"),
  events: (startJdn: number, endJdn: number) => call<EventView[]>("get_events", { startJdn, endJdn }),
  googleStatus: () => call<GoogleStatus>("google_status"),
  /** Signs in through the browser; the account's email, or null if the user cancelled. */
  googleConnect: () => call<string | null>("google_connect"),
  googleCancelSignIn: () => call<void>("google_cancel_sign_in"),
  googleDisconnect: (email: string) => call<void>("google_disconnect", { email }),
  openEventLink: (url: string) => call<void>("open_event_link", { url }),
  /** Opens the folder that holds settings.json and google-client.json. */
  openConfigFolder: () => call<void>("open_config_folder"),
  locateMe: () => call<LocatedPlace>("locate_me"),
  settings: () => call<Settings>("get_settings"),
  updateSettings: (settings: Settings) => call<Settings>("update_settings", { settings }),
  trayPreview: (tray: TraySettings) => call<string>("tray_preview", { tray }),
  /** Opens a window; the converter starts on `jdn` when given. */
  openWindow: (name: "settings" | "converter", jdn?: number) => call<void>("open_window", { name, jdn }),
  hidePopover: () => call<void>("hide_popover"),
  quit: () => call<void>("quit"),
};
