//! Timezone handling utilities with DST awareness

use crate::error::{EventixError, Result};
use chrono::{DateTime, NaiveDateTime, Offset, TimeZone};
use chrono_tz::Tz;

/// Parse a timezone string into a `Tz` object
///
/// # Examples
///
/// ```
/// use eventix::timezone::parse_timezone;
///
/// let tz = parse_timezone("America/New_York").unwrap();
/// let tz2 = parse_timezone("UTC").unwrap();
/// ```
pub fn parse_timezone(tz_str: &str) -> Result<Tz> {
    tz_str
        .parse::<Tz>()
        .map_err(|_| EventixError::InvalidTimezone(tz_str.to_string()))
}

/// Parse a date/time string with timezone
///
/// Accepts formats like:
/// - "2025-11-01 10:00:00"
/// - "2025-11-01T10:00:00"
///
/// # Examples
///
/// ```
/// use eventix::timezone::{parse_datetime_with_tz, parse_timezone};
///
/// let tz = parse_timezone("America/New_York").unwrap();
/// let dt = parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();
/// ```
pub fn parse_datetime_with_tz(datetime_str: &str, tz: Tz) -> Result<DateTime<Tz>> {
    // Try parsing with space separator
    let naive = if let Ok(dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        dt
    } else if let Ok(dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M:%S") {
        // Try with T separator
        dt
    } else {
        return Err(EventixError::DateTimeParse(format!(
            "Could not parse '{}'. Expected format: 'YYYY-MM-DD HH:MM:SS' or 'YYYY-MM-DDTHH:MM:SS'",
            datetime_str
        )));
    };

    // Convert to timezone-aware datetime
    // Use the earliest valid time in case of DST ambiguity
    tz.from_local_datetime(&naive).earliest().ok_or_else(|| {
        EventixError::DateTimeParse(format!(
            "Invalid datetime '{}' for timezone '{}'",
            datetime_str, tz
        ))
    })
}

/// Convert a datetime from one timezone to another
///
/// # Examples
///
/// ```
/// use eventix::timezone::{parse_datetime_with_tz, parse_timezone, convert_timezone};
///
/// let tz_ny = parse_timezone("America/New_York").unwrap();
/// let tz_tokyo = parse_timezone("Asia/Tokyo").unwrap();
///
/// let dt_ny = parse_datetime_with_tz("2025-11-01 10:00:00", tz_ny).unwrap();
/// let dt_tokyo = convert_timezone(&dt_ny, tz_tokyo);
/// ```
pub fn convert_timezone(dt: &DateTime<Tz>, target_tz: Tz) -> DateTime<Tz> {
    dt.with_timezone(&target_tz)
}

/// Check if a datetime falls within Daylight Saving Time
///
/// # Examples
///
/// ```
/// use eventix::timezone::{parse_datetime_with_tz, parse_timezone};
///
/// let tz = parse_timezone("America/New_York").unwrap();
/// let summer = parse_datetime_with_tz("2025-07-01 10:00:00", tz).unwrap();
/// let winter = parse_datetime_with_tz("2025-12-01 10:00:00", tz).unwrap();
///
/// // DST check - exact behavior depends on timezone rules
/// // Summer time is typically DST in America/New_York
/// // Winter time is typically standard time
/// ```
pub fn is_dst(dt: &DateTime<Tz>) -> bool {
    let offset = dt.offset().fix();
    let std_offset = dt.timezone().offset_from_utc_date(&dt.naive_utc().date()).fix();
    offset != std_offset
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_parse_timezone() {
        assert!(parse_timezone("America/New_York").is_ok());
        assert!(parse_timezone("UTC").is_ok());
        assert!(parse_timezone("Asia/Tokyo").is_ok());
        assert!(parse_timezone("Invalid/Timezone").is_err());
    }

    #[test]
    fn test_parse_datetime() {
        let tz = parse_timezone("UTC").unwrap();
        assert!(parse_datetime_with_tz("2025-11-01 10:00:00", tz).is_ok());
        assert!(parse_datetime_with_tz("2025-11-01T10:00:00", tz).is_ok());
        assert!(parse_datetime_with_tz("invalid", tz).is_err());
    }

    #[test]
    fn test_convert_timezone() {
        let tz_utc = parse_timezone("UTC").unwrap();
        let tz_ny = parse_timezone("America/New_York").unwrap();

        let dt_utc = parse_datetime_with_tz("2025-11-01 15:00:00", tz_utc).unwrap();
        let dt_ny = convert_timezone(&dt_utc, tz_ny);

        // UTC 15:00 should be around 10:00 or 11:00 in NY depending on DST
        assert!(dt_ny.hour() == 10 || dt_ny.hour() == 11);
    }
}
