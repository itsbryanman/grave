mod commands;

use clap::{Parser, Subcommand, ValueEnum};
use commands::CliError;

#[derive(Parser, Debug)]
#[command(name = "grave")]
#[command(about = "A professional-grade volatile retention format.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Bury(BuryArgs),
    Open(OpenArgs),
    Inspect(InspectArgs),
    Exhume(ExhumeArgs),
}

#[derive(clap::Args, Debug)]
pub struct BuryArgs {
    pub file: std::path::PathBuf,
    #[arg(long, value_enum, default_value_t = ProfileArg::Static)]
    pub profile: ProfileArg,
    #[arg(long = "half-life", default_value_t = 30)]
    pub half_life: u32,
    #[arg(long)]
    pub epitaph: Option<String>,
    #[arg(long)]
    pub hardcore: bool,
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
    #[arg(short = 'f', long)]
    pub force: bool,
}

#[derive(clap::Args, Debug)]
pub struct OpenArgs {
    pub file: std::path::PathBuf,
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
    #[arg(short = 'f', long)]
    pub force: bool,
    #[arg(long)]
    pub no_touch: bool,
    #[arg(long)]
    pub at: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct InspectArgs {
    pub file: std::path::PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct ExhumeArgs {
    pub file: std::path::PathBuf,
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
    #[arg(short = 'f', long)]
    pub force: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ProfileArg {
    Mold,
    Static,
    Burnin,
    Dataloss,
}

impl From<ProfileArg> for grave_core::RotProfile {
    fn from(value: ProfileArg) -> Self {
        match value {
            ProfileArg::Mold => Self::Mold,
            ProfileArg::Static => Self::Static,
            ProfileArg::Burnin => Self::BurnIn,
            ProfileArg::Dataloss => Self::DataLoss,
        }
    }
}

fn main() {
    let exit_code = match run() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            error.code
        }
    };
    std::process::exit(exit_code);
}

fn run() -> Result<(), CliError> {
    let cli = Cli::try_parse().map_err(|error| CliError::usage(error.to_string()))?;
    match cli.command {
        Command::Bury(args) => commands::bury::run(args),
        Command::Open(args) => commands::open::run(args),
        Command::Inspect(args) => commands::inspect::run(args),
        Command::Exhume(args) => commands::exhume::run(args),
    }
}
