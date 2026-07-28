pub mod bury;
pub mod exhume;
pub mod graveyard;
pub mod inspect;
pub mod mourn;
pub mod open;

use std::fmt;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, TimeZone, Utc};
use grave_core::GraveError;

#[derive(Debug)]
pub struct CliError {
    pub code: i32,
    pub message: String,
}

impl CliError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(64, message)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

pub fn now_epoch() -> u64 {
    Utc::now().timestamp().max(0) as u64
}

pub fn format_date(timestamp: u64) -> String {
    Utc.timestamp_opt(timestamp as i64, 0)
        .single()
        .map(|datetime| datetime.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn parse_date(value: &str) -> Result<u64, CliError> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        CliError::usage(format!(
            "The date '{value}' does not fit the YYYY-MM-DD ritual."
        ))
    })?;
    let datetime = date
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| CliError::usage(format!("The date '{value}' could not be spoken aloud.")))?;
    Ok(Utc.from_utc_datetime(&datetime).timestamp() as u64)
}

pub fn ensure_writable(path: &Path, force: bool) -> Result<(), CliError> {
    if path.exists() && !force {
        return Err(CliError::new(
            1,
            format!(
                "{} already occupies that plot. Use -f to disturb it.",
                path.display()
            ),
        ));
    }
    Ok(())
}

pub fn default_grave_path(input: &Path) -> PathBuf {
    let mut file_name = input
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string();
    file_name.push_str(".grave");
    input.with_file_name(file_name)
}

pub fn infer_mimetype(path: &Path, bytes: &[u8]) -> String {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("png") => "image/png".to_string(),
        Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
        Some("gif") => "image/gif".to_string(),
        Some("webp") => "image/webp".to_string(),
        Some("txt") | Some("md") | Some("rs") | Some("toml") | Some("json") | Some("yaml")
        | Some("yml") => "text/plain".to_string(),
        _ if bytes.is_ascii() => "text/plain".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

pub fn default_exhume_path(grave_path: &Path, original_filename: &str) -> PathBuf {
    let file_name = if original_filename.is_empty() {
        "exhumed.bin"
    } else {
        original_filename
    };
    grave_path.with_file_name(file_name)
}

pub fn default_open_image_path(grave_path: &Path, original_filename: &str) -> PathBuf {
    let stem = Path::new(original_filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("grave");
    grave_path.with_file_name(format!("{stem}.decayed.png"))
}

pub fn days_ago(earlier: u64, now: u64) -> u64 {
    now.saturating_sub(earlier) / grave_core::DAY_SECONDS
}

pub fn quoted(value: &str) -> String {
    if value.is_empty() {
        "none".to_string()
    } else {
        format!("\"{value}\"")
    }
}

pub fn decay_bar(q: u32) -> String {
    let slots = 12usize;
    let filled = ((q as usize * slots) + 9_999) / 10_000;
    let mut bar = String::with_capacity(slots);
    for index in 0..slots {
        if index < filled {
            bar.push('#');
        } else {
            bar.push('.');
        }
    }
    bar
}

pub fn map_core_error(error: GraveError) -> CliError {
    match error {
        GraveError::DateBeforeBurial => CliError::usage(error.to_string()),
        GraveError::BadMagic
        | GraveError::UnsupportedVersion(_)
        | GraveError::Truncated
        | GraveError::InvalidUtf8(_)
        | GraveError::InvalidProfile(_)
        | GraveError::CrcMismatch
        | GraveError::Disturbed
        | GraveError::PayloadTooLarge { .. } => CliError::new(65, error.to_string()),
        GraveError::Hardcore => CliError::new(
            66,
            "The dead do not return from consecrated ground. (This file was buried with --hardcore.)",
        ),
        _ => CliError::new(1, error.to_string()),
    }
}
