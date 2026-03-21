//! Calendar type for managing collections of events

use crate::error::{EventixError, Result};
use crate::event::Event;
use crate::recurrence::Recurrence;
use crate::timezone::local_day_window;
use crate::views::{DayIterator, WeekIterator};
use chrono::DateTime;
use chrono_tz::Tz;
use rrule::Frequency;

/// A calendar containing multiple events
#[derive(Debug, Clone)]
pub struct Calendar {
    /// Calendar name
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// List of events in this calendar
    pub events: Vec<Event>,

    /// Calendar timezone (default for new events)
    pub timezone: Option<Tz>,
}

impl Calendar {
    /// Create a new calendar with the given name
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Calendar;
    ///
    /// let cal = Calendar::new("My Calendar");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            events: Vec::new(),
            timezone: None,
        }
    }

    /// Set the calendar description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the default timezone for this calendar
    pub fn timezone(mut self, tz: Tz) -> Self {
        self.timezone = Some(tz);
        self
    }

    /// Add an event to the calendar
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::{Calendar, Event};
    ///
    /// let mut cal = Calendar::new("Work");
    /// let event = Event::builder()
    ///     .title("Meeting")
    ///     .start("2025-11-01 10:00:00", "UTC")
    ///     .duration_hours(1)
    ///     .build()
    ///     .unwrap();
    ///
    /// cal.add_event(event);
    /// assert_eq!(cal.events.len(), 1);
    /// ```
    pub fn add_event(&mut self, event: Event) {
        self.events.push(event);
    }

    /// Add multiple events to the calendar
    pub fn add_events(&mut self, events: Vec<Event>) {
        self.events.extend(events);
    }

    /// Remove an event by index
    pub fn remove_event(&mut self, index: usize) -> Option<Event> {
        if index < self.events.len() {
            Some(self.events.remove(index))
        } else {
            None
        }
    }

    /// Update an event by applying a function to it
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::{Calendar, Event};
    ///
    /// let mut cal = Calendar::new("My Calendar");
    /// let event = Event::builder()
    ///     .title("Meeting")
    ///     .start("2025-11-01 10:00:00", "UTC")
    ///     .duration_hours(1)
    ///     .build()
    ///     .unwrap();
    /// cal.add_event(event);
    ///
    /// cal.update_event(0, |event| {
    ///     event.confirm();
    /// });
    /// ```
    pub fn update_event<F>(&mut self, index: usize, f: F) -> Option<()>
    where
        F: FnOnce(&mut Event),
    {
        self.events.get_mut(index).map(f)
    }

    /// Get all events in the calendar
    pub fn get_events(&self) -> &[Event] {
        &self.events
    }

    /// Find events by title (case-insensitive partial match)
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::{Calendar, Event};
    ///
    /// let mut cal = Calendar::new("My Calendar");
    /// let event = Event::builder()
    ///     .title("Team Meeting")
    ///     .start("2025-11-01 10:00:00", "UTC")
    ///     .duration_hours(1)
    ///     .build()
    ///     .unwrap();
    ///
    /// cal.add_event(event);
    /// let found = cal.find_events_by_title("meeting");
    /// assert_eq!(found.len(), 1);
    /// ```
    pub fn find_events_by_title(&self, title: &str) -> Vec<&Event> {
        let title_lower = title.to_lowercase();
        self.events
            .iter()
            .filter(|e| e.title.to_lowercase().contains(&title_lower))
            .collect()
    }

    /// Get all events occurring within a date range
    ///
    /// This expands recurring events into individual occurrences.
    /// Uses [events_between_capped](Self::events_between_capped) with a
    /// per-event limit of 100,000 occurrences.
    ///
    /// **Note:** If any single event generates more than 100,000 occurrences
    /// within the range (e.g. a secondly recurrence over a large window),
    /// the result will be silently truncated.  Use
    /// [`events_between_capped`](Self::events_between_capped) with an
    /// explicit cap when result completeness is critical.
    pub fn events_between(
        &self,
        start: DateTime<Tz>,
        end: DateTime<Tz>,
    ) -> Result<Vec<EventOccurrence<'_>>> {
        self.events_between_capped(start, end, 100_000)
    }

    /// Get all events occurring within a date range, with an explicit
    /// per-event occurrence cap.
    ///
    /// `max_per_event` limits how many occurrences each individual event may
    /// contribute. This prevents dense sub-daily recurrences from causing
    /// unbounded memory use when querying large time windows.
    pub fn events_between_capped(
        &self,
        start: DateTime<Tz>,
        end: DateTime<Tz>,
        max_per_event: usize,
    ) -> Result<Vec<EventOccurrence<'_>>> {
        if start > end {
            return Err(crate::error::EventixError::ValidationError(
                "Start time must be before or equal to end time".to_string(),
            ));
        }

        let mut occurrences = Vec::new();

        for (index, event) in self.events.iter().enumerate() {
            let event_occurrences = event.occurrences_between(start, end, max_per_event)?;

            for occurrence_time in event_occurrences {
                occurrences.push(EventOccurrence {
                    event_index: index,
                    event,
                    occurrence_time,
                });
            }
        }

        // Sort by occurrence time
        occurrences.sort_by_key(|o| o.occurrence_time);

        Ok(occurrences)
    }

    /// Get all events occurring on a specific date
    pub fn events_on_date(&self, date: DateTime<Tz>) -> Result<Vec<EventOccurrence<'_>>> {
        let (start_dt, end_dt) = local_day_window(date.date_naive(), date.timezone())?;
        self.events_between(start_dt, end_dt)
    }

    /// Create a lazy iterator over calendar days starting from the given date.
    ///
    /// Each yielded item is a [`crate::Result`] containing a [`crate::DayView`]
    /// for the requested local day. Views bucket active occurrences whose time
    /// span intersects the day, so overnight events appear on every day they
    /// overlap. The iterator advances one day at a time until the supported
    /// date range is exhausted.
    pub fn days(&self, start: DateTime<Tz>) -> DayIterator<'_> {
        DayIterator::new(self, start)
    }

    /// Create a lazy iterator over calendar days moving backward in time.
    ///
    /// Each yielded item is a [`crate::Result`] containing a [`crate::DayView`].
    pub fn days_back(&self, start: DateTime<Tz>) -> DayIterator<'_> {
        DayIterator::backward(self, start)
    }

    /// Create a lazy iterator over ISO weeks (Monday through Sunday).
    ///
    /// The first yielded item is a [`crate::Result`] containing the Monday-Sunday
    /// week that contains `start`.
    pub fn weeks(&self, start: DateTime<Tz>) -> WeekIterator<'_> {
        WeekIterator::new(self, start)
    }

    /// Create a lazy iterator over ISO weeks moving backward in time.
    ///
    /// Each yielded item is a [`crate::Result`] containing a contiguous
    /// Monday-Sunday block.
    pub fn weeks_back(&self, start: DateTime<Tz>) -> WeekIterator<'_> {
        WeekIterator::backward(self, start)
    }

    /// Get the number of events in the calendar
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Clear all events from the calendar
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    /// Export calendar to JSON
    ///
    /// Includes recurrence rules and exception dates for full round-trip
    /// fidelity with [`from_json()`](Self::from_json).
    pub fn to_json(&self) -> Result<String> {
        let json_val = serde_json::json!({
            "name": self.name,
            "description": self.description,
            "events": self.events.iter().map(|e| {
                let mut ev = serde_json::json!({
                    "title": e.title,
                    "description": e.description,
                    "start_time": e.start_time.to_rfc3339(),
                    "end_time": e.end_time.to_rfc3339(),
                    "timezone": e.timezone.name(),
                    "attendees": e.attendees,
                    "location": e.location,
                    "uid": e.uid,
                    "status": e.status,
                });
                if let Some(ref rec) = e.recurrence {
                    ev["recurrence"] = recurrence_to_json(rec);
                }
                if !e.exdates.is_empty() {
                    ev["exdates"] = serde_json::json!(
                        e.exdates.iter().map(|d| d.to_rfc3339()).collect::<Vec<_>>()
                    );
                }
                ev
            }).collect::<Vec<_>>(),
            "timezone": self.timezone.map(|tz| tz.name()),
        });

        serde_json::to_string_pretty(&json_val).map_err(|e| {
            crate::error::EventixError::Other(format!("JSON serialization error: {}", e))
        })
    }

    /// Import calendar from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        use crate::timezone::parse_timezone;

        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| crate::error::EventixError::Other(format!("JSON parse error: {}", e)))?;

        let name = value["name"]
            .as_str()
            .ok_or_else(|| crate::error::EventixError::Other("Missing 'name' field".to_string()))?
            .to_string();

        let description = value["description"].as_str().map(|s| s.to_string());

        let timezone = value["timezone"]
            .as_str()
            .and_then(|tz_str| parse_timezone(tz_str).ok());

        let mut calendar = Calendar {
            name,
            description,
            events: Vec::new(),
            timezone,
        };

        if let Some(events_array) = value["events"].as_array() {
            for event_val in events_array {
                let title = event_val["title"].as_str().ok_or_else(|| {
                    crate::error::EventixError::Other("Event missing 'title'".to_string())
                })?;

                let start_str = event_val["start_time"].as_str().ok_or_else(|| {
                    crate::error::EventixError::Other("Event missing 'start_time'".to_string())
                })?;

                let end_str = event_val["end_time"].as_str().ok_or_else(|| {
                    crate::error::EventixError::Other("Event missing 'end_time'".to_string())
                })?;

                let tz_str = event_val["timezone"].as_str().ok_or_else(|| {
                    crate::error::EventixError::Other("Event missing 'timezone'".to_string())
                })?;

                let tz = parse_timezone(tz_str)?;
                let start_time: DateTime<chrono::Utc> =
                    chrono::DateTime::parse_from_rfc3339(start_str)
                        .map_err(|e| crate::error::EventixError::DateTimeParse(e.to_string()))?
                        .with_timezone(&chrono::Utc);
                let end_time: DateTime<chrono::Utc> = chrono::DateTime::parse_from_rfc3339(end_str)
                    .map_err(|e| crate::error::EventixError::DateTimeParse(e.to_string()))?
                    .with_timezone(&chrono::Utc);

                let start_time_tz = start_time.with_timezone(&tz);
                let end_time_tz = end_time.with_timezone(&tz);

                let event = Event {
                    title: title.to_string(),
                    description: event_val["description"].as_str().map(|s| s.to_string()),
                    start_time: start_time_tz,
                    end_time: end_time_tz,
                    timezone: tz,
                    attendees: event_val["attendees"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    recurrence: match event_val.get("recurrence") {
                        Some(v) => Some(json_to_recurrence(v, tz)?),
                        None => None,
                    },
                    recurrence_filter: None,
                    exdates: match event_val["exdates"].as_array() {
                        Some(arr) => {
                            let mut dates = Vec::with_capacity(arr.len());
                            for (i, v) in arr.iter().enumerate() {
                                let s = v.as_str().ok_or_else(|| {
                                    EventixError::Other(format!("exdates[{}]: expected string", i))
                                })?;
                                let dt = chrono::DateTime::parse_from_rfc3339(s).map_err(|e| {
                                    EventixError::DateTimeParse(format!("exdates[{}]: {}", i, e))
                                })?;
                                dates.push(dt.with_timezone(&tz));
                            }
                            dates
                        }
                        None => Vec::new(),
                    },
                    location: event_val["location"].as_str().map(|s| s.to_string()),
                    uid: event_val["uid"].as_str().map(|s| s.to_string()),
                    status: match event_val.get("status") {
                        None => crate::event::EventStatus::default(),
                        Some(v) => serde_json::from_value(v.clone()).map_err(|e| {
                            crate::error::EventixError::Other(format!(
                                "Invalid event status '{}': {}",
                                v, e
                            ))
                        })?,
                    },
                };

                calendar.add_event(event);
            }
        }

        Ok(calendar)
    }
}

