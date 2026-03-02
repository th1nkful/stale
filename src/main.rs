use anyhow::{Context, Result};
use clap::Parser;
use stale::{
    compute_hash, compute_hash_verbose, derive_name, expand_globs, find_git_root, load_sum_entry,
    resolve_pkg_version, save_sum_entry,
};
use std::path::{Path, PathBuf};
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
///   # Re-run tests when a specific package version changes
///   stale -p npm:express 'src/**' -- npm test
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
    ///
    /// When omitted, stale discovers the closest git repository root (by
    /// walking up to find a `.git` entry) and stores `.stale.sum` there.
    /// The search stops at the user's home directory to avoid escaping the
    /// project tree.  If no git root is found the file is stored in the
    /// current directory.
    #[arg(short = 'f', long, value_name = "PATH")]
    sum_file: Option<PathBuf>,

    /// Name for this entry in the .sum file.
    ///
    /// Defaults to a short hash derived from the supplied glob patterns and,
    /// when using git-root discovery, the working directory relative to the
    /// repository root.  This ensures that the same patterns run from different
    /// subdirectories produce distinct entries in the shared `.sum` file.
    #[arg(short, long, value_name = "NAME")]
    name: Option<String>,

    /// Extra strings to include in the hash.
    ///
    /// Useful for incorporating version numbers, environment variables, or
    /// other values that should trigger a re-run when they change.
    #[arg(short, long = "string", value_name = "STRING")]
    strings: Vec<String>,

    /// Look up a package version and include it in the hash.
    ///
    /// Format: `manager:package` (e.g., `npm:express`, `uv:requests`).
    /// Supported managers: npm (or js), uv (or py/python).
    #[arg(short = 'p', long = "pkg", value_name = "QUERY")]
    packages: Vec<String>,

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
    // Resolve the sum-file location.
    //
    // When the user supplies `-f`, use that path directly.  Otherwise walk up
    // to find the nearest git root and place `.stale.sum` there.  If no git
    // root is found, fall back to the current directory.
    let cwd = std::env::current_dir().context("Failed to determine current directory")?;
    let (sum_file, name_prefix) = match cli.sum_file {
        Some(ref path) => (path.clone(), None),
        None => {
            let home = std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from);
            if let Some(git_root) = find_git_root(&cwd, home.as_deref()) {
                let rel = cwd
                    .strip_prefix(&git_root)
                    .ok()
                    .filter(|r| *r != Path::new(""))
                    .map(|r| r.to_string_lossy().into_owned());
                (git_root.join(".stale.sum"), rel)
            } else {
                (PathBuf::from(".stale.sum"), None)
            }
        }
    };

    // Resolve package version queries into concrete version strings.
    let mut all_strings = cli.strings.clone();
    for query in &cli.packages {
        let version = resolve_pkg_version(query, &cwd)?;
        if cli.verbose {
            eprintln!("stale: {query} → {version}");
        }
        all_strings.push(format!("{query}={version}"));
    }

    // Resolve the entry name: explicit flag > derived from glob patterns + strings.
    // When using git-root discovery, the relative path from the git root to
    // the working directory is mixed into the derived name so that identical
    // glob patterns run from different subdirectories do not collide.
    let name = cli
        .name
        .clone()
        .unwrap_or_else(|| derive_name(&cli.globs, &all_strings, name_prefix.as_deref()));

    // Expand globs to a sorted, deduplicated file list.
    let files = expand_globs(&cli.globs)?;

    if files.is_empty() && all_strings.is_empty() {
        eprintln!("stale: warning: no files matched the provided patterns");
    }

    // Compute hash (with optional per-file breakdown for verbose mode).
    let current_hash = if cli.verbose {
        let (combined, per_file) = compute_hash_verbose(&files, &all_strings)?;
        for (path, hash) in &per_file {
            eprintln!("  {hash}  {path}");
        }
        combined
    } else {
        compute_hash(&files, &all_strings)?
    };

    if cli.verbose {
        eprintln!("stale: combined hash: {current_hash}");
        eprintln!("stale: entry name:    {name}");
    }

    // Load the stored hash for this named entry (if any).
    let stored_hash = load_sum_entry(&sum_file, &name)?;
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
        save_sum_entry(&sum_file, &name, &current_hash)?;
        if cli.verbose {
            eprintln!("stale: state saved to {}", sum_file.display());
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
