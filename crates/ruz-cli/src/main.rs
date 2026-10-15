//! `ruz`: the calendar engine from the command line.

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use ruz_core::cities;
use ruz_core::clock::{self, SystemClock};
use ruz_core::converter;
use ruz_core::format::{self, Digits};
use ruz_core::month::{self, CalendarContext};
use ruz_core::observances::DateStatus;
use ruz_core::prayer::view::prayer_day;
use ruz_core::settings::{PrayerLocation, PrayerSettings, Settings, TraySettings};
use ruz_core::{CalendarKind, GregorianDate, JalaliDate, Jdn, ObservanceDb, ObservanceFilter, convert};
use serde_json::Value;

#[derive(Parser)]
#[command(name = "ruz", version, about = "Persian, Gregorian and Lunar Hijri calendar")]
struct Cli {
    /// IANA time zone for "today" (default: the system's).
    #[arg(long, global = true)]
    tz: Option<String>,
    /// Use Latin digits.
    #[arg(long, global = true)]
    latin: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Today's date in all three calendars.
    Today {
        #[arg(long)]
        json: bool,
    },
    /// Convert a date, e.g. `ruz convert --from g 2025-03-21`.
    Convert {
        #[arg(long, value_enum)]
        from: Kind,
        /// YYYY-MM-DD in the `--from` calendar.
        date: String,
        #[arg(long)]
        json: bool,
    },
    /// A Solar Hijri month as a grid (default: the current month).
    Month {
        year: Option<i32>,
        month: Option<u8>,
        #[arg(long)]
        json: bool,
    },
    /// Holidays and occasions in a Solar Hijri year.
    Holidays {
        year: i32,
        /// Only days off.
        #[arg(long)]
        days_off: bool,
        /// Include every category, not just the default ones.
        #[arg(long)]
        all: bool,
    },
    /// Prayer times (Institute of Geophysics method) for a city.
    Prayer {
        /// City id or name, e.g. `tehran` or `مشهد`.
        #[arg(long, default_value = "tehran")]
        city: String,
        /// Gregorian YYYY-MM-DD (default: today).
        #[arg(long)]
        date: Option<String>,
    },
    /// Run an app command and print its JSON result (used by the UI dev server).
    #[command(hide = true)]
    Json {
        command: String,
        #[arg(default_value = "{}")]
        args: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Kind {
    #[value(alias = "j", alias = "jalali", alias = "shamsi")]
    Persian,
    #[value(alias = "g")]
    Gregorian,
    #[value(alias = "h", alias = "q")]
    Hijri,
}

impl From<Kind> for CalendarKind {
    fn from(k: Kind) -> Self {
        match k {
            Kind::Persian => CalendarKind::Persian,
            Kind::Gregorian => CalendarKind::Gregorian,
            Kind::Hijri => CalendarKind::Hijri,
        }
    }
}

fn parse_ymd(s: &str) -> Result<(i32, u8, u8)> {
    let parts: Vec<&str> = s.split(['-', '/']).collect();
    let [y, m, d] = parts.as_slice() else { bail!("expected YYYY-MM-DD, got {s:?}") };
    Ok((y.parse()?, m.parse()?, d.parse()?))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let tz = match &cli.tz {
        Some(name) => TimeZone::get(name).with_context(|| format!("unknown time zone {name:?}"))?,
        None => TimeZone::system(),
    };
    let mut ctx = CalendarContext::default();
    if cli.latin {
        ctx.digits = Digits::Latin;
    }
    let db = ObservanceDb::embedded();
    let today = clock::today(&SystemClock, &tz);

    match cli.command {
        Command::Today { json } => {
            let details = month::day_details(&ctx, db, today)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&details)?);
            } else {
                print_details(&details);
            }
        }
        Command::Convert { from, date, json } => {
            let (y, m, d) = parse_ymd(&date)?;
            let info = convert(from.into(), y, m, d, &ctx.hijri)?;
            let details = month::day_details(&ctx, db, info.jdn)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&details)?);
            } else {
                print_details(&details);
            }
        }
        Command::Month { year, month, json } => {
            let current = JalaliDate::from_jdn(today)?;
            let grid = month::month_grid(
                &ctx,
                db,
                year.unwrap_or(current.year),
                month.unwrap_or(current.month),
                today,
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&grid)?);
            } else {
                print_grid(&grid);
            }
        }
        Command::Holidays { year, days_off, all } => {
            if all {
                ctx.filter = ObservanceFilter {
                    official_main: true,
                    official_appendix: true,
                    official_professions: true,
                    historical: true,
                    shia_extra: true,
                };
            }
            let first = JalaliDate::new(year, 1, 1)?.to_jdn();
            let next = JalaliDate::new(year + 1, 1, 1)?.to_jdn();
            for jdn in first.0..next.0 {
                let d = month::day_details(&ctx, db, Jdn(jdn))?;
                for o in d.observances.iter().filter(|o| !days_off || o.day_off) {
                    let mark = if o.day_off { "*" } else { " " };
                    println!("{mark} {:<24} {:<18} {}", d.persian, d.gregorian_date.to_string(), o.title);
                }
            }
        }
        Command::Prayer { city, date } => {
            let found = cities::find(&city).or_else(|| cities::search(&city).into_iter().next());
            let city = found.with_context(|| format!("no city matches {city:?}"))?;
            let date = match date {
                Some(d) => {
                    let (y, m, d) = parse_ymd(&d)?;
                    GregorianDate::new(y, m, d)?
                }
                None => GregorianDate::from_jdn(today),
            };
            let settings = Settings {
                digits: ctx.digits,
                prayer: PrayerSettings {
                    enabled: true,
                    location: PrayerLocation::City { id: city.id.clone() },
                },
                ..Settings::default()
            };
            let view = prayer_day(&settings.prayer_place(), date, Timestamp::now(), ctx.locale, ctx.digits)?;
            println!("{} — {}", view.place, date);
            for t in &view.times {
                println!("  {:<14} {}{}", t.name, t.time, if t.next { "  ←" } else { "" });
            }
            if let Some(c) = view.countdown {
                println!("{c}");
            }
        }
        Command::Json { command, args } => {
            let args: Value = serde_json::from_str(&args).context("args must be JSON")?;
            println!("{}", serde_json::to_string(&run_json(&ctx, db, today, &command, &args)?)?);
        }
    }
    Ok(())
}

