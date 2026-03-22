//! ICS (iCalendar) import and export functionality

use crate::calendar::Calendar;
use crate::error::{EventixError, Result};
use crate::event::Event;
use crate::recurrence::Recurrence;
use chrono::{DateTime, TimeZone};
use chrono_tz::Tz;
use icalendar::{Calendar as ICalendar, Component, Event as IEvent, EventLike, Property};
use rrule::Frequency;
use std::fs;
use std::path::Path;

impl Calendar {
    /// Export this calendar to an ICS file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eventix::{Calendar, Event};
    ///
    /// let mut cal = Calendar::new("My Calendar");
    /// let event = Event::builder()
    ///     .title("Meeting")
    ///     .start("2025-11-01 10:00:00", "UTC")
    ///     .duration_hours(1)
    ///     .build()
    ///     .unwrap();
    ///
    /// cal.add_event(event);
    /// cal.export_to_ics("calendar.ics").unwrap();
    /// ```
    pub fn export_to_ics<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let ics_content = self.to_ics_string()?;
        fs::write(path, ics_content)
            .map_err(|e| EventixError::IcsError(format!("Failed to write ICS file: {}", e)))
    }

    /// Convert this calendar to an ICS string
    pub fn to_ics_string(&self) -> Result<String> {
        let mut ical = ICalendar::new();

        // Set calendar properties
        ical.name(&self.name);
        if let Some(ref desc) = self.description {
            ical.description(desc);
        }

        // Add each event
        for event in &self.events {
            let ical_event = event_to_ical(event)?;
            ical.push(ical_event);
        }

        Ok(ical.to_string())
    }

    /// Import a calendar from an ICS file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eventix::Calendar;
    ///
    /// let cal = Calendar::import_from_ics("calendar.ics").unwrap();
    /// println!("Imported {} events", cal.event_count());
    /// ```
    pub fn import_from_ics<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| EventixError::IcsError(format!("Failed to read ICS file: {}", e)))?;

        Self::from_ics_string(&content)
    }

    /// Parse a calendar from an ICS string
    pub fn from_ics_string(ics: &str) -> Result<Self> {
        // Parse the ICS content
        let ical = ics
            .parse::<ICalendar>()
            .map_err(|e| EventixError::IcsError(format!("Failed to parse ICS: {}", e)))?;

        let mut calendar = Calendar::new("Imported Calendar");

        // Extract calendar name if available
        if let Some(name) = ical.get_name() {
            calendar.name = name.to_string();
        }

        if let Some(desc) = ical.get_description() {
            calendar.description = Some(desc.to_string());
        }

        // Parse events
        for component in ical.components {
            if let icalendar::CalendarComponent::Event(ical_event) = component {
                match ical_to_event(&ical_event) {
                    Ok(event) => calendar.add_event(event),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse event: {}", e);
                        // Continue parsing other events
                    }
                }
            }
        }

        Ok(calendar)
    }
}

