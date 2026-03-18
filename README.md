# Eventix 📅

A high-level calendar and recurrence library for Rust with timezone-aware scheduling, exceptions, ICS import/export, and lazy calendar views.

[![Crates.io](https://img.shields.io/crates/v/eventix.svg)](https://crates.io/crates/eventix)
[![Documentation](https://docs.rs/eventix/badge.svg)](https://docs.rs/eventix)
[![CI](https://github.com/AriajSarkar/eventix/workflows/EventixCI/badge.svg)](https://github.com/AriajSarkar/eventix/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/AriajSarkar/eventix?utm_source=oss&utm_medium=github&utm_campaign=AriajSarkar%2Feventix&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews)

## Features

- 🌍 **Timezone-aware events** - Full support for timezones and DST handling using `chrono-tz`
- 🔄 **Recurrence patterns** - All seven RFC 5545 frequencies (secondly, minutely, hourly, daily, weekly, monthly, yearly) with advanced rules
- 🗓️ **Calendar view iterators** - Lazy day/week traversal for UI rendering and infinite-scroll agendas
- 🚫 **Exception handling** - Skip specific dates, weekends, or custom holiday lists
- 🚦 **Booking workflow** - Manage event status (`Confirmed`, `Tentative`, `Cancelled`) with smart gap validation
- 📅 **ICS support** - Import and export events using the iCalendar (`.ics`) format
- 🛠️ **Builder API** - Ergonomic, fluent interface for creating events and calendars
- 🔍 **Gap validation** - Find gaps between events, detect conflicts, analyze schedule density
- 📊 **Schedule analysis** - Occupancy metrics, conflict detection, availability finding
- ✅ **Type-safe** - Leverages Rust's type system for correctness

## Why Eventix?

| Feature | `eventix` | `icalendar` | `chrono` |
|---------|-----------|-------------|----------|
| **Primary Goal** | Booking & Scheduling | File Parsing | Date/Time Math |
| **Gap Finding** | ✅ Native Support | ❌ Manual Logic | ❌ Manual Logic |
| **Booking State** | ✅ Confirmed/Cancelled | ❌ No Concept | ❌ No Concept |
| **Timezone/DST** | ✅ Built-in (`chrono-tz`) | ⚠️ Partial | ✅ Built-in |
| **Recurrence** | ✅ RRule + Exdates | ✅ RRule | ❌ None |
| **View Iterators** | ✅ Day/Week lazy APIs | ❌ Manual Grouping | ❌ Manual Logic |

## Quick Start

Add eventix to your `Cargo.toml`:

```toml
[dependencies]
eventix = "0.5.0"
```

### Basic Usage

```rust
use eventix::{Calendar, Duration, Event, Recurrence};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a calendar
    let mut cal = Calendar::new("My Calendar");

    // Create a simple event
    let meeting = Event::builder()
        .title("Team Meeting")
        .description("Weekly sync with the team")
        .start("2025-11-01 10:00:00", "America/New_York")
        .duration(Duration::hours(1))
        .attendee("alice@example.com")
        .attendee("bob@example.com")
        .build()?;

    cal.add_event(meeting);

    // Create a recurring event
    let standup = Event::builder()
        .title("Daily Standup")
        .start("2025-11-01 09:00:00", "America/New_York")
        .duration(Duration::minutes(15))
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
use eventix::{Duration, Event, Recurrence, timezone};
let tz = timezone::parse_timezone("America/New_York")?;
let holiday = timezone::parse_datetime_with_tz("2025-11-27 09:00:00", tz)?;

let event = Event::builder()
    .title("Morning Standup")
    .start("2025-11-01 09:00:00", "America/New_York")
    .duration(Duration::minutes(15))
    .recurrence(Recurrence::daily().count(30))
    .skip_weekends(true)
    .exception_date(holiday)  // Skip Thanksgiving
    .build()?;
```

### Weekly Recurrence

```rust
use eventix::{Duration, Event, Recurrence};
let event = Event::builder()
    .title("Weekly Team Meeting")
    .start("2025-11-03 14:00:00", "UTC")
    .duration(Duration::hours(1))
    .recurrence(Recurrence::weekly().count(10))
    .build()?;
```

### Monthly Recurrence

```rust
use eventix::{Event, Recurrence};

let event = Event::builder()
    .title("Monthly All-Hands")
    .start("2025-11-01 15:00:00", "America/Los_Angeles")
    .duration(Duration::hours(2))
    .recurrence(Recurrence::monthly().count(12))
    .build()?;
```

### Sub-daily Recurrence (Hourly, Minutely, Secondly)

Sub-daily frequencies advance by a fixed UTC duration. This gives **"same elapsed
time"** semantics — not "same local wall-clock slot." During a DST transition the
local-time label may shift (e.g. 1:00 AM → 3:00 AM when clocks spring forward)
but the actual interval between occurrences is always exact.

```rust
use eventix::{Duration, Event, Recurrence};

// Every 4 hours — e.g. 08:00, 12:00, 16:00, 20:00...
let reminder = Event::builder()
    .title("Medication Reminder")
    .start("2025-06-01 08:00:00", "America/New_York")
    .duration(Duration::minutes(5))
    .recurrence(Recurrence::hourly().interval(4).count(6))
    .build()?;

// Every 15 minutes — e.g. pomodoro timer
let pomo = Event::builder()
    .title("Pomodoro")
    .start("2025-06-01 09:00:00", "UTC")
    .duration(Duration::minutes(1))
    .recurrence(Recurrence::minutely().interval(15).count(8))
    .build()?;

// Every 30 seconds — e.g. health-check ping
let ping = Event::builder()
    .title("Health Check")
    .start("2025-06-01 12:00:00", "UTC")
    .duration(Duration::seconds(1))
    .recurrence(Recurrence::secondly().interval(30).count(10))
    .build()?;
```

### Lazy Occurrence Iteration

The `OccurrenceIterator` computes each occurrence on demand, making it ideal for
large or unbounded recurrence patterns. It supports standard iterator combinators
(`.take()`, `.filter()`, `.collect()`, etc.):

```rust
use eventix::{Recurrence, timezone};

let tz = timezone::parse_timezone("UTC")?;
let start = timezone::parse_datetime_with_tz("2025-06-01 10:00:00", tz)?;

let daily = Recurrence::daily().count(365);

// Take only the first 5 occurrences lazily
let first_five: Vec<_> = daily.occurrences(start).take(5).collect();

// Filter to Mondays only (chrono::Weekday)
let mondays: Vec<_> = daily.occurrences(start)
    .filter(|dt| dt.weekday() == chrono::Weekday::Mon)
    .take(10)
    .collect();
```

### Calendar View Iterators

For UI rendering, `Calendar::days()` and `Calendar::weeks()` lazily bucket
active occurrences into day/week views. This avoids choosing a large query
window up front and maps cleanly into frontend components. The iterators yield
`Result<DayView>` / `Result<WeekView>` so expansion errors stay explicit.

```rust
use eventix::{Calendar, Event, Recurrence, timezone};

let mut cal = Calendar::new("Personal");
cal.add_event(
    Event::builder()
        .title("Standup")
        .start("2025-11-04 10:00:00", "America/New_York")
        .duration_minutes(15)
        .recurrence(Recurrence::daily().count(10))
        .build()?
);

let tz = timezone::parse_timezone("America/New_York")?;
let start = timezone::parse_datetime_with_tz("2025-11-03 00:00:00", tz)?;

let busy_days: Vec<_> = cal.days(start)
    .take(14)
    .collect::<eventix::Result<Vec<_>>>()?
    .into_iter()
    .filter(|day| !day.is_empty())
    .collect();

for week in cal.weeks(start).take(2) {
    let week = week?;
    println!("{} -> {}", week.start_date(), week.event_count());
}
```

`DayView::start()` and `DayView::end()` expose the actual half-open day window
`[start, end)`, so `end()` is the next midnight. Use `end_inclusive()` only for
display formatting. Day and week views are built by interval intersection, so
overnight events appear on every day they overlap. If you're passing large
`DayView` values through Yew props, wrapping them in `Rc<DayView>` can avoid
expensive prop clones.

### Booking Status

```rust
use eventix::{Duration, Event, EventStatus};
let mut event = Event::builder()
    .title("Tentative Meeting")
    .start("2025-11-01 10:00:00", "UTC")
    .duration(Duration::hours(1))
    .status(EventStatus::Tentative)
    .build()?;

// Later, confirm the booking
event.confirm();

// Or cancel it (automatically ignored by gap validation)
event.cancel();
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
    .duration(Duration::hours(1))
    .build()?;

// Generates: DTSTART;TZID=America/New_York:20251027T100000

// UTC events use standard Z suffix
let utc_event = Event::builder()
    .title("Global Call")
    .start("2025-10-27 15:00:00", "UTC")
    .duration(Duration::hours(1))
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
use eventix::{Calendar, timezone};

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
use eventix::{Calendar, Event, Duration, gap_validation, timezone};

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

# Day/week calendar views
cargo run --example calendar_views

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
- **`views`** - Lazy day/week calendar view iterators
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
use eventix::timezone;

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
