# Eventix 📅

A high-level calendar and recurrence library for Rust with timezone-aware scheduling, exceptions, and ICS import/export.

[![Crates.io](https://img.shields.io/crates/v/eventix.svg)](https://crates.io/crates/eventix)
[![Documentation](https://docs.rs/eventix/badge.svg)](https://docs.rs/eventix)
[![CI](https://github.com/AriajSarkar/eventix/workflows/Rust%20CI/badge.svg)](https://github.com/AriajSarkar/eventix/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)


## Features

- 🌍 **Timezone-aware events** - Full support for timezones and DST handling using `chrono-tz`
- 🔄 **Recurrence patterns** - Daily, weekly, monthly, and yearly recurrence with advanced rules
- 🚫 **Exception handling** - Skip specific dates, weekends, or custom holiday lists
- 📅 **ICS support** - Import and export events using the iCalendar (`.ics`) format
- 🛠️ **Builder API** - Ergonomic, fluent interface for creating events and calendars
- 🔍 **Gap validation** - Find gaps between events, detect conflicts, analyze schedule density
- 📊 **Schedule analysis** - Occupancy metrics, conflict detection, availability finding
- ✅ **Type-safe** - Leverages Rust's type system for correctness

## Quick Start

Add Eventix to your `Cargo.toml`:

```toml
[dependencies]
Eventix = "0.1"
```

### Basic Usage

```rust
use Eventix::{Calendar, Event, Recurrence};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a calendar
    let mut cal = Calendar::new("My Calendar");

    // Create a simple event
    let meeting = Event::builder()
        .title("Team Meeting")
        .description("Weekly sync with the team")
        .start("2025-11-01 10:00:00", "America/New_York")
        .duration_hours(1)
        .attendee("alice@example.com")
        .attendee("bob@example.com")
        .build()?;

    cal.add_event(meeting);

    // Create a recurring event
    let standup = Event::builder()
        .title("Daily Standup")
        .start("2025-11-01 09:00:00", "America/New_York")
        .duration_minutes(15)
        .recurrence(Recurrence::daily().count(30))
        .skip_weekends(true)
        .build()?;

    cal.add_event(standup);

    // Export to ICS
    cal.export_to_ics("calendar.ics")?;

    Ok(())
}
```

## Examples

### Daily Recurrence with Exceptions

```rust
use Eventix::{Event, Recurrence, timezone};

let tz = timezone::parse_timezone("America/New_York")?;
let holiday = timezone::parse_datetime_with_tz("2025-11-27 09:00:00", tz)?;

let event = Event::builder()
    .title("Morning Standup")
    .start("2025-11-01 09:00:00", "America/New_York")
    .duration_minutes(15)
    .recurrence(Recurrence::daily().count(30))
    .skip_weekends(true)
    .exception_date(holiday)  // Skip Thanksgiving
    .build()?;
```

### Weekly Recurrence

```rust
use Eventix::{Event, Recurrence};

let event = Event::builder()
    .title("Weekly Team Meeting")
    .start("2025-11-03 14:00:00", "UTC")
    .duration_hours(1)
    .recurrence(Recurrence::weekly().count(10))
    .build()?;
```

### Monthly Recurrence

```rust
use Eventix::{Event, Recurrence};

let event = Event::builder()
    .title("Monthly All-Hands")
    .start("2025-11-01 15:00:00", "America/Los_Angeles")
    .duration_hours(2)
    .recurrence(Recurrence::monthly().count(12))
    .build()?;
```

### ICS Import/Export

```rust
use eventix::Calendar;

// Export with timezone awareness
let mut cal = Calendar::new("Work Schedule");
// ... add events ...
cal.export_to_ics("schedule.ics")?;

// Import
let imported_cal = Calendar::import_from_ics("schedule.ics")?;
println!("Imported {} events", imported_cal.event_count());
```

**Timezone-Aware ICS Export:**

Events are exported with proper timezone information for compatibility with calendar applications:

```rust
// Non-UTC timezones include TZID parameter
let event = Event::builder()
    .title("Team Meeting")
    .start("2025-10-27 10:00:00", "America/New_York")
    .duration_hours(1)
    .build()?;

// Generates: DTSTART;TZID=America/New_York:20251027T100000

// UTC events use standard Z suffix
let utc_event = Event::builder()
    .title("Global Call")
    .start("2025-10-27 15:00:00", "UTC")
    .duration_hours(1)
    .build()?;

// Generates: DTSTART:20251027T150000Z
```

This ensures events display at the correct local time in:
- Google Calendar
- Microsoft Outlook
- Apple Calendar
- Any RFC 5545 compliant calendar application

### Query Events

```rust
use Eventix::{Calendar, timezone};

let cal = Calendar::new("My Calendar");
// ... add events ...

// Find events by title
let meetings = cal.find_events_by_title("meeting");

// Get events in a date range
let tz = timezone::parse_timezone("UTC")?;
let start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz)?;
let end = timezone::parse_datetime_with_tz("2025-11-30 23:59:59", tz)?;

let november_events = cal.events_between(start, end)?;

// Get events on a specific date
let date = timezone::parse_datetime_with_tz("2025-11-15 00:00:00", tz)?;
let events = cal.events_on_date(date)?;
```

### Gap Detection & Schedule Analysis

**Unique to Eventix** - Features not found in other calendar crates:

```rust
use Eventix::{Calendar, Event, gap_validation, timezone};
use chrono::Duration;

let mut cal = Calendar::new("Work Schedule");
// ... add events ...

let tz = timezone::parse_timezone("America/New_York")?;
let start = timezone::parse_datetime_with_tz("2025-11-03 08:00:00", tz)?;
let end = timezone::parse_datetime_with_tz("2025-11-03 18:00:00", tz)?;

// Find gaps between events (at least 30 minutes)
let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(30))?;
for gap in gaps {
    println!("Free: {} to {} ({} min)", 
        gap.start.format("%H:%M"),
        gap.end.format("%H:%M"),
        gap.duration_minutes()
    );
}

// Detect scheduling conflicts
let overlaps = gap_validation::find_overlaps(&cal, start, end)?;
if !overlaps.is_empty() {
    println!("⚠️  Found {} conflicts", overlaps.len());
}

// Analyze schedule density
let density = gap_validation::calculate_density(&cal, start, end)?;
println!("Schedule occupancy: {:.1}%", density.occupancy_percentage);
println!("Busy: {:.1}h, Free: {:.1}h", 
    density.busy_duration.num_minutes() as f64 / 60.0,
    density.free_duration.num_minutes() as f64 / 60.0
);

// Find available slots for a 1-hour meeting
let slots = gap_validation::find_available_slots(&cal, start, end, Duration::hours(1))?;
println!("Available times for 1-hour meeting: {}", slots.len());

// Check if specific time is available
let check_time = timezone::parse_datetime_with_tz("2025-11-03 14:00:00", tz)?;
let available = gap_validation::is_slot_available(&cal, check_time, check_time + Duration::hours(1))?;

// Get alternative times for conflicts
let alternatives = gap_validation::suggest_alternatives(
    &cal, 
    check_time, 
    Duration::hours(1),
    Duration::hours(2)  // search within 2 hours
)?;
```

## Documentation

Run the examples:

```bash
# Basic calendar usage
cargo run --example basic

# Recurrence patterns
cargo run --example recurrence

# ICS import/export
cargo run --example ics_export

# Gap validation and schedule analysis
cargo run --example gap_validation
```

View the full API documentation:

```bash
cargo doc --open
```

## Architecture

The crate is organized into several modules:

- **`calendar`** - Calendar container for managing events
- **`event`** - Event types and builder API
- **`recurrence`** - Recurrence rules and patterns
- **`ics`** - ICS format import/export
- **`timezone`** - Timezone handling and DST support
- **`gap_validation`** - Schedule analysis, gap detection, conflict resolution
- **`error`** - Error types and results

## Dependencies

- [`chrono`](https://crates.io/crates/chrono) - Date and time handling
- [`chrono-tz`](https://crates.io/crates/chrono-tz) - Timezone database
- [`rrule`](https://crates.io/crates/rrule) - Recurrence rule parsing
- [`icalendar`](https://crates.io/crates/icalendar) - ICS format support
- [`serde`](https://crates.io/crates/serde) - Serialization support

## Timezone Support

Eventix fully supports timezone-aware datetime handling with automatic DST transitions:

```rust
use Eventix::timezone;

// Parse timezone
let tz = timezone::parse_timezone("America/New_York")?;

// Parse datetime with timezone
let dt = timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz)?;

// Convert between timezones
let tokyo_tz = timezone::parse_timezone("Asia/Tokyo")?;
let dt_tokyo = timezone::convert_timezone(&dt, tokyo_tz);

// Check if datetime is in DST
let is_summer_time = timezone::is_dst(&dt);
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

Built with these excellent crates:
- `chrono` and `chrono-tz` for date/time handling
- `rrule` for recurrence rule support  
- `icalendar` for ICS format compatibility
