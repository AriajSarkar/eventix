use eventix::{Calendar, Event, EventStatus};

pub fn run() -> eventix::Result<()> {
    println!("\n=== 📅 Booking Workflow Example ===");

    let mut cal = Calendar::new("Office Room A");

    // 1. Create a Tentative Booking
    println!("1. Creating tentative booking for Client Meeting...");
    let mut meeting = Event::builder()
        .title("Client Meeting")
        .start("2025-11-10 10:00:00", "UTC")
        .duration_hours(1)
        .status(EventStatus::Tentative)
        .build()?;

    println!("   Status: {:?}", meeting.status);
    cal.add_event(meeting.clone());

    // 2. Confirm the Booking
    println!("\n2. Confirming the booking...");
    // Retrieve the event (mutable borrow would happen in a real app, here we simulate)
    meeting.confirm();
    println!("   Status: {:?}", meeting.status);

    // Update in calendar (remove old, add new for demo simplicity)
    cal.clear_events();
    cal.add_event(meeting.clone());

    // 3. Cancel the Booking
    println!("\n3. Cancelling the booking...");
    meeting.cancel();
    println!("   Status: {:?}", meeting.status);

    // Update in calendar
    cal.clear_events();
    cal.add_event(meeting);

    // 4. Verify Gap Validation
    println!("\n4. Verifying availability...");
    let start = eventix::timezone::parse_datetime_with_tz(
        "2025-11-10 09:00:00",
        eventix::timezone::parse_timezone("UTC")?,
    )?;
    let end = eventix::timezone::parse_datetime_with_tz(
        "2025-11-10 12:00:00",
        eventix::timezone::parse_timezone("UTC")?,
    )?;

    let density = eventix::gap_validation::calculate_density(&cal, start, end)?;

    println!(
        "   Occupancy: {}% (Should be 0% as event is Cancelled)",
        density.occupancy_percentage
    );

    if density.occupancy_percentage == 0.0 {
        println!("   ✅ SUCCESS: Cancelled event does not block time.");
    } else {
        println!("   ❌ FAILURE: Cancelled event is blocking time!");
    }

    Ok(())
}
