use anyhow::Result;
use clap::Parser;
use hash_guard::{
    compute_hash, compute_hash_verbose, expand_globs, load_state, save_state, HashState,
};
use std::path::PathBuf;
use std::process;

/// hash-guard — run or skip a command based on whether watched files have changed.
///
/// EXAMPLES:
///
///   # Re-run cargo test only when .rs files change
///   hash-guard 'src/**/*.rs' -- cargo test
///
///   # Rebuild Docker image only when relevant files change
///   hash-guard Dockerfile 'src/**' -- docker build -t myapp .
///
///   # Use a custom state file to track two separate sets of inputs
///   hash-guard -f .hg-tests.json 'tests/**' -- cargo test
///
///   # Exit 0 if files are unchanged, 1 if changed (no command needed)
///   hash-guard 'src/**/*.rs' && echo "nothing changed"
#[derive(Parser, Debug)]
#[command(
    name = "hash-guard",
    version,
    about = "Run or skip a command based on file content hashes",
    long_about = None,
)]
struct Cli {
    /// One or more file paths or glob patterns to watch.
    #[arg(required = true, value_name = "GLOB")]
    globs: Vec<String>,

    /// Path to the file used to store hash state.
    #[arg(
        short = 'f',
        long,
        default_value = ".hash-guard.json",
        value_name = "PATH"
    )]
    hash_file: PathBuf,

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
    // Expand globs to a sorted, deduplicated file list.
    let files = expand_globs(&cli.globs)?;

    if files.is_empty() {
        eprintln!("hash-guard: warning: no files matched the provided patterns");
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
        eprintln!("hash-guard: combined hash: {current_hash}");
    }

    // Load the stored state (if any).
    let stored = load_state(&cli.hash_file)?;
    let unchanged = stored
        .as_ref()
        .map(|s| s.hash == current_hash)
        .unwrap_or(false);

    if unchanged && !cli.force {
        if !cli.command.is_empty() {
            if cli.verbose {
                eprintln!("hash-guard: files unchanged — skipping command");
            } else {
                eprintln!("hash-guard: skipping (files unchanged)");
            }
        }
        // Exit 0 to signal "no changes" — useful for both the built-in command
        // and direct use in shell conditions.
        return Ok(0);
    }

    // If no command was given, just report whether files changed.
    if cli.command.is_empty() {
        if cli.verbose {
            eprintln!("hash-guard: files changed");
        }
        // Exit 1 to signal "files changed" so callers can compose with `||`.
        return Ok(1);
    }

    // Run the command.
    let (program, args) = cli.command.split_first().unwrap();

    if cli.verbose {
        eprintln!(
            "hash-guard: files changed — running: {}",
            cli.command.join(" ")
        );
    }

    let status = process::Command::new(program)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute '{}': {}", program, e))?;

    let code = status.code().unwrap_or(1);

    // Only persist the new hash when the command succeeded.
    if status.success() {
        save_state(&cli.hash_file, &HashState { hash: current_hash })?;
        if cli.verbose {
            eprintln!("hash-guard: state saved to {}", cli.hash_file.display());
        }
    } else if cli.verbose {
        eprintln!("hash-guard: command exited with code {code}; state not saved");
    }

    Ok(code)
}

fn main() {
    let cli = Cli::parse();

    match run(cli) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("hash-guard: error: {err:#}");
            process::exit(2);
        }
    }
}
