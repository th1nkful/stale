use anyhow::Result;
use clap::Parser;
use stale::{
    compute_hash, compute_hash_verbose, derive_name, expand_globs, load_sum_entry, save_sum_entry,
};
use std::path::PathBuf;
use std::process;

/// stale — run or skip a command based on whether watched files have changed.
///
/// EXAMPLES:
///
///   # Re-run cargo test only when .rs files change
///   stale 'src/**/*.rs' -- cargo test
///
///   # Rebuild Docker image only when relevant files change
///   stale Dockerfile 'src/**' -- docker build -t myapp .
///
///   # Use a named entry to track two separate sets of inputs in one .sum file
///   stale --name lint 'src/**/*.rs' -- cargo clippy
///   stale --name test 'tests/**' -- cargo test
///
///   # Exit 0 if files are unchanged, 1 if changed (no command needed)
///   stale 'src/**/*.rs' && echo "nothing changed"
#[derive(Parser, Debug)]
#[command(
    name = "stale",
    version,
    about = "Run or skip a command based on file content hashes",
    long_about = None,
)]
struct Cli {
    /// One or more file paths or glob patterns to watch.
    #[arg(required = true, value_name = "GLOB")]
    globs: Vec<String>,

    /// Path to the .sum file used to store hash state.
    #[arg(short = 'f', long, default_value = ".stale.sum", value_name = "PATH")]
    sum_file: PathBuf,

    /// Name for this entry in the .sum file.
    ///
    /// Defaults to a short hash derived from the supplied glob patterns so
    /// repeated invocations with the same patterns always reuse the same entry.
    #[arg(short, long, value_name = "NAME")]
    name: Option<String>,

    /// Always run the command even if the files have not changed.
    #[arg(long)]
    force: bool,

    /// Print matched files and their individual hashes.
    #[arg(short, long)]
    verbose: bool,

    /// The command to execute when files have changed (everything after `--`).
    #[arg(last = true, value_name = "COMMAND")]
    command: Vec<String>,
}

fn run(cli: Cli) -> Result<i32> {
    // Resolve the entry name: explicit flag > derived from glob patterns.
    let name = cli.name.clone().unwrap_or_else(|| derive_name(&cli.globs));

    // Expand globs to a sorted, deduplicated file list.
    let files = expand_globs(&cli.globs)?;

    if files.is_empty() {
        eprintln!("stale: warning: no files matched the provided patterns");
    }

    // Compute hash (with optional per-file breakdown for verbose mode).
    let current_hash = if cli.verbose {
        let (combined, per_file) = compute_hash_verbose(&files)?;
        for (path, hash) in &per_file {
            eprintln!("  {hash}  {path}");
        }
        combined
    } else {
        compute_hash(&files)?
    };

    if cli.verbose {
        eprintln!("stale: combined hash: {current_hash}");
        eprintln!("stale: entry name:    {name}");
    }

    // Load the stored hash for this named entry (if any).
    let stored_hash = load_sum_entry(&cli.sum_file, &name)?;
    let unchanged = stored_hash.as_deref() == Some(current_hash.as_str());

    if unchanged && !cli.force {
        if !cli.command.is_empty() {
            if cli.verbose {
                eprintln!("stale: files unchanged — skipping command");
            } else {
                eprintln!("stale: skipping (files unchanged)");
            }
        }
        // Exit 0 to signal "no changes".
        return Ok(0);
    }

    // If no command was given, just report whether files changed.
    if cli.command.is_empty() {
        if cli.verbose {
            eprintln!("stale: files changed");
        }
        // Exit 1 to signal "files changed" so callers can compose with `||`.
        return Ok(1);
    }

    // Run the command.
    let (program, args) = cli.command.split_first().unwrap();

    if cli.verbose {
        eprintln!("stale: files changed — running: {}", cli.command.join(" "));
    }

    let status = process::Command::new(program)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute '{}': {}", program, e))?;

    let code = status.code().unwrap_or(1);

    // Only persist the new hash when the command succeeded.
    if status.success() {
        save_sum_entry(&cli.sum_file, &name, &current_hash)?;
        if cli.verbose {
            eprintln!("stale: state saved to {}", cli.sum_file.display());
        }
    } else if cli.verbose {
        eprintln!("stale: command exited with code {code}; state not saved");
    }

    Ok(code)
}

fn main() {
    let cli = Cli::parse();

    match run(cli) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("stale: error: {err:#}");
            process::exit(2);
        }
    }
}
