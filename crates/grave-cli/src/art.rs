use chrono::{Datelike, TimeZone, Utc};
use grave_core::GraveHeader;

const STONE_WIDTH: usize = 16;

pub fn headstone_screen(header: &GraveHeader, now: u64) -> String {
    let mut lines = Vec::new();
    lines.push(center("✝  R I P", STONE_WIDTH));
    lines.push(String::new());
    lines.extend(wrap_and_center(display_name(header), STONE_WIDTH));
    lines.push(center(
        &format!("{} - {}", year(header.buried_at), year(now)),
        STONE_WIDTH,
    ));
    if !header.epitaph.trim().is_empty() {
        lines.push(String::new());
        lines.extend(wrap_text(&format!("\"{}\"", header.epitaph), STONE_WIDTH));
    }

    let mut art = String::new();
    art.push_str("                 ________________\n");
    art.push_str("                /                \\\n");
    for line in lines {
        art.push_str(&format!("               | {:<16} |\n", line));
    }
    art.push_str("       ________|________________|________\n");
    art.push_str("      ////////////////////////////////////\n\n");
    art.push_str("This file has reached terminal decomposition.\n");
    art.push_str(&format!(
        "Buried {} · {} · {}.",
        long_date(header.buried_at),
        visit_phrase(header.open_count),
        mourn_phrase(header.mourn_credit)
    ));
    art
}

fn display_name(header: &GraveHeader) -> &str {
    if header.original_filename.is_empty() {
        "unknown"
    } else {
        &header.original_filename
    }
}

fn year(timestamp: u64) -> i32 {
    datetime(timestamp).year()
}

fn long_date(timestamp: u64) -> String {
    datetime(timestamp).format("%B %-d, %Y").to_string()
}

fn datetime(timestamp: u64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(timestamp as i64, 0)
        .single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().expect("epoch"))
}

fn visit_phrase(open_count: u32) -> String {
    if open_count == 1 {
        "1 visit".to_string()
    } else {
        format!("{open_count} visits")
    }
}

fn mourn_phrase(mourn_credit: u32) -> String {
    match mourn_credit {
        0 => "never mourned".to_string(),
        1 => "mourned once".to_string(),
        2 => "mourned twice".to_string(),
        count => format!("mourned {count} times"),
    }
}

fn wrap_and_center(text: &str, width: usize) -> Vec<String> {
    wrap_text(text, width)
        .into_iter()
        .map(|line| center(&line, width))
        .collect()
}

fn center(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        return text.chars().take(width).collect();
    }
    let pad_left = (width - len) / 2;
    format!("{}{}", " ".repeat(pad_left), text)
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if word.chars().count() > width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let mut chunk = String::new();
            for ch in word.chars() {
                chunk.push(ch);
                if chunk.chars().count() == width {
                    lines.push(std::mem::take(&mut chunk));
                }
            }
            if !chunk.is_empty() {
                current = chunk;
            }
            continue;
        }

        let next_len = if current.is_empty() {
            word.chars().count()
        } else {
            current.chars().count() + 1 + word.chars().count()
        };
        if next_len > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use grave_core::{GraveFlags, GraveHeader, RotProfile, FORMAT_VERSION};

    use super::headstone_screen;

    #[test]
    fn headstone_wraps_the_epitaph() {
        let screen = headstone_screen(
            &GraveHeader {
                version: FORMAT_VERSION,
                burial_id: [0; 32],
                buried_at: 1_722_124_800,
                last_opened: 1_722_124_800,
                open_count: 14,
                profile: RotProfile::Mold,
                flags: GraveFlags::new(false),
                half_life_days: 30,
                mourn_credit: 2,
                epitaph: "she was beautiful once, and then the damp got in".to_string(),
                original_filename: "photo.jpg".to_string(),
                mimetype: "image/jpeg".to_string(),
            },
            1_742_124_800,
        );

        assert!(screen.contains("\"she was"));
        assert!(screen.contains("mourned twice"));
    }
}
