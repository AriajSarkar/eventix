//! Integration tests for timezone-aware ICS export/import

use eventix::{timezone, Calendar, Event, Recurrence};

#[test]
fn test_timezone_aware_ics_export() {
    // Create events in different timezones
    let mut cal = Calendar::new("Timezone Test Calendar");

    let ny_event = Event::builder()
        .title("New York Meeting")
        .start("2025-10-27 10:00:00", "America/New_York")
        .duration_hours(1)
        .build()
        .unwrap();

    cal.add_event(ny_event);

    let ics = cal.to_ics_string().unwrap();

    // Verify TZID parameter is included for non-UTC timezone
    assert!(ics.contains("TZID=America/New_York"));
    assert!(ics.contains("DTSTART;TZID=America/New_York:20251027T100000"));
    assert!(ics.contains("DTEND;TZID=America/New_York:20251027T110000"));
}

#[test]
fn test_utc_timezone_uses_z_suffix() {
    let mut cal = Calendar::new("UTC Test");

    let utc_event = Event::builder()
        .title("UTC Event")
        .start("2025-10-27 15:00:00", "UTC")
        .duration_hours(2)
        .build()
        .unwrap();

    cal.add_event(utc_event);

    let ics = cal.to_ics_string().unwrap();

    // UTC events should use Z suffix, not TZID
    assert!(ics.contains("DTSTART:20251027T150000Z"));
    assert!(ics.contains("DTEND:20251027T170000Z"));
    assert!(!ics.contains("TZID=UTC"));
}

#[test]
fn test_multiple_timezones_in_one_calendar() {
    let mut cal = Calendar::new("Multi-TZ Calendar");

    // Add events in different timezones
    let timezones = vec![
        "America/New_York",
        "Asia/Kolkata",
        "Europe/London",
        "Asia/Tokyo",
        "America/Los_Angeles",
    ];

    for (i, tz) in timezones.iter().enumerate() {
        let event = Event::builder()
            .title(&format!("Event {}", i + 1))
            .start("2025-10-27 10:00:00", tz)
            .duration_hours(1)
            .build()
            .unwrap();

        cal.add_event(event);
    }

    let ics = cal.to_ics_string().unwrap();

    // Verify all timezones are preserved
    for tz in timezones {
        assert!(ics.contains(&format!("TZID={}", tz)));
    }
}

#[test]
fn test_recurring_event_with_timezone() {
    let mut cal = Calendar::new("Recurring TZ Test");

    let recurring = Event::builder()
        .title("Weekly Meeting")
        .start("2025-10-28 09:00:00", "America/Los_Angeles")
        .duration_minutes(30)
        .recurrence(Recurrence::weekly().count(4))
        .build()
        .unwrap();

    cal.add_event(recurring);

    let ics = cal.to_ics_string().unwrap();

    // Check timezone is preserved in recurring event
    assert!(ics.contains("TZID=America/Los_Angeles"));
    assert!(ics.contains("DTSTART;TZID=America/Los_Angeles:20251028T090000"));
    assert!(ics.contains("RRULE:"));
}

#[test]
fn test_exception_dates_with_timezone() {
    let tz = timezone::parse_timezone("Asia/Tokyo").unwrap();
    let exdate = timezone::parse_datetime_with_tz("2025-11-05 10:00:00", tz).unwrap();

    let mut cal = Calendar::new("Exception Test");

    let event = Event::builder()
        .title("Daily Standup")
        .start("2025-10-27 10:00:00", "Asia/Tokyo")
        .duration_minutes(15)
        .recurrence(Recurrence::daily().count(10))
        .exception_date(exdate)
        .build()
        .unwrap();

    cal.add_event(event);

    let ics = cal.to_ics_string().unwrap();

    // Verify exception date includes TZID
    assert!(ics.contains("EXDATE;TZID=Asia/Tokyo:20251105T100000"));
}

#[test]
fn test_mixed_utc_and_local_timezones() {
    let mut cal = Calendar::new("Mixed TZ Test");

    // UTC event
    let utc_event = Event::builder()
        .title("UTC Event")
        .start("2025-10-27 12:00:00", "UTC")
        .duration_hours(1)
        .build()
        .unwrap();

    // Local timezone event
    let local_event = Event::builder()
        .title("Local Event")
        .start("2025-10-27 12:00:00", "Europe/Paris")
        .duration_hours(1)
        .build()
        .unwrap();

    cal.add_event(utc_event);
    cal.add_event(local_event);

    let ics = cal.to_ics_string().unwrap();

    // UTC should have Z suffix
    assert!(ics.contains("DTSTART:20251027T120000Z"));
    // Local should have TZID
    assert!(ics.contains("TZID=Europe/Paris"));
    assert!(ics.contains("DTSTART;TZID=Europe/Paris:20251027T120000"));
}

#[test]
fn test_ics_round_trip_preserves_timezone() {
    let mut cal = Calendar::new("Round Trip Test");

    let event = Event::builder()
        .title("Test Event")
        .description("Testing round-trip")
        .start("2025-10-27 14:30:00", "America/Chicago")
        .duration_minutes(90)
        .location("Chicago Office")
        .build()
        .unwrap();

    cal.add_event(event);

    // Export
    let ics_content = cal.to_ics_string().unwrap();

    // Verify export contains timezone info
    assert!(ics_content.contains("TZID=America/Chicago"));
    assert!(ics_content.contains("DTSTART;TZID=America/Chicago:20251027T143000"));
    assert!(ics_content.contains("DTEND;TZID=America/Chicago:20251027T160000"));

    // Import back
    let imported_cal = Calendar::from_ics_string(&ics_content).unwrap();
    assert_eq!(imported_cal.event_count(), 1);

    // Verify event details are preserved
    let imported_event = &imported_cal.get_events()[0];
    assert_eq!(imported_event.title, "Test Event");
    assert_eq!(
        imported_event.description,
        Some("Testing round-trip".to_string())
    );
    assert_eq!(imported_event.location, Some("Chicago Office".to_string()));
}

#[test]
fn test_dst_boundary_event() {
    // Create event during DST transition period
    let mut cal = Calendar::new("DST Test");

    // Use a safe time not during the 2 AM transition hour
    let event = Event::builder()
        .title("DST Boundary Event")
        .start("2025-03-09 03:30:00", "America/New_York")
        .duration_hours(1)
        .build()
        .unwrap();

    cal.add_event(event);

    let ics = cal.to_ics_string().unwrap();

    // Should preserve timezone information
    assert!(ics.contains("TZID=America/New_York"));
}

#[test]
fn test_all_day_event_utc() {
    let mut cal = Calendar::new("All Day Test");

    // All-day events typically use UTC or DATE format
    let event = Event::builder()
        .title("All Day Conference")
        .start("2025-10-27 00:00:00", "UTC")
        .end("2025-10-28 00:00:00")
        .build()
        .unwrap();

    cal.add_event(event);

    let ics = cal.to_ics_string().unwrap();

    // UTC format for all-day events
    assert!(ics.contains("DTSTART:20251027T000000Z"));
    assert!(ics.contains("DTEND:20251028T000000Z"));
}
