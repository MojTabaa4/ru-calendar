//! Ready-to-show prayer times for the popover and settings.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::Serialize;

use super::{Coordinates, Method, Prayer, PrayerError, prayer_times, round_to_minute};
use crate::day::GregorianDate;
use crate::format::{self, Digits, Locale};

/// Where to compute prayer times, and the time zone to show them in.
#[derive(Clone, Debug)]
pub struct PrayerPlace {
    pub name: String,
    pub coordinates: Coordinates,
    pub time_zone: TimeZone,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PrayerTimeView {
    pub prayer: Prayer,
    pub name: String,
    pub short: String,
    /// "۰۴:۴۶", local to the place.
    pub time: String,
    pub passed: bool,
    pub next: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PrayerDayView {
    pub place: String,
    pub times: Vec<PrayerTimeView>,
    /// "۱ ساعت و ۱۲ دقیقه تا اذان ظهر", when the day shown is today.
    pub countdown: Option<String>,
}

pub fn name(prayer: Prayer, locale: Locale) -> &'static str {
    match (prayer, locale) {
        (Prayer::Fajr, Locale::Fa) => "اذان صبح",
        (Prayer::Sunrise, Locale::Fa) => "طلوع آفتاب",
        (Prayer::Dhuhr, Locale::Fa) => "اذان ظهر",
        (Prayer::Asr, Locale::Fa) => "عصر",
        (Prayer::Sunset, Locale::Fa) => "غروب آفتاب",
        (Prayer::Maghrib, Locale::Fa) => "اذان مغرب",
        (Prayer::Isha, Locale::Fa) => "عشا",
        (Prayer::Midnight, Locale::Fa) => "نیمه‌شب شرعی",
        (Prayer::Fajr, Locale::En) => "Fajr",
        (Prayer::Sunrise, Locale::En) => "Sunrise",
        (Prayer::Dhuhr, Locale::En) => "Dhuhr",
        (Prayer::Asr, Locale::En) => "Asr",
        (Prayer::Sunset, Locale::En) => "Sunset",
        (Prayer::Maghrib, Locale::En) => "Maghrib",
        (Prayer::Isha, Locale::En) => "Isha",
        (Prayer::Midnight, Locale::En) => "Midnight",
    }
}

pub fn short_name(prayer: Prayer, locale: Locale) -> &'static str {
    match (prayer, locale) {
        (Prayer::Fajr, Locale::Fa) => "صبح",
        (Prayer::Sunrise, Locale::Fa) => "طلوع",
        (Prayer::Dhuhr, Locale::Fa) => "ظهر",
        (Prayer::Sunset, Locale::Fa) => "غروب",
        (Prayer::Maghrib, Locale::Fa) => "مغرب",
        (Prayer::Midnight, Locale::Fa) => "نیمه‌شب",
        (p, l) => name(p, l),
    }
}

fn clock(t: Timestamp, tz: &TimeZone, digits: Digits) -> String {
    let local = round_to_minute(t).to_zoned(tz.clone());
    format::localize_digits(&format!("{:02}:{:02}", local.hour(), local.minute()), digits)
}

fn countdown(until: i64, prayer: Prayer, locale: Locale, digits: Digits) -> String {
    let minutes = (until + 59) / 60;
    let (h, m) = (minutes / 60, minutes % 60);
    let n = |v: i64| format::number(v, digits);
    let target = name(prayer, locale);
    match (locale, h) {
        (Locale::Fa, 0) => format!("{} دقیقه تا {target}", n(m)),
        (Locale::Fa, _) if m == 0 => format!("{} ساعت تا {target}", n(h)),
        (Locale::Fa, _) => format!("{} ساعت و {} دقیقه تا {target}", n(h), n(m)),
        (Locale::En, 0) => format!("{} min until {target}", n(m)),
        (Locale::En, _) => format!("{} h {} min until {target}", n(h), n(m)),
    }
}

/// Iran's six official times on `date` at `place`; marks what's passed and next if
/// `date` is today there.
pub fn prayer_day(
    place: &PrayerPlace,
    date: GregorianDate,
    now: Timestamp,
    locale: Locale,
    digits: Digits,
) -> Result<PrayerDayView, PrayerError> {
    let times = prayer_times(date, place.coordinates, &Method::TEHRAN)?;
    let is_today = GregorianDate::from(now.to_zoned(place.time_zone.clone()).date()) == date;
    let next = is_today.then(|| Prayer::OFFICIAL.into_iter().find(|&p| times.get(p) > now)).flatten();

    let countdown = if !is_today {
        None
    } else if let Some(p) = next {
        Some(countdown(times.get(p).as_second() - now.as_second(), p, locale, digits))
    } else {
        // After midnight: count down to tomorrow's dawn.
        let tomorrow = GregorianDate::from_jdn(date.to_jdn() + 1);
        prayer_times(tomorrow, place.coordinates, &Method::TEHRAN).ok().map(|t| {
            countdown(t.get(Prayer::Fajr).as_second() - now.as_second(), Prayer::Fajr, locale, digits)
        })
    };

    Ok(PrayerDayView {
        place: place.name.clone(),
        times: Prayer::OFFICIAL
            .iter()
            .map(|&p| PrayerTimeView {
                prayer: p,
                name: name(p, locale).to_owned(),
                short: short_name(p, locale).to_owned(),
                time: clock(times.get(p), &place.time_zone, digits),
                passed: is_today && times.get(p) <= now,
                next: next == Some(p),
            })
            .collect(),
        countdown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tehran() -> PrayerPlace {
        PrayerPlace {
            name: "تهران".into(),
            coordinates: Coordinates { latitude: 35.6892, longitude: 51.3890 },
            time_zone: TimeZone::get("Asia/Tehran").unwrap(),
        }
    }

    fn at(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    #[test]
    fn marks_the_next_prayer_today() {
        let date = GregorianDate::new(2026, 10, 15).unwrap();
        // 10:00 in Tehran: dawn and sunrise passed, noon next.
        let view =
            prayer_day(&tehran(), date, at("2026-10-15T06:30:00Z"), Locale::Fa, Digits::Persian).unwrap();
        assert_eq!(view.times.len(), 6);
        assert_eq!(view.times[0].time, "۰۴:۴۶");
        assert!(view.times[0].passed && view.times[1].passed && !view.times[2].passed);
        assert!(view.times[2].next);
        assert_eq!(view.countdown.as_deref(), Some("۱ ساعت و ۵۱ دقیقه تا اذان ظهر"));
    }

    #[test]
    fn after_midnight_counts_to_tomorrows_dawn() {
        let date = GregorianDate::new(2026, 10, 15).unwrap();
        let view =
            prayer_day(&tehran(), date, at("2026-10-15T20:00:00Z"), Locale::En, Digits::Latin).unwrap();
        assert!(view.times.iter().all(|t| t.passed && !t.next));
        assert!(view.countdown.unwrap().ends_with("until Fajr"));
    }

    #[test]
    fn other_days_have_no_countdown() {
        let date = GregorianDate::new(2026, 10, 20).unwrap();
        let view =
            prayer_day(&tehran(), date, at("2026-10-15T06:30:00Z"), Locale::Fa, Digits::Persian).unwrap();
        assert!(view.countdown.is_none());
        assert!(view.times.iter().all(|t| !t.passed && !t.next));
    }
}
