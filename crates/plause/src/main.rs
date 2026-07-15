//! The `plause` CLI — a thin wrapper over [`plause_host`].
//!
//! Rule of the house: no hosting logic lives here. If a subcommand needs an
//! `if` statement about plugins or audio, that logic belongs in `plause-host`.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use plause_host::discovery;

/// A headless CLAP host for testing plugins.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Find installed .clap bundles.
    Scan {
        /// Directory to scan; defaults to CLAP_PATH plus the platform's
        /// standard CLAP locations.
        dir: Option<PathBuf>,
    },
    /// Load a plugin and report its descriptors, extensions, ports, and params.
    Inspect {
        /// Path to a .clap bundle.
        plugin: PathBuf,
        /// Emit machine-readable JSON instead of human-readable text.
        #[arg(long)]
        json: bool,
    },
    /// Render a fixture through a plugin offline: no audio device, fully
    /// deterministic, CI-friendly.
    Render {
        /// Path to a .clap bundle.
        #[arg(long)]
        plugin: PathBuf,
        /// JSON event fixture to play into the plugin.
        #[arg(long)]
        events: PathBuf,
        /// Where to write the rendered audio (WAV).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Where to write the event tap.
        #[arg(long)]
        tap: Option<PathBuf>,
        #[arg(long, default_value_t = 48_000)]
        sample_rate: u32,
        #[arg(long, default_value_t = 256)]
        block_size: u32,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Scan { dir } => scan(dir),
        Command::Inspect { .. } => {
            bail!(
                "`plause inspect` is not implemented yet — it is the milestone 1 deliverable (plugin loading via clack). See the roadmap in README.md."
            )
        }
        Command::Render { .. } => {
            bail!(
                "`plause render` is not implemented yet — it is the milestone 2 deliverable (offline engine + event tap). See the roadmap in README.md."
            )
        }
    }
}

fn scan(dir: Option<PathBuf>) -> Result<()> {
    let dirs = match dir {
        Some(dir) => vec![dir],
        None => discovery::default_search_paths(),
    };

    let mut total = 0usize;
    for dir in &dirs {
        let found =
            discovery::scan(dir).with_context(|| format!("failed to scan {}", dir.display()))?;
        for path in found {
            println!("{}", path.display());
            total += 1;
        }
    }

    if total == 0 {
        eprintln!("no .clap bundles found in:");
        for dir in &dirs {
            eprintln!("  {}", dir.display());
        }
        eprintln!("(set CLAP_PATH or pass a directory: `plause scan <dir>`)");
    }
    Ok(())
}
