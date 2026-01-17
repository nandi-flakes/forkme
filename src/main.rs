use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod git;
mod patch;

#[derive(Parser)]
#[command(name = "forkme")]
#[command(about = "A tool for managing forks using a patch-based approach")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a forkme-managed project
    Init {
        /// URL of the upstream repository
        #[arg(long)]
        url: Option<String>,

        /// Branch to track from upstream (default to main)
        #[arg(long, default_value = "main")]
        branch: String,

        /// Limit the git cloning depth (optional)
        #[arg(long)]
        depth: Option<usize>,
    },

    /// Apply patches to the source directory
    Apply,

    /// Sync changes from source back to patches
    Sync,

    /// Show the current status of the forkme project
    Status,

    /// Show statistics about patches
    Stats,

    /// Update: fetch upstream and rebase forkme branch
    Update,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { url, branch, depth } => commands::init::run(url, &branch, depth)?,
        Commands::Apply => commands::apply::run()?,
        Commands::Sync => commands::sync::run()?,
        Commands::Status => commands::status::run()?,
        Commands::Stats => commands::stats::run()?,
        Commands::Update => commands::update::run()?,
    }

    Ok(())
}
