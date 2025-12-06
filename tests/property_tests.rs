use chrono::{Duration, TimeZone};
use eventix::timezone;
use eventix::{Calendar, Event, Recurrence};
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
        let occurrences = recurrence.generate_occurrences(start, 200).unwrap();

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
        let occurrences = recurrence.generate_occurrences(start, 100).unwrap();

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
}
