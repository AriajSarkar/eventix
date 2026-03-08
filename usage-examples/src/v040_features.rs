//! v0.4.0 Feature Showcase
//!
//! Exercises all new and fixed behaviour shipped in v0.4.0:
//!
//! 1. **Sub-daily recurrence** — `hourly()`, `minutely()`, `secondly()`
//! 2. **DST-transparent sub-daily advancement** — hourly across spring-forward
//! 3. **Filter-before-cap** — `occurrences_between()` applies recurrence
//!    filter / exdates *before* `max_occurrences`, so filtered-out dates
//!    never consume result slots
//! 4. **Lazy recurrence iterator** — `Recurrence::occurrences()`

use chrono::{Datelike, Timelike};
use eventix::{
    gap_validation, timezone, Calendar, Duration, Event, Recurrence,
};

pub fn run() -> eventix::Result<()> {
    println!("\n{}", "=".repeat(60));
    println!("=== v0.4.0 Feature Showcase ===");
    println!("{}\n", "=".repeat(60));

    demo_hourly_recurrence()?;
    demo_minutely_recurrence()?;
    demo_secondly_recurrence()?;
    demo_subdaily_dst_spring_forward()?;
    demo_filter_before_cap()?;
    demo_lazy_iterator()?;
    demo_subdaily_gap_analysis()?;

    println!("\n✅ All v0.4.0 features verified!\n");
    Ok(())
}

// ─── 1. Hourly recurrence ───────────────────────────────────────────────────

fn demo_hourly_recurrence() -> eventix::Result<()> {
    println!("── 1. Hourly Recurrence ──");

    let event = Event::builder()
        .title("Medication Reminder")
        .start("2025-06-01 08:00:00", "America/New_York")
        .duration_minutes(5)
        .recurrence(Recurrence::hourly().interval(4).count(6))
        .build()?;

    let tz = timezone::parse_timezone("America/New_York")?;
    let start = timezone::parse_datetime_with_tz("2025-06-01 00:00:00", tz)?;
    let end = timezone::parse_datetime_with_tz("2025-06-02 23:59:59", tz)?;

    let occs = event.occurrences_between(start, end, 100)?;
    println!("   '{}' — every 4 hours, 6 total:", event.title);
    for occ in &occs {
        println!("     • {}", occ.format("%Y-%m-%d %H:%M %Z"));
    }
    assert_eq!(occs.len(), 6, "expected 6 hourly occurrences");

    // Verify interval spacing
    for i in 1..occs.len() {
        let gap = occs[i] - occs[i - 1];
        assert_eq!(gap, Duration::hours(4), "gap between #{} and #{} should be 4h", i - 1, i);
    }
    println!("   ✅ 4-hour intervals verified\n");
    Ok(())
}

// ─── 2. Minutely recurrence ─────────────────────────────────────────────────

fn demo_minutely_recurrence() -> eventix::Result<()> {
    println!("── 2. Minutely Recurrence ──");

    let event = Event::builder()
        .title("Pomodoro Timer")
        .start("2025-06-01 09:00:00", "UTC")
        .duration_minutes(1)
        .recurrence(Recurrence::minutely().interval(25).count(4))
        .build()?;

    let tz = timezone::parse_timezone("UTC")?;
    let start = timezone::parse_datetime_with_tz("2025-06-01 08:00:00", tz)?;
    let end = timezone::parse_datetime_with_tz("2025-06-01 12:00:00", tz)?;

    let occs = event.occurrences_between(start, end, 100)?;
    println!("   '{}' — every 25 minutes, 4 total:", event.title);
    for occ in &occs {
        println!("     • {}", occ.format("%H:%M:%S"));
    }
    assert_eq!(occs.len(), 4, "expected 4 minutely occurrences");

    for i in 1..occs.len() {
        assert_eq!(
            occs[i] - occs[i - 1],
            Duration::minutes(25),
            "gap should be 25 min"
        );
    }
    println!("   ✅ 25-minute intervals verified\n");
    Ok(())
}

// ─── 3. Secondly recurrence ─────────────────────────────────────────────────