/// Represents a specific occurrence of an event (useful for recurring events)
#[derive(Debug, Clone)]
pub struct EventOccurrence<'a> {
    /// Index of the event in the calendar
    pub event_index: usize,

    /// Reference to the event
    pub event: &'a Event,

    /// When this occurrence happens
    pub occurrence_time: DateTime<Tz>,
}

impl<'a> EventOccurrence<'a> {
    /// Get the end time of this occurrence
    pub fn end_time(&self) -> DateTime<Tz> {
        let duration = self.event.duration();
        self.occurrence_time + duration
    }

    /// Get the title of this occurrence
    pub fn title(&self) -> &str {
        &self.event.title
    }

    /// Get the description of this occurrence
    pub fn description(&self) -> Option<&str> {
        self.event.description.as_deref()
    }
}

/// Serialize a Recurrence to a JSON value
fn recurrence_to_json(rec: &Recurrence) -> serde_json::Value {
    let freq_str = match rec.frequency() {
        Frequency::Secondly => "secondly",
        Frequency::Minutely => "minutely",
        Frequency::Hourly => "hourly",
        Frequency::Daily => "daily",
        Frequency::Weekly => "weekly",
        Frequency::Monthly => "monthly",
        Frequency::Yearly => "yearly",
    };
    let mut obj = serde_json::json!({
        "frequency": freq_str,
        "interval": rec.get_interval(),
    });
    if let Some(c) = rec.get_count() {
        obj["count"] = serde_json::json!(c);
    }
    if let Some(u) = rec.get_until() {
        obj["until"] = serde_json::json!(u.to_rfc3339());
    }
    if let Some(weekdays) = rec.get_weekdays() {
        let days: Vec<&str> = weekdays
            .iter()
            .map(|wd| match *wd {
                chrono::Weekday::Mon => "MO",
                chrono::Weekday::Tue => "TU",
                chrono::Weekday::Wed => "WE",
                chrono::Weekday::Thu => "TH",
                chrono::Weekday::Fri => "FR",
                chrono::Weekday::Sat => "SA",
                chrono::Weekday::Sun => "SU",
            })
            .collect();
        obj["weekdays"] = serde_json::json!(days);
    }
    obj
}

