#![allow(clippy::unwrap_used)]

mod common;

use chrono::{Duration, TimeZone, Weekday};
use common::parse;
use eventix::recurrence::RecurrenceFilter;
use eventix::timezone;
use eventix::{gap_validation, Calendar, Event, EventBuilder, EventStatus, Recurrence};
use proptest::prelude::*;

proptest! {
    // START: Recurrence Tests
    #[test]
    fn test_recurrence_daily_count_invariant(
        count in 1u32..100,
        start_year in 2020i32..2030,
        start_month in 1u32..=12,
        start_day in 1u32..28, // Safe day range
        hour in 0u32..23,
        minute in 0u32..59
    ) {
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = tz.with_ymd_and_hms(start_year, start_month, start_day, hour, minute, 0).unwrap();

        let recurrence = Recurrence::daily().count(count);
        let occurrences = recurrence.generate_occurrences(start).unwrap();

        // Invariant: Should generate exactly 'count' occurrences
        prop_assert_eq!(occurrences.len(), count as usize);

        // Invariant: Occurrences should be strictly increasing
        for windows in occurrences.windows(2) {
            prop_assert!(windows[0] < windows[1]);
        }

        // Invariant: Daily recurrence should have 24 hours diff (ignoring DST for UTC)
        for windows in occurrences.windows(2) {
            let diff = windows[1] - windows[0];
            prop_assert_eq!(diff.num_hours(), 24);
        }
    }

    #[test]
    fn test_recurrence_weekly_interval_invariant(
        interval in 1u16..52,
        count in 1u32..50
    ) {
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = tz.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();

        let recurrence = Recurrence::weekly().interval(interval).count(count);
        let occurrences = recurrence.generate_occurrences(start).unwrap();

        // Invariant: Week difference should match interval
        for windows in occurrences.windows(2) {
            let diff = windows[1] - windows[0];
            prop_assert_eq!(diff.num_days(), 7 * interval as i64);
        }
    }
    // END: Recurrence Tests

    // START: Event Builder Tests
    #[test]
    fn test_event_builder_invariants(
        ref title in "[a-zA-Z0-9 ]+",
        duration_hours in 1i64..100,
        start_offset_hours in 0i64..1000
    ) {
        let tz = timezone::parse_timezone("UTC").unwrap();
        let base_time = tz.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let start_time = base_time + Duration::hours(start_offset_hours);

        let event_res = Event::builder()
            .title(title.clone())
            .start_datetime(start_time)
            .duration_hours(duration_hours)
            .build();

        // Invariant: Valid inputs should always produce a valid event
        prop_assert!(event_res.is_ok());
        let event = event_res.unwrap();

        // Invariant: Title match
        prop_assert_eq!(&event.title, title);

        // Invariant: Start < End
        prop_assert!(event.start_time < event.end_time);

        // Invariant: Duration match
        prop_assert_eq!(event.duration().num_hours(), duration_hours);
    }

    #[test]
    fn test_event_overlap_logic(
        start_offset in 0i64..100,
        duration in 1i64..10
    ) {
        let mut cal = Calendar::new("Prop Test");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let base = tz.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();

        let start1 = base + Duration::hours(start_offset);
        let event1 = Event::builder()
            .title("E1")
            .start_datetime(start1)
            .duration_hours(duration)
            .build().unwrap();

        cal.add_event(event1);

        // Check same slot availability
        // If we check the exact same time, it MUST NOT be available
        let is_available = eventix::gap_validation::is_slot_available(
            &cal,
            start1,
            start1 + Duration::hours(duration)
        ).unwrap();

        prop_assert!(!is_available);
    }
    // END: Event Builder Tests

    // START: Gap Validation Property Tests
    #[test]
    fn test_gaps_plus_busy_equals_total(
        num_events in 1usize..10,
        window_hours in 4i64..24
    ) {
        // INVARIANT: busy_duration + free_duration = total_duration
        let mut cal = Calendar::new("Density Test");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let base = tz.with_ymd_and_hms(2025, 6, 15, 8, 0, 0).unwrap();

        // Add random non-overlapping events
        for i in 0..num_events {
            let event = Event::builder()
                .title(format!("Event {}", i))
                .start_datetime(base + Duration::hours(i as i64 * 2))
                .duration_minutes(45)
                .build()
                .unwrap();
            cal.add_event(event);
        }

        let start = base;
        let end = base + Duration::hours(window_hours);
        let density = gap_validation::calculate_density(&cal, start, end).unwrap();

        // Core invariant: busy + free = total
        let busy_secs = density.busy_duration.num_seconds();
        let free_secs = density.free_duration.num_seconds();
        let total_secs = density.total_duration.num_seconds();

        prop_assert_eq!(
            busy_secs + free_secs,
            total_secs,
            "busy ({}) + free ({}) should equal total ({})",
            busy_secs, free_secs, total_secs
        );
    }

    #[test]
    fn test_density_percentage_bounds_non_overlapping(
        num_events in 0usize..10,
        event_duration_mins in 15i64..60
    ) {
        // INVARIANT: occupancy_percentage is always 0.0 <= x <= 100.0
        // (overlapping intervals are merged before summing busy time)
        let mut cal = Calendar::new("Percentage Test");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let base = tz.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap();

        // Space events 2 hours apart with max 60 min duration = no overlap
        for i in 0..num_events {
            let event = Event::builder()
                .title(format!("E{}", i))
                .start_datetime(base + Duration::hours(i as i64 * 2))
                .duration_minutes(event_duration_mins)
                .build()
                .unwrap();
            cal.add_event(event);
        }

        let start = base;
        let end = base + Duration::hours(24);
        let density = gap_validation::calculate_density(&cal, start, end).unwrap();

        prop_assert!(
            density.occupancy_percentage >= 0.0,
            "Occupancy cannot be negative"
        );
        prop_assert!(
            density.occupancy_percentage <= 100.0,
            "Non-overlapping events should not exceed 100% occupancy, got {:.2}%",
            density.occupancy_percentage
        );
    }

    #[test]
    fn test_gaps_are_non_overlapping(
        num_events in 2usize..8
    ) {
        // INVARIANT: Gaps returned should never overlap with each other
        let mut cal = Calendar::new("Gap Overlap Test");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let base = tz.with_ymd_and_hms(2025, 7, 1, 9, 0, 0).unwrap();

        // Create spaced events
        for i in 0..num_events {
            let event = Event::builder()
                .title(format!("Meeting {}", i))
                .start_datetime(base + Duration::hours(i as i64 * 3))
                .duration_hours(1)
                .build()
                .unwrap();
            cal.add_event(event);
        }

        let start = base - Duration::hours(1);
        let end = base + Duration::hours(num_events as i64 * 3 + 2);
        let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(0)).unwrap();

        // Verify gaps don't overlap — O(N) adjacent sweep
        // (find_gaps returns gaps in chronological order)
        for pair in gaps.windows(2) {
            prop_assert!(
                pair[0].end <= pair[1].start,
                "Gap ({} - {}) overlaps with next gap ({} - {})",
                pair[0].start, pair[0].end, pair[1].start, pair[1].end
            );
        }
    }

    #[test]
    fn test_no_overlaps_for_sequential_events(
        num_events in 2usize..20
    ) {
        // INVARIANT: Back-to-back events (A ends when B starts) should have 0 overlaps
        let mut cal = Calendar::new("Sequential Events");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let base = tz.with_ymd_and_hms(2025, 5, 1, 9, 0, 0).unwrap();

        // Create perfectly sequential (touching) events
        for i in 0..num_events {
            let event = Event::builder()
                .title(format!("Event {}", i))
                .start_datetime(base + Duration::hours(i as i64))
                .duration_hours(1)
                .build()
                .unwrap();
            cal.add_event(event);
        }

        let start = base;
        let end = base + Duration::hours(num_events as i64 + 1);
        let overlaps = gap_validation::find_overlaps(&cal, start, end).unwrap();

        // Sequential events should have ZERO overlaps
        prop_assert_eq!(
            overlaps.len(),
            0,
            "Sequential events should not have overlaps, found {}",
            overlaps.len()
        );
    }

    #[test]
    fn test_empty_calendar_has_one_big_gap(
        window_hours in 1i64..48
    ) {
        // INVARIANT: Empty calendar = one gap covering entire window
        let cal = Calendar::new("Empty");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = tz.with_ymd_and_hms(2025, 4, 1, 0, 0, 0).unwrap();
        let end = start + Duration::hours(window_hours);

        let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(0)).unwrap();

        prop_assert_eq!(gaps.len(), 1, "Empty calendar should have exactly one gap");
        prop_assert_eq!(gaps[0].start, start);
        prop_assert_eq!(gaps[0].end, end);
        prop_assert_eq!(gaps[0].duration_minutes(), window_hours * 60);
    }
    #[test]
    fn test_density_invariant_with_overlapping_events(
        num_events in 2usize..8,
        window_hours in 8i64..24
    ) {
        // INVARIANT: busy + free = total, even with overlapping events.
        // This catches the double-counting bug where overlapping intervals
        // inflated busy_duration beyond total_duration.
        let mut cal = Calendar::new("Overlap Density");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let base = tz.with_ymd_and_hms(2025, 8, 1, 8, 0, 0).unwrap();

        // Create events that WILL overlap: 2-hour events spaced 1 hour apart
        for i in 0..num_events {
            let event = Event::builder()
                .title(format!("Overlap {}", i))
                .start_datetime(base + Duration::hours(i as i64))
                .duration_hours(2)
                .build()
                .unwrap();
            cal.add_event(event);
        }

        let start = base;
        let end = base + Duration::hours(window_hours);
        let density = gap_validation::calculate_density(&cal, start, end).unwrap();

        let busy_secs = density.busy_duration.num_seconds();
        let free_secs = density.free_duration.num_seconds();
        let total_secs = density.total_duration.num_seconds();

        // Core invariant: busy + free = total
        prop_assert_eq!(
            busy_secs + free_secs,
            total_secs,
            "busy ({}) + free ({}) should equal total ({})",
            busy_secs, free_secs, total_secs
        );

        // free_duration must never be negative
        prop_assert!(
            free_secs >= 0,
            "free_duration must not be negative, got {}",
            free_secs
        );

        // occupancy must not exceed 100%
        prop_assert!(
            density.occupancy_percentage <= 100.0,
            "occupancy must not exceed 100%, got {:.2}%",
            density.occupancy_percentage
        );
    }
    // END: Gap Validation Property Tests
}