fn print_details(d: &month::DayDetails) {
    println!("{} {}", d.weekday, d.persian);
    println!("{}", d.gregorian);
    println!("{}{}", d.hijri, if d.hijri_approximate { " (≈)" } else { "" });
    for o in &d.observances {
        println!("  {} {}", if o.day_off { "*" } else { "-" }, o.title);
    }
}

fn print_grid(grid: &month::MonthGrid) {
    println!("{}    {} · {}", grid.title, grid.gregorian_span, grid.hijri_span);
    let header: Vec<String> = grid.weekdays.iter().map(|w| format!("{:>3}", w.short)).collect();
    println!("{}", header.join(" "));
    for week in grid.cells.chunks(7) {
        let row: Vec<String> = week
            .iter()
            .map(|c| {
                if !c.in_month {
                    return "   ".to_owned();
                }
                let mark = match (c.is_today, c.status) {
                    (true, _) => '<',
                    (_, DateStatus::DayOff) => '*',
                    (_, DateStatus::Weekend) => '.',
                    _ => ' ',
                };
                format!("{:>2}{mark}", c.label)
            })
            .collect();
        println!("{}", row.join(" "));
    }
    println!("* day off   . weekend   < today");
}

/// The same commands the Tauri app exposes, with the same argument and result shapes.
fn run_json(
    ctx: &CalendarContext,
    db: &ObservanceDb,
    today: Jdn,
    command: &str,
    args: &Value,
) -> Result<Value> {
    let int = |key: &str| args.get(key).and_then(Value::as_i64);
    Ok(match command {
        "get_today" => serde_json::to_value(month::today_view(ctx, today)?)?,
        "get_month" => {
            let current = JalaliDate::from_jdn(today)?;
            let year = int("year").map_or(current.year, |y| y as i32);
            let month = int("month").map_or(current.month, |m| m as u8);
            serde_json::to_value(month::month_grid(ctx, db, year, month, today)?)?
        }
        "get_day" => {
            let jdn = int("jdn").context("get_day needs jdn")?;
            serde_json::to_value(month::day_details(ctx, db, Jdn(jdn as i32))?)?
        }
        "convert_date" => {
            let from: CalendarKind = serde_json::from_value(args["from"].clone())?;
            let (y, m, d) = (int("year"), int("month"), int("day"));
            let (Some(y), Some(m), Some(d)) = (y, m, d) else { bail!("convert_date needs year, month, day") };
            serde_json::to_value(converter::convert_date(ctx, db, from, y as i32, m as u8, d as u8)?)?
        }
        "converter_months" => {
            let kind: CalendarKind = serde_json::from_value(args["kind"].clone())?;
            let year = int("year").context("converter_months needs year")?;
            serde_json::to_value(converter::months_view(ctx, kind, year as i32)?)?
        }
        "get_prayer_times" => {
            let settings = Settings::default();
            let date = ruz_core::GregorianDate::from_jdn(int("jdn").map_or(today, |j| Jdn(j as i32)));
            let place = settings.prayer_place();
            serde_json::to_value(prayer_day(&place, date, Timestamp::now(), ctx.locale, ctx.digits)?)?
        }
        "search_cities" => {
            let query = args.get("query").and_then(Value::as_str).unwrap_or("");
            serde_json::to_value(cities::search(query))?
        }
        // No system calendar or location from the command line.
        "platform_features" => serde_json::json!({ "calendars": false, "location": false }),
        "calendar_access" | "request_calendar_access" => Value::String("unsupported".into()),
        "list_calendars" | "get_events" => Value::Array(Vec::new()),
        // Nor Google: the bridge has no OAuth client or keychain.
        "google_status" => serde_json::json!({ "configured": false, "accounts": [] }),
        "google_connect" => bail!("unsupported"),
        "google_cancel_sign_in" | "google_disconnect" | "open_event_link" | "open_config_folder" => {
            Value::Null
        }
        "locate_me" => bail!("unsupported"),
        // The bridge keeps no state: settings are always the defaults, and updates are
        // only validated and echoed back.
        "get_settings" => serde_json::to_value(Settings::default())?,
        "update_settings" => {
            let settings: Settings = serde_json::from_value(args["settings"].clone())?;
            serde_json::to_value(settings.normalized())?
        }
        "tray_preview" => {
            let tray: TraySettings = serde_json::from_value(args["tray"].clone())?;
            let now = jiff::Zoned::now();
            let time = tray.show_time.then(|| (now.hour() as u8, now.minute() as u8));
            Value::String(format::tray_title(&ruz_core::day_info(today, &ctx.hijri)?, time, &tray.format))
        }
        "hide_popover" | "quit" | "open_window" => Value::Null,
        other => bail!("unknown command {other:?}"),
    })
}
