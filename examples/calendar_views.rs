//! Demonstrates lazy calendar day/week view iteration.

use eventix::{timezone, Calendar, Duration, Event, EventStatus, Recurrence};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut calendar = Calendar::new("Personal Planner");

    calendar.add_event(
        Event::builder()
            .title("Design Review")
            .start("2025-11-03 09:00:00", "America/New_York")
            .duration_hours(1)
            .location("Room A")
            .build()?,
    );

    calendar.add_event(
        Event::builder()
            .title("Daily Standup")
            .start("2025-11-04 10:00:00", "America/New_York")
            .duration(Duration::minutes(15))
            .recurrence(Recurrence::daily().count(10))
            .build()?,
    );

    calendar.add_event(
        Event::builder()
            .title("Draft Blog Post")
            .start("2025-11-05 13:00:00", "America/New_York")
            .duration_minutes(45)
            .status(EventStatus::Tentative)
            .build()?,
    );

    let tz = timezone::parse_timezone("America/New_York")?;
    let start = timezone::parse_datetime_with_tz("2025-11-03 00:00:00", tz)?;

    println!("Next 7 days:");
    for day in calendar.days(start).take(7) {
        let day = day?;
        println!("{} -> {} events", day.date(), day.event_count());
        for occurrence in day.events() {
            println!(
                "  - {} at {} ({:?})",
                occurrence.title(),
                occurrence.occurrence_time.format("%H:%M %Z"),
                occurrence.status
            );
        }
    }

    println!("\nNext 2 weeks:");
    for week in calendar.weeks(start).take(2) {
        let week = week?;
        println!(
            "Week {} to {} -> {} events",
            week.start_date(),
            week.end_date(),
            week.event_count()
        );
        for day in week.days() {
            println!("  {} -> {} events", day.date(), day.event_count());
        }
    }

    // A UI layer can map DayView values directly into components.
    let busy_day_titles: Vec<_> = calendar
        .days(start)
        .take(14)
        .collect::<eventix::Result<Vec<_>>>()?
        .into_iter()
        .filter(|day| !day.is_empty())
        .map(|day| {
            let titles = day
                .events()
                .iter()
                .map(|event| event.title().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} => {}", day.date(), titles)
        })
        .collect();

    println!("\nBusy days:");
    for line in busy_day_titles {
        println!("  {line}");
    }

    Ok(())
}
