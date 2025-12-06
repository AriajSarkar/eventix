//! Event types and builder API

use crate::error::{EventixError, Result};
use crate::recurrence::{Recurrence, RecurrenceFilter};
use crate::timezone::{parse_datetime_with_tz, parse_timezone};
use chrono::{DateTime, Duration, TimeZone};
use chrono_tz::Tz;

use serde::{Deserialize, Serialize};

/// Status of an event in the booking lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EventStatus {
    /// The event is confirmed and occupies time (default)
    #[default]
    Confirmed,
    /// The event is tentative/provisional and occupies time
    Tentative,
    /// The event is cancelled and does NOT occupy time
    Cancelled,
    /// The time slot is blocked (similar to Confirmed)
    Blocked,
}

/// A calendar event with timezone-aware start and end times
#[derive(Debug, Clone)]
pub struct Event {
    /// Event title
    pub title: String,

    /// Optional description
    pub description: Option<String>,

    /// Start time with timezone
    pub start_time: DateTime<Tz>,

    /// End time with timezone
    pub end_time: DateTime<Tz>,

    /// Timezone for the event
    pub timezone: Tz,

    /// Optional list of attendees
    pub attendees: Vec<String>,

    /// Optional recurrence pattern
    pub recurrence: Option<Recurrence>,

    /// Optional recurrence filter (skip weekends, holidays, etc.)
    pub recurrence_filter: Option<RecurrenceFilter>,

    /// Specific dates to exclude from recurrence
    pub exdates: Vec<DateTime<Tz>>,

    /// Location of the event
    pub location: Option<String>,

    /// Unique identifier for the event
    pub uid: Option<String>,

    /// Status of the event (Confirmed, Cancelled, etc.)
    pub status: EventStatus,
}

impl Event {
    /// Create a new event builder
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Event;
    ///
    /// let event = Event::builder()
    ///     .title("Team Meeting")
    ///     .start("2025-11-01 10:00:00", "America/New_York")
    ///     .duration_hours(1)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> EventBuilder {
        EventBuilder::new()
    }

    /// Get all occurrences of this event within a date range
    ///
    /// For non-recurring events, returns a single occurrence.
    /// For recurring events, generates all occurrences based on the recurrence rule.
    pub fn occurrences_between(
        &self,
        start: DateTime<Tz>,
        end: DateTime<Tz>,
        max_occurrences: usize,
    ) -> Result<Vec<DateTime<Tz>>> {
        if let Some(ref recurrence) = self.recurrence {
            let mut occurrences =
                recurrence.generate_occurrences(self.start_time, max_occurrences)?;

            // Filter by date range
            occurrences.retain(|dt| *dt >= start && *dt <= end);

            // Apply recurrence filter if present
            if let Some(ref filter) = self.recurrence_filter {
                occurrences = filter.filter_occurrences(occurrences);
            }

            // Remove exception dates
            occurrences.retain(|dt| {
                !self.exdates.iter().any(|exdate| exdate.date_naive() == dt.date_naive())
            });

            Ok(occurrences)
        } else {
            // Non-recurring event
            if self.start_time >= start && self.start_time <= end {
                Ok(vec![self.start_time])
            } else {
                Ok(vec![])
            }
        }
    }

    /// Check if this event occurs on a specific date
    pub fn occurs_on(&self, date: DateTime<Tz>) -> Result<bool> {
        let start = date.date_naive().and_hms_opt(0, 0, 0).ok_or_else(|| {
            EventixError::ValidationError("Invalid start time for date check".to_string())
        })?;
        let end = date.date_naive().and_hms_opt(23, 59, 59).ok_or_else(|| {
            EventixError::ValidationError("Invalid end time for date check".to_string())
        })?;

        let start_dt = self.timezone.from_local_datetime(&start).earliest().ok_or_else(|| {
            EventixError::ValidationError("Ambiguous start time for date check".to_string())
        })?;
        let end_dt = self.timezone.from_local_datetime(&end).latest().ok_or_else(|| {
            EventixError::ValidationError("Ambiguous end time for date check".to_string())
        })?;

        let occurrences = self.occurrences_between(start_dt, end_dt, 1)?;
        Ok(!occurrences.is_empty())
    }

    /// Get the duration of this event
    pub fn duration(&self) -> Duration {
        self.end_time.signed_duration_since(self.start_time)
    }

    /// Check if the event is considered "active" (occupies time)
    ///
    /// Returns true for Confirmed, Tentative, and Blocked.
    /// Returns false for Cancelled.
    pub fn is_active(&self) -> bool {
        self.status != EventStatus::Cancelled
    }

    /// Confirm the event
    pub fn confirm(&mut self) {
        self.status = EventStatus::Confirmed;
    }

    /// Cancel the event
    pub fn cancel(&mut self) {
        self.status = EventStatus::Cancelled;
    }

    /// Set the event as tentative
    pub fn tentative(&mut self) {
        self.status = EventStatus::Tentative;
    }

    /// Reschedule the event to a new time
    ///
    /// This updates the start and end times. If the event was Cancelled,
    /// it automatically resets the status to Confirmed.
    pub fn reschedule(&mut self, new_start: DateTime<Tz>, new_end: DateTime<Tz>) -> Result<()> {
        if new_end <= new_start {
            return Err(EventixError::ValidationError(
                "Event end time must be after start time".to_string(),
            ));
        }
        self.start_time = new_start;
        self.end_time = new_end;

        // If rescheduling a cancelled event, assume it's valid again
        if self.status == EventStatus::Cancelled {
            self.status = EventStatus::Confirmed;
        }
        Ok(())
    }
}