#[test]
fn test_event_builder_bulk_field_setters_and_filters() {
    let monday = parse("2025-11-03 09:00:00", "UTC");
    let tuesday = parse("2025-11-04 09:00:00", "UTC");
    let thursday = parse("2025-11-06 09:00:00", "UTC");

    let event = EventBuilder::default()
        .title("Covered recurrence")
        .start_datetime(monday)
        .duration_hours(1)
        .attendee("initial@example.com")
        .attendees(vec!["alice@example.com".to_string(), "bob@example.com".to_string()])
        .recurrence(Recurrence::daily().count(7))
        .skip_weekends(true)
        .exception_dates(vec![tuesday, thursday])
        .status(EventStatus::Blocked)
        .build()
        .unwrap();

    assert_eq!(
        event.attendees,
        vec!["alice@example.com".to_string(), "bob@example.com".to_string()]
    );
    assert_eq!(event.status, EventStatus::Blocked);

    let occurrences = event
        .occurrences_between(
            parse("2025-11-03 00:00:00", "UTC"),
            parse("2025-11-10 00:00:00", "UTC"),
            16,
        )
        .unwrap();

    assert_eq!(
        occurrences,
        vec![monday, parse("2025-11-05 09:00:00", "UTC"), parse("2025-11-07 09:00:00", "UTC"),]
    );
}