/// Deserialize a Recurrence from a JSON value
fn json_to_recurrence(val: &serde_json::Value, tz: Tz) -> crate::error::Result<Recurrence> {
    let freq_str = val["frequency"]
        .as_str()
        .ok_or_else(|| EventixError::Other("Recurrence missing 'frequency'".to_string()))?;
    let frequency = match freq_str {
        "secondly" => Frequency::Secondly,
        "minutely" => Frequency::Minutely,
        "hourly" => Frequency::Hourly,
        "daily" => Frequency::Daily,
        "weekly" => Frequency::Weekly,
        "monthly" => Frequency::Monthly,
        "yearly" => Frequency::Yearly,
        _ => {
            return Err(EventixError::Other(format!(
                "Unknown frequency: {}",
                freq_str
            )))
        }
    };
    let interval_raw = val["interval"].as_u64().unwrap_or(1);
    let interval = u16::try_from(interval_raw).map_err(|_| {
        EventixError::Other(format!(
            "Recurrence interval {} exceeds u16::MAX",
            interval_raw
        ))
    })?;

    // RFC 5545: COUNT and UNTIL must not both be present
    if !val["count"].is_null() && !val["until"].is_null() {
        return Err(EventixError::Other(
            "Recurrence cannot have both 'count' and 'until'".to_string(),
        ));
    }

    let mut rec = Recurrence::new(frequency).interval(interval);
    if let Some(c) = val["count"].as_u64() {
        let count = u32::try_from(c)
            .map_err(|_| EventixError::Other(format!("Recurrence count {} exceeds u32::MAX", c)))?;
        rec = rec.count(count);
    }
    if let Some(until_str) = val["until"].as_str() {
        let parsed = chrono::DateTime::parse_from_rfc3339(until_str)
            .map_err(|e| EventixError::DateTimeParse(format!("recurrence until: {}", e)))?;
        rec = rec.until(parsed.with_timezone(&tz));
    }
    if let Some(weekdays_arr) = val["weekdays"].as_array() {
        let mut weekdays = Vec::new();
        for wd_val in weekdays_arr {
            if let Some(wd_str) = wd_val.as_str() {
                let wd = match wd_str {
                    "MO" => chrono::Weekday::Mon,
                    "TU" => chrono::Weekday::Tue,
                    "WE" => chrono::Weekday::Wed,
                    "TH" => chrono::Weekday::Thu,
                    "FR" => chrono::Weekday::Fri,
                    "SA" => chrono::Weekday::Sat,
                    "SU" => chrono::Weekday::Sun,
                    _ => return Err(EventixError::Other(format!("Unknown weekday: {}", wd_str))),
                };
                weekdays.push(wd);
            }
        }
        if !weekdays.is_empty() {
            rec = rec.weekdays(weekdays);
        }
    }
    Ok(rec)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::Event;

    #[test]
    fn test_calendar_creation() {
        let cal = Calendar::new("Test Calendar").description("A test calendar");

        assert_eq!(cal.name, "Test Calendar");
        assert_eq!(cal.description, Some("A test calendar".to_string()));
        assert_eq!(cal.event_count(), 0);
    }

    #[test]
    fn test_add_events() {
        let mut cal = Calendar::new("My Calendar");

        let event = Event::builder()
            .title("Event 1")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap();

        cal.add_event(event);
        assert_eq!(cal.event_count(), 1);
    }

    #[test]
    fn test_update_event() {
        let mut cal = Calendar::new("My Calendar");
        let event = Event::builder()
            .title("Event 1")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap();
        cal.add_event(event);

        // Update successful
        let updated = cal.update_event(0, |e| {
            e.cancel(); // Change status to test closure execution
            e.title = "Updated Title".to_string();
        });
        assert!(updated.is_some());
        assert_eq!(cal.events[0].title, "Updated Title");
        assert!(!cal.events[0].is_active()); // Verify status was changed

        // Update invalid index
        let result = cal.update_event(99, |_| {});
        assert!(result.is_none());
    }

    #[test]
    fn test_find_events() {
        let mut cal = Calendar::new("My Calendar");

        let event1 = Event::builder()
            .title("Team Meeting")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap();

        let event2 = Event::builder()
            .title("Code Review")
            .start("2025-11-02 14:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap();

        cal.add_event(event1);
        cal.add_event(event2);

        let found = cal.find_events_by_title("meeting");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Team Meeting");
    }

    #[test]
    fn test_json_serialization() {
        let mut cal = Calendar::new("Test");
        let event = Event::builder()
            .title("Event")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap();

        cal.add_event(event);

        let json = cal.to_json().unwrap();
        let restored = Calendar::from_json(&json).unwrap();

        assert_eq!(restored.name, "Test");
        assert_eq!(restored.event_count(), 1);
    }

    #[test]
    fn test_json_recurrence_roundtrip() {
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let exdate = crate::timezone::parse_datetime_with_tz("2025-01-08 09:00:00", tz).unwrap();

        let mut cal = Calendar::new("Recurrence JSON");
        let event = Event::builder()
            .title("Daily Standup")
            .start("2025-01-06 09:00:00", "UTC")
            .duration_minutes(15)
            .recurrence(Recurrence::daily().interval(2).count(10))
            .exception_date(exdate)
            .build()
            .unwrap();
        cal.add_event(event);

        let json = cal.to_json().unwrap();
        // Verify recurrence and exdates are in the JSON
        assert!(
            json.contains("\"frequency\""),
            "JSON should contain recurrence frequency"
        );
        assert!(json.contains("\"exdates\""), "JSON should contain exdates");

        let restored = Calendar::from_json(&json).unwrap();
        assert_eq!(restored.event_count(), 1);

        let ev = &restored.events[0];
        let rec = ev.recurrence.as_ref().unwrap();
        assert_eq!(rec.frequency(), rrule::Frequency::Daily);
        assert_eq!(rec.get_interval(), 2);
        assert_eq!(rec.get_count(), Some(10));
        assert_eq!(ev.exdates.len(), 1);
    }

    #[test]
    fn test_json_import_rejects_bad_recurrence() {
        // Malformed recurrence frequency should fail import, not silently drop
        let json = r#"{
            "name": "Test",
            "events": [{
                "title": "Bad Recurrence",
                "start_time": "2025-01-06T09:00:00+00:00",
                "end_time": "2025-01-06T10:00:00+00:00",
                "timezone": "UTC",
                "recurrence": { "frequency": "biweekly", "interval": 1 }
            }]
        }"#;
        let result = Calendar::from_json(json);
        assert!(
            result.is_err(),
            "Should reject unknown recurrence frequency"
        );
    }

    #[test]
    fn test_json_import_rejects_bad_exdate() {
        // Malformed exdate should fail import, not silently drop
        let json = r#"{
            "name": "Test",
            "events": [{
                "title": "Bad Exdate",
                "start_time": "2025-01-06T09:00:00+00:00",
                "end_time": "2025-01-06T10:00:00+00:00",
                "timezone": "UTC",
                "exdates": ["not-a-date"]
            }]
        }"#;
        let result = Calendar::from_json(json);
        assert!(result.is_err(), "Should reject unparseable exdate");
    }

    #[test]
    fn test_json_import_rejects_overflowing_interval() {
        let json = r#"{
            "name": "Test",
            "events": [{
                "title": "Big Interval",
                "start_time": "2025-01-06T09:00:00+00:00",
                "end_time": "2025-01-06T10:00:00+00:00",
                "timezone": "UTC",
                "recurrence": { "frequency": "daily", "interval": 999999, "count": 5 }
            }]
        }"#;
        let result = Calendar::from_json(json);
        assert!(result.is_err(), "Should reject interval exceeding u16::MAX");
    }

    #[test]
    fn test_json_import_rejects_count_and_until() {
        // Even non-canonical types (e.g. count as string) should be caught
        let json = r#"{
            "name": "Test",
            "events": [{
                "title": "Both",
                "start_time": "2025-01-06T09:00:00+00:00",
                "end_time": "2025-01-06T10:00:00+00:00",
                "timezone": "UTC",
                "recurrence": { "frequency": "daily", "count": "10", "until": "2025-02-01T00:00:00+00:00" }
            }]
        }"#;
        let result = Calendar::from_json(json);
        assert!(result.is_err(), "Should reject both count and until");
    }
    #[test]
    fn test_events_between_invalid_range() {
        use crate::timezone::parse_datetime_with_tz;
        use crate::timezone::parse_timezone;
        let cal = Calendar::new("Test");
        let tz = parse_timezone("UTC").unwrap();
        let start = parse_datetime_with_tz("2025-11-01 12:00:00", tz).unwrap();
        let end = parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();

        let result = cal.events_between(start, end);
        assert!(result.is_err());
    }
}