/// Builder for creating events with a fluent API
pub struct EventBuilder {
    title: Option<String>,
    description: Option<String>,
    start_time: Option<DateTime<Tz>>,
    end_time: Option<DateTime<Tz>>,
    timezone: Option<Tz>,
    attendees: Vec<String>,
    recurrence: Option<Recurrence>,
    recurrence_filter: Option<RecurrenceFilter>,
    exdates: Vec<DateTime<Tz>>,
    location: Option<String>,
    uid: Option<String>,
    status: EventStatus,
}

impl EventBuilder {
    /// Create a new event builder
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            start_time: None,
            end_time: None,
            timezone: None,
            attendees: Vec::new(),
            recurrence: None,
            recurrence_filter: None,
            exdates: Vec::new(),
            location: None,
            uid: None,
            status: EventStatus::default(),
        }
    }

    /// Set the event title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the event description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the start time using a string and timezone
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Event;
    ///
    /// let event = Event::builder()
    ///     .title("Meeting")
    ///     .start("2025-11-01 10:00:00", "America/New_York")
    ///     .duration_hours(1)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn start(mut self, datetime: &str, timezone: &str) -> Self {
        if let Ok(tz) = parse_timezone(timezone) {
            self.timezone = Some(tz);
            if let Ok(dt) = parse_datetime_with_tz(datetime, tz) {
                self.start_time = Some(dt);
            }
        }
        self
    }

    /// Set the start time directly
    pub fn start_datetime(mut self, datetime: DateTime<Tz>) -> Self {
        self.timezone = Some(datetime.timezone());
        self.start_time = Some(datetime);
        self
    }

    /// Set the end time using a string
    pub fn end(mut self, datetime: &str) -> Self {
        if let Some(tz) = self.timezone {
            if let Ok(dt) = parse_datetime_with_tz(datetime, tz) {
                self.end_time = Some(dt);
            }
        }
        self
    }

    /// Set the end time directly
    pub fn end_datetime(mut self, datetime: DateTime<Tz>) -> Self {
        self.end_time = Some(datetime);
        self
    }

    /// Set the duration in hours (calculates end_time from start_time)
    pub fn duration_hours(mut self, hours: i64) -> Self {
        if let Some(start) = self.start_time {
            self.end_time = Some(start + Duration::hours(hours));
        }
        self
    }

    /// Set the duration in minutes (calculates end_time from start_time)
    pub fn duration_minutes(mut self, minutes: i64) -> Self {
        if let Some(start) = self.start_time {
            self.end_time = Some(start + Duration::minutes(minutes));
        }
        self
    }

    /// Add an attendee
    pub fn attendee(mut self, attendee: impl Into<String>) -> Self {
        self.attendees.push(attendee.into());
        self
    }

    /// Set multiple attendees
    pub fn attendees(mut self, attendees: Vec<String>) -> Self {
        self.attendees = attendees;
        self
    }

    /// Set the recurrence pattern
    pub fn recurrence(mut self, recurrence: Recurrence) -> Self {
        self.recurrence = Some(recurrence);
        self
    }

    /// Enable skipping weekends for recurring events
    pub fn skip_weekends(mut self, skip: bool) -> Self {
        let filter = self.recurrence_filter.unwrap_or_default();
        self.recurrence_filter = Some(filter.skip_weekends(skip));
        self
    }

    /// Add exception dates (dates to skip)
    pub fn exception_dates(mut self, dates: Vec<DateTime<Tz>>) -> Self {
        self.exdates = dates;
        self
    }

    /// Add a single exception date
    pub fn exception_date(mut self, date: DateTime<Tz>) -> Self {
        self.exdates.push(date);
        self
    }

    /// Set the location
    pub fn location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set a unique identifier
    pub fn uid(mut self, uid: impl Into<String>) -> Self {
        self.uid = Some(uid.into());
        self
    }

    /// Set the event status
    pub fn status(mut self, status: EventStatus) -> Self {
        self.status = status;
        self
    }

    /// Build the event
    pub fn build(self) -> Result<Event> {
        let title = self
            .title
            .ok_or_else(|| EventixError::ValidationError("Event title is required".to_string()))?;

        let start_time = self.start_time.ok_or_else(|| {
            EventixError::ValidationError("Event start time is required".to_string())
        })?;

        let end_time = self.end_time.ok_or_else(|| {
            EventixError::ValidationError("Event end time is required".to_string())
        })?;

        let timezone = self.timezone.ok_or_else(|| {
            EventixError::ValidationError("Event timezone is required".to_string())
        })?;

        if end_time <= start_time {
            return Err(EventixError::ValidationError(
                "Event end time must be after start time".to_string(),
            ));
        }

        Ok(Event {
            title,
            description: self.description,
            start_time,
            end_time,
            timezone,
            attendees: self.attendees,
            recurrence: self.recurrence,
            recurrence_filter: self.recurrence_filter,
            exdates: self.exdates,
            location: self.location,
            uid: self.uid,
            status: self.status,
        })
    }
}

impl Default for EventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder() {
        let event = Event::builder()
            .title("Test Event")
            .description("A test event")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(2)
            .attendee("alice@example.com")
            .build()
            .unwrap();

        assert_eq!(event.title, "Test Event");
        assert_eq!(event.description, Some("A test event".to_string()));
        assert_eq!(event.attendees.len(), 1);
        assert_eq!(event.duration(), Duration::hours(2));
    }

    #[test]
    fn test_event_validation() {
        // Missing title
        let result = Event::builder().start("2025-11-01 10:00:00", "UTC").duration_hours(1).build();
        assert!(result.is_err());

        // End before start
        let result = Event::builder()
            .title("Test")
            .start("2025-11-01 10:00:00", "UTC")
            .end("2025-11-01 09:00:00")
            .build();
        assert!(result.is_err());
    }
}
