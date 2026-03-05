use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "suru")]
#[command(about = "Suru language compiler")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Parse a Suru source file and print the AST
    Parse(ParseArgs),
    /// Type-check a Suru source file
    Check(CheckArgs),
}

#[derive(clap::Args)]
pub struct CheckArgs {
    /// Input file path
    pub file: String,
}

#[derive(clap::Args)]
pub struct ParseArgs {
    /// Input file path
    pub file: String,
}
