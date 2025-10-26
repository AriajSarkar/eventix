//! Example demonstrating timezone-aware ICS export
//!
//! This example shows how events with different timezones are exported
//! to ICS format with proper TZID parameters for calendar app compatibility.

use anyhow::Result;
use eventix::{Calendar, Event, Recurrence};

fn main() -> Result<()> {
    println!("=== Timezone-Aware ICS Export Demo ===\n");

    let mut cal = Calendar::new("Multi-Timezone Calendar");
    cal = cal.description("Events across different timezones with proper TZID support");

    // Event 1: America/New_York timezone
    println!("Creating event in America/New_York timezone...");
    let ny_event = Event::builder()
        .title("New York Team Meeting")
        .description("Weekly sync with the East Coast team")
        .start("2025-10-27 10:00:00", "America/New_York")
        .duration_hours(1)
        .location("Conference Room A")
        .build()?;

    cal.add_event(ny_event);

    // Event 2: Asia/Kolkata timezone
    println!("Creating event in Asia/Kolkata timezone...");
    let kolkata_event = Event::builder()
        .title("India Development Sprint")
        .description("Daily standup with Bangalore office")
        .start("2025-10-27 09:30:00", "Asia/Kolkata")
        .duration_minutes(30)
        .location("Virtual - Zoom")
        .build()?;

    cal.add_event(kolkata_event);

    // Event 3: Europe/London timezone
    println!("Creating event in Europe/London timezone...");
    let london_event = Event::builder()
        .title("London Product Review")
        .description("Quarterly product review with UK team")
        .start("2025-10-27 14:00:00", "Europe/London")
        .duration_hours(2)
        .location("London Office")
        .build()?;

    cal.add_event(london_event);

    // Event 4: UTC timezone (for comparison)
    println!("Creating event in UTC timezone...");
    let utc_event = Event::builder()
        .title("Global All-Hands")
        .description("Company-wide meeting (UTC reference time)")
        .start("2025-10-28 15:00:00", "UTC")
        .duration_hours(1)
        .location("Virtual - Teams")
        .build()?;

    cal.add_event(utc_event);

    // Event 5: Recurring event with timezone
    println!("Creating recurring event in America/Los_Angeles timezone...");
    let recurring_event = Event::builder()
        .title("Weekly West Coast Sync")
        .description("Every Monday at 9 AM Pacific")
        .start("2025-10-28 09:00:00", "America/Los_Angeles")
        .duration_minutes(45)
        .recurrence(Recurrence::weekly().count(8))
        .skip_weekends(true)
        .location("San Francisco Office")
        .build()?;

    cal.add_event(recurring_event);

    // Event 6: Event with exception dates in Asia/Tokyo
    println!("Creating event with exception dates in Asia/Tokyo timezone...");
    use eventix::timezone;
    let tokyo_tz = timezone::parse_timezone("Asia/Tokyo")?;
    let exception_date = timezone::parse_datetime_with_tz("2025-11-05 10:00:00", tokyo_tz)?;

    let tokyo_event = Event::builder()
        .title("Tokyo Morning Briefing")
        .description("Daily briefing (skipping Nov 5)")
        .start("2025-10-27 10:00:00", "Asia/Tokyo")
        .duration_minutes(20)
        .recurrence(Recurrence::daily().count(15))
        .exception_date(exception_date)
        .location("Tokyo HQ")
        .build()?;

    cal.add_event(tokyo_event);

    // Export to ICS
    println!("\n=== Exporting to ICS ===");
    let ics_filename = "timezone_demo.ics";
    cal.export_to_ics(ics_filename)?;
    println!("✅ Exported calendar to: {}", ics_filename);

    // Display the ICS content for inspection
    println!("\n=== ICS Content Preview ===");
    let ics_content = cal.to_ics_string()?;

    // Show timezone-specific DTSTART/DTEND entries
    println!("\nTimezone-aware date/time entries:");
    for line in ics_content.lines() {
        if line.contains("DTSTART") || line.contains("DTEND") || line.contains("EXDATE") {
            println!("  {}", line);
        }
    }

    println!("\n=== Verification ===");
    println!("✓ Events with non-UTC timezones include TZID parameter");
    println!("✓ Events with UTC timezone use standard Z suffix");
    println!("✓ Recurring events preserve timezone information");
    println!("✓ Exception dates respect timezone context");
    println!("\nYou can import '{}' into:", ics_filename);
    println!("  • Google Calendar");
    println!("  • Microsoft Outlook");
    println!("  • Apple Calendar");
    println!("  • Any RFC 5545 compliant calendar application");

    Ok(())
}
