//! Error types for the eventix library

use thiserror::Error;

/// Result type alias for eventix operations
pub type Result<T> = std::result::Result<T, EventixError>;

/// Error types that can occur in eventix operations
#[derive(Error, Debug)]
pub enum EventixError {
    /// Error parsing date/time strings
    #[error("Failed to parse date/time: {0}")]
    DateTimeParse(String),

    /// Error parsing timezone
    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),

    /// Error with recurrence rules
    #[error("Recurrence error: {0}")]
    RecurrenceError(String),

    /// Error during ICS operations
    #[error("ICS error: {0}")]
    IcsError(String),

    /// Error with event validation
    #[error("Event validation error: {0}")]
    ValidationError(String),

    /// IO errors
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}
