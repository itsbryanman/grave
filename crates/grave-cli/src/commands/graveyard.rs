use std::cmp::Reverse;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use comfy_table::presets::ASCII_BORDERS_ONLY_CONDENSED;
use comfy_table::{Cell, Row, Table};
use grave_core::{decay_snapshot, inspect_grave_file, GraveInspection, TERMINAL_Q};

use crate::commands::{days_ago, decay_bar, map_core_error, now_epoch, CliError};
use crate::{GraveyardArgs, GraveyardSortArg};

pub fn run(args: GraveyardArgs) -> Result<(), CliError> {
    let dir = args.dir.unwrap_or_else(|| PathBuf::from("."));
    let now = now_epoch();
    let mut graves = Vec::new();

    for entry in fs::read_dir(&dir).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        if !is_grave_file(&path) {
            continue;
        }

        let mut file = File::open(&path).map_err(io_error)?;
        let inspection = inspect_grave_file(&mut file).map_err(map_core_error)?;
        graves.push(GraveyardEntry::from_inspection(path, inspection, now)?);
    }

    if graves.is_empty() {
        println!("No one is buried here. Yet.");
        return Ok(());
    }

    sort_entries(&mut graves, args.sort);

    let mut table = Table::new();
    table.load_preset(ASCII_BORDERS_ONLY_CONDENSED);
    table.set_header(Row::from(vec![
        Cell::new("Plot"),
        Cell::new("Profile"),
        Cell::new("Age"),
        Cell::new("Neglect"),
        Cell::new("Decay"),
    ]));

    for entry in graves {
        if entry.terminal {
            table.add_row(Row::from(vec![
                Cell::new(format!("✝ {}", entry.terminal_label)),
                Cell::new(""),
                Cell::new(""),
                Cell::new(""),
                Cell::new(""),
            ]));
            continue;
        }

        let decay_text = if entry.disturbed {
            "disturbed".to_string()
        } else {
            format!("{}  {:.1}%", decay_bar(entry.q), entry.percent)
        };
        table.add_row(Row::from(vec![
            Cell::new(entry.display_name),
            Cell::new(entry.profile),
            Cell::new(format!("{}d", entry.age_days)),
            Cell::new(format!("{}d", entry.neglect_days)),
            Cell::new(decay_text),
        ]));
    }

    println!("{table}");
    Ok(())
}

fn is_grave_file(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("grave")
}

fn sort_entries(entries: &mut [GraveyardEntry], sort: GraveyardSortArg) {
    entries.sort_by_key(|entry| {
        let key = match sort {
            GraveyardSortArg::Decay => entry.q as u64,
            GraveyardSortArg::Age => entry.age_days,
            GraveyardSortArg::Neglect => entry.neglect_days,
        };
        (Reverse(key), entry.path_name.clone())
    });
}

fn io_error(error: std::io::Error) -> CliError {
    CliError::new(1, error.to_string())
}

#[derive(Clone, Debug)]
struct GraveyardEntry {
    path_name: String,
    display_name: String,
    terminal_label: String,
    profile: &'static str,
    age_days: u64,
    neglect_days: u64,
    q: u32,
    percent: f64,
    disturbed: bool,
    terminal: bool,
}

impl GraveyardEntry {
    fn from_inspection(
        path: PathBuf,
        inspection: GraveInspection,
        now: u64,
    ) -> Result<Self, CliError> {
        let header = inspection.header;
        let age_days = days_ago(header.buried_at, now);
        let neglect_days = days_ago(header.last_opened, now);
        let display_name = display_name(&header.original_filename, &path);
        let terminal_label = terminal_label(&header.epitaph, &display_name);

        let (q, percent, terminal) = if inspection.disturbed {
            (10_000, 100.0, false)
        } else {
            let snapshot = decay_snapshot(&header, now).map_err(map_core_error)?;
            (
                snapshot.q,
                snapshot.intensity * 100.0,
                snapshot.q >= TERMINAL_Q,
            )
        };

        Ok(Self {
            path_name: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string(),
            display_name,
            terminal_label,
            profile: header.profile.label(),
            age_days,
            neglect_days,
            q,
            percent,
            disturbed: inspection.disturbed,
            terminal,
        })
    }
}

fn display_name(original_filename: &str, path: &Path) -> String {
    if original_filename.is_empty() {
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown.grave")
            .to_string()
    } else {
        original_filename.to_string()
    }
}

fn terminal_label(epitaph: &str, display_name: &str) -> String {
    if epitaph.trim().is_empty() {
        display_name.to_string()
    } else {
        epitaph.to_string()
    }
}
