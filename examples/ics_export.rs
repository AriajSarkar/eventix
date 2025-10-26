//! Example demonstrating ICS (iCalendar) import and export

use eventix::{Calendar, Event, Recurrence};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("=== eventix ICS Import/Export Example ===\n");

    // Create a calendar with various events
    let mut cal = Calendar::new("Work Calendar")
        .description("My work schedule and meetings");

    // Add a one-time event
    let presentation = Event::builder()
        .title("Client Presentation")
        .description("Present Q4 results to the client")
        .start("2025-11-15 14:00:00", "America/New_York")
        .duration_hours(2)
        .attendee("client@example.com")
        .attendee("manager@example.com")
        .location("Board Room")
        .uid("presentation-2025-11-15@mycompany.com")
        .build()?;

    cal.add_event(presentation);

    // Add a recurring event
    let standup = Event::builder()
        .title("Daily Standup")
        .description("Quick team sync")
        .start("2025-11-01 09:15:00", "America/New_York")
        .duration_minutes(15)
        .recurrence(Recurrence::daily().count(20))
        .skip_weekends(true)
        .attendee("team@example.com")
        .location("Conference Room B")
        .build()?;

    cal.add_event(standup);

    // Add a weekly recurring meeting
    let planning = Event::builder()
        .title("Sprint Planning")
        .description("Plan the upcoming sprint")
        .start("2025-11-04 13:00:00", "America/New_York")
        .duration_hours(2)
        .recurrence(Recurrence::weekly().interval(2).count(6))
        .location("Zoom Meeting")
        .build()?;

    cal.add_event(planning);

    // Add a monthly meeting
    let review = Event::builder()
        .title("Monthly Business Review")
        .description("Review monthly metrics and KPIs")
        .start("2025-11-01 16:00:00", "America/New_York")
        .duration_hours(1)
        .recurrence(Recurrence::monthly().count(12))
        .location("Executive Conference Room")
        .build()?;

    cal.add_event(review);

    println!("Created calendar '{}' with {} events\n", cal.name, cal.event_count());

    // Export to ICS file
    let ics_path = "work_calendar.ics";
    println!("Exporting to {}...", ics_path);
    cal.export_to_ics(ics_path)?;
    println!("✓ Export successful!\n");

    // Display the ICS content
    println!("=== ICS File Content (preview) ===");
    let ics_content = cal.to_ics_string()?;
    let preview_lines: Vec<&str> = ics_content.lines().take(30).collect();
    println!("{}", preview_lines.join("\n"));
    println!("... (truncated) ...\n");

    // Import the calendar back
    println!("=== Importing from ICS ===");
    let imported_cal = Calendar::import_from_ics(ics_path)?;
    
    println!("Imported calendar: {}", imported_cal.name);
    if let Some(desc) = &imported_cal.description {
        println!("Description: {}", desc);
    }
    println!("Number of events imported: {}\n", imported_cal.event_count());

    // Display imported events
    println!("=== Imported Events ===");
    for (i, event) in imported_cal.get_events().iter().enumerate() {
        println!("\n{}. {}", i + 1, event.title);
        if let Some(desc) = &event.description {
            println!("   Description: {}", desc);
        }
        println!("   Start: {}", event.start_time.format("%Y-%m-%d %H:%M %Z"));
        println!("   End: {}", event.end_time.format("%Y-%m-%d %H:%M %Z"));
        
        if let Some(loc) = &event.location {
            println!("   Location: {}", loc);
        }
        
        if !event.attendees.is_empty() {
            println!("   Attendees: {}", event.attendees.join(", "));
        }
        
        if let Some(uid) = &event.uid {
            println!("   UID: {}", uid);
        }
    }

    // Verify the calendar can be used normally
    println!("\n=== Working with Imported Calendar ===");
    let found = imported_cal.find_events_by_title("standup");
    println!("Found {} event(s) matching 'standup'", found.len());
    
    // Clean up
    println!("\n=== Cleanup ===");
    if Path::new(ics_path).exists() {
        std::fs::remove_file(ics_path)?;
        println!("✓ Removed temporary ICS file");
    }

    println!("\n=== Alternative: Working with ICS Strings ===");
    
    // You can also work with ICS strings directly without files
    let ics_string = imported_cal.to_ics_string()?;
    println!("ICS string length: {} bytes", ics_string.len());
    
    // Parse from string
    let cal_from_string = Calendar::from_ics_string(&ics_string)?;
    println!("Successfully parsed calendar from string");
    println!("Events: {}", cal_from_string.event_count());

    println!("\n✓ ICS import/export example completed!");

    Ok(())
}
