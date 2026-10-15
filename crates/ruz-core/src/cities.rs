//! Cities to compute prayer times for (`data/cities.toml`).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::format::Locale;
use crate::prayer::Coordinates;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Region {
    /// Iran.
    Ir,
    World,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct City {
    pub id: String,
    pub fa: String,
    pub en: String,
    pub region: Region,
    pub lat: f64,
    pub lon: f64,
    /// IANA time zone, e.g. "Asia/Tehran".
    pub tz: String,
}

impl City {
    pub fn coordinates(&self) -> Coordinates {
        Coordinates { latitude: self.lat, longitude: self.lon }
    }

    pub fn name(&self, locale: Locale) -> &str {
        if locale == Locale::Fa { &self.fa } else { &self.en }
    }
}

#[derive(Deserialize)]
struct File {
    city: Vec<City>,
}

pub fn cities() -> &'static [City] {
    static CITIES: OnceLock<Vec<City>> = OnceLock::new();
    CITIES.get_or_init(|| {
        toml::from_str::<File>(include_str!("../data/cities.toml"))
            .expect("embedded data/cities.toml is valid (checked by tests)")
            .city
    })
}

pub fn find(id: &str) -> Option<&'static City> {
    cities().iter().find(|c| c.id == id)
}

/// Folds Arabic letter forms and spacing so "تهران", "تهران " and "Tehran" all match.
fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\u{200c}' | '\u{200f}' | ' ' | '-'))
        .map(|c| match c {
            'ي' | 'ى' => 'ی',
            'ك' => 'ک',
            'أ' | 'إ' | 'آ' => 'ا',
            'ة' => 'ه',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// Cities whose Persian or English name contains `query`, Iran first; all of them for "".
pub fn search(query: &str) -> Vec<&'static City> {
    let q = fold(query.trim());
    cities().iter().filter(|c| q.is_empty() || fold(&c.fa).contains(&q) || fold(&c.en).contains(&q)).collect()
}

/// Great-circle distance in kilometres.
pub fn distance_km(lat: f64, lon: f64, city: &City) -> f64 {
    let (p1, p2) = (lat.to_radians(), city.lat.to_radians());
    let dp = p2 - p1;
    let dl = (city.lon - lon).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * 6371.0 * a.sqrt().asin()
}

/// The listed city closest to a point.
pub fn nearest(lat: f64, lon: f64) -> &'static City {
    cities()
        .iter()
        .min_by(|a, b| distance_km(lat, lon, a).total_cmp(&distance_km(lat, lon, b)))
        .expect("the city list isn't empty")
}

/// Where the device is, from "Use my location".
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct LocatedPlace {
    pub latitude: f64,
    pub longitude: f64,
    /// The system's IANA time zone.
    pub time_zone: String,
    pub nearest: City,
    pub distance_km: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_is_valid() {
        let all = cities();
        assert!(all.len() >= 31 + 10);
        assert_eq!(all.iter().filter(|c| c.region == Region::Ir).count(), 31, "one per province");
        for (i, c) in all.iter().enumerate() {
            assert!(all[..i].iter().all(|o| o.id != c.id), "duplicate id {}", c.id);
            assert!(jiff::tz::TimeZone::get(&c.tz).is_ok(), "{}: unknown time zone {}", c.id, c.tz);
            assert!((-90.0..=90.0).contains(&c.lat) && (-180.0..=180.0).contains(&c.lon), "{}", c.id);
            assert!(!c.fa.is_empty() && !c.en.is_empty());
        }
    }

    #[test]
    fn searching() {
        assert_eq!(search("تهران")[0].id, "tehran");
        assert_eq!(search("tehr")[0].id, "tehran");
        assert_eq!(search("مشهد")[0].id, "mashhad");
        assert_eq!(search("بندر عباس")[0].id, "bandar-abbas");
        assert_eq!(search("خرم آباد")[0].id, "khorramabad");
        assert_eq!(search("").len(), cities().len());
        assert!(search("zzzz").is_empty());
    }

    #[test]
    fn nearest_city() {
        assert_eq!(nearest(35.70, 51.42).id, "tehran");
        assert_eq!(nearest(51.45, -0.20).id, "london");
        let tehran = find("tehran").unwrap();
        let mashhad = find("mashhad").unwrap();
        let km = distance_km(tehran.lat, tehran.lon, mashhad);
        assert!((km - 740.0).abs() < 20.0, "{km}");
    }
}
