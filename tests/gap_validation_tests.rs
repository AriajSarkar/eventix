//! Integration tests for gap validation functionality
//!
//! These tests validate the unique gap detection and schedule analysis
//! features that set eventix apart from other calendar crates.

use chrono::Duration;
use eventix::{gap_validation, timezone, Calendar, Event, Recurrence};

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
        assert!(
            gap.duration_minutes() >= 30,
            "Each gap should be at least 30 minutes"
        );
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
        assert!(
            overlap.event_count() >= 2,
            "Each overlap should involve at least 2 events"
        );
        assert!(
            overlap.duration_minutes() > 0,
            "Overlap should have positive duration"
        );
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

    assert!(
        light_density.is_light(),
        "Light schedule should be detected"
    );
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
        assert!(
            slot.duration >= Duration::hours(2),
            "Each slot should fit 2-hour meeting"
        );
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
    assert!(
        gap.duration_hours() >= 6,
        "Longest gap should be at least 6 hours"
    );
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
    assert!(
        density.free_duration > density.busy_duration,
        "Should have more free time"
    );
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
