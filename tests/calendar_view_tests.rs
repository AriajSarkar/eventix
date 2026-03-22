#![allow(clippy::unwrap_used)]

mod common;

use common::parse;
use eventix::{timezone, Calendar, Duration, Event, EventStatus, EventixError, Recurrence};
use serde_json::json;

fn collect_ok<T>(iter: impl Iterator<Item = eventix::Result<T>>) -> Vec<T> {
    iter.collect::<eventix::Result<Vec<_>>>().unwrap()
}

fn next_ok<T>(mut iter: impl Iterator<Item = eventix::Result<T>>) -> T {
    iter.next().unwrap().unwrap()
}

fn build_calendar() -> Calendar {
    let mut calendar = Calendar::new("Integration Views");

    calendar.add_event(
        Event::builder()
            .title("Planning")
            .start("2025-11-03 09:00:00", "America/New_York")
            .duration_hours(1)
            .build()
            .unwrap(),
    );

    calendar.add_event(
        Event::builder()
            .title("Standup")
            .start("2025-11-04 10:00:00", "America/New_York")
            .duration_minutes(15)
            .recurrence(Recurrence::daily().count(5))
            .build()
            .unwrap(),
    );

    calendar.add_event(
        Event::builder()
            .title("Overnight Deploy")
            .start("2025-11-05 23:30:00", "America/New_York")
            .duration(Duration::hours(2))
            .build()
            .unwrap(),
    );

    calendar.add_event(
        Event::builder()
            .title("Cancelled")
            .start("2025-11-06 11:00:00", "America/New_York")
            .duration_minutes(30)
            .status(EventStatus::Cancelled)
            .build()
            .unwrap(),
    );

    calendar
}

#[test]
fn test_views_match_events_on_date_for_active_events() {
    let calendar = build_calendar();
    let tz = timezone::parse_timezone("America/New_York").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-03 00:00:00", tz).unwrap();

    for day in collect_ok(calendar.days(start).take(7)) {
        let query =
            timezone::parse_datetime_with_tz(&format!("{} 00:00:00", day.date()), tz).unwrap();
        let expected: Vec<_> = calendar
            .events_on_date(query)
            .unwrap()
            .into_iter()
            .filter(|occurrence| occurrence.event.is_active())
            .map(|occurrence| {
                (
                    occurrence.title().to_string(),
                    occurrence.occurrence_time,
                    occurrence.end_time(),
                    occurrence.event.status,
                )
            })
            .collect();
        let actual: Vec<_> = day
            .events()
            .iter()
            .map(|occurrence| {
                (
                    occurrence.title().to_string(),
                    occurrence.occurrence_time,
                    occurrence.end_time(),
                    occurrence.status,
                )
            })
            .collect();

        assert_eq!(actual, expected, "day {}", day.date());
    }
}

#[test]
fn test_views_dst_boundary() {
    let mut calendar = Calendar::new("DST");
    calendar.add_event(
        Event::builder()
            .title("Night Shift")
            .start("2025-11-02 00:30:00", "America/New_York")
            .duration_minutes(30)
            .recurrence(Recurrence::hourly().count(4))
            .build()
            .unwrap(),
    );

    let tz = timezone::parse_timezone("America/New_York").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-02 00:00:00", tz).unwrap();
    let day = next_ok(calendar.days(start));

    assert_eq!(day.event_count(), 4);
    assert_eq!(day.end() - day.start(), Duration::hours(25));
    assert_eq!(day.end_inclusive().date_naive(), day.date());
}

#[test]
fn test_views_cancelled_events_excluded() {
    let calendar = build_calendar();
    let tz = timezone::parse_timezone("America/New_York").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-06 00:00:00", tz).unwrap();
    let day = next_ok(calendar.days(start));

    assert!(day
        .events()
        .iter()
        .all(|occurrence| occurrence.status != EventStatus::Cancelled));
    assert_eq!(day.event_count(), 2);
}

#[test]
fn test_weeks_align_to_monday() {
    let calendar = build_calendar();
    let tz = timezone::parse_timezone("America/New_York").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-06 18:00:00", tz).unwrap();
    let week = next_ok(calendar.weeks(start));

    assert_eq!(week.start_date().to_string(), "2025-11-03");
    assert_eq!(week.end_date().to_string(), "2025-11-09");
    assert_eq!(week.event_count(), 8);
}

