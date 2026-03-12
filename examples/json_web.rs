//! JSON Serialization Example
//!
//! Shows how to export and import calendars as JSON.
//! Great for REST APIs, web apps, and database storage.
//!
//! Run with: cargo run --example json_web

use eventix::{Calendar, Event, Utc};
use serde_json::json;

fn main() -> anyhow::Result<()> {
    println!("=== JSON Example ===\n");

    // ─────────────────────────────────────────────
    // 1. CREATE A CALENDAR
    // ─────────────────────────────────────────────
    let mut cal = Calendar::new("Team Schedule");

    cal.add_event(
        Event::builder()
            .title("Morning Standup")
            .start("2025-01-15 09:00:00", "UTC")
            .duration_minutes(15)
            .build()?,
    );

    cal.add_event(
        Event::builder()
            .title("Sprint Planning")
            .start("2025-01-15 14:00:00", "UTC")
            .duration_hours(2)
            .build()?,
    );

    // ─────────────────────────────────────────────
    // 2. EXPORT TO JSON
    // ─────────────────────────────────────────────
    println!("📤 Export to JSON:");
    let json_output = cal.to_json()?;
    println!("{}\n", json_output);

    // ─────────────────────────────────────────────
    // 3. IMPORT FROM JSON (like from a web form)
    // ─────────────────────────────────────────────
    println!("📥 Import from JSON:");

    let from_web = r#"
    {
        "name": "Client Calendar",
        "events": [{
            "title": "Client Demo",
            "start_time": "2025-02-01T14:00:00+00:00",
            "end_time": "2025-02-01T15:00:00+00:00",
            "timezone": "UTC",
            "status": "Confirmed",
            "attendees": ["client@example.com"],
            "description": null,
            "location": "Zoom",
            "uid": null
        }],
        "timezone": "UTC"
    }
    "#;

    let imported = Calendar::from_json(from_web)?;
    println!("Imported: '{}' with {} event(s)", imported.name, imported.event_count());

    // ─────────────────────────────────────────────
    // 4. BUILD API RESPONSE (using serde_json::json!)
    // ─────────────────────────────────────────────
    println!("\n🌐 API Response:");

    let response = json!({
        "ok": true,
        "calendar_name": cal.name,
        "event_count": cal.event_count(),
        "timestamp": Utc::now().to_rfc3339()
    });

    println!("{}", serde_json::to_string_pretty(&response)?);

    println!("\n✅ Done!");
    Ok(())
}
