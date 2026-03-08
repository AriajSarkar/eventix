#![allow(clippy::unwrap_used)]

//! Performance benchmarks for eventix
//!
//! Run with: cargo bench
//!
//! Benchmark groups:
//!
//! 1. **Gap / overlap / density analysis** — sweep-line algorithms at scale
//! 2. **Recurrence generation** — lazy vs eager, all 7 frequencies
//! 3. **Sub-daily dense recurrence** — secondly/minutely/hourly over large windows
//! 4. **Lazy capped `occurrences_between`** — the key perf path: dense series,
//!    small cap, must NOT scan the whole window
//! 5. **Large synthetic calendar workloads** — 1K–10K events
//! 6. **ICS export throughput** — serialization at volume
//! 7. **Multi-timezone recurring expansion** — DST-heavy workloads

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use eventix::{gap_validation, timezone, Calendar, Duration, Event, Recurrence};

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Create a calendar with `n` non-overlapping 1-hour events spaced 2 hours apart.
fn create_calendar_with_events(num_events: usize) -> Calendar {
    let mut cal = Calendar::new("Benchmark Calendar");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let base = timezone::parse_datetime_with_tz("2025-01-01 00:00:00", tz).unwrap();

    for i in 0..num_events {
        let event = Event::builder()
            .title(format!("Event {}", i))
            .start_datetime(base + Duration::hours(i as i64 * 2))
            .duration_hours(1)
            .build()
            .unwrap();
        cal.add_event(event);
    }
    cal
}

/// Create a calendar with overlapping events (for overlap detection benchmarks).
fn create_calendar_with_overlaps(num_events: usize) -> Calendar {
    let mut cal = Calendar::new("Overlap Calendar");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let base = timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

    for i in 0..num_events {
        let event = Event::builder()
            .title(format!("Overlap Event {}", i))
            .start_datetime(base + Duration::minutes(i as i64 * 30))
            .duration_hours(2) // 2-hour events every 30 mins = guaranteed overlaps
            .build()
            .unwrap();
        cal.add_event(event);
    }
    cal
}

