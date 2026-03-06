#![allow(clippy::unwrap_used)]

//! Performance benchmarks for eventix
//!
//! Run with: cargo bench

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use eventix::{gap_validation, timezone, Calendar, Duration, Event, Recurrence};

/// Create a calendar with the specified number of non-overlapping events
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

/// Create a calendar with overlapping events (for overlap detection benchmarks)
fn create_calendar_with_overlaps(num_events: usize) -> Calendar {
    let mut cal = Calendar::new("Overlap Calendar");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let base = timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

    for i in 0..num_events {
        let event = Event::builder()
            .title(format!("Overlap Event {}", i))
            .start_datetime(base + Duration::minutes(i as i64 * 30))
            .duration_hours(2) // 2-hour events starting every 30 mins = guaranteed overlaps
            .build()
            .unwrap();
        cal.add_event(event);
    }
    cal
}

fn bench_find_overlaps(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_overlaps");

    for size in [10, 50, 100, 200, 500].iter() {
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

    for size in [10, 50, 100, 200, 500].iter() {
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

    for size in [10, 50, 100, 200].iter() {
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

fn bench_lazy_vs_eager_recurrence(c: &mut Criterion) {
    let mut group = c.benchmark_group("recurrence_generation");
    let tz = timezone::parse_timezone("UTC").unwrap();
    let start = timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

    for count in [10u32, 100, 500, 1000].iter() {
        let recurrence = Recurrence::daily().count(*count);

        // Benchmark eager (Vec allocation)
        group.bench_with_input(BenchmarkId::new("eager_vec", count), count, |b, &c| {
            b.iter(|| {
                recurrence
                    .generate_occurrences(black_box(start), black_box(c as usize))
                    .unwrap()
            })
        });

        // Benchmark lazy (Iterator)
        group.bench_with_input(BenchmarkId::new("lazy_collect", count), count, |b, _| {
            b.iter(|| {
                let result: Vec<_> = recurrence.occurrences(black_box(start)).collect();
                result
            })
        });

        // Benchmark lazy with take (partial consumption)
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

fn bench_is_slot_available(c: &mut Criterion) {
    let mut group = c.benchmark_group("is_slot_available");

    for size in [10, 50, 100, 200].iter() {
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
    bench_lazy_vs_eager_recurrence,
    bench_is_slot_available,
);
criterion_main!(benches);
