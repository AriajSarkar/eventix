//! Integration tests for gap validation functionality
//!
//! These tests validate the unique gap detection and schedule analysis
//! features that set eventix apart from other calendar crates.
#![allow(clippy::unwrap_used, clippy::len_zero)]

mod common;

use common::parse;
use eventix::gap_validation::{EventOverlap, ScheduleDensity, TimeGap};
use eventix::{gap_validation, timezone, Calendar, Duration, Event, EventStatus, Recurrence};
use serde_json::json;

#[test]
fn test_comprehensive_gap_detection() {
    // Create a realistic daily schedule
    let mut cal = Calendar::new("Daily Schedule");

    let events = vec![
        ("Morning Standup", "09:00:00", 15),
        ("Code Review", "10:00:00", 60),
        ("Lunch Break", "12:30:00", 60),
        ("Client Call", "14:00:00", 45),
        ("Team Sync", "16:00:00", 30),
    ];

    for (title, time, duration) in events {
        let event = Event::builder()
            .title(title)
            .start(&format!("2025-11-01 {}", time), "UTC")
            .duration_minutes(duration)
            .build()
            .unwrap();
        cal.add_event(event);
    }

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

    // Find all gaps of at least 30 minutes
    let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(30)).unwrap();

    assert!(gaps.len() >= 3, "Should find multiple gaps in the schedule");

    // Verify gap durations
    for gap in &gaps {
        assert!(gap.duration_minutes() >= 30, "Each gap should be at least 30 minutes");
    }
}

#[test]
fn test_overlap_detection_complex() {
    // Test with multiple overlapping events
    let mut cal = Calendar::new("Overlapping Schedule");

    // Create intentionally overlapping events
    let event1 = Event::builder()
        .title("Conference Call")
        .start("2025-11-01 10:00:00", "UTC")
        .duration_hours(2)
        .build()
        .unwrap();

    let event2 = Event::builder()
        .title("Team Meeting")
        .start("2025-11-01 11:00:00", "UTC")
        .duration_hours(1)
        .build()
        .unwrap();

    let event3 = Event::builder()
        .title("One-on-One")
        .start("2025-11-01 11:30:00", "UTC")
        .duration_minutes(30)
        .build()
        .unwrap();

    cal.add_event(event1);
    cal.add_event(event2);
    cal.add_event(event3);

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 09:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 14:00:00", tz).unwrap();

    let overlaps = gap_validation::find_overlaps(&cal, start, end).unwrap();

    assert!(overlaps.len() > 0, "Should detect overlapping events");

    // Verify overlap details
    for overlap in &overlaps {
        assert!(overlap.event_count() >= 2, "Each overlap should involve at least 2 events");
        assert!(overlap.duration_minutes() > 0, "Overlap should have positive duration");
    }
}

