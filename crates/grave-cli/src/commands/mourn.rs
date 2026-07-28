use std::fs::OpenOptions;

use grave_core::{inspect_grave_file, mourn, MournOutcome};

use crate::commands::{map_core_error, now_epoch, CliError};
use crate::MournArgs;

pub fn run(args: MournArgs) -> Result<(), CliError> {
    let now = now_epoch();
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&args.file)
        .map_err(io_error)?;
    let inspection = inspect_grave_file(&mut file).map_err(map_core_error)?;
    let label = display_label(&inspection.header.original_filename, &args.file);

    match mourn(&mut file, now).map_err(map_core_error)? {
        MournOutcome::PaidRespects { mourn_credit } => {
            println!("{}", success_message(&label, mourn_credit));
        }
        MournOutcome::AlreadyMournedRecently => {
            println!("{}", recent_mourning_message());
        }
    }

    Ok(())
}

fn display_label(original_filename: &str, path: &std::path::Path) -> String {
    if original_filename.is_empty() {
        path.display().to_string()
    } else {
        original_filename.to_string()
    }
}

fn success_message(label: &str, mourn_credit: u32) -> String {
    format!("You paid your respects to {label}. Mourn credit is now {mourn_credit}.")
}

fn recent_mourning_message() -> &'static str {
    "You have already paid your respects this week."
}

fn io_error(error: std::io::Error) -> CliError {
    CliError::new(1, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::recent_mourning_message;

    #[test]
    fn rate_limit_message_matches_the_spec() {
        assert_eq!(
            recent_mourning_message(),
            "You have already paid your respects this week."
        );
    }
}
