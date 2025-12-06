//! Gap and overlap validation for calendar events
//!
//! This module provides functionality to detect gaps between events,
//! find overlapping events, and analyze schedule density - features
//! not commonly found in other calendar libraries.

use crate::calendar::Calendar;
use crate::error::Result;
use chrono::{DateTime, Duration};
use chrono_tz::Tz;

/// Represents a time gap between two events
#[derive(Debug, Clone)]
pub struct TimeGap {
    /// Start of the gap
    pub start: DateTime<Tz>,
    /// End of the gap
    pub end: DateTime<Tz>,
    /// Duration of the gap
    pub duration: Duration,
    /// Event before this gap (if any)
    pub before_event: Option<String>,
    /// Event after this gap (if any)
    pub after_event: Option<String>,
}

impl TimeGap {
    /// Create a new time gap
    pub fn new(
        start: DateTime<Tz>,
        end: DateTime<Tz>,
        before_event: Option<String>,
        after_event: Option<String>,
    ) -> Self {
        let duration = end.signed_duration_since(start);
        Self {
            start,
            end,
            duration,
            before_event,
            after_event,
        }
    }

    /// Get duration in minutes
    pub fn duration_minutes(&self) -> i64 {
        self.duration.num_minutes()
    }

    /// Get duration in hours
    pub fn duration_hours(&self) -> i64 {
        self.duration.num_hours()
    }

    /// Check if this gap is at least a certain duration
    pub fn is_at_least(&self, min_duration: Duration) -> bool {
        self.duration >= min_duration
    }
}

/// Represents an overlap between two or more events
#[derive(Debug, Clone)]
pub struct EventOverlap {
    /// Start of the overlap
    pub start: DateTime<Tz>,
    /// End of the overlap
    pub end: DateTime<Tz>,
    /// Duration of the overlap
    pub duration: Duration,
    /// Events involved in this overlap
    pub events: Vec<String>,
}

impl EventOverlap {
    /// Create a new event overlap
    pub fn new(start: DateTime<Tz>, end: DateTime<Tz>, events: Vec<String>) -> Self {
        let duration = end.signed_duration_since(start);
        Self {
            start,
            end,
            duration,
            events,
        }
    }

    /// Get duration in minutes
    pub fn duration_minutes(&self) -> i64 {
        self.duration.num_minutes()
    }

    /// Number of overlapping events
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

/// Schedule density metrics
#[derive(Debug, Clone)]
pub struct ScheduleDensity {
    /// Total time span analyzed
    pub total_duration: Duration,
    /// Total time occupied by events
    pub busy_duration: Duration,
    /// Total free time
    pub free_duration: Duration,
    /// Percentage of time occupied (0.0 - 100.0)
    pub occupancy_percentage: f64,
    /// Number of events
    pub event_count: usize,
    /// Number of gaps
    pub gap_count: usize,
    /// Number of overlaps
    pub overlap_count: usize,
}

impl ScheduleDensity {
    /// Check if the schedule is considered busy (>60% occupied)
    pub fn is_busy(&self) -> bool {
        self.occupancy_percentage > 60.0
    }

    /// Check if the schedule is considered light (<30% occupied)
    pub fn is_light(&self) -> bool {
        self.occupancy_percentage < 30.0
    }

