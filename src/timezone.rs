//! Timezone handling utilities with DST awareness

use crate::error::{EventixError, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Offset, TimeZone};
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

/// Resolve a local datetime in a timezone, preserving wall-clock semantics
/// across DST gaps by applying the pre-gap UTC offset.
pub(crate) fn resolve_local(tz: Tz, naive: NaiveDateTime) -> Option<DateTime<Tz>> {
    if let Some(dt) = tz.from_local_datetime(&naive).earliest() {
        return Some(dt);
    }

    let day_before = naive - chrono::Duration::days(1);
    let pre_gap_dt = tz.from_local_datetime(&day_before).earliest()?;
    let pre_offset = pre_gap_dt.offset().fix();
    let utc_naive = naive - pre_offset;
    Some(chrono::Utc.from_utc_datetime(&utc_naive).with_timezone(&tz))
}

/// Compute the inclusive start and exclusive end of a local calendar day.
pub(crate) fn local_day_window(date: NaiveDate, tz: Tz) -> Result<(DateTime<Tz>, DateTime<Tz>)> {
    let start_naive = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| EventixError::ValidationError("Invalid start time".to_string()))?;
    let next_date = date
        .succ_opt()
        .ok_or_else(|| EventixError::ValidationError("Invalid end time".to_string()))?;
    let end_naive = next_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| EventixError::ValidationError("Invalid end time".to_string()))?;

    let start_dt = resolve_local(tz, start_naive)
        .ok_or_else(|| EventixError::ValidationError("Ambiguous start time".to_string()))?;
    let end_dt = resolve_local(tz, end_naive)
        .ok_or_else(|| EventixError::ValidationError("Ambiguous end time".to_string()))?;

    Ok((start_dt, end_dt))
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
    #![allow(clippy::unwrap_used)]
    use super::*;
    use chrono::{Duration, Timelike};

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

    #[test]
    fn test_local_day_window_dst_fall_back() {
        let tz = parse_timezone("America/New_York").unwrap();
        let date = chrono::NaiveDate::from_ymd_opt(2025, 11, 2).unwrap();

        let (start, end) = local_day_window(date, tz).unwrap();

        assert_eq!(start.date_naive(), date);
        assert_eq!(end.date_naive(), date.succ_opt().unwrap());
        assert_eq!(end - start, Duration::hours(25));
    }
}
