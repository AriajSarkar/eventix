//! Example demonstrating gap validation and schedule analysis features
//!
//! This showcases unique features not found in other calendar crates

use eventix::{gap_validation, timezone, Calendar, Duration, Event, Recurrence};

fn main() -> anyhow::Result<()> {
    println!("=== eventix Gap Validation & Schedule Analysis ===\n");

    // Create a realistic work schedule
    let mut cal = Calendar::new("Work Week Schedule");

    // Monday schedule
    let events = vec![
        ("Daily Standup", "09:00:00", 15),
        ("Deep Work Session", "09:30:00", 90),
        ("Code Review", "11:00:00", 60),
        ("Lunch Break", "12:30:00", 60),
        ("Client Call", "14:00:00", 45),
        ("Team Planning", "15:30:00", 60),
        ("Wrap-up", "17:00:00", 30),
    ];

    for (title, time, duration_mins) in events {
        let event = Event::builder()
            .title(title)
            .start(&format!("2025-11-03 {}", time), "America/New_York")
            .duration_minutes(duration_mins)
            .build()?;
        cal.add_event(event);
    }

    let tz = timezone::parse_timezone("America/New_York")?;
    let work_start = timezone::parse_datetime_with_tz("2025-11-03 08:00:00", tz)?;
    let work_end = timezone::parse_datetime_with_tz("2025-11-03 18:00:00", tz)?;

    // 1. Find all gaps in the schedule
    println!("=== 1. Gap Detection ===");
    let gaps = gap_validation::find_gaps(&cal, work_start, work_end, Duration::minutes(15))?;

    println!("Found {} gaps of at least 15 minutes:\n", gaps.len());
    for (i, gap) in gaps.iter().enumerate() {
        println!("  Gap {}:", i + 1);
        println!("    Time: {} to {}", gap.start.format("%H:%M"), gap.end.format("%H:%M"));
        println!(
            "    Duration: {} minutes ({:.1} hours)",
            gap.duration_minutes(),
            gap.duration_hours() as f64 + (gap.duration_minutes() % 60) as f64 / 60.0
        );
        if let Some(ref before) = gap.before_event {
            println!("    After: {}", before);
        }
        if let Some(ref after) = gap.after_event {
            println!("    Before: {}", after);
        }
        println!();
    }

    // 2. Calculate schedule density
    println!("=== 2. Schedule Density Analysis ===");
    let density = gap_validation::calculate_density(&cal, work_start, work_end)?;

    println!("Schedule Metrics:");
    println!("  Total Work Hours: {:.1}", density.total_duration.num_minutes() as f64 / 60.0);
    println!("  Busy Time: {:.1} hours", density.busy_duration.num_minutes() as f64 / 60.0);
    println!("  Free Time: {:.1} hours", density.free_duration.num_minutes() as f64 / 60.0);
    println!("  Occupancy: {:.1}%", density.occupancy_percentage);
    println!("  Total Events: {}", density.event_count);
    println!("  Gaps Found: {}", density.gap_count);
    println!("  Conflicts: {}", density.overlap_count);

    if density.is_busy() {
        println!("  ⚠️  This is a BUSY schedule (>60% occupied)");
    } else if density.is_light() {
        println!("  ✓ This is a LIGHT schedule (<30% occupied)");
    } else {
        println!("  ✓ This is a MODERATE schedule");
    }
    println!();

    // 3. Find the longest available gap
    println!("=== 3. Longest Available Time Slot ===");
    if let Some(longest) = gap_validation::find_longest_gap(&cal, work_start, work_end)? {
        println!("Longest available slot:");
        println!("  Time: {} to {}", longest.start.format("%H:%M"), longest.end.format("%H:%M"));
        println!("  Duration: {:.1} hours", longest.duration_minutes() as f64 / 60.0);
        println!("  Perfect for: Deep work, focused tasks, or important meetings\n");
    }

    // 4. Find available slots for a specific meeting duration
    println!("=== 4. Available Slots for 1-Hour Meeting ===");
    let one_hour_slots =
        gap_validation::find_available_slots(&cal, work_start, work_end, Duration::hours(1))?;

    if one_hour_slots.is_empty() {
        println!("No 1-hour slots available!");
    } else {
        println!("Found {} slots for 1-hour meeting:\n", one_hour_slots.len());
        for (i, slot) in one_hour_slots.iter().enumerate() {
            println!(
                "  Option {}: {} - {} ({:.1} hours available)",
                i + 1,
                slot.start.format("%H:%M"),
                slot.end.format("%H:%M"),
                slot.duration_hours() as f64 + (slot.duration_minutes() % 60) as f64 / 60.0
            );
        }
    }
    println!();

    // 5. Check if specific slot is available
    println!("=== 5. Slot Availability Check ===");
    let check_start = timezone::parse_datetime_with_tz("2025-11-03 13:30:00", tz)?;
    let check_end = timezone::parse_datetime_with_tz("2025-11-03 14:30:00", tz)?;

    println!("Checking if 1:30 PM - 2:30 PM is available...");
    if gap_validation::is_slot_available(&cal, check_start, check_end)? {
        println!("  ✓ Slot is AVAILABLE");
    } else {
        println!("  ✗ Slot has CONFLICTS");
    }
    println!();

    // 6. Demonstrate overlap detection
    println!("=== 6. Overlap Detection ===");

    // Add a conflicting event
    cal.add_event(
        Event::builder()
            .title("Emergency Meeting")
            .start("2025-11-03 14:15:00", "America/New_York")
            .duration_minutes(30)
            .build()?,
    );

    let overlaps = gap_validation::find_overlaps(&cal, work_start, work_end)?;

    if overlaps.is_empty() {
        println!("No scheduling conflicts detected!");
    } else {
        println!("⚠️  Found {} conflict(s):\n", overlaps.len());
        for (i, overlap) in overlaps.iter().enumerate() {
            println!("  Conflict {}:", i + 1);
            println!(
                "    Time: {} to {}",
                overlap.start.format("%H:%M"),
                overlap.end.format("%H:%M")
            );
            println!("    Duration: {} minutes", overlap.duration_minutes());
            println!("    Conflicting events:");
            for event in &overlap.events {
                println!("      - {}", event);
            }
            println!();
        }
    }

    // 7. Suggest alternative times for conflicting event
    println!("=== 7. Conflict Resolution Suggestions ===");
    let conflict_time = timezone::parse_datetime_with_tz("2025-11-03 14:15:00", tz)?;

    println!("Requested time has conflicts. Suggesting alternatives...\n");
    let alternatives = gap_validation::suggest_alternatives(
        &cal,
        conflict_time,
        Duration::minutes(30),
        Duration::hours(2),
    )?;

    if alternatives.is_empty() {
        println!("No alternative times found in nearby time window.");
    } else {
        println!("Available alternative times:");
        for (i, alt_time) in alternatives.iter().take(5).enumerate() {
            println!("  {}. {} (30 minutes)", i + 1, alt_time.format("%I:%M %p"));
        }
    }
    println!();

    // 8. Analyze schedule with recurring events
    println!("=== 8. Recurring Events Analysis ===");
    let mut recurring_cal = Calendar::new("Weekly Schedule");

    recurring_cal.add_event(
        Event::builder()
            .title("Weekly Team Sync")
            .start("2025-11-03 10:00:00", "America/New_York")
            .duration_hours(1)
            .recurrence(Recurrence::weekly().count(4))
            .build()?,
    );

    let month_start = timezone::parse_datetime_with_tz("2025-11-03 00:00:00", tz)?;
    let month_end = timezone::parse_datetime_with_tz("2025-11-30 23:59:59", tz)?;

    let recurring_density =
        gap_validation::calculate_density(&recurring_cal, month_start, month_end)?;

    println!("Monthly recurring event analysis:");
    println!("  Recurring Events: {}", recurring_density.event_count);
    println!(
        "  Total Busy Time: {:.1} hours",
        recurring_density.busy_duration.num_minutes() as f64 / 60.0
    );
    println!(
        "  Average per week: {:.1} hours",
        recurring_density.busy_duration.num_minutes() as f64 / 60.0 / 4.0
    );

    println!("\n✓ Gap validation and schedule analysis complete!");

    Ok(())
}
