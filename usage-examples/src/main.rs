//! Example usage of the published eventix crate
//!
//! This demonstrates using eventix as a dependency from crates.io

use eventix::{gap_validation, timezone, Calendar, Duration, Event, Recurrence};

mod booking_workflow;

fn main() -> eventix::Result<()> {
    println!("=== Using Published Eventix Crate ===\n");

    // Create a calendar
    let mut cal = Calendar::new("My Work Calendar");

    // Create a simple event
    let meeting = Event::builder()
        .title("Team Standup")
        .description("Daily team sync")
        .start("2025-10-27 09:00:00", "America/New_York")
        .duration_minutes(15)
        .build()?;

    cal.add_event(meeting);

    // Create a recurring event
    let weekly_meeting = Event::builder()
        .title("Weekly Planning")
        .start("2025-10-28 14:00:00", "America/New_York")
        .duration_hours(1)
        .recurrence(Recurrence::weekly().count(4))
        .skip_weekends(true)
        .build()?;

    cal.add_event(weekly_meeting);

    println!("✅ Created calendar with {} events", cal.event_count());

    // Find gaps in schedule
    let tz = timezone::parse_timezone("America/New_York")?;
    let start = timezone::parse_datetime_with_tz("2025-10-27 08:00:00", tz)?;
    let end = timezone::parse_datetime_with_tz("2025-10-27 18:00:00", tz)?;

    let gaps = gap_validation::find_gaps(&cal, start, end, Duration::minutes(30))?;
    println!("📅 Found {} gaps of at least 30 minutes", gaps.len());

    // Export to ICS
    let output_dir = std::path::Path::new("examples_output");
    if !output_dir.exists() {
        std::fs::create_dir(output_dir)?;
    }

    cal.export_to_ics("examples_output/schedule.ics")?;
    println!("💾 Exported calendar to examples_output/schedule.ics");

    booking_workflow::run()?;

    Ok(())
}
