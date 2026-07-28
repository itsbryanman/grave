use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write};

use grave_core::{
    decay_snapshot, encode_png, inspect_grave_file, render_grave, touch, RenderedPayload,
    TERMINAL_Q,
};

use crate::art::headstone_screen;
use crate::commands::{
    default_open_image_path, ensure_writable, format_date, map_core_error, now_epoch, parse_date,
    CliError,
};
use crate::OpenArgs;

pub fn run(args: OpenArgs) -> Result<(), CliError> {
    let at = args.at;
    let when = match at.as_deref() {
        Some(value) => parse_date(value)?,
        None => now_epoch(),
    };

    let inspection = {
        let mut file = std::fs::File::open(&args.file).map_err(io_error)?;
        inspect_grave_file(&mut file).map_err(map_core_error)?
    };
    if !inspection.disturbed {
        let snapshot = decay_snapshot(&inspection.header, when).map_err(map_core_error)?;
        if snapshot.q >= TERMINAL_Q {
            println!("{}", headstone_screen(&inspection.header, when));
            return Err(CliError::new(
                67,
                format!(
                    "{} has reached terminal decomposition as of {}.",
                    inspection.header.original_filename,
                    format_date(when)
                ),
            ));
        }
    }

    let bytes = fs::read(&args.file).map_err(io_error)?;
    let rendered = render_grave(&bytes, when).map_err(map_core_error)?;
    let text_to_stdout =
        matches!(rendered.payload, RenderedPayload::Text(_)) && args.output.is_none();
    if rendered.disturbed {
        status_line(text_to_stdout, "The grave has been disturbed.");
    }

    match rendered.payload {
        RenderedPayload::Image(image) => {
            let output = args.output.unwrap_or_else(|| {
                default_open_image_path(&args.file, &rendered.header.original_filename)
            });
            reject_self_overwrite(&args.file, &output)?;
            ensure_writable(&output, args.force)?;
            let png = encode_png(&image).map_err(map_core_error)?;
            fs::write(&output, png).map_err(io_error)?;
            println!(
                "{} lies at {:.1}% decay under the {} profile.",
                rendered.header.original_filename,
                rendered.snapshot.intensity * 100.0,
                rendered.header.profile.label()
            );
            println!("The visitation yielded {}.", output.display());
        }
        RenderedPayload::Text(text) => {
            if let Some(output) = args.output {
                reject_self_overwrite(&args.file, &output)?;
                ensure_writable(&output, args.force)?;
                fs::write(&output, text.body.as_bytes()).map_err(io_error)?;
                println!(
                    "{} lies at {:.1}% decay under the {} profile.",
                    rendered.header.original_filename,
                    rendered.snapshot.intensity * 100.0,
                    rendered.header.profile.label()
                );
                println!("The visitation yielded {}.", output.display());
            } else {
                status_line(
                    true,
                    format!(
                        "{} lies at {:.1}% decay under the {} profile.",
                        rendered.header.original_filename,
                        rendered.snapshot.intensity * 100.0,
                        rendered.header.profile.label()
                    ),
                );
                io::stdout()
                    .write_all(text.body.as_bytes())
                    .map_err(io_error)?;
            }
        }
    }

    if at.is_some() {
        status_line(
            text_to_stdout,
            "This vigil was held outside the present hour and left no trace.",
        );
        return Ok(());
    }

    if args.no_touch || rendered.disturbed {
        return Ok(());
    }

    match OpenOptions::new().read(true).write(true).open(&args.file) {
        Ok(mut file) => match touch(&mut file, when) {
            Ok(()) => {}
            Err(grave_core::GraveError::Disturbed) => {}
            Err(error) => return Err(map_core_error(error)),
        },
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {}
        Err(error) => return Err(io_error(error)),
    }

    Ok(())
}

fn io_error(error: impl ToString) -> CliError {
    CliError::new(1, error.to_string())
}

fn reject_self_overwrite(
    grave_path: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), CliError> {
    if grave_path == output {
        return Err(CliError::new(
            1,
            "You may not exhume a visitation directly over the grave itself.",
        ));
    }
    Ok(())
}

fn status_line(use_stderr: bool, message: impl AsRef<str>) {
    if use_stderr {
        eprintln!("{}", message.as_ref());
    } else {
        println!("{}", message.as_ref());
    }
}