fn demo_secondly_recurrence() -> eventix::Result<()> {
    println!("── 3. Secondly Recurrence ──");

    let event = Event::builder()
        .title("Heartbeat Ping")
        .start("2025-06-01 12:00:00", "UTC")
        .duration(Duration::seconds(1))
        .recurrence(Recurrence::secondly().interval(30).count(5))
        .build()?;

    let tz = timezone::parse_timezone("UTC")?;
    let start = timezone::parse_datetime_with_tz("2025-06-01 11:59:00", tz)?;
    let end = timezone::parse_datetime_with_tz("2025-06-01 12:05:00", tz)?;

    let occs = event.occurrences_between(start, end, 100)?;
    println!("   '{}' — every 30 seconds, 5 total:", event.title);
    for occ in &occs {
        println!("     • {}", occ.format("%H:%M:%S"));
    }
    assert_eq!(occs.len(), 5, "expected 5 secondly occurrences");

    for i in 1..occs.len() {
        assert_eq!(
            occs[i] - occs[i - 1],
            Duration::seconds(30),
            "gap should be 30s"
        );
    }
    println!("   ✅ 30-second intervals verified\n");
    Ok(())
}

// ─── 4. Sub-daily across DST spring-forward ─────────────────────────────────

fn demo_subdaily_dst_spring_forward() -> eventix::Result<()> {
    println!("── 4. Hourly Across DST Spring-Forward ──");
    println!("   (2025-03-09 2:00 AM → 3:00 AM in America/New_York)");

    let event = Event::builder()
        .title("Hourly Check-In")
        .start("2025-03-09 00:00:00", "America/New_York")
        .duration_minutes(10)
        .recurrence(Recurrence::hourly().count(5))
        .build()?;

    let tz = timezone::parse_timezone("America/New_York")?;
    let start = timezone::parse_datetime_with_tz("2025-03-08 23:00:00", tz)?;
    let end = timezone::parse_datetime_with_tz("2025-03-09 06:00:00", tz)?;

    let occs = event.occurrences_between(start, end, 100)?;
    println!("   Occurrences:");
    for occ in &occs {
        println!("     • {}", occ.format("%Y-%m-%d %H:%M %Z"));
    }
    assert_eq!(occs.len(), 5, "expected 5 hourly occurrences across DST");

    // Every pair is exactly 1 hour apart (UTC duration arithmetic)
    for i in 1..occs.len() {
        assert_eq!(
            occs[i] - occs[i - 1],
            Duration::hours(1),
            "must be exactly 1h apart even across DST"
        );
    }
    // The 2:00 AM slot is skipped — next valid local hour is 3:00 AM EDT
    let third = occs[2]; // 00:00 → 01:00 → <skip 2:00> → 03:00 → ...
    // Actually: 00:00 EST, 01:00 EST, 03:00 EDT (spring-forward skips 2 AM)
    // occs[2] should show hour 3
    assert_eq!(Timelike::hour(&third), 3, "third occurrence should be 3 AM EDT (spring-forward)");
    println!("   ✅ DST gap handled — jump from 1:00 AM EST → 3:00 AM EDT\n");
    Ok(())
}

// ─── 5. Filter-before-cap fix ───────────────────────────────────────────────

fn demo_filter_before_cap() -> eventix::Result<()> {
    println!("── 5. Filter-Before-Cap Fix ──");
    println!("   (Weekend-skip filter no longer consumes max_occurrences slots)");

    // Create a daily event that skips weekends — 10 total occurrences.
    // Starting Monday 2025-06-02 → series has weekends on Jun 7 (Sat), 8 (Sun).
    let event = Event::builder()
        .title("Weekday Standup")
        .start("2025-06-02 09:00:00", "UTC")
        .duration_minutes(15)
        .recurrence(Recurrence::daily().count(14)) // 14 candidates ⇒ includes weekends
        .skip_weekends(true) // filter them out
        .build()?;

    let tz = timezone::parse_timezone("UTC")?;
    let start = timezone::parse_datetime_with_tz("2025-06-01 00:00:00", tz)?;
    let end = timezone::parse_datetime_with_tz("2025-06-30 00:00:00", tz)?;

    // Ask for up to 10 occurrences — all 10 weekday slots should be returned,
    // not fewer because weekends were filtered after capping.
    let occs = event.occurrences_between(start, end, 10)?;
    println!("   '{}' — daily with skip_weekends, max 10:", event.title);
    for occ in &occs {
        let day = occ.format("%a %Y-%m-%d").to_string();
        println!("     • {}", day);
    }

    // Every occurrence should be a weekday
    for occ in &occs {
        let wd = Datelike::weekday(occ);
        assert!(
            wd != chrono::Weekday::Sat && wd != chrono::Weekday::Sun,
            "found weekend occurrence: {}",
            occ
        );
    }

    assert_eq!(
        occs.len(),
        10,
        "should get 10 weekday occurrences (filter-before-cap)"
    );
    println!("   ✅ Got exactly 10 weekday results — filter-before-cap works!\n");
    Ok(())
}

