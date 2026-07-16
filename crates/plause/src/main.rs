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
        Command::Inspect { plugin, json } => inspect(&plugin, json),
        Command::Render { .. } => {
            bail!(
                "`plause render` is not implemented yet — it is the milestone 2 deliverable (offline engine + event tap). See the roadmap in README.md."
            )
        }
    }
}

fn inspect(path: &std::path::Path, json: bool) -> Result<()> {
    let info = plause_host::instance::inspect(path)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    println!("{}", info.path);
    for plugin in &info.plugins {
        let d = &plugin.descriptor;
        println!();
        println!(
            "{} ({}){}",
            d.name.as_deref().unwrap_or("<unnamed>"),
            d.id,
            d.version
                .as_deref()
                .map(|v| format!(" v{v}"))
                .unwrap_or_default(),
        );
        if let Some(vendor) = &d.vendor {
            println!("  vendor:     {vendor}");
        }
        if let Some(description) = &d.description {
            println!("  about:      {description}");
        }
        println!("  features:   {}", d.features.join(", "));
        println!("  extensions: {}", plugin.extensions.join(", "));

        println!("  audio ports:");
        for (dir, ports) in [
            ("in ", &plugin.audio_ports.inputs),
            ("out", &plugin.audio_ports.outputs),
        ] {
            for p in ports {
                let port_type = p.port_type.as_deref().unwrap_or("?");
                let main = if p.is_main { ", main" } else { "" };
                println!(
                    "    {dir} [{}] \"{}\" — {}ch ({port_type}{main})",
                    p.id, p.name, p.channel_count
                );
            }
        }

        println!("  note ports:");
        for (dir, ports) in [
            ("in ", &plugin.note_ports.inputs),
            ("out", &plugin.note_ports.outputs),
        ] {
            for p in ports {
                let preferred = p.preferred_dialect.as_deref().unwrap_or("?");
                println!(
                    "    {dir} [{}] \"{}\" — dialects: {} (preferred: {preferred})",
                    p.id,
                    p.name,
                    p.supported_dialects.join(", "),
                );
            }
        }

        println!("  params:");
        for p in &plugin.params {
            println!(
                "    [{}] {} — {}..{} (default {}){}",
                p.id,
                p.name,
                p.min_value,
                p.max_value,
                p.default_value,
                if p.flags.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", p.flags.join(", "))
                },
            );
        }
    }
    Ok(())
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
