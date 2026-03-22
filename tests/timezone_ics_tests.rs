//! Integration tests for timezone-aware ICS export/import

#![allow(clippy::unwrap_used)]

mod common;

use common::parse;
use eventix::{timezone, Calendar, Event, EventixError, Recurrence};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_ics_path(label: &str) -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = PathBuf::from("target").join("coverage-tests");
    fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{label}-{stamp}.ics"))
}

fn sample_event(title: &str, datetime: &str, tz_name: &str) -> Event {
    Event::builder()
        .title(title)
        .start(datetime, tz_name)
        .duration_hours(1)
        .build()
        .unwrap()
}

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
            .title(format!("Event {}", i + 1))
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
    assert_eq!(imported_event.description, Some("Testing round-trip".to_string()));
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

#[test]
fn test_ics_import_missing_property() {
    let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:Test Missing\nEND:VEVENT\nEND:VCALENDAR";
    let cal = eventix::Calendar::from_ics_string(ics).unwrap();
    assert_eq!(cal.event_count(), 0, "Should skip event when properties like DTSTART are missing");
}

#[test]
fn test_ics_import_invalid_datetime_format() {
    let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:Test Format\nDTSTART:INVALID_DATE\nDTEND:INVALID_DATE\nEND:VEVENT\nEND:VCALENDAR";
    let cal = eventix::Calendar::from_ics_string(ics).unwrap();
    assert_eq!(cal.event_count(), 0, "Should skip event for unparseable dates");
}

#[test]
fn test_timezone_dst_gap_and_is_dst_detection() {
    let ny = timezone::parse_timezone("America/New_York").unwrap();
    let err = timezone::parse_datetime_with_tz("2025-03-09 02:30:00", ny).unwrap_err();
    assert!(
        matches!(err, EventixError::DateTimeParse(message) if message.contains("Invalid datetime"))
    );

    let summer = parse("2025-07-01 10:00:00", "America/New_York");
    let winter = parse("2025-12-01 10:00:00", "America/New_York");
    assert!(timezone::is_dst(&summer));
    assert!(!timezone::is_dst(&winter));
}

#[test]
fn test_ics_file_round_trip_preserves_metadata_and_exported_fields() {
    let path = temp_ics_path("round-trip");
    let mut calendar = Calendar::new("Coverage Export").description("Calendar level description");
    let event = Event::builder()
        .title("Field Coverage")
        .description("Event description")
        .start("2025-11-03 09:00:00", "America/New_York")
        .duration_hours(1)
        .location("Main Room")
        .uid("coverage-uid")
        .attendees(vec!["alice@example.com".to_string(), "bob@example.com".to_string()])
        .build()
        .unwrap();
    calendar.add_event(event);

    calendar.export_to_ics(&path).unwrap();
    let ics = fs::read_to_string(&path).unwrap();

    assert!(ics.contains("NAME:Coverage Export"));
    assert!(ics.contains("Calendar level description"));
    assert!(ics.contains("UID:coverage-uid"));
    assert!(ics.contains("ATTENDEE:mailto:alice@example.com"));
    assert!(ics.contains("ATTENDEE:mailto:bob@example.com"));

    let imported = Calendar::import_from_ics(&path).unwrap();
    assert_eq!(imported.name, "Coverage Export");
    assert_eq!(imported.description.as_deref(), Some("Calendar level description"));
    assert_eq!(imported.event_count(), 1);

    let imported_event = &imported.get_events()[0];
    assert_eq!(imported_event.title, "Field Coverage");
    assert_eq!(imported_event.description.as_deref(), Some("Event description"));
    assert_eq!(imported_event.location.as_deref(), Some("Main Room"));
    assert_eq!(imported_event.uid.as_deref(), Some("coverage-uid"));
}

#[test]
fn test_ics_file_errors_are_wrapped() {
    let mut calendar = Calendar::new("Broken export");
    calendar.add_event(sample_event("Meeting", "2025-11-01 10:00:00", "UTC"));

    let export_path = PathBuf::from("target")
        .join("coverage-tests")
        .join("missing-parent")
        .join("calendar.ics");
    let err = calendar.export_to_ics(&export_path).unwrap_err();
    assert!(
        matches!(err, EventixError::IcsError(message) if message.contains("Failed to write ICS file"))
    );

    let import_path = temp_ics_path("missing-file");
    let err = Calendar::import_from_ics(&import_path).unwrap_err();
    assert!(
        matches!(err, EventixError::IcsError(message) if message.contains("Failed to read ICS file"))
    );
}

#[test]
fn test_ics_import_skips_invalid_events_and_handles_exdate_fallbacks() {
    let ics = "\
BEGIN:VCALENDAR
NAME:Coverage Import
DESCRIPTION:Imported metadata
BEGIN:VEVENT
SUMMARY:Invalid TZID EXDATE
DESCRIPTION:Keeps parsing malformed RRULE segments
DTSTART;TZID=America/New_York:20251103T090000
DTEND;TZID=America/New_York:20251103T100000
RRULE:FREQ=DAILY;COUNT=3;BROKEN
EXDATE;TZID=Not/AZone:20251104T090000
END:VEVENT
BEGIN:VEVENT
SUMMARY:Floating EXDATE
DESCRIPTION:Floating exception date
LOCATION:Lab
UID:floating-uid
DTSTART;TZID=America/New_York:20251105T090000
DTEND;TZID=America/New_York:20251105T100000
RRULE:FREQ=DAILY;COUNT=3
EXDATE:20251106T090000
END:VEVENT
BEGIN:VEVENT
SUMMARY:Broken EXDATE
DTSTART:20251107T090000Z
DTEND:20251107T100000Z
RRULE:FREQ=DAILY;COUNT=2
EXDATE;TZID=UTC:not-a-date
END:VEVENT
BEGIN:VEVENT
SUMMARY:Missing End
DTSTART:20251108T090000Z
END:VEVENT
END:VCALENDAR";

    let calendar = Calendar::from_ics_string(ics).unwrap();
    assert_eq!(calendar.name, "Coverage Import");
    assert_eq!(calendar.description.as_deref(), Some("Imported metadata"));
    assert_eq!(calendar.event_count(), 2);

    let invalid_tzid = calendar
        .get_events()
        .iter()
        .find(|event| event.title == "Invalid TZID EXDATE")
        .unwrap();
    assert_eq!(invalid_tzid.exdates, vec![parse("2025-11-04 09:00:00", "America/New_York")]);

    let floating = calendar
        .get_events()
        .iter()
        .find(|event| event.title == "Floating EXDATE")
        .unwrap();
    assert_eq!(floating.uid.as_deref(), Some("floating-uid"));
    assert_eq!(floating.location.as_deref(), Some("Lab"));
    assert_eq!(floating.description.as_deref(), Some("Floating exception date"));
    assert_eq!(floating.exdates, vec![parse("2025-11-06 09:00:00", "America/New_York")]);
}
