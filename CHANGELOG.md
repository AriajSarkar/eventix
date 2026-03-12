# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-03-07

### Added
- **Lazy recurrence iteration**: `Recurrence::occurrences()` returns an `OccurrenceIterator` for memory-efficient, on-demand occurrence generation.
- **Eager cap helper**: `Recurrence::generate_occurrences_capped()` provides capped eager collection, replacing the old `max_occurrences` parameter of `generate_occurrences()`.
- **Weekday filtering**: `Recurrence::weekdays()` builder method to restrict occurrences to specific days of the week.
- **Benchmark suite**: Criterion benchmarks for overlap/gap detection, density analysis, recurrence generation, and slot availability.
- **JSON/web examples**: Examples showing calendar import/export as JSON for API workflows.

### Changed
- **[BREAKING] `generate_occurrences()` signature**: Changed from `generate_occurrences(start, max_occurrences)` to `generate_occurrences(start)` — now collects all occurrences defined by the recurrence's own `count`/`until` bounds. Use `generate_occurrences_capped(start, max)` for the old capped behavior.
- **[BREAKING] Count semantics**: `Recurrence::count(n)` now caps *emitted* occurrences, not scanned candidates. With weekday filters, `count(14)` yields exactly 14 matching days instead of scanning 14 slots. This affects both `generate_occurrences()` and `occurrences_between()`.
- **[BREAKING] Intersection-based filtering**: `Event::occurrences_between()` now uses time-range intersection (`dt < end && dt + duration > start`) instead of start-in-range filtering. Events starting before a query window but extending into it are now correctly included.
- **[BREAKING] Monthly/Yearly day clamping**: Monthly and yearly recurrence now clamps the day to the valid range (e.g. Jan 31 → Feb 28) instead of terminating the series. Code that relied on early termination will see additional occurrences.
- **Full sub-daily recurrence support**: `Hourly`, `Minutely`, and `Secondly` frequencies are now fully implemented. Sub-daily advancement uses UTC-duration arithmetic (`DateTime<Tz> + Duration`) so DST transitions are handled transparently without any local-time look-up. New convenience constructors `Recurrence::hourly()`, `minutely()`, and `secondly()` are provided. `Recurrence::new()` remains infallible and accepts all seven RFC 5545 frequencies.
- **Faster overlap detection**: `find_overlaps()` uses an O(N log N) sweep-line algorithm instead of O(N²).
- **Correct boundary handling**: Back-to-back events that merely touch at a boundary are no longer reported as overlapping.
- **DST-safe recurrence**: Daily/Weekly recurrence uses local date arithmetic to preserve wall-clock time across DST transitions.
- **DST spring-forward resilience**: Recurrence generation uses pre-gap UTC offset conversion to land on the correct post-transition wall-clock time (e.g. 2:30 AM EST → 3:30 AM EDT), matching Google Calendar / RFC 5545 behaviour. Subsequent occurrences return to the originally intended time rather than drifting.
- **Deterministic overlap ordering**: `find_overlaps()` uses `BTreeSet` for consistent results.
- **Zero-duration events**: No longer interfere with overlap detection.
- **[BREAKING]** `generate_occurrences()` now returns an error for unbounded recurrences (no `count` or `until`). Use `occurrences()` for lazy iteration or `generate_occurrences_capped()` for a hard cap.

### Fixed
- `occurs_on()` now correctly finds later occurrences of recurring events by using lazy iteration with post-filter capping.
- `occurrences_between()` now applies recurrence filters and exception dates lazily *before* the `max_occurrences` cap, so filtered-out dates don't consume result slots and capped queries stop after enough accepted results.
- `interval(0)` returns no occurrences instead of looping infinitely (previously caused an infinite loop). Consistent across all frequencies including weekly with BYDAY rules.
- `to_rrule_string()` now converts `UNTIL` values to UTC before appending the `Z` suffix, per RFC 5545 compliance.

## [0.3.1] - 2025-12-18

### Added
- **EventBuilder.duration()**: New flexible `.duration()` method that accepts any `Duration` object for complex durations (e.g., `Duration::hours(1) + Duration::minutes(10)`)
- Complementary to existing `duration_hours()` and `duration_minutes()` convenience methods
- Comprehensive test coverage for duration functionality

### Changed
- Updated README examples to demonstrate `Duration` usage
- Added clippy lint allowances to test modules for cleaner test code

## [0.3.0] - 2025-12-06

### Added
- **Booking State Machine**: Introduced `EventStatus` enum with `Confirmed`, `Tentative`, `Cancelled`, and `Blocked` states.
- **State Transitions**: Added `confirm()`, `cancel()`, `tentative()`, and `reschedule()` methods to `Event` struct.
- **Smart Gap Validation**: `find_gaps` and `calculate_density` now automatically ignore `Cancelled` events.
- **EventBuilder Support**: Added `.status()` method to `EventBuilder`.

### Changed
- **[BREAKING]** `Event` struct now has a public `status` field.
- **[BREAKING]** `EventBuilder` now has a public `.status()` method.
- **[BREAKING]** `Event` struct now has a public `is_active()` method.

## [0.2.0] - 2025-12-06

### Added
- Initial release with core features:
  - Timezone-aware events
  - Recurrence patterns (daily, weekly, monthly, yearly)
  - Exception dates
  - Gap validation and schedule optimization logic
  - ICS file import/export support