/// Convert a eventix Event to an iCalendar Event
fn event_to_ical(event: &Event) -> Result<IEvent> {
    let mut ical_event = IEvent::new();

    // Set UID
    if let Some(ref uid) = event.uid {
        ical_event.uid(uid);
    } else {
        // Generate a UID if not present
        let uid = format!("{}@eventix", uuid::Uuid::new_v4());
        ical_event.uid(&uid);
    }

    // Set summary (title)
    ical_event.summary(&event.title);

    // Set description
    if let Some(ref desc) = event.description {
        ical_event.description(desc);
    }

    // Set location
    if let Some(ref loc) = event.location {
        ical_event.location(loc);
    }

    // Set start and end times with timezone awareness
    // If the timezone is UTC, use the standard format without TZID
    // Otherwise, include TZID parameter for local times
    let tz_name = event.timezone.name();

    if tz_name == "UTC" {
        // For UTC, use the standard UTC format (with Z suffix)
        let start_utc = event.start_time.with_timezone(&chrono::Utc);
        let end_utc = event.end_time.with_timezone(&chrono::Utc);
        ical_event.starts(start_utc);
        ical_event.ends(end_utc);
    } else {
        // For other timezones, use TZID parameter with local time
        // Format: DTSTART;TZID=America/New_York:20251027T100000
        let start_local = event.start_time.format("%Y%m%dT%H%M%S").to_string();
        let end_local = event.end_time.format("%Y%m%dT%H%M%S").to_string();

        // Create properties with TZID parameter
        let mut dtstart = Property::new("DTSTART", &start_local);
        dtstart.add_parameter("TZID", tz_name);
        ical_event.append_property(dtstart);

        let mut dtend = Property::new("DTEND", &end_local);
        dtend.add_parameter("TZID", tz_name);
        ical_event.append_property(dtend);
    }

    // Add attendees
    for attendee in &event.attendees {
        ical_event.add_multi_property("ATTENDEE", &format!("mailto:{}", attendee));
    }

    // Add recurrence rule if present
    if let Some(ref recurrence) = event.recurrence {
        let rrule_str = recurrence.to_rrule_string(event.start_time)?;
        // Extract just the RRULE part
        if let Some(rrule_part) = rrule_str.lines().find(|l| l.starts_with("RRULE:")) {
            let rrule_value = rrule_part.strip_prefix("RRULE:").unwrap_or(rrule_part);
            ical_event.add_property("RRULE", rrule_value);
        }
    }

    // Add exception dates with timezone (EXDATE is a multi-property in RFC 5545)
    // Normalize each exdate to the event timezone before formatting so the
    // stamped local time matches the TZID label.
    let event_tz = event.start_time.timezone();
    for exdate in &event.exdates {
        if tz_name == "UTC" {
            let exdate_utc = exdate.with_timezone(&chrono::Utc);
            let exdate_str = exdate_utc.format("%Y%m%dT%H%M%S").to_string();
            ical_event.add_multi_property("EXDATE", &format!("{}Z", exdate_str));
        } else {
            let exdate_local = exdate.with_timezone(&event_tz);
            let exdate_str = exdate_local.format("%Y%m%dT%H%M%S").to_string();
            let mut exdate_prop = Property::new("EXDATE", &exdate_str);
            exdate_prop.add_parameter("TZID", tz_name);
            ical_event.append_multi_property(exdate_prop);
        }
    }

    Ok(ical_event)
}

/// Convert an iCalendar Event to a eventix Event
fn ical_to_event(ical_event: &IEvent) -> Result<Event> {
    // Extract required fields
    let summary = ical_event
        .get_summary()
        .ok_or_else(|| EventixError::IcsError("Event missing SUMMARY".to_string()))?;

    // Try to extract DTSTART and DTEND properties with timezone info
    let (start_time, _timezone) = extract_datetime_with_tz(ical_event, "DTSTART")?;
    let (end_time, _) = extract_datetime_with_tz(ical_event, "DTEND")?;

    // Build the event
    let mut builder = Event::builder()
        .title(summary)
        .start_datetime(start_time)
        .end_datetime(end_time);

    // Add optional fields
    if let Some(desc) = ical_event.get_description() {
        builder = builder.description(desc);
    }

    if let Some(loc) = ical_event.get_location() {
        builder = builder.location(loc);
    }

    if let Some(uid) = ical_event.get_uid() {
        builder = builder.uid(uid);
    }

    // Parse RRULE if present — reject unsupported rules instead of silently
    // degrading, since dropping BYMONTH etc. would produce a broader schedule.
    let props = ical_event.properties();
    for (key, prop) in props {
        if key == "RRULE" {
            let rrule_value = prop.value();
            let recurrence = parse_rrule_value(rrule_value, start_time)?;
            builder = builder.recurrence(recurrence);
        }
    }

    // Parse EXDATE properties (stored in multi_properties per RFC 5545)
    let event_tz = start_time.timezone();
    if let Some(exdate_props) = ical_event.multi_properties().get("EXDATE") {
        for prop in exdate_props {
            let value = prop.value();
            // Determine timezone for this EXDATE
            let exdate_tz = if let Some(tzid_param) = prop.params().get("TZID") {
                crate::timezone::parse_timezone(tzid_param.value()).unwrap_or(event_tz)
            } else if value.ends_with('Z') {
                crate::timezone::parse_timezone("UTC").unwrap_or(event_tz)
            } else {
                event_tz
            };

            let dt_str = value.trim_end_matches('Z');
            let exdate_dt = parse_ical_datetime_value(dt_str, exdate_tz).map_err(|e| {
                EventixError::IcsError(format!("Failed to parse EXDATE '{}': {}", value, e))
            })?;
            builder = builder.exception_date(exdate_dt);
        }
    }

    builder.build()
}