    /// Check if the schedule has any overlaps
    pub fn has_conflicts(&self) -> bool {
        self.overlap_count > 0
    }
}

/// Find all gaps between events in a time range
///
/// # Examples
///
/// ```
/// use eventix::{Calendar, Event, gap_validation};
/// use eventix::timezone::parse_datetime_with_tz;
/// use chrono::Duration;
///
/// let mut cal = Calendar::new("Test");
///
/// let event1 = Event::builder()
///     .title("Meeting 1")
///     .start("2025-11-01 09:00:00", "UTC")
///     .duration_hours(1)
///     .build()
///     .unwrap();
///
/// let event2 = Event::builder()
///     .title("Meeting 2")
///     .start("2025-11-01 11:00:00", "UTC")
///     .duration_hours(1)
///     .build()
///     .unwrap();
///
/// cal.add_event(event1);
/// cal.add_event(event2);
///
/// let tz = eventix::timezone::parse_timezone("UTC").unwrap();
/// let start = parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
/// let end = parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();
///
/// let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(30)).unwrap();
/// assert!(gaps.len() > 0);
/// ```
pub fn find_gaps(
    calendar: &Calendar,
    start: DateTime<Tz>,
    end: DateTime<Tz>,
    min_gap_duration: Duration,
) -> Result<Vec<TimeGap>> {
    let mut occurrences = calendar.events_between(start, end)?;

    // Sort by start time
    occurrences.sort_by_key(|o| o.occurrence_time);

    let mut gaps = Vec::new();
    let mut current_time = start;

    for occurrence in occurrences.iter() {
        let event_start = occurrence.occurrence_time;

        // Check if there's a gap before this event
        if event_start > current_time {
            let gap =
                TimeGap::new(current_time, event_start, None, Some(occurrence.title().to_string()));

            if gap.duration >= min_gap_duration {
                gaps.push(gap);
            }
        }

        // Move current time to end of this event
        let event_end = occurrence.end_time();
        if event_end > current_time {
            current_time = event_end;
        }
    }

    // Check for gap at the end
    if end > current_time {
        let gap = TimeGap::new(current_time, end, None, None);
        if gap.duration >= min_gap_duration {
            gaps.push(gap);
        }
    }

    Ok(gaps)
}

/// Find all overlapping events in a time range
///
/// # Examples
///
/// ```
/// use eventix::{Calendar, Event, gap_validation};
/// use eventix::timezone::parse_datetime_with_tz;
///
/// let mut cal = Calendar::new("Test");
///
/// let event1 = Event::builder()
///     .title("Meeting 1")
///     .start("2025-11-01 09:00:00", "UTC")
///     .duration_hours(2)
///     .build()
///     .unwrap();
///
/// let event2 = Event::builder()
///     .title("Meeting 2")
///     .start("2025-11-01 10:00:00", "UTC")
///     .duration_hours(1)
///     .build()
///     .unwrap();
///
/// cal.add_event(event1);
/// cal.add_event(event2);
///
/// let tz = eventix::timezone::parse_timezone("UTC").unwrap();
/// let start = parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
/// let end = parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();
///
/// let overlaps = gap_validation::find_overlaps(&cal, start, end).unwrap();
/// assert_eq!(overlaps.len(), 1);
/// ```
pub fn find_overlaps(
    calendar: &Calendar,
    start: DateTime<Tz>,
    end: DateTime<Tz>,
) -> Result<Vec<EventOverlap>> {
    let occurrences = calendar.events_between(start, end)?;
    let mut overlaps = Vec::new();

    // Check each pair of events for overlap
    for i in 0..occurrences.len() {
        for j in (i + 1)..occurrences.len() {
            let event1 = &occurrences[i];
            let event2 = &occurrences[j];

            let start1 = event1.occurrence_time;
            let end1 = event1.end_time();
            let start2 = event2.occurrence_time;
            let end2 = event2.end_time();

            // Check if they overlap
            if start1 < end2 && start2 < end1 {
                let overlap_start = start1.max(start2);
                let overlap_end = end1.min(end2);

                let overlap = EventOverlap::new(
                    overlap_start,
                    overlap_end,
                    vec![event1.title().to_string(), event2.title().to_string()],
                );

                overlaps.push(overlap);
            }
        }
    }

    Ok(overlaps)
}

/// Calculate schedule density metrics
///
/// # Examples
///
/// ```
/// use eventix::{Calendar, Event, gap_validation};
/// use eventix::timezone::parse_datetime_with_tz;
///
/// let mut cal = Calendar::new("Test");
///
/// let event = Event::builder()
///     .title("Meeting")
///     .start("2025-11-01 09:00:00", "UTC")
///     .duration_hours(2)
///     .build()
///     .unwrap();
///
/// cal.add_event(event);
///
/// let tz = eventix::timezone::parse_timezone("UTC").unwrap();
/// let start = parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
/// let end = parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();
///
/// let density = gap_validation::calculate_density(&cal, start, end).unwrap();
/// assert!(density.occupancy_percentage > 0.0);
/// ```
pub fn calculate_density(
    calendar: &Calendar,
    start: DateTime<Tz>,
    end: DateTime<Tz>,
) -> Result<ScheduleDensity> {
    let total_duration = end.signed_duration_since(start);
    let occurrences = calendar.events_between(start, end)?;

    // Calculate busy time
    let mut busy_duration = Duration::zero();
    for occurrence in occurrences.iter() {
        let event_start = occurrence.occurrence_time.max(start);
        let event_end = occurrence.end_time().min(end);
        if event_end > event_start {
            busy_duration += event_end.signed_duration_since(event_start);
        }
    }

    let free_duration = total_duration - busy_duration;
    let occupancy_percentage = if total_duration.num_seconds() > 0 {
        (busy_duration.num_seconds() as f64 / total_duration.num_seconds() as f64) * 100.0
    } else {
        0.0
    };

    let gaps = find_gaps(calendar, start, end, Duration::minutes(0))?;
    let overlaps = find_overlaps(calendar, start, end)?;

    Ok(ScheduleDensity {
        total_duration,
        busy_duration,
        free_duration,
        occupancy_percentage,
        event_count: occurrences.len(),
        gap_count: gaps.len(),
        overlap_count: overlaps.len(),
    })
}

/// Find the longest available gap in a time range
///
/// Returns the longest continuous gap that could fit a meeting.
pub fn find_longest_gap(
    calendar: &Calendar,
    start: DateTime<Tz>,
    end: DateTime<Tz>,
) -> Result<Option<TimeGap>> {
    let gaps = find_gaps(calendar, start, end, Duration::minutes(0))?;
    Ok(gaps.into_iter().max_by_key(|g| g.duration))
}

/// Find all gaps of at least a specified duration
///
/// Useful for finding time slots for meetings of a specific length.
pub fn find_available_slots(
    calendar: &Calendar,
    start: DateTime<Tz>,
    end: DateTime<Tz>,
    required_duration: Duration,
) -> Result<Vec<TimeGap>> {
    find_gaps(calendar, start, end, required_duration)
}

/// Check if a time slot is available (no conflicts)
pub fn is_slot_available(
    calendar: &Calendar,
    slot_start: DateTime<Tz>,
    slot_end: DateTime<Tz>,
) -> Result<bool> {
    // To catch events that might end during our slot, we need to query from
    // a wider range - start from beginning of day or before slot_start
    let query_start = slot_start - Duration::days(1);
    let occurrences = calendar.events_between(query_start, slot_end)?;

    for occurrence in occurrences.iter() {
        let event_start = occurrence.occurrence_time;
        let event_end = occurrence.end_time();

        // Check for any overlap between event and slot
        if event_start < slot_end && slot_start < event_end {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Suggest alternative times for a conflicting event
///
/// Finds available slots near the requested time.
///
/// # Examples
///
/// ```
/// use eventix::{Calendar, Event, gap_validation};
/// use eventix::timezone::parse_datetime_with_tz;
/// use chrono::Duration;
///
/// let mut cal = Calendar::new("Test");
/// let tz = eventix::timezone::parse_timezone("UTC").unwrap();
///
/// // Existing event 9-10
/// let event = Event::builder()
///     .title("Meeting")
///     .start("2025-11-01 09:00:00", "UTC")
///     .duration_hours(1)
///     .build()
///     .unwrap();
/// cal.add_event(event);
///
/// // Attempt to schedule 9:30-10:30 (conflict)
/// let requested = parse_datetime_with_tz("2025-11-01 09:30:00", tz).unwrap();
///
/// // Find alternatives within +/- 4 hours
/// let alternatives = gap_validation::suggest_alternatives(
///     &cal,
///     requested,
///     Duration::hours(1), // 1 hour duration
///     Duration::hours(4)  // Search window
/// ).unwrap();
///
/// assert!(alternatives.len() > 0);
/// ```
pub fn suggest_alternatives(
    calendar: &Calendar,
    requested_start: DateTime<Tz>,
    duration: Duration,
    search_window: Duration,
) -> Result<Vec<DateTime<Tz>>> {
    let search_start = requested_start - search_window;
    let search_end = requested_start + search_window;

    let gaps = find_gaps(calendar, search_start, search_end, duration)?;

    let mut suggestions = Vec::new();
    for gap in gaps {
        // Check if the requested duration fits in this gap
        if gap.duration >= duration {
            // Suggest the start of the gap
            suggestions.push(gap.start);

            // Also suggest slots within the gap if it's large enough
            let mut slot_start = gap.start + Duration::hours(1);
            while slot_start + duration <= gap.end {
                suggestions.push(slot_start);
                slot_start += Duration::hours(1);
            }
        }
    }

    suggestions.sort();
    Ok(suggestions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timezone::parse_datetime_with_tz;
    use crate::Calendar;
    use crate::Event;

    fn create_test_calendar() -> Result<Calendar> {
        let mut cal = Calendar::new("Test Calendar");

        let event1 = Event::builder()
            .title("Morning Meeting")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_hours(1)
            .build()?;

        let event2 = Event::builder()
            .title("Lunch")
            .start("2025-11-01 12:00:00", "UTC")
            .duration_hours(1)
            .build()?;

        let event3 = Event::builder()
            .title("Afternoon Meeting")
            .start("2025-11-01 15:00:00", "UTC")
            .duration_hours(2)
            .build()?;

        cal.add_event(event1);
        cal.add_event(event2);
        cal.add_event(event3);

        Ok(cal)
    }

    #[test]
    fn test_find_gaps() {
        let cal = create_test_calendar().unwrap();
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
        let end = parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

        let gaps = find_gaps(&cal, start, end, Duration::minutes(30)).unwrap();

        // Should find gaps: 8-9am, 10am-12pm, 1-3pm, 5-6pm
        assert!(gaps.len() >= 3);
    }

    #[test]
    fn test_find_overlaps_no_conflict() {
        let cal = create_test_calendar().unwrap();
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
        let end = parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

        let overlaps = find_overlaps(&cal, start, end).unwrap();

        // No overlapping events in our test calendar
        assert_eq!(overlaps.len(), 0);
    }

    #[test]
    fn test_find_overlaps_with_conflict() {
        let mut cal = Calendar::new("Test");

        let event1 = Event::builder()
            .title("Meeting 1")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_hours(2)
            .build()
            .unwrap();

        let event2 = Event::builder()
            .title("Meeting 2")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap();

        cal.add_event(event1);
        cal.add_event(event2);

        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
        let end = parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

        let overlaps = find_overlaps(&cal, start, end).unwrap();

        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].duration_minutes(), 60);
    }

    #[test]
    fn test_calculate_density() {
        let cal = create_test_calendar().unwrap();
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
        let end = parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

        let density = calculate_density(&cal, start, end).unwrap();

        assert_eq!(density.event_count, 3);
        assert!(density.occupancy_percentage > 0.0);
        assert!(density.occupancy_percentage < 100.0);
        assert_eq!(density.overlap_count, 0);
    }

    #[test]
    fn test_is_slot_available() {
        let cal = create_test_calendar().unwrap();
        let tz = crate::timezone::parse_timezone("UTC").unwrap();

        // Available slot
        let slot_start = parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();
        let slot_end = parse_datetime_with_tz("2025-11-01 11:00:00", tz).unwrap();
        assert!(is_slot_available(&cal, slot_start, slot_end).unwrap());

        // Conflicting slot
        let conflict_start = parse_datetime_with_tz("2025-11-01 09:30:00", tz).unwrap();
        let conflict_end = parse_datetime_with_tz("2025-11-01 10:30:00", tz).unwrap();
        assert!(!is_slot_available(&cal, conflict_start, conflict_end).unwrap());
    }

    #[test]
    fn test_find_longest_gap() {
        let cal = create_test_calendar().unwrap();
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
        let end = parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

        let longest = find_longest_gap(&cal, start, end).unwrap();

        assert!(longest.is_some());
        let gap = longest.unwrap();
        assert!(gap.duration_minutes() >= 120); // At least 2 hours
    }

    #[test]
    fn test_find_available_slots() {
        let cal = create_test_calendar().unwrap();
        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
        let end = parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

        // Find slots for 1-hour meeting
        let slots = find_available_slots(&cal, start, end, Duration::hours(1)).unwrap();

        assert!(slots.len() > 0);
        for slot in slots {
            assert!(slot.duration >= Duration::hours(1));
        }
    }

    #[test]
    fn test_suggest_alternatives() {
        let cal = create_test_calendar().unwrap();
        let tz = crate::timezone::parse_timezone("UTC").unwrap();

        // Try to schedule during morning meeting (conflict)
        let requested = parse_datetime_with_tz("2025-11-01 09:30:00", tz).unwrap();

        let alternatives =
            suggest_alternatives(&cal, requested, Duration::hours(1), Duration::hours(4)).unwrap();

        assert!(alternatives.len() > 0);
    }

    #[test]
    fn test_schedule_density_busy() {
        let mut cal = Calendar::new("Busy");

        // Create a packed schedule
        for hour in 9..17 {
            let event = Event::builder()
                .title(format!("Meeting {}", hour))
                .start(&format!("2025-11-01 {:02}:00:00", hour), "UTC")
                .duration_minutes(45)
                .build()
                .unwrap();
            cal.add_event(event);
        }

        let tz = crate::timezone::parse_timezone("UTC").unwrap();
        let start = parse_datetime_with_tz("2025-11-01 09:00:00", tz).unwrap();
        let end = parse_datetime_with_tz("2025-11-01 17:00:00", tz).unwrap();

        let density = calculate_density(&cal, start, end).unwrap();

        assert!(density.is_busy());
        assert!(density.occupancy_percentage > 60.0);
    }
}
