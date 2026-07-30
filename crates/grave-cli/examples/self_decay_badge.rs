use std::env;
use std::error::Error;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use grave_core::{inspect_decay_snapshot, inspect_grave};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let grave_path = args.next().ok_or("missing grave path")?;
    let output_path = args.next().ok_or("missing output path")?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let inspection = inspect_grave(&fs::read(&grave_path)?)?;

    let (age_days, percent, color) = if inspection.disturbed {
        (0u64, 100.0f64, "7f1d1d")
    } else {
        let snapshot = inspect_decay_snapshot(&inspection.header, now)?;
        (
            snapshot.age_days,
            snapshot.intensity * 100.0,
            badge_color(snapshot.intensity),
        )
    };

    let message = format!("buried {age_days} days ago · {percent:.1}% decay");
    let payload = format!(
        "{{\"schemaVersion\":1,\"label\":\"self decay\",\"message\":\"{message}\",\"color\":\"{color}\",\"cacheSeconds\":3600}}\n"
    );
    fs::write(output_path, payload)?;
    Ok(())
}

fn badge_color(intensity: f64) -> &'static str {
    if intensity < 0.15 {
        "6b4c8a"
    } else if intensity < 0.45 {
        "8b4e42"
    } else if intensity < 0.8 {
        "4a5d43"
    } else {
        "2f3b2b"
    }
}
