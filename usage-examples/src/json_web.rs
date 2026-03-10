//! JSON & Web Integration
//!
//! Simple examples for REST APIs and web frontends.

use eventix::{Calendar, Event, Utc};
use serde_json::{json, Value};

/// Import calendar from JSON (like from a POST request)
fn import_from_web() -> eventix::Result<Calendar> {
    let json = r#"
    {
        "name": "User Calendar",
        "events": [{
            "title": "Meeting",
            "start_time": "2025-03-15T14:00:00+00:00",
            "end_time": "2025-03-15T15:00:00+00:00",
            "timezone": "UTC",
            "status": "Confirmed",
            "attendees": [],
            "description": null,
            "location": null,
            "uid": null
        }],
        "timezone": "UTC"
    }
    "#;

    Calendar::from_json(json)
}

/// Export calendar to JSON (for API response)
fn export_to_web(cal: &Calendar) -> eventix::Result<String> {
    cal.to_json()
}

/// Build an API response
fn api_response(cal: &Calendar) -> Value {
    json!({
        "ok": true,
        "data": {
            "name": cal.name,
            "event_count": cal.event_count()
        },
        "timestamp": Utc::now().to_rfc3339()
    })
}

/// Run the JSON example
pub fn run() -> eventix::Result<()> {
    println!("\n=== JSON/Web Example ===\n");

    // 1. Import
    let mut cal = import_from_web()?;
    println!("📥 Imported: '{}' ({} events)", cal.name, cal.event_count());

    // 2. Add events
    cal.add_event(
        Event::builder()
            .title("Weekly Sync")
            .start("2025-03-20 10:00:00", "UTC")
            .duration_hours(1)
            .build()?,
    );
    println!("➕ Added event");

    // 3. Export
    let json = export_to_web(&cal)?;
    println!("📤 Exported JSON:\n{}", json);

    // 4. API response
    let response = api_response(&cal);
    println!(
        "\n🌐 API Response:\n{}",
        serde_json::to_string_pretty(&response)
            .map_err(|e| eventix::EventixError::Other(e.to_string()))?
    );

    // 5. Save to file (with proper error handling)
    use std::path::PathBuf;

    let output_dir = PathBuf::from("examples_output");
    let output_file = output_dir.join("calendar.json");

    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        eprintln!("❌ Failed to create directory '{}': {}", output_dir.display(), e);
        return Err(eventix::EventixError::Other(format!(
            "Failed to create directory '{}': {}",
            output_dir.display(),
            e
        )));
    }

    match std::fs::write(&output_file, &json) {
        Ok(_) => println!("\n💾 Saved to {}", output_file.display()),
        Err(e) => {
            eprintln!("❌ Failed to write file '{}': {}", output_file.display(), e);
            return Err(eventix::EventixError::Other(format!(
                "Failed to write file '{}': {}",
                output_file.display(),
                e
            )));
        }
    }

    Ok(())
}
