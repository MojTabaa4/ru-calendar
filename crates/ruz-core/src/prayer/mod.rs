//! Prayer times by the method of the Institute of Geophysics, University of Tehran,
//! the method behind Iran's official timetables.

pub mod solar;
pub mod view;

use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use self::solar::{cos_deg, sin_deg, sun_position, tan_deg};
use crate::day::{GregorianDate, Jdn};

/// A place on Earth, in degrees (north and east positive). Iran's timetables use the
/// sea-level horizon, so elevation plays no part.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Prayer {
    Fajr,
    Sunrise,
    Dhuhr,
    Asr,
    Sunset,
    Maghrib,
    Isha,
    /// Religious midnight: halfway from sunset to the next dawn.
    Midnight,
}

impl Prayer {
    pub const ALL: [Prayer; 8] = [
        Prayer::Fajr,
        Prayer::Sunrise,
        Prayer::Dhuhr,
        Prayer::Asr,
        Prayer::Sunset,
        Prayer::Maghrib,
        Prayer::Isha,
        Prayer::Midnight,
    ];

    /// The six times on Iran's official timetables (no Asr or Isha).
    pub const OFFICIAL: [Prayer; 6] =
        [Prayer::Fajr, Prayer::Sunrise, Prayer::Dhuhr, Prayer::Sunset, Prayer::Maghrib, Prayer::Midnight];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Method {
    /// Depression of the sun below the horizon, in degrees.
    pub fajr_angle: f64,
    pub isha_angle: f64,
    pub maghrib_angle: f64,
    /// Shadow length (in object lengths) added at noon for Asr; 1 for Shia and Shafi'i.
    pub asr_shadow: f64,
    /// Minutes added to each time, indexed by [`Prayer`].
    pub adjust_minutes: [f64; 8],
}

impl Method {
    /// Institute of Geophysics, University of Tehran: 17.7° / 4.5° / 14°, midnight
    /// halfway from sunset to dawn. Iran's published timetables put dawn about a minute
    /// before the bare 17.7° time, so Fajr is moved a minute earlier to match them.
    pub const TEHRAN: Method = Method {
        fajr_angle: 17.7,
        isha_angle: 14.0,
        maghrib_angle: 4.5,
        asr_shadow: 1.0,
        adjust_minutes: [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PrayerError {
    #[error("the sun doesn't rise or set here on this day")]
    NoSunriseOrSunset,
}

/// One day's times, as instants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrayerTimes {
    pub date: GregorianDate,
    times: [Timestamp; 8],
}

impl PrayerTimes {
    pub fn get(&self, prayer: Prayer) -> Timestamp {
        self.times[prayer as usize]
    }
}

/// Solar hours of one day in local mean solar time (hour 12 ≈ noon at this longitude).
#[derive(Clone, Copy)]
struct SolarHours {
    fajr: f64,
    sunrise: f64,
    dhuhr: f64,
    asr: f64,
    sunset: f64,
    maghrib: f64,
    isha: f64,
}

struct Day {
    /// Julian date of 0h UT on the civil date.
    jd0: f64,
    coords: Coordinates,
}

impl Day {
    fn julian_date(&self, solar_hour: f64) -> f64 {
        self.jd0 - self.coords.longitude / 360.0 + solar_hour / 24.0
    }

    fn noon(&self, solar_hour: f64) -> f64 {
        12.0 - sun_position(self.julian_date(solar_hour)).equation_of_time
    }

    /// Hours from noon until the sun reaches `altitude` degrees, or `None` if it never does.
    fn hour_angle(&self, altitude: f64, solar_hour: f64) -> Option<f64> {
        let decl = sun_position(self.julian_date(solar_hour)).declination;
        let lat = self.coords.latitude;
        let cos_t = (sin_deg(altitude) - sin_deg(decl) * sin_deg(lat)) / (cos_deg(decl) * cos_deg(lat));
        (-1.0..=1.0).contains(&cos_t).then(|| cos_t.acos().to_degrees() / 15.0)
    }

    /// Altitude at which a shadow is `shadow` lengths longer than at noon.
    fn asr_altitude(&self, shadow: f64, noon: f64) -> f64 {
        let decl = sun_position(self.julian_date(noon)).declination;
        (1.0 / (shadow + tan_deg((self.coords.latitude - decl).abs()))).atan().to_degrees()
    }

    /// Two refinement passes: each evaluates the sun's position at the previous estimate.
    fn solve(&self, method: &Method) -> Result<SolarHours, PrayerError> {
        // Refraction and the sun's radius: the upper limb touches the horizon at -0.833°.
        let horizon = -0.833;
        let mut h = SolarHours {
            fajr: 5.0,
            sunrise: 6.0,
            dhuhr: 12.0,
            asr: 13.0,
            sunset: 18.0,
            maghrib: 18.0,
            isha: 18.0,
        };
        for _ in 0..2 {
            let before = |altitude: f64, at: f64| self.hour_angle(altitude, at).map(|t| self.noon(at) - t);
            let after = |altitude: f64, at: f64| self.hour_angle(altitude, at).map(|t| self.noon(at) + t);
            let sunrise = before(horizon, h.sunrise).ok_or(PrayerError::NoSunriseOrSunset)?;
            let sunset = after(horizon, h.sunset).ok_or(PrayerError::NoSunriseOrSunset)?;
            let night = 24.0 + sunrise - sunset;
            // Far from the equator twilight may never end; then each time is the angle's
            // share of the night (1/60 of the night per degree), measured from sunrise or sunset.
            let portion = |angle: f64| angle / 60.0 * night;
            let limit_before = |time: Option<f64>, angle: f64| match time {
                Some(t) if sunrise - t <= portion(angle) => t,
                _ => sunrise - portion(angle),
            };
            let limit_after = |time: Option<f64>, angle: f64| match time {
                Some(t) if t - sunset <= portion(angle) => t,
                _ => sunset + portion(angle),
            };
            h = SolarHours {
                fajr: limit_before(before(-method.fajr_angle, h.fajr), method.fajr_angle),
                sunrise,
                dhuhr: self.noon(h.dhuhr),
                asr: after(self.asr_altitude(method.asr_shadow, h.dhuhr), h.asr).unwrap_or(sunset),
                sunset,
                maghrib: limit_after(after(-method.maghrib_angle, h.maghrib), method.maghrib_angle),
                isha: limit_after(after(-method.isha_angle, h.isha), method.isha_angle),
            };
        }
        Ok(h)
    }

    fn timestamp(&self, solar_hour: f64) -> Timestamp {
        // Seconds since the Unix epoch at 0h UT of the date, plus the time in UT.
        let midnight_utc = (self.jd0 + 0.5 - 2_440_588.0) * 86_400.0;
        let ut_hours = solar_hour - self.coords.longitude / 15.0;
        Timestamp::from_second((midnight_utc + ut_hours * 3600.0).round() as i64).expect("in jiff's range")
    }
}

pub fn prayer_times(
    date: GregorianDate,
    coords: Coordinates,
    method: &Method,
) -> Result<PrayerTimes, PrayerError> {
    let day = |jdn: Jdn| Day { jd0: f64::from(jdn.0) - 0.5, coords };
    let today = day(date.to_jdn());
    let tomorrow = day(date.to_jdn() + 1);
    let h = today.solve(method)?;
    // Midnight uses tomorrow's dawn before any adjustment, like sunset.
    let next_fajr = tomorrow.timestamp(tomorrow.solve(method)?.fajr);
    let sunset = today.timestamp(h.sunset);
    let midnight = sunset + next_fajr.duration_since(sunset) / 2;
    let mut times = [
        today.timestamp(h.fajr),
        today.timestamp(h.sunrise),
        today.timestamp(h.dhuhr),
        today.timestamp(h.asr),
        sunset,
        today.timestamp(h.maghrib),
        today.timestamp(h.isha),
        midnight,
    ];
    for (time, minutes) in times.iter_mut().zip(method.adjust_minutes) {
        *time += SignedDuration::from_secs_f64(minutes * 60.0);
    }
    Ok(PrayerTimes { date, times })
}

/// Rounds to the nearest minute, as timetables show times.
pub fn round_to_minute(t: Timestamp) -> Timestamp {
    Timestamp::from_second((t.as_second() + 30).div_euclid(60) * 60).expect("in jiff's range")
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::tz::TimeZone;

    const TEHRAN: Coordinates = Coordinates { latitude: 35.6892, longitude: 51.3890 };

    fn minutes_of_day(t: Timestamp) -> i64 {
        let z = round_to_minute(t).to_zoned(TimeZone::get("Asia/Tehran").unwrap());
        i64::from(z.hour()) * 60 + i64::from(z.minute())
    }

    fn parse(hhmm: &str) -> i64 {
        let (h, m) = hhmm.split_once(':').unwrap();
        h.parse::<i64>().unwrap() * 60 + m.parse::<i64>().unwrap()
    }

    #[test]
    fn matches_official_timetables_within_a_minute() {
        // Iran's official times for 23 Mehr 1405 (15 October 2026), as shown on time.ir:
        // dawn, sunrise, noon, sunset, maghrib, midnight.
        let official = [
            ("Tehran", 35.6892, 51.3890, ["04:46", "06:10", "11:50", "17:29", "17:47", "23:08"]),
            ("Mashhad", 36.2605, 59.6168, ["04:13", "05:38", "11:17", "16:56", "17:13", "22:35"]),
            ("Bandar Abbas", 27.1832, 56.2666, ["04:27", "05:45", "11:31", "17:17", "17:33", "22:52"]),
        ];
        let date = GregorianDate::new(2026, 10, 15).unwrap();
        for (city, latitude, longitude, expected) in official {
            let times = prayer_times(date, Coordinates { latitude, longitude }, &Method::TEHRAN).unwrap();
            for (prayer, hhmm) in Prayer::OFFICIAL.iter().zip(expected) {
                let diff = minutes_of_day(times.get(*prayer)) - parse(hhmm);
                assert!(diff.abs() <= 1, "{city} {prayer:?}: off by {diff} min");
            }
        }
    }

    #[test]
    fn times_are_in_order() {
        let times = prayer_times(GregorianDate::new(2026, 3, 21).unwrap(), TEHRAN, &Method::TEHRAN).unwrap();
        let all: Vec<_> = Prayer::ALL.iter().map(|&p| times.get(p)).collect();
        assert!(all.windows(2).all(|w| w[0] < w[1]), "{all:?}");
    }

    #[test]
    fn high_latitudes_fall_back_to_a_share_of_the_night() {
        // Stockholm in June: the sun never gets 17.7° below the horizon.
        let stockholm = Coordinates { latitude: 59.33, longitude: 18.07 };
        let times =
            prayer_times(GregorianDate::new(2026, 6, 21).unwrap(), stockholm, &Method::TEHRAN).unwrap();
        assert!(times.get(Prayer::Fajr) < times.get(Prayer::Sunrise));
        assert!(times.get(Prayer::Isha) > times.get(Prayer::Sunset));
    }

    #[test]
    fn polar_day_has_no_times() {
        let svalbard = Coordinates { latitude: 78.22, longitude: 15.65 };
        let result = prayer_times(GregorianDate::new(2026, 6, 21).unwrap(), svalbard, &Method::TEHRAN);
        assert_eq!(result, Err(PrayerError::NoSunriseOrSunset));
    }
}
