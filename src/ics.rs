//! ICS (iCalendar) import and export functionality

use crate::calendar::Calendar;
use crate::error::{EventixError, Result};
use crate::event::Event;
use chrono::{DateTime, TimeZone};
use chrono_tz::Tz;
use icalendar::{Calendar as ICalendar, Component, Event as IEvent, EventLike, Property};
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
        ical_event.add_property("ATTENDEE", format!("mailto:{}", attendee));
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

    // Add exception dates with timezone
    for exdate in &event.exdates {
        let exdate_str = exdate.format("%Y%m%dT%H%M%S").to_string();
        if tz_name == "UTC" {
            ical_event.add_property("EXDATE", format!("{}Z", exdate_str));
        } else {
            let mut exdate_prop = Property::new("EXDATE", &exdate_str);
            exdate_prop.add_parameter("TZID", tz_name);
            ical_event.append_property(exdate_prop);
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

    // TODO: Parse RRULE and EXDATE if present
    // This would require more sophisticated parsing of the iCalendar properties

    builder.build()
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
                // Parse the timezone from TZID parameter (use Debug format)
                // Debug format is: Parameter { key: "TZID", val: "America/New_York" }
                let tz_str_raw = format!("{:?}", tzid_param);
                // Extract the value after 'val: "'
                let tz_str = if let Some(start_idx) = tz_str_raw.find("val: \"") {
                    let start = start_idx + 6; // Length of 'val: "'
                    let remaining = &tz_str_raw[start..];
                    if let Some(end_idx) = remaining.find('"') {
                        remaining[..end_idx].to_string()
                    } else {
                        return Err(EventixError::InvalidTimezone(format!(
                            "Cannot parse TZID parameter: {}",
                            tz_str_raw
                        )));
                    }
                } else {
                    return Err(EventixError::InvalidTimezone(format!(
                        "Cannot parse TZID parameter: {}",
                        tz_str_raw
                    )));
                };
                crate::timezone::parse_timezone(&tz_str)?
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
fn parse_ical_datetime_value(dt_str: &str, tz: Tz) -> Result<DateTime<Tz>> {
    // Format: YYYYMMDDTHHMMSS
    if dt_str.len() < 15 {
        return Err(EventixError::DateTimeParse(format!("Invalid datetime format: {}", dt_str)));
    }

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
}
