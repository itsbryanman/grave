use std::fs;
use std::path::Path;

use dialoguer::Input;
use grave_core::{bury, prognosis, read_header, BuryOptions};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::commands::{
    default_grave_path, ensure_writable, format_date, infer_mimetype, map_core_error, now_epoch,
    quoted, CliError,
};
use crate::BuryArgs;

pub fn run(args: BuryArgs) -> Result<(), CliError> {
    let input = fs::read(&args.file).map_err(io_error)?;
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| default_grave_path(&args.file));
    ensure_writable(&output, args.force)?;
    if args.hardcore {
        confirm_hardcore(&args.file)?;
    }

    let mut burial_id = [0u8; 32];
    OsRng.fill_bytes(&mut burial_id);

    let buried_at = now_epoch();
    let grave_bytes = bury(
        &input,
        BuryOptions {
            burial_id,
            buried_at,
            profile: args.profile.into(),
            hardcore: args.hardcore,
            half_life_days: args.half_life,
            epitaph: args.epitaph.unwrap_or_default(),
            original_filename: original_filename(&args.file),
            mimetype: infer_mimetype(&args.file, &input),
        },
    )
    .map_err(map_core_error)?;
    fs::write(&output, &grave_bytes).map_err(io_error)?;

    let header = read_header(&grave_bytes).map_err(map_core_error)?;
    let terminal_at = prognosis(&header, buried_at).map_err(map_core_error)?;

    println!(
        "{} was laid to rest on {}.",
        header.original_filename,
        format_date(header.buried_at)
    );
    println!(
        "  Profile: {} | Half-life: {} days | Epitaph: {}",
        header.profile.label(),
        header.half_life_days,
        quoted(&header.epitaph)
    );
    println!(
        "  Estimated terminal decomposition: {}.",
        format_date(terminal_at)
    );
    println!("  Grave file: {}", output.display());

    Ok(())
}

fn original_filename(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn io_error(error: std::io::Error) -> CliError {
    CliError::new(1, error.to_string())
}

fn confirm_hardcore(path: &Path) -> Result<(), CliError> {
    let filename = original_filename(path);
    eprintln!(
        "Hardcore burial permanently replaces the grave's contents every time it is opened. Exhumation will be refused, terminal decomposition will be final, and recovery will not be possible once the file is rewritten."
    );
    let typed = Input::<String>::new()
        .with_prompt(format!("Type '{filename}' to continue"))
        .interact_text()
        .map_err(|error| CliError::new(1, error.to_string()))?;
    if typed == filename {
        Ok(())
    } else {
        Err(CliError::new(
            1,
            "The grave rejected the oath. Hardcore burial was cancelled.",
        ))
    }
}