#[test]
fn test_backward_weeks_are_contiguous_monday_to_sunday_windows() {
    let calendar = build_calendar();
    let tz = timezone::parse_timezone("America/New_York").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-11-12 18:00:00", tz).unwrap();
    let weeks = collect_ok(calendar.weeks_back(start).take(2));

    assert_eq!(
        weeks[0].days().iter().map(|day| day.date().to_string()).collect::<Vec<_>>(),
        vec![
            "2025-11-10".to_string(),
            "2025-11-11".to_string(),
            "2025-11-12".to_string(),
            "2025-11-13".to_string(),
            "2025-11-14".to_string(),
            "2025-11-15".to_string(),
            "2025-11-16".to_string(),
        ]
    );
    assert_eq!(weeks[1].start_date().to_string(), "2025-11-03");
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
fn test_calendar_mutators_and_occurrence_helpers() {
    let tz = timezone::parse_timezone("America/New_York").unwrap();
    let mut calendar =
        Calendar::new("Coverage Calendar").description("Calendar metadata").timezone(tz);

    let planning = Event::builder()
        .title("Planning")
        .description("Detailed planning session")
        .start("2025-11-01 09:00:00", "America/New_York")
        .duration_hours(1)
        .build()
        .unwrap();
    let review = sample_event("Review", "2025-11-01 11:00:00", "America/New_York");

    calendar.add_events(vec![planning, review]);

    assert_eq!(calendar.timezone, Some(tz));
    assert_eq!(calendar.event_count(), 2);

    let removed = calendar.remove_event(1).unwrap();
    assert_eq!(removed.title, "Review");
    assert!(calendar.remove_event(99).is_none());

    let day = parse("2025-11-01 00:00:00", "America/New_York");
    let occurrences = calendar.events_on_date(day).unwrap();
    assert_eq!(occurrences.len(), 1);
    assert_eq!(occurrences[0].title(), "Planning");
    assert_eq!(occurrences[0].description(), Some("Detailed planning session"));
    assert_eq!(occurrences[0].end_time(), parse("2025-11-01 10:00:00", "America/New_York"));

    calendar.clear_events();
    assert_eq!(calendar.event_count(), 0);
}

#[test]
fn test_calendar_from_json_reports_missing_event_fields() {
    let base = json!({
        "name": "Broken Calendar",
        "events": [{
            "title": "Meeting",
            "start_time": "2025-11-01T10:00:00+00:00",
            "end_time": "2025-11-01T11:00:00+00:00",
            "timezone": "UTC"
        }]
    });

    for (field, needle) in [
        ("title", "Event missing 'title'"),
        ("start_time", "Event missing 'start_time'"),
        ("end_time", "Event missing 'end_time'"),
        ("timezone", "Event missing 'timezone'"),
    ] {
        let mut payload = base.clone();
        payload["events"].as_array_mut().unwrap()[0]
            .as_object_mut()
            .unwrap()
            .remove(field);

        let err = Calendar::from_json(&payload.to_string()).unwrap_err();
        assert!(matches!(err, EventixError::Other(message) if message.contains(needle)));
    }
}

#[test]
fn test_calendar_from_json_rejects_invalid_exdates_and_status() {
    let mut exdate_payload = json!({
        "name": "Broken Calendar",
        "events": [{
            "title": "Meeting",
            "start_time": "2025-11-01T10:00:00+00:00",
            "end_time": "2025-11-01T11:00:00+00:00",
            "timezone": "UTC",
            "exdates": [123]
        }]
    });
    let err = Calendar::from_json(&exdate_payload.to_string()).unwrap_err();
    assert!(
        matches!(err, EventixError::Other(message) if message.contains("exdates[0]: expected string"))
    );

    exdate_payload["events"][0]["exdates"] = json!(["2025-11-02T10:00:00+00:00"]);
    exdate_payload["events"][0]["status"] = json!("not-a-real-status");
    let err = Calendar::from_json(&exdate_payload.to_string()).unwrap_err();
    assert!(
        matches!(err, EventixError::Other(message) if message.contains("Invalid event status"))
    );
}
