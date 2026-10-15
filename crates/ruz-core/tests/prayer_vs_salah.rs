//! Cross-checks our prayer times against the `salah` crate (an independent port of
//! the Adhan library) using the same Tehran-method angles.

use chrono::Datelike;
use ruz_core::GregorianDate;
use ruz_core::prayer::{Coordinates, Method, Prayer, prayer_times, round_to_minute};

/// Our method without the timetable adjustment, so both compute the bare angles.
const BARE_TEHRAN: Method = Method { adjust_minutes: [0.0; 8], ..Method::TEHRAN };

#[test]
fn agrees_with_salah_to_the_minute() {
    // Cities within 45° of the equator: further out, short summer nights bring in the
    // high-latitude rules, where we cap Fajr and Isha differently from salah.
    let cities = [
        (35.6892, 51.3890),   // Tehran
        (36.2605, 59.6168),   // Mashhad
        (38.0800, 46.2919),   // Tabriz
        (27.1832, 56.2666),   // Bandar Abbas
        (29.4963, 60.8629),   // Zahedan
        (25.2048, 55.2708),   // Dubai
        (41.0082, 28.9784),   // Istanbul
        (43.6532, -79.3832),  // Toronto
        (34.0522, -118.2437), // Los Angeles
        (-33.8688, 151.2093), // Sydney
        (21.3891, 39.8579),   // Mecca
    ];
    let pairs = [
        (Prayer::Fajr, salah::Prayer::Fajr),
        (Prayer::Sunrise, salah::Prayer::Sunrise),
        (Prayer::Dhuhr, salah::Prayer::Dhuhr),
        (Prayer::Asr, salah::Prayer::Asr),
        // salah 0.7 ignores the method's 4.5° Maghrib angle and gives sunset instead; our
        // Maghrib is checked against Iran's timetables in the unit tests.
        (Prayer::Sunset, salah::Prayer::Maghrib),
        (Prayer::Isha, salah::Prayer::Isha),
    ];
    let mut worst = 0;
    let mut skipped = 0;
    for (latitude, longitude) in cities {
        for day_of_year in (0..365).step_by(5) {
            let date = chrono::NaiveDate::from_yo_opt(2026, day_of_year + 1).unwrap();
            let gregorian = GregorianDate::new(2026, date.month() as u8, date.day() as u8).unwrap();
            let ours = prayer_times(gregorian, Coordinates { latitude, longitude }, &BARE_TEHRAN).unwrap();
            // salah panics on a few inputs (e.g. Zahedan, 2026-08-14); skip those.
            let Ok(theirs) = std::panic::catch_unwind(|| {
                salah::PrayerTimes::new(
                    date,
                    salah::Coordinates::new(latitude, longitude),
                    salah::Method::Tehran.parameters(),
                )
            }) else {
                skipped += 1;
                continue;
            };
            for (ours_prayer, their_prayer) in pairs {
                // Both rounded to the nearest minute, as timetables show them.
                let diff = round_to_minute(ours.get(ours_prayer)).as_second()
                    - theirs.time(their_prayer).timestamp();
                worst = worst.max(diff.abs());
                assert!(diff.abs() <= 60, "{latitude},{longitude} {date} {ours_prayer:?}: {diff}s");
            }
        }
    }
    eprintln!("largest difference: {worst}s, skipped {skipped}");
    assert!(skipped <= 5, "salah failed on {skipped} inputs");
}
