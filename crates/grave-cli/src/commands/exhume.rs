use std::fs;

use grave_core::{exhume, inspect_grave};

use crate::commands::{default_exhume_path, ensure_writable, map_core_error, CliError};
use crate::ExhumeArgs;

pub fn run(args: ExhumeArgs) -> Result<(), CliError> {
    let bytes = fs::read(&args.file).map_err(io_error)?;
    let inspection = inspect_grave(&bytes).map_err(map_core_error)?;
    let output = args
        .output
        .unwrap_or_else(|| default_exhume_path(&args.file, &inspection.header.original_filename));
    ensure_writable(&output, args.force)?;

    let payload = exhume(&bytes).map_err(map_core_error)?;
    fs::write(&output, payload).map_err(io_error)?;
    println!(
        "{} has been exhumed. It remembers nothing.",
        output.display()
    );
    Ok(())
}

fn io_error(error: std::io::Error) -> CliError {
    CliError::new(1, error.to_string())
}
