//! Basic example showing how to create a calendar and add simple events

use eventix::{Calendar, Event};

fn main() -> anyhow::Result<()> {
    println!("=== eventix Basic Example ===\n");

    // Create a new calendar
    let mut cal = Calendar::new("My Personal Calendar")
        .description("A calendar for tracking my daily activities");

    // Create a simple one-time event
    let meeting = Event::builder()
        .title("Team Standup")
        .description("Daily team sync meeting")
        .start("2025-11-01 09:00:00", "America/New_York")
        .duration_minutes(15)
        .attendee("alice@example.com")
        .attendee("bob@example.com")
        .location("Conference Room A")
        .build()?;

    cal.add_event(meeting);

    // Create another event
    let lunch = Event::builder()
        .title("Lunch with Client")
        .start("2025-11-01 12:00:00", "America/New_York")
        .duration_hours(1)
        .location("Downtown Restaurant")
        .build()?;

    cal.add_event(lunch);

    // Create an afternoon event
    let review = Event::builder()
        .title("Code Review Session")
        .description("Review PRs from this week")
        .start("2025-11-01 15:00:00", "America/New_York")
        .duration_hours(2)
        .attendee("developer1@example.com")
        .attendee("developer2@example.com")
        .build()?;

    cal.add_event(review);

    // Display calendar info
    println!("Calendar: {}", cal.name);
    if let Some(desc) = &cal.description {
        println!("Description: {}", desc);
    }
    println!("Total events: {}\n", cal.event_count());

    // List all events
    println!("=== Events ===");
    for (i, event) in cal.get_events().iter().enumerate() {
        println!("\n{}. {}", i + 1, event.title);
        if let Some(desc) = &event.description {
            println!("   Description: {}", desc);
        }
        println!("   Start: {}", event.start_time.format("%Y-%m-%d %H:%M:%S %Z"));
        println!("   End: {}", event.end_time.format("%Y-%m-%d %H:%M:%S %Z"));
        println!("   Duration: {} minutes", event.duration().num_minutes());
        
        if let Some(loc) = &event.location {
            println!("   Location: {}", loc);
        }
        
        if !event.attendees.is_empty() {
            println!("   Attendees: {}", event.attendees.join(", "));
        }
    }

    // Search for events
    println!("\n=== Search Results ===");
    let found = cal.find_events_by_title("review");
    println!("Found {} event(s) matching 'review':", found.len());
    for event in found {
        println!("  - {}", event.title);
    }

    // Export to JSON
    println!("\n=== JSON Export ===");
    let json = cal.to_json()?;
    println!("{}", json);

    Ok(())
}
