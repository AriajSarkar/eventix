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
//! - **ICS support**: Import and export events using the iCalendar format
//! - **Builder API**: Ergonomic, fluent interface for creating events and calendars
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

pub mod calendar;
pub mod event;
pub mod recurrence;
pub mod ics;
pub mod timezone;
pub mod gap_validation;

mod error;

pub use calendar::Calendar;
pub use event::{Event, EventBuilder};
pub use recurrence::Recurrence;
pub use error::{EventixError, Result};

// Re-export commonly used types
pub use chrono::{DateTime, Duration, NaiveDateTime};
pub use chrono_tz::Tz;
