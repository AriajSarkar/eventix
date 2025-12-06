//! Integration tests for Booking State Machine

use eventix::{gap_validation, timezone, Calendar, Duration, Event, EventStatus};

#[test]
fn test_event_status_lifecycle() {
    let mut event = Event::builder()
        .title("Meeting")
        .start("2025-11-01 10:00:00", "UTC")
        .duration_hours(1)
        .build()
        .unwrap();

    // Default status should be Confirmed
    assert_eq!(event.status, EventStatus::Confirmed);
    assert!(event.is_active());

    // Cancel
    event.cancel();
    assert_eq!(event.status, EventStatus::Cancelled);
    assert!(!event.is_active());

    // Confirm
    event.confirm();
    assert_eq!(event.status, EventStatus::Confirmed);
    assert!(event.is_active());

    // Tentative
    event.tentative();
    assert_eq!(event.status, EventStatus::Tentative);
    assert!(event.is_active());
}

#[test]
fn test_rescheduling_resets_cancelled_status() {
    let mut event = Event::builder()
        .title("Meeting")
        .start("2025-11-01 10:00:00", "UTC")
        .duration_hours(1)
        .build()
        .unwrap();

    event.cancel();
    assert_eq!(event.status, EventStatus::Cancelled);

    let tz = timezone::parse_timezone("UTC").unwrap();
    let new_start = timezone::parse_datetime_with_tz("2025-11-02 10:00:00", tz).unwrap();
    let new_end = timezone::parse_datetime_with_tz("2025-11-02 11:00:00", tz).unwrap();

    event.reschedule(new_start, new_end).unwrap();

    assert_eq!(event.start_time, new_start);
    assert_eq!(event.end_time, new_end);
    assert_eq!(event.status, EventStatus::Confirmed); // Should be reset to Confirmed
}

#[test]
fn test_rescheduling_validation() {
    let mut event = Event::builder()
        .title("Meeting")
        .start("2025-11-01 10:00:00", "UTC")
        .duration_hours(1)
        .build()
        .unwrap();

    let tz = timezone::parse_timezone("UTC").unwrap();
    let new_start = timezone::parse_datetime_with_tz("2025-11-02 10:00:00", tz).unwrap();
    let invalid_end = timezone::parse_datetime_with_tz("2025-11-02 09:00:00", tz).unwrap();

    // Should fail because end is before start
    let result = event.reschedule(new_start, invalid_end);
    assert!(result.is_err());
}

#[test]
fn test_gap_validation_ignores_cancelled_events() {
    let mut cal = Calendar::new("Booking Calendar");

    // Add a confirmed event 9-10
    cal.add_event(
        Event::builder()
            .title("Confirmed")
            .start("2025-11-01 09:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    // Add a cancelled event 10-11
    cal.add_event(
        Event::builder()
            .title("Cancelled")
            .start("2025-11-01 10:00:00", "UTC")
            .duration_hours(1)
            .status(EventStatus::Cancelled)
            .build()
            .unwrap(),
    );

    // Add a confirmed event 11-12
    cal.add_event(
        Event::builder()
            .title("Confirmed 2")
            .start("2025-11-01 11:00:00", "UTC")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-01 08:00:00", tz).unwrap();
    let end = timezone::parse_datetime_with_tz("2025-11-01 13:00:00", tz).unwrap();

    // The gap should be from 10:00 to 11:00 because the middle event is cancelled
    let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(30)).unwrap();

    // Should have exactly 3 gaps: 08-09, 10-11, 12-13
    assert_eq!(gaps.len(), 3, "Expected 3 gaps in the schedule");

    // Gaps:
    // 08:00 - 09:00 (1h)
    // 10:00 - 11:00 (1h) - This exists ONLY because the event is cancelled
    // 12:00 - 13:00 (1h)

    let cancelled_slot_gap = gaps.iter().find(|gap| {
        gap.start.format("%H:%M:%S").to_string() == "10:00:00"
            && gap.end.format("%H:%M:%S").to_string() == "11:00:00"
    });

    assert!(cancelled_slot_gap.is_some(), "Should find a gap where the cancelled event is");
}
