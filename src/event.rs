//! Event types and builder API

use crate::error::{EventixError, Result};
use crate::recurrence::{Recurrence, RecurrenceFilter};
use crate::timezone::{local_day_window, parse_datetime_with_tz, parse_timezone};
use chrono::{DateTime, Duration};
use chrono_tz::Tz;

use serde::{Deserialize, Serialize};

/// Status of an event in the booking lifecycle
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
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
    ///
    /// Filtering is applied lazily: each candidate occurrence is checked against
    /// the time-window intersection, recurrence filter, and exception dates
    /// *before* counting toward `max_occurrences`. This ensures:
    /// - Filtered-out dates never consume result slots.
    /// - At most `max_occurrences` accepted results are collected, regardless
    ///   of how dense the underlying recurrence is.
    pub fn occurrences_between(
        &self,
        start: DateTime<Tz>,
        end: DateTime<Tz>,
        max_occurrences: usize,
    ) -> Result<Vec<DateTime<Tz>>> {
        if max_occurrences == 0 {
            return Ok(vec![]);
        }

        if let Some(ref recurrence) = self.recurrence {
            let duration = self.duration();

            let occurrences: Vec<DateTime<Tz>> = recurrence
                .occurrences(self.start_time)
                // Stop once occurrences are entirely past the query window.
                // Series is chronological, so once dt >= end nothing later
                // can intersect either.
                .take_while(|dt| *dt < end)
                // Intersection filter: occurrence's time span overlaps [start, end]
                .filter(|dt| *dt + duration > start)
                // Apply recurrence filter (skip weekends / skip dates) per element
                .filter(|dt| !self.is_occurrence_excluded(dt))
                // Stop as soon as we have enough accepted results — never
                // allocate beyond what the caller asked for.
                .take(max_occurrences)
                .collect();

            Ok(occurrences)
        } else {
            // Non-recurring event: intersection check
            let event_end = self.end_time;
            if self.start_time < end && event_end > start {
                Ok(vec![self.start_time])
            } else {
                Ok(vec![])
            }
        }
    }

    /// Check whether a single occurrence should be excluded by recurrence
    /// filter or exception dates.
    ///
    /// Returns `true` when the occurrence must be **skipped**.
    fn is_occurrence_excluded(&self, dt: &DateTime<Tz>) -> bool {
        // Recurrence filter (skip weekends, skip specific dates, …)
        if let Some(ref filter) = self.recurrence_filter {
            if filter.should_skip(dt) {
                return true;
            }
        }
        // Exception dates — match at full DateTime precision (RFC 5545).
        // For sub-daily recurrence this skips only the targeted occurrence,
        // not the entire day.
        self.exdates.contains(dt)
    }

    /// Check if this event occurs on a specific date
    pub fn occurs_on(&self, date: DateTime<Tz>) -> Result<bool> {
        let (start_dt, end_dt) = local_day_window(date.date_naive(), self.timezone)?;
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
        matches!(
            self.status,
            EventStatus::Confirmed | EventStatus::Tentative | EventStatus::Blocked
        )
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

    /// Block the event (similar to Confirmed, but explicit)
    pub fn block(&mut self) {
        self.status = EventStatus::Blocked;
    }

    /// Reschedule the event to a new time
    ///
    /// This updates the start and end times. If the event was Cancelled,
    /// it automatically resets the status to Confirmed.
    ///
    /// This also updates the event's timezone to match the new start time.
    pub fn reschedule(&mut self, new_start: DateTime<Tz>, new_end: DateTime<Tz>) -> Result<()> {
        if new_end <= new_start {
            return Err(EventixError::ValidationError(
                "Event end time must be after start time".to_string(),
            ));
        }
        self.start_time = new_start;
        self.end_time = new_end;
        self.timezone = new_start.timezone();

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
    /// First parsing error encountered during builder chain
    parse_error: Option<EventixError>,
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
            parse_error: None,
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
        match parse_timezone(timezone) {
            Ok(tz) => {
                self.timezone = Some(tz);
                match parse_datetime_with_tz(datetime, tz) {
                    Ok(dt) => self.start_time = Some(dt),
                    Err(e) => {
                        if self.parse_error.is_none() {
                            self.parse_error = Some(e);
                        }
                    }
                }
            }
            Err(e) => {
                if self.parse_error.is_none() {
                    self.parse_error = Some(e);
                }
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
            match parse_datetime_with_tz(datetime, tz) {
                Ok(dt) => self.end_time = Some(dt),
                Err(e) => {
                    if self.parse_error.is_none() {
                        self.parse_error = Some(e);
                    }
                }
            }
        } else if self.parse_error.is_none() {
            self.parse_error = Some(EventixError::ValidationError(
                "Cannot set end time: start() with timezone must be called first".to_string(),
            ));
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

    /// Set the duration (calculates end_time from start_time)
    pub fn duration(mut self, duration: Duration) -> Self {
        if let Some(start) = self.start_time {
            self.end_time = Some(start + duration);
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
        // Surface any parsing error captured during the builder chain
        if let Some(err) = self.parse_error {
            return Err(err);
        }

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
    #![allow(clippy::unwrap_used)]
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
    fn test_event_builder_duration() {
        let event = Event::builder()
            .title("Test Event")
            .description("A test event")
            .start("2025-11-01 10:00:00", "UTC")
            .duration(Duration::hours(1) + Duration::minutes(10))
            .attendee("alice@example.com")
            .build()
            .unwrap();

        assert_eq!(event.title, "Test Event");
        assert_eq!(event.description, Some("A test event".to_string()));
        assert_eq!(event.attendees.len(), 1);
        assert_eq!(event.end_time.to_rfc3339(), "2025-11-01T11:10:00+00:00");
        let duration_in_secs = (60.0 * 60.0) + (10.0 * 60.0); // 1 hour 10 minutes = 4200 seconds
        assert_eq!(event.duration().as_seconds_f32(), duration_in_secs);
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

        // Zero-duration events are rejected by the builder
        let result = Event::builder()
            .title("Zero")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_minutes(0)
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_occurrences_between_filter_before_cap() {
        use crate::timezone::parse_timezone;
        use crate::Recurrence;
        use chrono::Datelike;

        let tz = parse_timezone("UTC").unwrap();

        // Daily recurrence starting Friday 2025-01-03, with weekend skipping.
        // Fri, Sat, Sun, Mon, Tue, Wed, Thu...
        // With skip_weekends, valid days are Fri(3), Mon(6), Tue(7), Wed(8)...
        let event = Event::builder()
            .title("Daily standup")
            .start("2025-01-03 09:00:00", "UTC") // Friday
            .duration_hours(1)
            .recurrence(Recurrence::daily().count(30))
            .skip_weekends(true)
            .build()
            .unwrap();

        let start = crate::timezone::parse_datetime_with_tz("2025-01-03 00:00:00", tz).unwrap();
        let end = crate::timezone::parse_datetime_with_tz("2025-01-15 00:00:00", tz).unwrap();

        // max_occurrences=3: should return 3 weekday results, not be eaten
        // by Sat/Sun consuming cap slots before filter removes them.
        let occs = event.occurrences_between(start, end, 3).unwrap();
        assert_eq!(occs.len(), 3);

        // All results should be weekdays
        for occ in &occs {
            let wd = occ.weekday();
            assert!(
                wd != chrono::Weekday::Sat && wd != chrono::Weekday::Sun,
                "weekend snuck through: {:?}",
                wd
            );
        }
    }

    /// Stress test: secondly recurrence over a 24-hour window requesting only 10.
    /// The window contains 86 400 candidate seconds but we must collect at most 10.
    #[test]
    fn test_dense_secondly_does_not_over_allocate() {
        use crate::timezone::parse_timezone;
        use crate::Recurrence;

        let tz = parse_timezone("UTC").unwrap();

        let event = Event::builder()
            .title("Tick")
            .start("2025-06-01 00:00:00", "UTC")
            .duration(Duration::seconds(1))
            .recurrence(Recurrence::secondly().interval(1).count(100_000))
            .build()
            .unwrap();

        let start = crate::timezone::parse_datetime_with_tz("2025-06-01 00:00:00", tz).unwrap();
        let end = crate::timezone::parse_datetime_with_tz("2025-06-02 00:00:00", tz).unwrap();

        let occs = event.occurrences_between(start, end, 10).unwrap();
        assert_eq!(occs.len(), 10);
        // Verify spacing
        for i in 1..occs.len() {
            assert_eq!(occs[i] - occs[i - 1], Duration::seconds(1));
        }
    }

    /// Stress test: minutely recurrence over a 30-day window requesting only 5.
    #[test]
    fn test_dense_minutely_capped_early() {
        use crate::timezone::parse_timezone;
        use crate::Recurrence;

        let tz = parse_timezone("UTC").unwrap();

        let event = Event::builder()
            .title("Ping")
            .start("2025-06-01 00:00:00", "UTC")
            .duration(Duration::seconds(10))
            .recurrence(Recurrence::minutely().interval(1).count(100_000))
            .build()
            .unwrap();

        let start = crate::timezone::parse_datetime_with_tz("2025-06-01 00:00:00", tz).unwrap();
        let end = crate::timezone::parse_datetime_with_tz("2025-07-01 00:00:00", tz).unwrap();

        let occs = event.occurrences_between(start, end, 5).unwrap();
        assert_eq!(occs.len(), 5);
        for i in 1..occs.len() {
            assert_eq!(occs[i] - occs[i - 1], Duration::minutes(1));
        }
    }

    /// Stress test: hourly recurrence with weekend filter over a 1-year window.
    /// Ensures filter + cap work together lazily without blowup.
    #[test]
    fn test_dense_hourly_with_weekend_filter() {
        use crate::timezone::parse_timezone;
        use crate::Recurrence;
        use chrono::Datelike;

        let tz = parse_timezone("UTC").unwrap();

        let event = Event::builder()
            .title("Hourly Check")
            .start("2025-01-06 08:00:00", "UTC") // Monday
            .duration_minutes(5)
            .recurrence(Recurrence::hourly().interval(1).count(100_000))
            .skip_weekends(true)
            .build()
            .unwrap();

        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 00:00:00", tz).unwrap();
        let end = crate::timezone::parse_datetime_with_tz("2026-01-01 00:00:00", tz).unwrap();

        let occs = event.occurrences_between(start, end, 20).unwrap();
        assert_eq!(occs.len(), 20);
        for occ in &occs {
            let wd = occ.weekday();
            assert!(
                wd != chrono::Weekday::Sat && wd != chrono::Weekday::Sun,
                "weekend occurrence found: {}",
                occ
            );
        }
    }

    #[test]
    fn test_occurrences_between_zero_cap_returns_empty() {
        let event = Event::builder()
            .title("One-off")
            .start("2025-01-10 09:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap();

        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-10 00:00:00", tz).unwrap();
        let end = crate::timezone::parse_datetime_with_tz("2025-01-11 00:00:00", tz).unwrap();

        let occs = event.occurrences_between(start, end, 0).unwrap();
        assert!(occs.is_empty());
    }

    #[test]
    fn test_builder_surfaces_invalid_timezone() {
        let result = Event::builder()
            .title("Bad TZ")
            .start("2025-01-01 10:00:00", "Invalid/Zone")
            .duration_hours(1)
            .build();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // Should surface the real timezone error, not "Start time is required"
        assert!(!err.contains("required"), "Expected timezone parse error, got: {}", err);
    }

    #[test]
    fn test_builder_surfaces_invalid_datetime() {
        let result = Event::builder()
            .title("Bad DT")
            .start("not-a-date", "UTC")
            .duration_hours(1)
            .build();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(!err.contains("required"), "Expected datetime parse error, got: {}", err);
    }

    #[test]
    fn test_exdate_precision_subdaily() {
        use crate::timezone::parse_timezone;
        use crate::Recurrence;
        use chrono::Timelike;

        let tz = parse_timezone("UTC").unwrap();

        // Hourly event with exdate at exactly 12:00
        let exdate = crate::timezone::parse_datetime_with_tz("2025-06-01 12:00:00", tz).unwrap();
        let event = Event::builder()
            .title("Hourly")
            .start("2025-06-01 10:00:00", "UTC")
            .duration_minutes(5)
            .recurrence(Recurrence::hourly().count(5))
            .exception_date(exdate)
            .build()
            .unwrap();

        let start = crate::timezone::parse_datetime_with_tz("2025-06-01 00:00:00", tz).unwrap();
        let end = crate::timezone::parse_datetime_with_tz("2025-06-02 00:00:00", tz).unwrap();

        let occs = event.occurrences_between(start, end, 100).unwrap();
        // Should have 4 occurrences (10:00, 11:00, 13:00, 14:00) — 12:00 excluded
        assert_eq!(
            occs.len(),
            4,
            "exdate should skip only the 12:00 occurrence, got: {:?}",
            occs.iter().map(|d| d.format("%H:%M").to_string()).collect::<Vec<_>>()
        );
        // Verify 12:00 is not in the list
        for occ in &occs {
            assert_ne!(occ.hour(), 12, "12:00 should be excluded");
        }
    }

    #[test]
    fn test_occurs_on_true_for_matching_day() {
        let event = Event::builder()
            .title("Meeting")
            .start("2025-11-03 10:00:00", "America/New_York")
            .duration_hours(1)
            .build()
            .unwrap();

        let tz = crate::timezone::parse_timezone("America/New_York").unwrap();
        let date = crate::timezone::parse_datetime_with_tz("2025-11-03 00:00:00", tz).unwrap();

        assert!(event.occurs_on(date).unwrap());
    }

    #[test]
    fn test_occurs_on_false_for_non_matching_day() {
        let event = Event::builder()
            .title("Meeting")
            .start("2025-11-03 10:00:00", "America/New_York")
            .duration_hours(1)
            .build()
            .unwrap();

        let tz = crate::timezone::parse_timezone("America/New_York").unwrap();
        let date = crate::timezone::parse_datetime_with_tz("2025-11-04 00:00:00", tz).unwrap();

        assert!(!event.occurs_on(date).unwrap());
    }
}
