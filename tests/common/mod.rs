//! Shared helpers for integration tests (`parse` only — each `tests/*.rs` crate is separate).

use chrono::DateTime;
use chrono_tz::Tz;
use eventix::timezone;

pub fn parse(datetime: &str, tz_name: &str) -> DateTime<Tz> {
    let tz = timezone::parse_timezone(tz_name).unwrap();
    timezone::parse_datetime_with_tz(datetime, tz).unwrap()
}
