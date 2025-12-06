//! # eventix
//!
//! A high-level calendar and recurrence library for Rust with timezone-aware scheduling,
//! exceptions, and ICS import/export capabilities.
//!
//! ## Features
//!
//! - **Timezone-aware events**: All date/time fields use `chrono` with `chrono-tz` for proper timezone handling
//! - **Recurrence rules**: Support for daily, weekly, monthly, and yearly recurrence patterns
//! - **Exceptions**: Skip specific dates or apply custom filters (e.g., skip weekends)
//! - **ICS support**: Import and export events using the iCalendar format (RFC 5545 compliant with TZID support)
//! - **Builder API**: Ergonomic, fluent interface for creating events and calendars
//! - **Gap validation**: Find gaps between events, detect conflicts, and analyze schedule density
//! - **Schedule analysis**: Unique features for occupancy metrics, availability finding, and conflict resolution
//!
//! ## Quick Start
//!
//! ```rust
//! use eventix::{Calendar, Event, Recurrence};
//!
//! let mut cal = Calendar::new("My Calendar");
//!
//! let event = Event::builder()
//!     .title("Weekly Team Meeting")
//!     .description("Discuss project progress")
//!     .start("2025-11-01 10:00:00", "America/New_York")
//!     .duration_hours(1)
//!     .recurrence(Recurrence::weekly().count(10))
//!     .build()
//!     .expect("Failed to build event");
//!
//! cal.add_event(event);
//! ```
//!
//! ## Timezone-Aware ICS Export
//!
//! Events are exported with proper timezone information for compatibility with calendar applications:
//!
//! ```rust
//! use eventix::{Calendar, Event};
//!
//! let mut cal = Calendar::new("Work Schedule");
//!
//! // Non-UTC timezones include TZID parameter
//! let event = Event::builder()
//!     .title("Team Meeting")
//!     .start("2025-10-27 10:00:00", "America/New_York")
//!     .duration_hours(1)
//!     .build()
//!     .unwrap();
//!
//! cal.add_event(event);
//! cal.export_to_ics("schedule.ics").unwrap();
//!
//! // Generates: DTSTART;TZID=America/New_York:20251027T100000
//! // Compatible with Google Calendar, Outlook, and Apple Calendar
//! ```
//!
//! ## Schedule Analysis (Unique Feature)
//!
//! Find gaps, detect conflicts, and analyze schedule density:
//!
//! ```rust
//! use eventix::{Calendar, Event, gap_validation, timezone, Duration};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut cal = Calendar::new("Work Schedule");
//!
//! // Add some events...
//! let event1 = Event::builder()
//!     .title("Morning Meeting")
//!     .start("2025-11-03 09:00:00", "America/New_York")
//!     .duration_hours(1)
//!     .build()?;
//!
//! let event2 = Event::builder()
//!     .title("Afternoon Call")
//!     .start("2025-11-03 14:00:00", "America/New_York")
//!     .duration_hours(1)
//!     .build()?;
//!
//! cal.add_event(event1);
//! cal.add_event(event2);
//!
//! // Find gaps in schedule
//! let tz = timezone::parse_timezone("America/New_York")?;
//! let start = timezone::parse_datetime_with_tz("2025-11-03 08:00:00", tz)?;
//! let end = timezone::parse_datetime_with_tz("2025-11-03 18:00:00", tz)?;
//!
//! let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(30))?;
//! println!("Found {} gaps of at least 30 minutes", gaps.len());
//!
//! // Calculate schedule density
//! let density = gap_validation::calculate_density(&cal, start, end)?;
//! println!("Schedule occupancy: {:.1}%", density.occupancy_percentage);
//! # Ok(())
//! # }
//! ```
//!
//! ## Modules
//!
//! - [`calendar`] - Calendar container for managing collections of events
//! - [`event`] - Event types and builder API
//! - [`gap_validation`] - Schedule analysis, gap detection, and conflict resolution (unique feature)
//! - [`ics`] - ICS (iCalendar) import/export with TZID support
//! - [`recurrence`] - Recurrence patterns (daily, weekly, monthly, yearly)
//! - [`timezone`] - Timezone utilities with DST awareness
//!
//! ## Examples
//!
//! See the `examples/` directory for more comprehensive examples:
//! - `basic.rs` - Simple calendar creation and event management
//! - `recurrence.rs` - Daily, weekly, monthly, and yearly recurrence patterns
//! - `ics_export.rs` - ICS import/export functionality
//! - `timezone_ics_export.rs` - Timezone-aware ICS export demonstration
//! - `gap_validation.rs` - Schedule analysis and gap detection features

pub mod calendar;
pub mod event;
pub mod gap_validation;
pub mod ics;
pub mod recurrence;
pub mod timezone;

mod error;

pub use calendar::Calendar;
pub use error::{EventixError, Result};
pub use event::{Event, EventBuilder};
pub use recurrence::Recurrence;

// Re-export commonly used types
pub use chrono::{DateTime, Duration, NaiveDateTime};
pub use chrono_tz::Tz;