#[test]
fn test_recurrence_rrule_and_filter_helpers() {
    let start = parse("2025-11-03 09:00:00", "UTC");
    let recurrence = Recurrence::weekly()
        .interval(2)
        .count(5)
        .weekdays(vec![Weekday::Mon, Weekday::Wed]);
    let until = parse("2025-12-31 09:00:00", "UTC");
    let yearly = Recurrence::yearly().until(until);

    assert_eq!(recurrence.get_interval(), 2);
    assert_eq!(recurrence.get_count(), Some(5));
    assert_eq!(yearly.get_until(), Some(until));
    assert_eq!(recurrence.get_weekdays().unwrap(), [Weekday::Mon, Weekday::Wed]);

    let rrule = recurrence.to_rrule_string(start).unwrap();
    assert!(rrule.contains("RRULE:FREQ=WEEKLY;INTERVAL=2;COUNT=5;BYDAY=MON,WED"));

    let filter = RecurrenceFilter::new()
        .skip_weekends(true)
        .skip_dates(vec![parse("2025-11-04 09:00:00", "UTC")]);
    let filtered = filter.filter_occurrences(vec![
        parse("2025-11-03 09:00:00", "UTC"),
        parse("2025-11-04 09:00:00", "UTC"),
        parse("2025-11-08 09:00:00", "UTC"),
    ]);
    assert_eq!(filtered, vec![parse("2025-11-03 09:00:00", "UTC")]);
}