/// Create a large calendar mixing one-off and recurring events.
fn create_large_mixed_calendar(one_off: usize, recurring: usize) -> Calendar {
    let mut cal = Calendar::new("Large Mixed Calendar");
    let tz = timezone::parse_timezone("America/New_York").unwrap();
    let base = timezone::parse_datetime_with_tz("2025-01-01 08:00:00", tz).unwrap();

    for i in 0..one_off {
        let event = Event::builder()
            .title(format!("OneOff {}", i))
            .start_datetime(base + Duration::hours(i as i64))
            .duration_minutes(45)
            .build()
            .unwrap();
        cal.add_event(event);
    }

    for i in 0..recurring {
        let event = Event::builder()
            .title(format!("Recurring {}", i))
            .start_datetime(base + Duration::hours(i as i64 * 3))
            .duration_hours(1)
            .recurrence(Recurrence::daily().count(30))
            .build()
            .unwrap();
        cal.add_event(event);
    }
    cal
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. Gap / Overlap / Density analysis
// ═══════════════════════════════════════════════════════════════════════════

fn bench_find_overlaps(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_overlaps");

    for size in [10, 50, 100, 500, 1000].iter() {
        let cal = create_calendar_with_overlaps(*size);
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-01-01 00:00:00", tz).unwrap();
        let end = timezone::parse_datetime_with_tz("2025-12-31 23:59:59", tz).unwrap();

        group.bench_with_input(BenchmarkId::new("sweep_line", size), size, |b, _| {
            b.iter(|| {
                gap_validation::find_overlaps(black_box(&cal), black_box(start), black_box(end))
                    .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_find_gaps(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_gaps");

    for size in [10, 50, 100, 500, 1000].iter() {
        let cal = create_calendar_with_events(*size);
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-01-01 00:00:00", tz).unwrap();
        let end = timezone::parse_datetime_with_tz("2025-12-31 23:59:59", tz).unwrap();

        group.bench_with_input(BenchmarkId::new("sorted_sweep", size), size, |b, _| {
            b.iter(|| {
                gap_validation::find_gaps(
                    black_box(&cal),
                    black_box(start),
                    black_box(end),
                    black_box(Duration::minutes(0)),
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_calculate_density(c: &mut Criterion) {
    let mut group = c.benchmark_group("calculate_density");

    for size in [10, 50, 100, 500, 1000].iter() {
        let cal = create_calendar_with_events(*size);
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-01-01 00:00:00", tz).unwrap();
        let end = timezone::parse_datetime_with_tz("2025-12-31 23:59:59", tz).unwrap();

        group.bench_with_input(BenchmarkId::new("full_analysis", size), size, |b, _| {
            b.iter(|| {
                gap_validation::calculate_density(black_box(&cal), black_box(start), black_box(end))
                    .unwrap()
            })
        });
    }
    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. Recurrence generation — all 7 frequencies, lazy vs eager
// ═══════════════════════════════════════════════════════════════════════════

fn bench_recurrence_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("recurrence_generation");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

    for count in [10u32, 100, 500, 1000].iter() {
        let recurrence = Recurrence::daily().count(*count);

        group.bench_with_input(BenchmarkId::new("eager_daily", count), count, |b, &c| {
            b.iter(|| {
                recurrence
                    .generate_occurrences_capped(black_box(start), black_box(c as usize))
                    .unwrap()
            })
        });

        group.bench_with_input(BenchmarkId::new("lazy_daily", count), count, |b, _| {
            b.iter(|| {
                let result: Vec<_> = recurrence.occurrences(black_box(start)).collect();
                result
            })
        });

        // Lazy partial consumption — only take 10%
        let take_count = (*count as usize) / 10;
        group.bench_with_input(BenchmarkId::new("lazy_take_10pct", count), count, |b, _| {
            b.iter(|| {
                let result: Vec<_> =
                    recurrence.occurrences(black_box(start)).take(take_count).collect();
                result
            })
        });
    }
    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. Sub-daily dense recurrence — secondly / minutely / hourly at scale
// ═══════════════════════════════════════════════════════════════════════════

fn bench_subdaily_recurrence(c: &mut Criterion) {
    let mut group = c.benchmark_group("subdaily_recurrence");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-06-01 00:00:00", tz).unwrap();

    // Secondly: generate N occurrences
    for count in [100u32, 1000, 10_000].iter() {
        let recurrence = Recurrence::secondly().interval(1).count(*count);
        group.bench_with_input(BenchmarkId::new("secondly", count), count, |b, _| {
            b.iter(|| {
                let result: Vec<_> = recurrence.occurrences(black_box(start)).collect();
                result
            })
        });
    }

    // Minutely: generate N occurrences
    for count in [100u32, 1000, 10_000].iter() {
        let recurrence = Recurrence::minutely().interval(1).count(*count);
        group.bench_with_input(BenchmarkId::new("minutely", count), count, |b, _| {
            b.iter(|| {
                let result: Vec<_> = recurrence.occurrences(black_box(start)).collect();
                result
            })
        });
    }

    // Hourly: generate N occurrences
    for count in [100u32, 1000, 10_000].iter() {
        let recurrence = Recurrence::hourly().interval(1).count(*count);
        group.bench_with_input(BenchmarkId::new("hourly", count), count, |b, _| {
            b.iter(|| {
                let result: Vec<_> = recurrence.occurrences(black_box(start)).collect();
                result
            })
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. Lazy capped `occurrences_between` — the critical perf path
//
//    Dense sub-daily recurrence (e.g. 100K secondly occurrences) over a huge
//    window, but only requesting a small number of results.  The lazy
//    pipeline must short-circuit after `max_occurrences` accepted entries.
// ═══════════════════════════════════════════════════════════════════════════

fn bench_occurrences_between_capped(c: &mut Criterion) {
    let mut group = c.benchmark_group("occurrences_between_capped");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let window_start = timezone::parse_datetime_with_tz("2025-06-01 00:00:00", tz).unwrap();
    let window_end = timezone::parse_datetime_with_tz("2025-07-01 00:00:00", tz).unwrap();

    // Secondly — 100K candidates in a 30-day window, cap at 10/100/1000
    for cap in [10usize, 100, 1000].iter() {
        let event = Event::builder()
            .title("Dense Secondly")
            .start("2025-06-01 00:00:00", "UTC")
            .duration(Duration::seconds(1))
            .recurrence(Recurrence::secondly().interval(1).count(100_000))
            .build()
            .unwrap();

        group.bench_with_input(BenchmarkId::new("secondly_100k_cap", cap), cap, |b, &cap| {
            b.iter(|| {
                event
                    .occurrences_between(
                        black_box(window_start),
                        black_box(window_end),
                        black_box(cap),
                    )
                    .unwrap()
            })
        });
    }

    // Minutely — 100K candidates, cap at 10/100
    for cap in [10usize, 100].iter() {
        let event = Event::builder()
            .title("Dense Minutely")
            .start("2025-06-01 00:00:00", "UTC")
            .duration(Duration::seconds(10))
            .recurrence(Recurrence::minutely().interval(1).count(100_000))
            .build()
            .unwrap();

        group.bench_with_input(BenchmarkId::new("minutely_100k_cap", cap), cap, |b, &cap| {
            b.iter(|| {
                event
                    .occurrences_between(
                        black_box(window_start),
                        black_box(window_end),
                        black_box(cap),
                    )
                    .unwrap()
            })
        });
    }

    // Hourly with weekend filter — 50K candidates, cap at 20
    {
        let event = Event::builder()
            .title("Hourly Filtered")
            .start("2025-06-02 08:00:00", "UTC") // Monday
            .duration_minutes(5)
            .recurrence(Recurrence::hourly().interval(1).count(50_000))
            .skip_weekends(true)
            .build()
            .unwrap();

        group.bench_function("hourly_50k_skip_weekends_cap20", |b| {
            b.iter(|| {
                event
                    .occurrences_between(
                        black_box(window_start),
                        black_box(window_end),
                        black_box(20),
                    )
                    .unwrap()
            })
        });
    }

    // Daily with weekend filter — realistic corporate scheduling
    {
        let event = Event::builder()
            .title("Daily Standup")
            .start("2025-01-06 09:00:00", "America/New_York") // Monday
            .duration_minutes(15)
            .recurrence(Recurrence::daily().count(365))
            .skip_weekends(true)
            .build()
            .unwrap();

        let ny_tz = timezone::parse_timezone("America/New_York").unwrap();
        let year_start = timezone::parse_datetime_with_tz("2025-01-01 00:00:00", ny_tz).unwrap();
        let year_end = timezone::parse_datetime_with_tz("2025-12-31 23:59:59", ny_tz).unwrap();

        group.bench_function("daily_365_skip_weekends_cap50", |b| {
            b.iter(|| {
                event
                    .occurrences_between(black_box(year_start), black_box(year_end), black_box(50))
                    .unwrap()
            })
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. Large synthetic calendar workloads
//    (1K–10K events, mix of one-off and recurring)
// ═══════════════════════════════════════════════════════════════════════════

fn bench_large_scale_calendar(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_scale_calendar");
    group.sample_size(20); // fewer samples for expensive benches

    let tz = timezone::parse_timezone("America/New_York").unwrap();
    let year_start = timezone::parse_datetime_with_tz("2025-01-01 00:00:00", tz).unwrap();
    let year_end = timezone::parse_datetime_with_tz("2025-12-31 23:59:59", tz).unwrap();

    // 1K one-off events — gap analysis across a full year
    {
        let cal = create_calendar_with_events(1000);
        group.bench_function("gap_analysis_1k_events", |b| {
            b.iter(|| {
                gap_validation::find_gaps(
                    black_box(&cal),
                    black_box(year_start),
                    black_box(year_end),
                    black_box(Duration::minutes(30)),
                )
                .unwrap()
            })
        });
    }

    // 5K one-off events — overlap detection
    {
        let cal = create_calendar_with_overlaps(5000);
        group.bench_function("overlap_sweep_5k_events", |b| {
            b.iter(|| {
                gap_validation::find_overlaps(
                    black_box(&cal),
                    black_box(year_start),
                    black_box(year_end),
                )
                .unwrap()
            })
        });
    }

    // Mixed calendar: 500 one-off + 100 recurring (×30 days each = 3500 expanded)
    {
        let cal = create_large_mixed_calendar(500, 100);
        group.bench_function("density_mixed_500_100rec", |b| {
            b.iter(|| {
                gap_validation::calculate_density(
                    black_box(&cal),
                    black_box(year_start),
                    black_box(year_end),
                )
                .unwrap()
            })
        });
    }

    // Suggest alternatives with a dense schedule — 200 events
    {
        let cal = create_calendar_with_events(200);
        let check_time = timezone::parse_datetime_with_tz("2025-01-01 02:00:00", tz).unwrap();
        group.bench_function("suggest_alternatives_200_events", |b| {
            b.iter(|| {
                gap_validation::suggest_alternatives(
                    black_box(&cal),
                    black_box(check_time),
                    black_box(Duration::hours(1)),
                    black_box(Duration::hours(8)),
                )
                .unwrap()
            })
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. ICS export throughput — serialize large calendars
// ═══════════════════════════════════════════════════════════════════════════

fn bench_ics_export(c: &mut Criterion) {
    let mut group = c.benchmark_group("ics_export");
    group.sample_size(20);

    for size in [50, 200, 1000].iter() {
        let cal = create_calendar_with_events(*size);
        group.bench_with_input(BenchmarkId::new("to_ics_string", size), size, |b, _| {
            b.iter(|| black_box(&cal).to_ics_string().unwrap())
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. Multi-timezone DST-heavy recurrence
// ═══════════════════════════════════════════════════════════════════════════

fn bench_dst_recurrence(c: &mut Criterion) {
    let mut group = c.benchmark_group("dst_recurrence");

    // Daily recurrence across both DST transitions in America/New_York
    // (spring-forward Mar 9, fall-back Nov 2 in 2025)
    {
        let event = Event::builder()
            .title("Daily across DST")
            .start("2025-01-01 02:30:00", "America/New_York") // 2:30 AM — DST gap time
            .duration_hours(1)
            .recurrence(Recurrence::daily().count(365))
            .build()
            .unwrap();

        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-01-01 00:00:00", tz).unwrap();
        let end = timezone::parse_datetime_with_tz("2025-12-31 23:59:59", tz).unwrap();

        group.bench_function("daily_365_across_dst_nyc", |b| {
            b.iter(|| {
                event
                    .occurrences_between(black_box(start), black_box(end), black_box(365))
                    .unwrap()
            })
        });
    }

    // Hourly across spring-forward — sub-daily DST stress
    {
        let event = Event::builder()
            .title("Hourly across spring-forward")
            .start("2025-03-08 22:00:00", "America/New_York")
            .duration_minutes(10)
            .recurrence(Recurrence::hourly().count(48)) // 2 days of hourly
            .build()
            .unwrap();

        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-03-08 00:00:00", tz).unwrap();
        let end = timezone::parse_datetime_with_tz("2025-03-11 00:00:00", tz).unwrap();

        group.bench_function("hourly_48_spring_forward", |b| {
            b.iter(|| {
                event
                    .occurrences_between(black_box(start), black_box(end), black_box(48))
                    .unwrap()
            })
        });
    }

    // Multiple timezones — simulate a global org
    {
        let timezones = [
            "America/New_York",
            "America/Los_Angeles",
            "Europe/London",
            "Europe/Berlin",
            "Asia/Tokyo",
            "Australia/Sydney",
        ];

        let mut cal = Calendar::new("Global Org");
        for tz_name in timezones.iter() {
            let event = Event::builder()
                .title(format!("Regional Standup {}", tz_name))
                .start("2025-01-06 09:00:00", tz_name)
                .duration_minutes(30)
                .recurrence(Recurrence::daily().count(260)) // ~1 year of weekdays
                .skip_weekends(true)
                .build()
                .unwrap();
            cal.add_event(event);

            // Add some hourly monitoring per region
            let monitor = Event::builder()
                .title(format!("Monitor {}", tz_name))
                .start("2025-01-01 00:00:00", tz_name)
                .duration_minutes(5)
                .recurrence(Recurrence::hourly().interval(6).count(1460)) // every 6h for a year
                .build()
                .unwrap();
            cal.add_event(monitor);
        }

        let tz = timezone::parse_timezone("UTC").unwrap();
        let q1_start = timezone::parse_datetime_with_tz("2025-01-01 00:00:00", tz).unwrap();
        let q1_end = timezone::parse_datetime_with_tz("2025-03-31 23:59:59", tz).unwrap();

        group.bench_function("global_org_q1_density_6tz", |b| {
            b.iter(|| {
                gap_validation::calculate_density(
                    black_box(&cal),
                    black_box(q1_start),
                    black_box(q1_end),
                )
                .unwrap()
            })
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 8. Slot availability — point queries at scale
// ═══════════════════════════════════════════════════════════════════════════

fn bench_is_slot_available(c: &mut Criterion) {
    let mut group = c.benchmark_group("is_slot_available");

    for size in [10, 50, 100, 500, 1000].iter() {
        let cal = create_calendar_with_events(*size);
        let tz = timezone::parse_timezone("UTC").unwrap();
        let slot_start_gap = timezone::parse_datetime_with_tz("2025-01-01 01:00:00", tz).unwrap();
        let slot_end_gap = slot_start_gap + Duration::hours(1);
        let slot_start_conflict =
            timezone::parse_datetime_with_tz("2025-01-01 02:30:00", tz).unwrap();
        let slot_end_conflict = slot_start_conflict + Duration::hours(1);

        group.bench_with_input(BenchmarkId::new("check_available_gap", size), size, |b, _| {
            b.iter(|| {
                gap_validation::is_slot_available(
                    black_box(&cal),
                    black_box(slot_start_gap),
                    black_box(slot_end_gap),
                )
                .unwrap()
            })
        });

        group.bench_with_input(BenchmarkId::new("check_conflict", size), size, |b, _| {
            b.iter(|| {
                gap_validation::is_slot_available(
                    black_box(&cal),
                    black_box(slot_start_conflict),
                    black_box(slot_end_conflict),
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_find_overlaps,
    bench_find_gaps,
    bench_calculate_density,
    bench_recurrence_generation,
    bench_subdaily_recurrence,
    bench_occurrences_between_capped,
    bench_large_scale_calendar,
    bench_ics_export,
    bench_dst_recurrence,
    bench_is_slot_available,
);
criterion_main!(benches);
