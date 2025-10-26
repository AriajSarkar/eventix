//! ICS (iCalendar) import and export functionality

use std::fs;
use std::path::Path;
use chrono::{DateTime, TimeZone};
use chrono_tz::Tz;
use icalendar::{Calendar as ICalendar, Component, Event as IEvent, EventLike};
use crate::calendar::Calendar;
use crate::event::Event;
use crate::error::{EventixError, Result};

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
        let ical = ics.parse::<ICalendar>()
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
    
    // Set start and end times (convert to UTC for iCalendar)
    let start_utc = event.start_time.with_timezone(&chrono::Utc);
    let end_utc = event.end_time.with_timezone(&chrono::Utc);
    ical_event.starts(start_utc);
    ical_event.ends(end_utc);
    
    // Add attendees
    for attendee in &event.attendees {
        ical_event.add_property("ATTENDEE", &format!("mailto:{}", attendee));
    }
    
    // Add recurrence rule if present
    if let Some(ref recurrence) = event.recurrence {
        let rrule_str = recurrence.to_rrule_string(event.start_time)?;
        // Extract just the RRULE part
        if let Some(rrule_part) = rrule_str.lines().find(|l| l.starts_with("RRULE:")) {
            let rrule_value = rrule_part.strip_prefix("RRULE:").unwrap();
            ical_event.add_property("RRULE", rrule_value);
        }
    }
    
    // Add exception dates
    for exdate in &event.exdates {
        ical_event.add_property("EXDATE", &exdate.format("%Y%m%dT%H%M%S").to_string());
    }
    
    Ok(ical_event)
}

/// Convert an iCalendar Event to a eventix Event
fn ical_to_event(ical_event: &IEvent) -> Result<Event> {
    // Extract required fields
    let summary = ical_event.get_summary()
        .ok_or_else(|| EventixError::IcsError("Event missing SUMMARY".to_string()))?;
    
    let start = ical_event.get_start()
        .ok_or_else(|| EventixError::IcsError("Event missing DTSTART".to_string()))?;
    
    let end = ical_event.get_end()
        .ok_or_else(|| EventixError::IcsError("Event missing DTEND".to_string()))?;
    
    // Parse datetime - try to extract timezone
    let start_str = format!("{:?}", start);
    let end_str = format!("{:?}", end);
    let (start_time, _timezone) = parse_ical_datetime(&start_str)?;
    let end_time = parse_ical_datetime(&end_str)?.0;
    
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

/// Parse an iCalendar datetime string
fn parse_ical_datetime(dt_str: &str) -> Result<(DateTime<Tz>, Tz)> {
    // Try to parse with timezone information
    // Format could be: 20251101T100000Z (UTC) or 20251101T100000 with TZID parameter
    
    // For now, assume UTC if no timezone specified
    let tz: Tz = "UTC".parse().unwrap();
    
    // Parse the datetime string (format: YYYYMMDDTHHMMSS)
    let dt_str_clean = dt_str.trim_end_matches('Z');
    
    if dt_str_clean.len() >= 15 {
        let year: i32 = dt_str_clean[0..4].parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid year in: {}", dt_str)))?;
        let month: u32 = dt_str_clean[4..6].parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid month in: {}", dt_str)))?;
        let day: u32 = dt_str_clean[6..8].parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid day in: {}", dt_str)))?;
        let hour: u32 = dt_str_clean[9..11].parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid hour in: {}", dt_str)))?;
        let minute: u32 = dt_str_clean[11..13].parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid minute in: {}", dt_str)))?;
        let second: u32 = dt_str_clean[13..15].parse()
            .map_err(|_| EventixError::DateTimeParse(format!("Invalid second in: {}", dt_str)))?;
        
        let naive = chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(hour, minute, second))
            .ok_or_else(|| EventixError::DateTimeParse(format!("Invalid datetime: {}", dt_str)))?;
        
        let dt = tz.from_local_datetime(&naive)
            .earliest()
            .ok_or_else(|| EventixError::DateTimeParse(format!("Cannot create datetime: {}", dt_str)))?;
        
        Ok((dt, tz))
    } else {
        Err(EventixError::DateTimeParse(format!("Invalid datetime format: {}", dt_str)))
    }
}



#[cfg(test)]
mod tests {
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
