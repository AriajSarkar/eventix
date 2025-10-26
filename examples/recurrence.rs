//! Example demonstrating various recurrence patterns

use eventix::{Calendar, Event, Recurrence, timezone};

fn main() -> anyhow::Result<()> {
    println!("=== eventix Recurrence Example ===\n");

    let mut cal = Calendar::new("Recurring Events Calendar");

    // Daily recurrence - Morning routine
    println!("1. Daily Recurrence (30 days)");
    let daily_event = Event::builder()
        .title("Morning Exercise")
        .description("Daily workout routine")
        .start("2025-11-01 06:30:00", "America/New_York")
        .duration_hours(1)
        .recurrence(Recurrence::daily().count(30))
        .skip_weekends(true) // Skip weekends
        .build()?;

    cal.add_event(daily_event);

    // Weekly recurrence - Team meeting
    println!("2. Weekly Recurrence (10 weeks)");
    let weekly_event = Event::builder()
        .title("Weekly Team Meeting")
        .description("Discuss project progress and blockers")
        .start("2025-11-03 10:00:00", "America/New_York") // Monday
        .duration_hours(1)
        .recurrence(Recurrence::weekly().count(10))
        .attendee("team@example.com")
        .location("Zoom")
        .build()?;

    cal.add_event(weekly_event);

    // Bi-weekly recurrence - Sprint planning
    println!("3. Bi-weekly Recurrence (6 sprints)");
    let biweekly_event = Event::builder()
        .title("Sprint Planning")
        .description("Plan work for the next 2-week sprint")
        .start("2025-11-03 14:00:00", "America/New_York")
        .duration_hours(2)
        .recurrence(Recurrence::weekly().interval(2).count(6))
        .build()?;

    cal.add_event(biweekly_event);

    // Monthly recurrence - All-hands meeting
    println!("4. Monthly Recurrence (12 months)");
    let monthly_event = Event::builder()
        .title("Monthly All-Hands Meeting")
        .description("Company-wide updates and announcements")
        .start("2025-11-05 15:00:00", "America/New_York")
        .duration_hours(1)
        .recurrence(Recurrence::monthly().count(12))
        .location("Main Auditorium")
        .build()?;

    cal.add_event(monthly_event);

    // Yearly recurrence - Annual review
    println!("5. Yearly Recurrence (5 years)");
    let yearly_event = Event::builder()
        .title("Annual Performance Review")
        .start("2025-12-15 09:00:00", "America/New_York")
        .duration_hours(2)
        .recurrence(Recurrence::yearly().count(5))
        .build()?;

    cal.add_event(yearly_event);

    // Event with exception dates
    println!("6. Event with Exception Dates");
    let tz = timezone::parse_timezone("America/New_York")?;
    let thanksgiving = timezone::parse_datetime_with_tz("2025-11-27 10:00:00", tz)?;
    let christmas = timezone::parse_datetime_with_tz("2025-12-25 10:00:00", tz)?;

    let event_with_exceptions = Event::builder()
        .title("Daily Standup (No Holidays)")
        .start("2025-11-01 10:00:00", "America/New_York")
        .duration_minutes(15)
        .recurrence(Recurrence::daily().count(60))
        .skip_weekends(true)
        .exception_date(thanksgiving)
        .exception_date(christmas)
        .build()?;

    cal.add_event(event_with_exceptions);

    // Display occurrences for one of the recurring events
    println!("\n=== Sample Occurrences ===");
    let sample_event = &cal.get_events()[0]; // Daily exercise
    
    let start_date = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz)?;
    let end_date = timezone::parse_datetime_with_tz("2025-11-15 23:59:59", tz)?;
    
    let occurrences = sample_event.occurrences_between(start_date, end_date, 50)?;
    
    println!("\n'{}' occurrences (first 15 days):", sample_event.title);
    for (i, occurrence) in occurrences.iter().enumerate().take(15) {
        println!("  {}. {}", i + 1, occurrence.format("%Y-%m-%d %A %H:%M"));
    }

    // Show calendar summary
    println!("\n=== Calendar Summary ===");
    println!("Total recurring event patterns: {}", cal.event_count());
    
    // Calculate total occurrences in November 2025
    let nov_start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz)?;
    let nov_end = timezone::parse_datetime_with_tz("2025-11-30 23:59:59", tz)?;
    
    let all_occurrences = cal.events_between(nov_start, nov_end)?;
    println!("Total occurrences in November 2025: {}", all_occurrences.len());

    // Display events by date
    println!("\n=== Events on November 5, 2025 ===");
    let specific_date = timezone::parse_datetime_with_tz("2025-11-05 00:00:00", tz)?;
    let events_on_date = cal.events_on_date(specific_date)?;
    
    for occurrence in events_on_date {
        println!("  - {} at {}", 
            occurrence.title(), 
            occurrence.occurrence_time.format("%H:%M"));
    }

    Ok(())
}
