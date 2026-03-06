# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.2] - 2026-03-06

### Added
- **Lazy recurrence iteration**: Added `Recurrence::occurrences()` and the public `OccurrenceIterator` type for memory-efficient, on-demand occurrence generation.
- **Benchmark coverage**: Added a `criterion` benchmark suite covering overlap detection, gap detection, density analysis, recurrence generation, and slot availability.
- **JSON/web examples**: Added examples showing how to import/export calendars as JSON for API-style workflows without expanding the main public API surface.

### Changed
- **Faster overlap detection**: `gap_validation::find_overlaps()` now uses a sweep-line approach for much better scalability on larger calendars.
- **Correct boundary handling**: Back-to-back events that merely touch at the boundary are no longer reported as overlapping.
- **Safer edge-case handling**: Zero-duration events no longer interfere with overlap detection.
- **Stronger schedule-analysis validation**: Expanded integration and property-based test coverage for `gap_validation` invariants and edge cases.

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