/// Parse an RRULE value string into a Recurrence.
///
/// Supports: FREQ, INTERVAL, COUNT, UNTIL, BYDAY
fn parse_rrule_value(rrule_str: &str, dtstart: DateTime<Tz>) -> Result<Recurrence> {
    let mut frequency = None;
    let mut interval = 1u16;
    let mut count = None;
    let mut until = None;
    let mut by_weekday = None;

    for part in rrule_str.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key {
            "FREQ" => {
                frequency = Some(match value {
                    "SECONDLY" => Frequency::Secondly,
                    "MINUTELY" => Frequency::Minutely,
                    "HOURLY" => Frequency::Hourly,
                    "DAILY" => Frequency::Daily,
                    "WEEKLY" => Frequency::Weekly,
                    "MONTHLY" => Frequency::Monthly,
                    "YEARLY" => Frequency::Yearly,
                    _ => {
                        return Err(EventixError::IcsError(format!(
                            "Unknown RRULE frequency: {}",
                            value
                        )))
                    }
                });
            }
            "INTERVAL" => {
                interval = value.parse().map_err(|_| {
                    EventixError::IcsError(format!("Invalid RRULE INTERVAL: {}", value))
                })?;
            }
            "COUNT" => {
                count = Some(value.parse().map_err(|_| {
                    EventixError::IcsError(format!("Invalid RRULE COUNT: {}", value))
                })?);
            }
            "UNTIL" => {
                // UNTIL can be a date or datetime, possibly with Z suffix
                let dt_str = value.trim_end_matches('Z');
                let tz = if value.ends_with('Z') {
                    crate::timezone::parse_timezone("UTC")?
                } else {
                    dtstart.timezone()
                };
                until = Some(parse_ical_datetime_value(dt_str, tz)?);
            }
            "BYDAY" => {
                let mut weekdays = Vec::new();
                for day_str in value.split(',') {
                    let day_str = day_str.trim();
                    let wd = match day_str {
                        "MO" => chrono::Weekday::Mon,
                        "TU" => chrono::Weekday::Tue,
                        "WE" => chrono::Weekday::Wed,
                        "TH" => chrono::Weekday::Thu,
                        "FR" => chrono::Weekday::Fri,
                        "SA" => chrono::Weekday::Sat,
                        "SU" => chrono::Weekday::Sun,
                        other => {
                            return Err(EventixError::IcsError(format!(
                                "Unsupported BYDAY value '{}' (ordinal prefixes like 1MO or -1FR are not supported)",
                                other
                            )))
                        }
                    };
                    weekdays.push(wd);
                }
                if !weekdays.is_empty() {
                    by_weekday = Some(weekdays);
                }
            }
            other => {
                return Err(EventixError::IcsError(format!(
                    "Unsupported RRULE component: {}",
                    other
                )))
            }
        }
    }

    let freq = frequency
        .ok_or_else(|| EventixError::IcsError("RRULE missing FREQ component".to_string()))?;

    // RFC 5545 §3.3.10: COUNT and UNTIL MUST NOT both appear in the same rule
    if count.is_some() && until.is_some() {
        return Err(EventixError::IcsError(
            "RRULE must not contain both COUNT and UNTIL".to_string(),
        ));
    }

    let mut recurrence = Recurrence::new(freq).interval(interval);
    if let Some(c) = count {
        recurrence = recurrence.count(c);
    }
    if let Some(u) = until {
        recurrence = recurrence.until(u);
    }
    if let Some(wd) = by_weekday {
        recurrence = recurrence.weekdays(wd);
    }
    Ok(recurrence)
}