// ─── 6. Lazy iterator usage ─────────────────────────────────────────────────

fn demo_lazy_iterator() -> eventix::Result<()> {
    println!("── 6. Lazy Recurrence Iterator ──");

    // Infinite daily series (no count/until) — take only what we need
    let recurrence = Recurrence::daily().interval(1);

    let tz = timezone::parse_timezone("Europe/London")?;
    let start = timezone::parse_datetime_with_tz("2025-06-01 08:00:00", tz)?;

    // Take next 5 occurrences lazily — no allocation of unbounded vec
    let next_five: Vec<_> = recurrence.occurrences(start).take(5).collect();
    println!("   Lazy daily iterator — first 5:");
    for occ in &next_five {
        println!("     • {}", occ.format("%Y-%m-%d %H:%M %Z"));
    }
    assert_eq!(next_five.len(), 5);

    // Minutely lazy iterator — take 3
    let minutely = Recurrence::minutely().interval(10);
    let next_three: Vec<_> = minutely.occurrences(start).take(3).collect();
    println!("   Lazy minutely (every 10 min) — first 3:");
    for occ in &next_three {
        println!("     • {}", occ.format("%H:%M:%S"));
    }
    assert_eq!(next_three.len(), 3);
    println!("   ✅ Lazy iterators work for all frequencies\n");
    Ok(())
}

// ─── 7. Sub-daily gap analysis ──────────────────────────────────────────────

fn demo_subdaily_gap_analysis() -> eventix::Result<()> {
    println!("── 7. Sub-Daily Gap Analysis ──");

    let mut cal = Calendar::new("Monitoring Dashboard");

    // Hourly health-checks, 8 times
    let health_checks = Event::builder()
        .title("Health Check")
        .start("2025-06-01 08:00:00", "UTC")
        .duration_minutes(5)
        .recurrence(Recurrence::hourly().count(8))
        .build()?;

    // 15-minute metric scrapes, 16 times (covers 4 hours)
    let metrics = Event::builder()
        .title("Metric Scrape")
        .start("2025-06-01 08:02:00", "UTC")
        .duration(Duration::seconds(30))
        .recurrence(Recurrence::minutely().interval(15).count(16))
        .build()?;

    cal.add_event(health_checks);
    cal.add_event(metrics);

    let tz = timezone::parse_timezone("UTC")?;
    let start = timezone::parse_datetime_with_tz("2025-06-01 08:00:00", tz)?;
    let end = timezone::parse_datetime_with_tz("2025-06-01 16:00:00", tz)?;

    let density = gap_validation::calculate_density(&cal, start, end)?;
    println!(
        "   Schedule density: {:.1}% occupied ({} events in window)",
        density.occupancy_percentage, density.event_count
    );

    let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(30))?;
    println!("   Gaps ≥ 30 min: {}", gaps.len());
    for gap in &gaps {
        println!(
            "     • {} → {} ({} min)",
            gap.start.format("%H:%M"),
            gap.end.format("%H:%M"),
            (gap.end - gap.start).num_minutes()
        );
    }

    let overlaps = gap_validation::find_overlaps(&cal, start, end)?;
    println!("   Overlaps detected: {}", overlaps.len());
    if !overlaps.is_empty() {
        for ov in &overlaps {
            println!(
                "     • {} at {}",
                ov.events.join(" ↔ "),
                ov.start.format("%H:%M:%S")
            );
        }
    }

    println!("   ✅ Sub-daily gap analysis complete\n");
    Ok(())
}