#[test]
fn test_schedule_density_analysis() {
    // Create schedules with different densities
    let mut light_cal = Calendar::new("Light Schedule");
    let mut busy_cal = Calendar::new("Busy Schedule");

    // Light schedule: 2 hours of meetings in 10-hour window
    light_cal.add_event(
        Event::builder()
            .title("Quick Sync")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );
    light_cal.add_event(
        Event::builder()
            .title("Status Update")
            .start("2025-11-01 15:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    // Busy schedule: 8 hours of meetings in 10-hour window
    for hour in 9..17 {
        busy_cal.add_event(
            Event::builder()
                .title(format!("Meeting {}", hour))
                .start(&format!("2025-11-01 {:02}:00:00", hour), "UTC")
                .duration_minutes(50)
                .build()
                .unwrap(),
        );
    }

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

    let light_density = gap_validation::calculate_density(&light_cal, start, end).unwrap();
    let busy_density = gap_validation::calculate_density(&busy_cal, start, end).unwrap();

    assert!(light_density.is_light(), "Light schedule should be detected");
    assert!(busy_density.is_busy(), "Busy schedule should be detected");
    assert!(busy_density.occupancy_percentage > light_density.occupancy_percentage);
}

#[test]
fn test_find_available_slots_for_meeting() {
    let mut cal = Calendar::new("Work Calendar");

    // Add morning and afternoon meetings
    cal.add_event(
        Event::builder()
            .title("Morning Meeting")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    cal.add_event(
        Event::builder()
            .title("Afternoon Meeting")
            .start("2025-11-01 15:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

    // Find slots for 2-hour meeting
    let slots = gap_validation::find_available_slots(&cal, start, end, Duration::hours(2)).unwrap();

    assert!(slots.len() > 0, "Should find available slots");

    // Verify all slots are long enough
    for slot in slots {
        assert!(slot.duration >= Duration::hours(2), "Each slot should fit 2-hour meeting");
    }
}

#[test]
fn test_conflict_resolution_suggestions() {
    let mut cal = Calendar::new("Conflicting Schedule");

    // Create a meeting that conflicts with requested time
    cal.add_event(
        Event::builder()
            .title("Existing Meeting")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let requested = timezone::parse_datetime_with_tz("2025-11-01 10:30:00", tz).unwrap();

    // Ask for alternative times
    let alternatives = gap_validation::suggest_alternatives(
        &cal,
        requested,
        Duration::hours(1),
        Duration::hours(3),
    )
    .unwrap();

    assert!(alternatives.len() > 0, "Should suggest alternative times");

    // Verify alternatives are actually available
    for alt_time in alternatives {
        let alt_end = alt_time + Duration::hours(1);
        assert!(
            gap_validation::is_slot_available(&cal, alt_time, alt_end).unwrap(),
            "Suggested alternative should be available"
        );
    }
}

#[test]
fn test_recurring_events_gap_detection() {
    let mut cal = Calendar::new("Recurring Schedule");

    // Add recurring daily standup
    cal.add_event(
        Event::builder()
            .title("Daily Standup")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_minutes(15)
            .recurrence(Recurrence::daily().count(5))
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-05 18:00:00", tz).unwrap();

    let gaps = gap_validation::find_gaps(&cal, start, end, Duration::hours(1)).unwrap();

    // Should find large gaps between recurring events
    assert!(gaps.len() > 0, "Should find gaps in recurring schedule");
}

#[test]
fn test_longest_gap_finder() {
    let mut cal = Calendar::new("Test Calendar");

    cal.add_event(
        Event::builder()
            .title("Morning Brief")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_minutes(30)
            .build()
            .unwrap(),
    );

    cal.add_event(
        Event::builder()
            .title("End of Day")
            .start("2025-11-01 16:00:00", "UTC")
            .duration_minutes(30)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

    let longest_gap = gap_validation::find_longest_gap(&cal, start, end).unwrap();

    assert!(longest_gap.is_some(), "Should find longest gap");
    let gap = longest_gap.unwrap();

    // The gap between 9:30 and 16:00 should be the longest (6.5 hours)
    assert!(gap.duration_hours() >= 6, "Longest gap should be at least 6 hours");
}

#[test]
fn test_slot_availability_edge_cases() {
    let mut cal = Calendar::new("Edge Cases");

    cal.add_event(
        Event::builder()
            .title("Exact Boundary")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();

    // Test slot ending exactly when event starts (should be available)
    let before_start = timezone::parse_datetime_with_tz("2025-11-01 09:00:00", tz).unwrap();
    let before_end = timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();
    assert!(
        gap_validation::is_slot_available(&cal, before_start, before_end).unwrap(),
        "Slot ending at event start should be available"
    );

    // Test slot starting exactly when event ends (should be available)
    let after_start = timezone::parse_datetime_with_tz("2025-11-01 11:00:00", tz).unwrap();
    let after_end = timezone::parse_datetime_with_tz("2025-11-01 12:00:00", tz).unwrap();
    assert!(
        gap_validation::is_slot_available(&cal, after_start, after_end).unwrap(),
        "Slot starting at event end should be available"
    );

    // Test slot overlapping by 1 minute (should not be available)
    let overlap_start = timezone::parse_datetime_with_tz("2025-11-01 10:30:00", tz).unwrap();
    let overlap_end = timezone::parse_datetime_with_tz("2025-11-01 11:30:00", tz).unwrap();
    assert!(
        !gap_validation::is_slot_available(&cal, overlap_start, overlap_end).unwrap(),
        "Overlapping slot should not be available"
    );
}

#[test]
fn test_multi_timezone_gap_detection() {
    let mut cal = Calendar::new("Multi-Timezone");

    // Event in New York time
    cal.add_event(
        Event::builder()
            .title("US Meeting")
            .start("2025-11-01 09:00:00", "America/New_York")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    // Event in Tokyo time
    cal.add_event(
        Event::builder()
            .title("Japan Meeting")
            .start("2025-11-01 22:00:00", "Asia/Tokyo")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    // Analyze in UTC
    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-02 00:00:00", tz).unwrap();

    let gaps = gap_validation::find_gaps(&cal, start, end, Duration::hours(1)).unwrap();

    assert!(gaps.len() > 0, "Should find gaps across timezones");
}

#[test]
fn test_density_metrics_comprehensive() {
    let mut cal = Calendar::new("Metrics Test");

    // Create a schedule with known characteristics
    cal.add_event(
        Event::builder()
            .title("Event 1")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(2)
            .build()
            .unwrap(),
    );

    cal.add_event(
        Event::builder()
            .title("Event 2")
            .start("2025-11-01 14:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

    let density = gap_validation::calculate_density(&cal, start, end).unwrap();

    // 3 hours busy in 10-hour window = 30%
    assert_eq!(density.event_count, 2);
    assert!(
        (density.occupancy_percentage - 30.0).abs() < 1.0,
        "Should be approximately 30% occupied"
    );
    assert!(density.free_duration > density.busy_duration, "Should have more free time");
    assert!(!density.has_conflicts(), "Should have no conflicts");
    // 30% is right at the boundary - not considered "light" (which is <30%)
    assert!(!density.is_busy(), "Should not be considered busy");
}

#[test]
fn test_gap_metadata() {
    let mut cal = Calendar::new("Metadata Test");

    cal.add_event(
        Event::builder()
            .title("First Meeting")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    cal.add_event(
        Event::builder()
            .title("Second Meeting")
            .start("2025-11-01 11:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

    let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(0)).unwrap();

    // Check that gaps have correct metadata about surrounding events
    for gap in gaps {
        if gap.after_event.is_some() || gap.before_event.is_some() {
            assert!(
                gap.duration_minutes() > 0,
                "Gap with event references should have positive duration"
            );
        }
    }
}

#[test]
fn test_touching_events_not_overlapping() {
    // CRITICAL EDGE CASE: Events that share an exact boundary time should NOT overlap.
    // Event A ends at 10:00, Event B starts at 10:00 = NO OVERLAP
    let mut cal = Calendar::new("Touching Events");

    // Event A: 09:00 - 10:00
    cal.add_event(
        Event::builder()
            .title("Event A")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    // Event B: 10:00 - 11:00 (starts exactly when A ends)
    cal.add_event(
        Event::builder()
            .title("Event B")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    // Event C: 11:00 - 12:00 (starts exactly when B ends)
    cal.add_event(
        Event::builder()
            .title("Event C")
            .start("2025-11-01 11:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 13:00:00", tz).unwrap();

    let overlaps = gap_validation::find_overlaps(&cal, start, end).unwrap();

    // Back-to-back events should have ZERO overlaps
    assert_eq!(
        overlaps.len(),
        0,
        "Touching events (A ends when B starts) should NOT be detected as overlapping"
    );
}

#[test]
fn test_overlaps_sweep_line_performance() {
    // Test that sweep line algorithm handles many events efficiently
    let mut cal = Calendar::new("Many Events");

    // Create 100 events distributed across 28 days at the same time
    // Multiple events per day will overlap (verifies correct detection at scale)
    for i in 0..100 {
        cal.add_event(
            Event::builder()
                .title(format!("Event {}", i))
                .start(&format!("2025-11-{:02} 10:00:00", (i % 28) + 1), "UTC")
                .duration_hours(1)
                .build()
                .unwrap(),
        );
    }

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-30 23:59:59", tz).unwrap();

    // This should complete quickly with O(N log N) algorithm
    let overlaps = gap_validation::find_overlaps(&cal, start, end).unwrap();

    // 100 events % 28 days = ~3-4 events per day at same time = overlaps expected
    assert!(overlaps.len() > 0, "Should detect overlaps on same-day events");
}

#[test]
fn test_zero_duration_events_no_false_overlaps() {
    // EDGE CASE: Zero-duration events (start == end) should not cause false overlaps.
    // The builder intentionally rejects zero-duration events, but imported/manual data
    // can still contain them, so find_overlaps must handle them defensively.
    let cal = Calendar::from_json(
        r#"
        {
            "name": "Zero Duration Test",
            "events": [
                {
                    "title": "Zero Duration Event",
                    "start_time": "2025-06-15T09:00:00+00:00",
                    "end_time": "2025-06-15T09:00:00+00:00",
                    "timezone": "UTC",
                    "status": "Confirmed",
                    "attendees": [],
                    "description": null,
                    "location": null,
                    "uid": null
                },
                {
                    "title": "Event A",
                    "start_time": "2025-06-15T10:00:00+00:00",
                    "end_time": "2025-06-15T11:00:00+00:00",
                    "timezone": "UTC",
                    "status": "Confirmed",
                    "attendees": [],
                    "description": null,
                    "location": null,
                    "uid": null
                },
                {
                    "title": "Event B",
                    "start_time": "2025-06-15T12:00:00+00:00",
                    "end_time": "2025-06-15T13:00:00+00:00",
                    "timezone": "UTC",
                    "status": "Confirmed",
                    "attendees": [],
                    "description": null,
                    "location": null,
                    "uid": null
                }
            ],
            "timezone": "UTC"
        }
        "#,
    )
    .unwrap();

    let tz = timezone::parse_timezone("UTC").unwrap();

    let start = timezone::parse_datetime_with_tz("2025-06-15 00:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-06-15 23:59:59", tz).unwrap();

    let overlaps = gap_validation::find_overlaps(&cal, start, end).unwrap();

    // Zero-duration imported events should be ignored, leaving no false overlaps.
    assert_eq!(overlaps.len(), 0, "Zero-duration events should not produce false overlaps");
}

#[test]
fn test_density_with_overlapping_events() {
    // CRITICAL: This test catches the double-counting bug where overlapping
    // events inflate busy_duration beyond total_duration, making free_duration negative.
    let mut cal = Calendar::new("Overlapping Density");

    // Event A: 09:00 - 11:00 (2h)
    cal.add_event(
        Event::builder()
            .title("Event A")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_hours(2)
            .build()
            .unwrap(),
    );

    // Event B: 10:00 - 12:00 (2h, overlaps A by 1 hour)
    cal.add_event(
        Event::builder()
            .title("Event B")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(2)
            .build()
            .unwrap(),
    );

    // Event C: 14:00 - 16:00 (2h, separate)
    cal.add_event(
        Event::builder()
            .title("Event C")
            .start("2025-11-01 14:00:00", "UTC")
            .duration_hours(2)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

    let density = gap_validation::calculate_density(&cal, start, end).unwrap();

    // Merged busy: 09:00-12:00 (3h) + 14:00-16:00 (2h) = 5h wall-clock busy
    // Total: 10h, so 50% occupancy
    let busy_secs = density.busy_duration.num_seconds();
    let free_secs = density.free_duration.num_seconds();
    let total_secs = density.total_duration.num_seconds();

    assert_eq!(
        busy_secs + free_secs,
        total_secs,
        "busy ({}) + free ({}) must equal total ({})",
        busy_secs,
        free_secs,
        total_secs
    );
    assert!(free_secs >= 0, "free_duration must never be negative, got {}s", free_secs);
    assert!(
        density.occupancy_percentage <= 100.0,
        "occupancy must not exceed 100%, got {:.2}%",
        density.occupancy_percentage
    );
    assert!(
        (density.occupancy_percentage - 50.0).abs() < 1.0,
        "expected ~50%, got {:.2}%",
        density.occupancy_percentage
    );
    assert_eq!(density.overlap_count, 1, "should detect the overlap");
}

#[test]
fn test_density_fully_contained_event_not_double_counted() {
    // Event B is fully inside Event A — should not add any extra busy time
    let mut cal = Calendar::new("Contained");

    // Event A: 09:00 - 17:00 (8h)
    cal.add_event(
        Event::builder()
            .title("All Day Block")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_hours(8)
            .build()
            .unwrap(),
    );

    // Event B: 10:00 - 11:00 (1h, fully inside A)
    cal.add_event(
        Event::builder()
            .title("Nested Meeting")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 18:00:00", tz).unwrap();

    let density = gap_validation::calculate_density(&cal, start, end).unwrap();

    // Busy = 8h (just Event A, B is fully contained), Total = 10h -> 80%
    assert_eq!(
        density.busy_duration.num_hours(),
        8,
        "Fully contained event should not add extra busy time"
    );
    assert!(density.free_duration.num_seconds() >= 0, "free_duration must never be negative");
}

#[test]
fn test_gap_validation_invalid_time_range() {
    let cal = Calendar::new("Invalid Range");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 12:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();

    // start > end is an invalid range that must be explicitly rejected with an error
    let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(0));
    assert!(gaps.is_err(), "Should return error for invalid range");

    let overlaps = gap_validation::find_overlaps(&cal, start, end);
    assert!(overlaps.is_err(), "Should return error for invalid range");

    let density = gap_validation::calculate_density(&cal, start, end);
    assert!(density.is_err(), "Should return error for invalid range");
}

#[test]
fn test_calculate_density_zero_range() {
    let cal = Calendar::new("Zero Range");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();
    let end = start;

    // start == end is a zero-duration range that must be explicitly rejected
    let density = gap_validation::calculate_density(&cal, start, end);
    assert!(density.is_err(), "Zero range should be rejected by validation");
}

#[test]
fn test_suggest_alternatives_impossible_duration() {
    let cal = Calendar::new("Impossible Alt");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let req = timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();

    // Zero duration must fail
    let zero_dur =
        gap_validation::suggest_alternatives(&cal, req, Duration::minutes(0), Duration::hours(1));
    assert!(zero_dur.is_err());

    // Requesting a 10-hour duration in a 1-hour search window
    let alternatives =
        gap_validation::suggest_alternatives(&cal, req, Duration::hours(10), Duration::hours(1))
            .unwrap();
    assert_eq!(alternatives.len(), 0, "Should not return alternatives if duration exceeds window");
}

#[test]
fn test_suggest_alternatives_empty_calendar() {
    let cal = Calendar::new("Empty Cal");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let req = timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();

    let alternatives =
        gap_validation::suggest_alternatives(&cal, req, Duration::hours(1), Duration::hours(2))
            .unwrap();
    assert!(alternatives.len() >= 4, "Should suggest multiple slots in empty calendar");
}

#[test]
fn test_is_slot_available_invalid_time_range() {
    let cal = Calendar::new("Invalid Slot Range");
    let tz = timezone::parse_timezone("UTC").unwrap();
    // slot_start > slot_end is an invalid slot and must be explicitly rejected
    let slot_start = timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();
    let slot_end = timezone::parse_datetime_with_tz("2025-11-01 09:00:00", tz).unwrap();

    let available = gap_validation::is_slot_available(&cal, slot_start, slot_end);
    assert!(available.is_err(), "Invalid slot is gracefully rejected");
}

#[test]
fn test_find_gaps_negative_min_duration() {
    let cal = Calendar::new("Test");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 12:00:00", tz).unwrap();
    let result = gap_validation::find_gaps(&cal, start, end, Duration::minutes(-5));
    assert!(result.is_err(), "Negative gap duration should return an error");
}

#[test]
fn test_suggest_alternatives_invalid_search_window() {
    let cal = Calendar::new("Test");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let req = timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap();
    let result =
        gap_validation::suggest_alternatives(&cal, req, Duration::hours(1), Duration::minutes(-1));
    assert!(result.is_err(), "Negative search window should return an error");
}

#[test]
fn test_gap_validation_slot_availability_uses_event_duration() {
    let mut calendar = Calendar::new("Long event");
    calendar.add_event(
        Event::builder()
            .title("Overnight deploy")
            .start("2025-11-01 09:00:00", "UTC")
            .end("2025-11-03 09:00:00")
            .build()
            .unwrap(),
    );

    let slot_start = parse("2025-11-03 08:30:00", "UTC");
    let slot_end = parse("2025-11-03 09:30:00", "UTC");

    assert!(!gap_validation::is_slot_available(&calendar, slot_start, slot_end).unwrap());
}

#[test]
fn test_gap_validation_helpers_and_zero_duration_events() {
    let gap = TimeGap::new(
        parse("2025-11-01 09:00:00", "UTC"),
        parse("2025-11-01 12:00:00", "UTC"),
        Some("Before".to_string()),
        Some("After".to_string()),
    );
    assert_eq!(gap.duration_hours(), 3);
    assert!(gap.is_at_least(Duration::hours(2)));

    let overlap = EventOverlap::new(
        parse("2025-11-01 10:00:00", "UTC"),
        parse("2025-11-01 11:00:00", "UTC"),
        vec!["A".to_string(), "B".to_string()],
    );
    assert_eq!(overlap.event_count(), 2);

    let density = ScheduleDensity {
        total_duration: Duration::hours(4),
        busy_duration: Duration::hours(1),
        free_duration: Duration::hours(3),
        occupancy_percentage: 25.0,
        event_count: 1,
        gap_count: 2,
        overlap_count: 1,
    };
    assert!(density.is_light());
    assert!(density.has_conflicts());

    let zero_duration = json!({
        "name": "Zero Duration",
        "events": [{
            "title": "Marker",
            "start_time": "2025-11-01T10:00:00+00:00",
            "end_time": "2025-11-01T10:00:00+00:00",
            "timezone": "UTC"
        }]
    });
    let calendar = Calendar::from_json(&zero_duration.to_string()).unwrap();
    let density = gap_validation::calculate_density(
        &calendar,
        parse("2025-11-01 09:00:00", "UTC"),
        parse("2025-11-01 12:00:00", "UTC"),
    )
    .unwrap();
    assert_eq!(density.busy_duration, Duration::zero());
    assert_eq!(density.occupancy_percentage, 0.0);
}

#[test]
fn test_gap_validation_cancelled_slots_and_alternative_suggestions() {
    let mut blocked = Calendar::new("Cancelled");
    blocked.add_event(
        Event::builder()
            .title("Cancelled block")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(2)
            .status(EventStatus::Cancelled)
            .build()
            .unwrap(),
    );
    assert!(gap_validation::is_slot_available(
        &blocked,
        parse("2025-11-01 10:30:00", "UTC"),
        parse("2025-11-01 11:30:00", "UTC"),
    )
    .unwrap());

    let suggestions = gap_validation::suggest_alternatives(
        &Calendar::new("Open"),
        parse("2025-11-01 12:00:00", "UTC"),
        Duration::hours(1),
        Duration::hours(3),
    )
    .unwrap();
    assert_eq!(
        suggestions,
        vec![
            parse("2025-11-01 09:00:00", "UTC"),
            parse("2025-11-01 10:00:00", "UTC"),
            parse("2025-11-01 11:00:00", "UTC"),
            parse("2025-11-01 12:00:00", "UTC"),
            parse("2025-11-01 13:00:00", "UTC"),
            parse("2025-11-01 14:00:00", "UTC"),
        ]
    );
}