/// Extract datetime with timezone from an iCalendar property
fn extract_datetime_with_tz(ical_event: &IEvent, prop_name: &str) -> Result<(DateTime<Tz>, Tz)> {
    // Try to find the property directly from the inner properties
    let props = ical_event.properties();

    for (key, prop) in props {
        if key == prop_name {
            let value = prop.value();

            // Check if there's a TZID parameter
            let timezone = if let Some(tzid_param) = prop.params().get("TZID") {
                crate::timezone::parse_timezone(tzid_param.value())?
            } else if value.ends_with('Z') {
                // UTC timezone
                crate::timezone::parse_timezone("UTC")?
            } else {
                // Default to UTC if no timezone specified
                crate::timezone::parse_timezone("UTC")?
            };

            // Parse the datetime value (format: 20251027T143000 or 20251027T143000Z)
            let dt_str = value.trim_end_matches('Z');
            let datetime = parse_ical_datetime_value(dt_str, timezone)?;

            return Ok((datetime, timezone));
        }
    }

    Err(EventixError::IcsError(format!("Property {} not found", prop_name)))
}

/// Parse an iCalendar datetime value string
///
/// Accepts both DATE-TIME format (`YYYYMMDDTHHMMSS`, 15+ chars) and
/// DATE-only format (`YYYYMMDD`, exactly 8 chars). DATE-only values
/// default to midnight (00:00:00).
fn parse_ical_datetime_value(dt_str: &str, tz: Tz) -> Result<DateTime<Tz>> {
    // DATE-only format: YYYYMMDD (8 chars, no 'T' separator)
    let (year, month, day, hour, minute, second) = if dt_str.len() == 8 && !dt_str.contains('T') {
        let year: i32 = dt_str[0..4]
            .parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid year in: {}", dt_str)))?;
        let month: u32 = dt_str[4..6]
            .parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid month in: {}", dt_str)))?;
        let day: u32 = dt_str[6..8]
            .parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid day in: {}", dt_str)))?;
        (year, month, day, 0, 0, 0)
    } else if dt_str.len() >= 15 {
        // DATE-TIME format: YYYYMMDDTHHMMSS
        let year: i32 = dt_str[0..4]
            .parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid year in: {}", dt_str)))?;
        let month: u32 = dt_str[4..6]
            .parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid month in: {}", dt_str)))?;
        let day: u32 = dt_str[6..8]
            .parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid day in: {}", dt_str)))?;
        let hour: u32 = dt_str[9..11]
            .parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid hour in: {}", dt_str)))?;
        let minute: u32 = dt_str[11..13]
            .parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid minute in: {}", dt_str)))?;
        let second: u32 = dt_str[13..15]
            .parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid second in: {}", dt_str)))?;
        (year, month, day, hour, minute, second)
    } else {
        return Err(EventixError::DateTimeParse(format!("Invalid datetime format: {}", dt_str)));
    };

    let naive = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|d| d.and_hms_opt(hour, minute, second))
        .ok_or_else(|| EventixError::DateTimeParse(format!("Invalid datetime: {}", dt_str)))?;

    let dt = tz.from_local_datetime(&naive).earliest().ok_or_else(|| {
        EventixError::DateTimeParse(format!("Cannot create datetime: {}", dt_str))
    })?;

    Ok(dt)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_ics_export() {
        let mut cal = Calendar::new("Test Calendar");
        let event = Event::builder()
            .title("Test Event")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap();

        cal.add_event(event);

        let ics = cal.to_ics_string().unwrap();
        assert!(ics.contains("Test Calendar"));
        assert!(ics.contains("Test Event"));
    }

    #[test]
    fn test_ics_rrule_roundtrip() {
        let mut cal = Calendar::new("RRULE Test");
        let event = Event::builder()
            .title("Daily Standup")
            .start("2025-01-06 09:00:00", "UTC")
            .duration_minutes(15)
            .recurrence(Recurrence::daily().interval(2).count(10))
            .build()
            .unwrap();

        cal.add_event(event);

        let ics = cal.to_ics_string().unwrap();
        assert!(ics.contains("RRULE:"), "exported ICS should contain RRULE");

        // Re-import
        let imported = Calendar::from_ics_string(&ics).unwrap();
        assert_eq!(imported.event_count(), 1);

        let imported_event = &imported.events[0];
        let rec = imported_event.recurrence.as_ref().unwrap();
        assert_eq!(rec.frequency(), rrule::Frequency::Daily);
        assert_eq!(rec.get_interval(), 2);
        assert_eq!(rec.get_count(), Some(10));
    }

    #[test]
    fn test_ics_exdate_roundtrip() {
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let exdate = crate::timezone::parse_datetime_with_tz("2025-01-08 09:00:00", tz).unwrap();

        let mut cal = Calendar::new("EXDATE Test");
        let event = Event::builder()
            .title("Recurring")
            .start("2025-01-06 09:00:00", "UTC")
            .duration_minutes(15)
            .recurrence(Recurrence::daily().count(10))
            .exception_date(exdate)
            .build()
            .unwrap();

        cal.add_event(event);

        let ics = cal.to_ics_string().unwrap();
        assert!(ics.contains("EXDATE"), "exported ICS should contain EXDATE");

        let imported = Calendar::from_ics_string(&ics).unwrap();
        assert_eq!(imported.events[0].exdates.len(), 1);
    }

    #[test]
    fn test_parse_rrule_value() {
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 10:00:00", tz).unwrap();

        // Basic FREQ + COUNT
        let rec = parse_rrule_value("FREQ=WEEKLY;COUNT=4", start).unwrap();
        assert_eq!(rec.frequency(), rrule::Frequency::Weekly);
        assert_eq!(rec.get_count(), Some(4));
        assert_eq!(rec.get_interval(), 1);

        // FREQ + INTERVAL + BYDAY
        let rec = parse_rrule_value("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE,FR", start).unwrap();
        assert_eq!(rec.get_interval(), 2);
        let wd = rec.get_weekdays().unwrap();
        assert_eq!(wd.len(), 3);
        assert!(wd.contains(&chrono::Weekday::Mon));
        assert!(wd.contains(&chrono::Weekday::Wed));
        assert!(wd.contains(&chrono::Weekday::Fri));

        // FREQ + UNTIL
        let rec = parse_rrule_value("FREQ=DAILY;UNTIL=20250201T000000Z", start).unwrap();
        assert!(rec.get_until().is_some());

        // FREQ + UNTIL with DATE-only format (no time component)
        let rec = parse_rrule_value("FREQ=DAILY;UNTIL=20250201", start).unwrap();
        assert!(rec.get_until().is_some());
    }

    #[test]
    fn test_parse_rrule_rejects_unsupported_parts() {
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 10:00:00", tz).unwrap();

        // BYMONTH is unsupported — must return Err, not silently drop
        let result = parse_rrule_value("FREQ=DAILY;COUNT=90;BYMONTH=3", start);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("BYMONTH"));

        // BYSETPOS is unsupported
        let result = parse_rrule_value("FREQ=MONTHLY;BYDAY=MO;BYSETPOS=1", start);
        assert!(result.is_err());

        // Ordinal-prefixed BYDAY like 1MO or -1FR must be rejected
        let result = parse_rrule_value("FREQ=MONTHLY;BYDAY=1MO", start);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("1MO"));

        let result = parse_rrule_value("FREQ=MONTHLY;BYDAY=-1FR", start);
        assert!(result.is_err());

        // COUNT + UNTIL together must be rejected per RFC 5545
        let result = parse_rrule_value("FREQ=DAILY;COUNT=10;UNTIL=20250201T000000Z", start);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("COUNT") && err_msg.contains("UNTIL"));
    }

    #[test]
    fn test_parse_rrule_value_covers_yearly_and_numeric_errors() {
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 10:00:00", tz).unwrap();

        let yearly = parse_rrule_value("FREQ=YEARLY;COUNT=1", start).unwrap();
        assert_eq!(yearly.frequency(), rrule::Frequency::Yearly);

        let err = parse_rrule_value("FREQ=FORTNIGHTLY;COUNT=1", start).unwrap_err();
        assert!(err.to_string().contains("FORTNIGHTLY"));

        let err = parse_rrule_value("FREQ=DAILY;INTERVAL=abc", start).unwrap_err();
        assert!(err.to_string().contains("INTERVAL"));

        let err = parse_rrule_value("FREQ=DAILY;COUNT=abc", start).unwrap_err();
        assert!(err.to_string().contains("COUNT"));
    }

    #[test]
    fn test_parse_ical_datetime_value_rejects_dst_gap() {
        let tz = crate::timezone::parse_timezone("America/New_York").unwrap();
        let err = parse_ical_datetime_value("20250309T023000", tz).unwrap_err();
        assert!(
            matches!(err, EventixError::DateTimeParse(message) if message.contains("Cannot create datetime"))
        );
    }

    #[test]
    fn test_parse_ical_datetime_value_date_only() {
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let dt = parse_ical_datetime_value("20251101", tz).unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.month(), 11);
    }

    #[test]
    fn test_parse_ical_datetime_value_invalid_short_string() {
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let err = parse_ical_datetime_value("2025110", tz).unwrap_err();
        assert!(matches!(err, EventixError::DateTimeParse(_)));
    }

    #[test]
    fn test_from_ics_string_rejects_unparseable_icalendar() {
        let err = Calendar::from_ics_string("<<<not valid>>>").unwrap_err();
        assert!(
            matches!(err, EventixError::IcsError(message) if message.contains("Failed to parse ICS"))
        );
    }

    #[test]
    fn test_export_to_ics_and_import_from_ics_roundtrip_path() {
        let mut cal = Calendar::new("Path Roundtrip");
        cal.add_event(
            Event::builder()
                .title("Disk Event")
                .start("2025-11-01 12:00:00", "UTC")
                .duration_hours(1)
                .build()
                .unwrap(),
        );

        let path = std::env::temp_dir().join(format!("eventix_path_{}.ics", uuid::Uuid::new_v4()));
        cal.export_to_ics(&path).unwrap();

        let imported = Calendar::import_from_ics(&path).unwrap();
        assert_eq!(imported.name, "Path Roundtrip");
        assert_eq!(imported.event_count(), 1);
        assert_eq!(imported.events[0].title, "Disk Event");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_import_from_ics_missing_file_errors() {
        let path =
            std::env::temp_dir().join(format!("eventix_missing_{}.ics", uuid::Uuid::new_v4()));
        let err = Calendar::import_from_ics(&path).unwrap_err();
        assert!(
            matches!(err, EventixError::IcsError(message) if message.contains("Failed to read ICS file"))
        );
    }

    #[test]
    fn test_from_ics_string_skips_bad_event_continues_others() {
        let ics = "\
BEGIN:VCALENDAR
BEGIN:VEVENT
SUMMARY:Good
DTSTART:20251101T100000Z
DTEND:20251101T110000Z
END:VEVENT
BEGIN:VEVENT
SUMMARY:Bad
END:VEVENT
END:VCALENDAR";

        let cal = Calendar::from_ics_string(ics).unwrap();
        assert_eq!(cal.event_count(), 1);
        assert_eq!(cal.events[0].title, "Good");
    }

    #[test]
    fn test_parse_rrule_secondly_from_ics_import() {
        let ics = "\
BEGIN:VCALENDAR
BEGIN:VEVENT
SUMMARY:Secondly
DTSTART:20251101T100000Z
DTEND:20251101T100001Z
RRULE:FREQ=SECONDLY;COUNT=3
END:VEVENT
END:VCALENDAR";

        let cal = Calendar::from_ics_string(ics).unwrap();
        let ev = &cal.events[0];
        assert_eq!(ev.recurrence.as_ref().unwrap().frequency(), rrule::Frequency::Secondly);
    }
}
