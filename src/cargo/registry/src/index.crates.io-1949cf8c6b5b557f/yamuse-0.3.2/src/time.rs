//! the two clock readings and the one date format this crate needs.
//!
//! not `chrono`. the whole requirement is "seconds since the epoch", "milliseconds
//! since the epoch" and one iso-8601 rendering, which is a few dozen lines —
//! against a dependency tree that would otherwise be the largest in the crate.
//!
//! every reading saturates rather than panicking. a clock before the epoch is a
//! misconfigured machine, and refusing to report playback because of it would be
//! the wrong trade.

use std::time::{SystemTime, UNIX_EPOCH};

/// seconds since the unix epoch.
pub(crate) fn unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// seconds since the unix epoch, with fractional precision.
///
/// the rotor's feedback endpoint takes a float here, unlike everything else.
pub(crate) fn unix_secs_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// milliseconds since the unix epoch.
///
/// only ynison reads the clock this finely, so it is dead code without that feature.
#[cfg_attr(not(feature = "realtime"), allow(dead_code))]
pub(crate) fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// an iso-8601 timestamp for "now", to the second: `yyyy-mm-ddThh:mm:ssZ`.
pub(crate) fn iso_now() -> String {
    iso_from_unix_secs(unix_secs())
}

/// render a unix timestamp in the shape the play-audio endpoint expects.
///
/// split out from [`iso_now`] so the formatting can be tested against fixed
/// instants rather than whatever the clock happens to say.
pub(crate) fn iso_from_unix_secs(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let time_of_day = secs.rem_euclid(86_400);
    let (hour, minute, second) = (
        time_of_day / 3600,
        (time_of_day % 3600) / 60,
        time_of_day % 60,
    );
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// days since the unix epoch to a gregorian date (howard hinnant's algorithm).
pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let month = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_converts_to_its_known_date() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn leap_years_and_century_boundaries_convert_correctly() {
        assert_eq!(civil_from_days(59), (1970, 3, 1));
        // 2000-02-29 — a leap year despite being a century
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        // 2026-07-30
        assert_eq!(civil_from_days(20_664), (2026, 7, 30));
    }

    #[test]
    fn timestamps_render_in_the_shape_the_api_expects() {
        assert_eq!(iso_from_unix_secs(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_from_unix_secs(1_700_000_000), "2023-11-14T22:13:20Z");
        // the last second of a day, which off-by-one arithmetic gets wrong
        assert_eq!(iso_from_unix_secs(86_399), "1970-01-01T23:59:59Z");
        assert_eq!(iso_from_unix_secs(86_400), "1970-01-02T00:00:00Z");
    }

    #[test]
    fn a_clock_before_the_epoch_still_renders_a_valid_timestamp() {
        // euclidean division, not truncating: -1 is the second before the epoch,
        // not a negative time-of-day on the epoch itself
        assert_eq!(iso_from_unix_secs(-1), "1969-12-31T23:59:59Z");
    }

    #[test]
    fn the_live_readings_agree_with_each_other() {
        let secs = unix_secs();
        let millis = unix_millis();
        assert!(millis / 1000 - secs <= 1, "{millis} vs {secs}");
        // the seconds count is nowhere near f64's exact-integer range
        #[allow(clippy::cast_precision_loss)]
        let as_float = secs as f64;
        assert!((unix_secs_f64() - as_float).abs() < 2.0);

        let now = iso_now();
        assert_eq!(now.len(), 20, "yyyy-mm-ddThh:mm:ssZ");
        assert!(now.ends_with('Z'));
        assert_eq!(now.as_bytes()[10], b'T');
    }
}
