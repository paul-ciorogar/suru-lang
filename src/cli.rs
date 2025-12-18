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
}

#[derive(clap::Args)]
pub struct ParseArgs {
    /// Input file path
    pub file: String,
}
