//! Position of the sun from NOAA's solar calculator equations (after Meeus), good to
//! a few seconds of time for dates within a few centuries of 2000.

/// The sun's apparent declination (degrees) and the equation of time (hours) at a Julian date.
#[derive(Clone, Copy, Debug)]
pub struct SunPosition {
    pub declination: f64,
    pub equation_of_time: f64,
}

pub(crate) fn sin_deg(d: f64) -> f64 {
    d.to_radians().sin()
}

pub(crate) fn cos_deg(d: f64) -> f64 {
    d.to_radians().cos()
}

pub(crate) fn tan_deg(d: f64) -> f64 {
    d.to_radians().tan()
}

pub fn sun_position(julian_date: f64) -> SunPosition {
    let t = (julian_date - 2_451_545.0) / 36_525.0;
    let mean_longitude = (280.466_46 + t * (36_000.769_83 + t * 0.000_303_2)).rem_euclid(360.0);
    let mean_anomaly = 357.529_11 + t * (35_999.050_29 - 0.000_153_7 * t);
    let eccentricity = 0.016_708_634 - t * (0.000_042_037 + 0.000_000_126_7 * t);
    let center = sin_deg(mean_anomaly) * (1.914_602 - t * (0.004_817 + 0.000_014 * t))
        + sin_deg(2.0 * mean_anomaly) * (0.019_993 - 0.000_101 * t)
        + sin_deg(3.0 * mean_anomaly) * 0.000_289;
    let node = 125.04 - 1934.136 * t;
    let apparent_longitude = mean_longitude + center - 0.005_69 - 0.004_78 * sin_deg(node);
    let mean_obliquity =
        23.0 + (26.0 + (21.448 - t * (46.815 + t * (0.000_59 - t * 0.001_813))) / 60.0) / 60.0;
    let obliquity = mean_obliquity + 0.002_56 * cos_deg(node);

    let declination = (sin_deg(obliquity) * sin_deg(apparent_longitude)).asin().to_degrees();
    let y = tan_deg(obliquity / 2.0).powi(2);
    let (l, m, e) = (mean_longitude, mean_anomaly, eccentricity);
    let equation_of_time_radians = y * sin_deg(2.0 * l) - 2.0 * e * sin_deg(m)
        + 4.0 * e * y * sin_deg(m) * cos_deg(2.0 * l)
        - 0.5 * y * y * sin_deg(4.0 * l)
        - 1.25 * e * e * sin_deg(2.0 * m);
    // Radians of hour angle to hours.
    SunPosition { declination, equation_of_time: equation_of_time_radians.to_degrees() / 15.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solstices_and_equation_of_time() {
        // 2026-06-21 12:00 UT: declination near +23.44°.
        let june = sun_position(2_461_213.0);
        assert!((june.declination - 23.44).abs() < 0.05, "{june:?}");
        // Early November the sun runs about 16 minutes fast.
        let november = sun_position(2_461_349.0);
        assert!((november.equation_of_time * 60.0 - 16.4).abs() < 0.5, "{november:?}");
        // Mid-February it runs about 14 minutes slow.
        let february = sun_position(2_461_087.0);
        assert!((february.equation_of_time * 60.0 + 14.2).abs() < 0.5, "{february:?}");
    }
}
