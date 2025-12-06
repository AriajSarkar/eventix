# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-12-06

### Added
- **GitHub Actions Workflows**: Added `release.yml` and `rust.yml` for automated CI/CD and releases.
- **Rustfmt**: Added `rustfmt.toml` to enforce consistent code style.

### Changed
- **Refactoring**: Significant systematic CI improvements and codebase refactoring.
- **Features**: Consolidated calendar and event management features including timezone handling, recurrence, ICS, and gap validation.
- **Documentation**: Updated `README.md` and refined examples for better clarity.

## [0.1.2] - 2025-10-26

### Improved
- **Enhanced documentation** in main `lib.rs`:
  - Added timezone-aware ICS export example
  - Added schedule analysis example showcasing gap validation features
  - Added module overview with descriptions
  - Added references to example files
  - Improved feature highlights to emphasize unique gap validation capabilities
- All 25 doc tests passing (2 new examples)

## [0.1.1] - 2025-10-26

### Added
- **Timezone-aware ICS export**: Events now export with proper `TZID` parameters for non-UTC timezones
  - Non-UTC timezones use format: `DTSTART;TZID=America/New_York:20251027T100000`
  - UTC timezone uses standard Z suffix: `DTSTART:20251027T150000Z`
  - Exception dates (`EXDATE`) preserve timezone context with `TZID` parameter
- Enhanced ICS import to properly parse and preserve `TZID` parameters
- New example: `examples/timezone_ics_export.rs` demonstrating multi-timezone calendars
- Comprehensive test suite: `tests/timezone_ics_tests.rs` with 9 timezone-specific tests
- Documentation updates in README.md for timezone-aware ICS functionality

### Changed
- ICS export now uses `Property::new()` with `add_parameter()` for TZID support
- Import functionality enhanced to extract timezone from TZID parameter
- Events maintain local time context during round-trip import/export

### Fixed
- ICS round-trip compatibility now preserves timezone information
- Calendar app compatibility improved (Google Calendar, Outlook, Apple Calendar)

### Technical Details
- Implements RFC 5545 iCalendar specification for timezone parameters
- Backward compatible - existing code continues to work without changes
- All 65 tests passing (22 unit + 20 integration + 23 doc tests)

## [0.1.0] - 2025-10-26

### Added
- Initial release of Eventix calendar library
- Core event and calendar management
- Timezone-aware event scheduling with `chrono-tz`
- Recurrence patterns (daily, weekly, monthly, yearly)
- Exception handling (skip dates, weekends, custom filters)
- ICS import/export functionality
- Gap validation and schedule analysis features
- Builder pattern API for ergonomic event creation
- JSON serialization support
- Comprehensive test suite (56 tests)
- Complete documentation with examples

[0.2.0]: https://github.com/AriajSarkar/eventix/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/AriajSarkar/eventix/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/AriajSarkar/eventix/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/AriajSarkar/eventix/releases/tag/v0.1.0
