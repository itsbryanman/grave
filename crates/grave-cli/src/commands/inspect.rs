use std::fs;

use comfy_table::presets::ASCII_BORDERS_ONLY_CONDENSED;
use comfy_table::{Cell, Row, Table};
use grave_core::{inspect_decay_snapshot, inspect_grave, prognosis};

use crate::commands::{
    days_ago, decay_bar, format_date, map_core_error, now_epoch, quoted, CliError,
};
use crate::InspectArgs;

pub fn run(args: InspectArgs) -> Result<(), CliError> {
    let bytes = fs::read(&args.file).map_err(io_error)?;
    let inspection = inspect_grave(&bytes).map_err(map_core_error)?;
    let now = now_epoch();
    let disturbed = inspection.disturbed;
    let header = inspection.header;

    let (q, percent, prognosis_date) = if disturbed {
        (10_000u32, 100.0f64, now)
    } else {
        let snapshot = inspect_decay_snapshot(&header, now).map_err(map_core_error)?;
        let terminal_at = prognosis(&header, now).map_err(map_core_error)?;
        (snapshot.q, snapshot.intensity * 100.0, terminal_at)
    };

    let mut table = Table::new();
    table.load_preset(ASCII_BORDERS_ONLY_CONDENSED);
    table.set_header(Row::from(vec![
        Cell::new(args.file.display().to_string()),
        Cell::new(""),
    ]));
    table.add_row(row(
        "Interred",
        format!("{} ({})", header.original_filename, header.mimetype),
    ));
    table.add_row(row(
        "Buried",
        format!(
            "{} ({} days ago)",
            format_date(header.buried_at),
            days_ago(header.buried_at, now)
        ),
    ));
    table.add_row(row(
        "Last visited",
        format!(
            "{} ({} days ago)",
            format_date(header.last_opened),
            days_ago(header.last_opened, now)
        ),
    ));
    table.add_row(row("Visits", header.open_count.to_string()));
    table.add_row(row("Profile", header.profile.label()));
    table.add_row(row("Decay", format!("{}  {:.1}%", decay_bar(q), percent)));
    table.add_row(row(
        "Prognosis",
        format!("terminal by {}", format_date(prognosis_date)),
    ));
    table.add_row(row("Epitaph", quoted(&header.epitaph)));
    if disturbed {
        table.add_row(row("Disturbance", "The grave has been disturbed."));
    }

    println!("{table}");
    Ok(())
}

fn io_error(error: std::io::Error) -> CliError {
    CliError::new(1, error.to_string())
}

fn row(left: impl Into<String>, right: impl Into<String>) -> Row {
    Row::from(vec![Cell::new(left.into()), Cell::new(right.into())])
}
