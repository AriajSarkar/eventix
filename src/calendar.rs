//! Calendar type for managing collections of events

use crate::error::{EventixError, Result};
use crate::event::Event;
use chrono::{DateTime, TimeZone};
use chrono_tz::Tz;

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
    /// // ... add event ...
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
    pub fn events_between(
        &self,
        start: DateTime<Tz>,
        end: DateTime<Tz>,
    ) -> Result<Vec<EventOccurrence<'_>>> {
        let mut occurrences = Vec::new();

        for (index, event) in self.events.iter().enumerate() {
            let event_occurrences = event.occurrences_between(start, end, 1000)?;

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
        let start = date
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| EventixError::ValidationError("Invalid start time".to_string()))?;
        let end = date
            .date_naive()
            .and_hms_opt(23, 59, 59)
            .ok_or_else(|| EventixError::ValidationError("Invalid end time".to_string()))?;

        let tz = date.timezone();
        let start_dt = tz
            .from_local_datetime(&start)
            .earliest()
            .ok_or_else(|| EventixError::ValidationError("Ambiguous start time".to_string()))?;
        let end_dt = tz
            .from_local_datetime(&end)
            .latest()
            .ok_or_else(|| EventixError::ValidationError("Ambiguous end time".to_string()))?;

        self.events_between(start_dt, end_dt)
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
    pub fn to_json(&self) -> Result<String> {
        // Create a simplified version for JSON serialization
        let json_val = serde_json::json!({
            "name": self.name,
            "description": self.description,
            "events": self.events.iter().map(|e| serde_json::json!({
                "title": e.title,
                "description": e.description,
                "start_time": e.start_time.to_rfc3339(),
                "end_time": e.end_time.to_rfc3339(),
                "timezone": e.timezone.name(),
                "attendees": e.attendees,
                "location": e.location,
                "uid": e.uid,
            })).collect::<Vec<_>>(),
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

        let timezone = value["timezone"].as_str().and_then(|tz_str| parse_timezone(tz_str).ok());

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
                            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                        })
                        .unwrap_or_default(),
                    recurrence: None,
                    recurrence_filter: None,
                    exdates: Vec::new(),
                    location: event_val["location"].as_str().map(|s| s.to_string()),
                    uid: event_val["uid"].as_str().map(|s| s.to_string()),
                    status: crate::event::EventStatus::default(), // Default to Confirmed if missing
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

#[cfg(test)]
mod tests {
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
}
