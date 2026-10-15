# Ruz (روز)

A Persian calendar for the macOS menu bar and the Windows/Linux system tray,
written in Rust with [Tauri v2](https://tauri.app). It shows today's Solar Hijri
date in the menu bar and opens a month calendar with Iranian holidays and the
Gregorian and Lunar Hijri dates of each day.

## Layout

| Path | What |
|---|---|
| `crates/ruz-core` | Calendar engine: Jalali, Gregorian and Hijri dates, holidays, month views. No Tauri. |
| `crates/ruz-core/data` | Holiday and Iranian Hijri data (TOML), with sources in `SOURCES.md`. |
| `crates/ruz-cli` | `ruz` command-line tool over the engine. |
| `src-tauri` | The tray app: tray icon, popover window, commands. |
| `src` | Frontend (Svelte 5 + TypeScript), right-to-left Persian UI. |

## Develop

Needs Rust 1.98.1 (pinned in `rust-toolchain.toml`) and [Bun](https://bun.sh).

```bash
bun install
bun run tauri dev
```

The calendar appears in the menu bar (macOS) or tray (Windows, Linux). Click it
to open the month view. In debug builds:

- `RUZ_OPEN_POPOVER=1` opens the calendar at launch.
- `RUZ_FAKE_NOW=2026-03-20T23:59:30+03:30` pretends it is that time, to test the day rolling over.

`bun run dev` alone serves the UI at http://localhost:1420/popover.html in a normal
browser. Calls that would go to the app are answered by the CLI instead.

The CLI:

```bash
cargo run -p ruz-cli -- today
cargo run -p ruz-cli -- month 1405 1
cargo run -p ruz-cli -- convert --from g 2026-03-21
cargo run -p ruz-cli -- holidays 1405 --days-off
cargo run -p ruz-cli -- prayer --city mashhad --date 2026-10-15
```

Checks:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
bun run check
```

After changing view-model types in `ruz-core`, regenerate the TypeScript types in
`src/lib/bindings` with `cargo test -p ruz-core --features ts`.

## Install on macOS

```bash
scripts/build-app.sh
```

This builds a release bundle, signs it, installs it as `/Applications/Ruz.app` and
launches it. `--no-launch` installs without launching; `RUZ_INSTALL=~/Applications/Ruz.app`
installs elsewhere. It needs `rcodesign` once:

```bash
cargo install apple-codesign --locked --bin rcodesign
```

The bundle is signed with a self-signed certificate that `scripts/make-cert.sh`
creates on the first run, in `~/Library/Application Support/ai.intelcad.ruz/dev-signing`.
No keychain is used. Because the signature names that certificate rather than the
binary's hash, macOS keeps the Calendar and Location permissions, and Keychain
access to the Google sign-ins, across rebuilds. Keep the certificate: a new one
resets them. `scripts/verify-app.sh` checks an installed bundle.

## macOS Calendar and location

On macOS, Ruz shows your Calendar events and Birthdays in the month view and can
use your location for prayer times. macOS only asks for these permissions for a
real app, so test them with a bundle rather than `tauri dev`. `scripts/build-app.sh`
installs one; for a quick debug bundle:

```bash
bun run tauri build --debug --bundles app
```

```bash
open target/debug/bundle/macos/Ruz.app
```

Quit any `tauri dev` copy first: only one Ruz runs at a time. To reset the
permissions while testing:

```bash
tccutil reset Calendar ai.intelcad.ruz
```

## Google Calendar

Ruz can show events from one or more Google accounts (read-only). It signs in
through your browser and keeps each account's sign-in in the system keychain
(macOS Keychain, Windows Credential Manager, Secret Service on Linux). It works on
every platform.

Google needs an OAuth client for the app. Create your own once:

1. Create a project at <https://console.cloud.google.com/projectcreate>, for
   example "Ruz".
2. Turn on the Google Calendar API for it:
   <https://console.cloud.google.com/apis/library/calendar-json.googleapis.com>,
   then **Enable**.
3. Set up the consent screen at <https://console.cloud.google.com/auth/overview>
   and click **Get started**:
   - App name "Ruz" and your email as the support email;
   - Audience **External**;
   - your email as the contact address, then **Create**.
4. Under **Audience**, either:
   - **Publish app** (recommended for personal use). Google then shows a "Google
     hasn't verified this app" page at sign-in: choose **Advanced**, then **Go to
     Ruz**. Sign-ins last until you disconnect.
   - Or stay in **Testing** and add every Google account you'll connect under
     **Test users**. Google ends Testing sign-ins after 7 days, so you'd reconnect
     weekly.
5. Under **Clients**, click **Create client**, choose **Desktop app**, name it
   "Ruz", click **Create**, then **Download JSON**.
6. Save the file as `google-client.json` in Ruz's settings folder. In Ruz, Settings
   → Calendars → **Open settings folder** opens it. The folder is:
   - macOS: `~/Library/Application Support/ai.intelcad.ruz/`
   - Windows: `%APPDATA%\ai.intelcad.ruz\`
   - Linux: `~/.config/ai.intelcad.ruz/`

   Instead of the file, you can set `RUZ_GOOGLE_CLIENT_ID` and
   `RUZ_GOOGLE_CLIENT_SECRET`. Set them when Ruz runs, or when you build it to
   build the client into the app.
7. In Settings → Calendars, click **Connect to Google** (اتصال به گوگل). Pick the
   account in the browser and allow access, then choose which calendars to show.
   **Add account** connects more accounts.

Details:

- Ruz asks only for `calendar.readonly` and your email address. **Disconnect**
  (قطع اتصال) revokes the sign-in at Google and deletes it from the keychain.
- Events are cached in memory. Cached events show at once and are refreshed in
  the background after 5 minutes. Without a network, the last events fetched stay
  on screen.
- Clicking a Google event in the popover opens it in Google Calendar.
- If a Google calendar is also added to macOS Calendar, its events show once.
- Calendars switched off in Google Calendar start hidden. So does Google's
  "Holidays in Iran", since Ruz shows the official occasions itself.

## Settings

Open them from the gear in the calendar or the tray menu. Changes apply at once and
are saved as `settings.json` in the app's config folder
(`~/Library/Application Support/ai.intelcad.ruz` on macOS,
`%APPDATA%\ai.intelcad.ruz` on Windows, `~/.config/ai.intelcad.ruz` on Linux).
A file that can't be read is kept as `settings.json.invalid` and defaults are used.
Start at login uses a Launch Agent on macOS, the Run registry key on Windows and
an autostart entry on Linux.

## Notes

- Lunar Hijri dates use Umm al-Qura, shown with "≈", until Iran's official month
  starts are added to `crates/ruz-core/data/hijri_iran.toml`.
- The Jalali algorithm (Borkowski) and ICU's astronomical calendar disagree only on
  whether 1502 or 1503 SH is a leap year; see `crates/ruz-core/tests/jalali_vs_icu.rs`.
- Prayer times follow the Institute of Geophysics (University of Tehran) method:
  Fajr 17.7°, Maghrib 4.5°, Isha 14°, midnight halfway from sunset to dawn, with
  Fajr a minute earlier to match Iran's published timetables. Tests check them
  against official times for three cities and against the `salah` crate.
- Font: [Vazirmatn](https://github.com/rastikerdar/vazirmatn), SIL Open Font License.
