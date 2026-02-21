mod checksum;
mod compress;
mod index;
mod restore;
mod scan;
mod tar_writer;
mod utils;
mod verify;
mod diff;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

/// Archivum — deterministic, split, checksummed, compressed archive system
#[derive(Parser)]
#[command(
    name = "archivum",
    version = env!("CARGO_PKG_VERSION"),
    author = "Ankit Chaubey <ankitchaubey.dev@gmail.com>",
    about = "Deterministic, split, checksummed, compressed archive system with faithful restore",
    long_about = None,
    after_help = "EXAMPLES:\n  archivum create ./photos ./backup\n  archivum create ./photos ./backup --split-gb 2 --compress zstd\n  archivum list ./backup/index.arc.json\n  archivum restore ./backup/index.arc.json ./restored\n  archivum verify ./backup/index.arc.json\n  archivum diff ./backup/index.arc.json ./photos"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new archive from a source directory
    Create {
        /// Source directory to archive
        #[arg(value_name = "SOURCE")]
        source: PathBuf,

        /// Output directory for archive parts
        #[arg(value_name = "OUTPUT")]
        output: PathBuf,

        /// Max size of each tar part in gigabytes
        #[arg(long, default_value = "4", value_name = "GB")]
        split_gb: f64,

        /// Compression algorithm: none, gzip, zstd
        #[arg(long, default_value = "none", value_name = "ALGO")]
        compress: String,

        /// Exclude glob patterns (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        exclude: Vec<String>,

        /// Number of parallel checksum threads
        #[arg(long, default_value = "4", value_name = "N")]
        threads: usize,
    },

    /// List contents and statistics of an archive
    List {
        /// Path to index.arc.json
        #[arg(value_name = "INDEX")]
        index: PathBuf,

        /// Show all file entries (not just summary)
        #[arg(long, short)]
        verbose: bool,

        /// Filter entries by glob pattern
        #[arg(long, value_name = "PATTERN")]
        filter: Option<String>,
    },

    /// Restore an archive to a target directory
    Restore {
        /// Path to index.arc.json
        #[arg(value_name = "INDEX")]
        index: PathBuf,

        /// Target directory to restore into
        #[arg(value_name = "TARGET")]
        target: PathBuf,

        /// Only restore files matching this glob pattern
        #[arg(long, value_name = "PATTERN")]
        filter: Option<String>,

        /// Overwrite existing files
        #[arg(long, short)]
        force: bool,

        /// Restore Unix permissions (requires root for exact uid/gid)
        #[arg(long)]
        restore_permissions: bool,
    },

    /// Verify archive integrity (checksums + structure)
    Verify {
        /// Path to index.arc.json
        #[arg(value_name = "INDEX")]
        index: PathBuf,

        /// Continue on errors instead of stopping
        #[arg(long, short)]
        continue_on_error: bool,
    },

    /// Compare archive index against a source directory (drift detection)
    Diff {
        /// Path to index.arc.json
        #[arg(value_name = "INDEX")]
        index: PathBuf,

        /// Source directory to compare against
        #[arg(value_name = "SOURCE")]
        source: PathBuf,

        /// Show only changed files
        #[arg(long)]
        changed_only: bool,
    },

    /// Print detailed info about a specific file in the archive
    Info {
        /// Path to index.arc.json
        #[arg(value_name = "INDEX")]
        index: PathBuf,

        /// Relative path of the file inside the archive
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Extract a single file from the archive
    Extract {
        /// Path to index.arc.json
        #[arg(value_name = "INDEX")]
        index: PathBuf,

        /// Relative path of the file inside the archive
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output path (default: current directory)
        #[arg(long, value_name = "OUTPUT")]
        output: Option<PathBuf>,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "error:".red().bold(), e);
        for cause in e.chain().skip(1) {
            eprintln!("  {} {}", "caused by:".yellow(), cause);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create {
            source,
            output,
            split_gb,
            compress,
            exclude,
            threads,
        } => {
            utils::print_banner();
            let algo = compress::CompressionAlgo::parse(&compress)
                .with_context(|| format!("Unknown compression algorithm: '{compress}'"))?;

            let split_bytes = (split_gb * 1024.0 * 1024.0 * 1024.0) as u64;

            println!(
                "{} {} → {}",
                "Creating archive:".cyan().bold(),
                source.display().to_string().yellow(),
                output.display().to_string().yellow()
            );
            println!(
                "  split={} GB  compress={}  exclude={} patterns  threads={}",
                split_gb,
                algo.name().green(),
                exclude.len(),
                threads
            );
            println!();

            let scan = scan::scan_directory(&source, &exclude)
                .with_context(|| format!("Failed to scan {}", source.display()))?;

            let mut idx = index::ArchivumIndex::build(scan, algo.clone());

            std::fs::create_dir_all(&output)
                .with_context(|| format!("Failed to create output dir {}", output.display()))?;

            // Compute checksums
            checksum::compute_checksums(&source, &mut idx, threads)?;

            // Write tar parts
            tar_writer::write_archive(&source, &output, &mut idx, split_bytes, &algo)?;

            // Save index
            let index_path = output.join("index.arc.json");
            idx.write(&index_path)?;

            let total_files = idx.header.total_files;
            let total_size = idx.header.total_size;
            let parts = idx.header.total_parts;

            println!();
            println!("{}", "─".repeat(60).dimmed());
            println!("{}", "  Archive created successfully!".green().bold());
            println!(
                "  Files : {}  |  Dirs : {}  |  Parts : {}",
                total_files.to_string().yellow(),
                idx.header.total_dirs.to_string().yellow(),
                parts.to_string().yellow()
            );
            println!(
                "  Size  : {}  |  Compression: {}",
                utils::human(total_size).cyan(),
                algo.name().green()
            );
            println!(
                "  Index : {}",
                index_path.display().to_string().cyan()
            );
            println!("{}", "─".repeat(60).dimmed());
        }

        Commands::List {
            index,
            verbose,
            filter,
        } => {
            let idx = index::ArchivumIndex::read(&index)
                .with_context(|| format!("Failed to read index: {}", index.display()))?;
            idx.print_summary(verbose, filter.as_deref())?;
        }

        Commands::Restore {
            index,
            target,
            filter,
            force,
            restore_permissions,
        } => {
            utils::print_banner();
            restore::restore(&index, &target, filter.as_deref(), force, restore_permissions)?;
        }

        Commands::Verify { index, continue_on_error } => {
            utils::print_banner();
            verify::verify(&index, continue_on_error)?;
        }

        Commands::Diff {
            index,
            source,
            changed_only,
        } => {
            diff::diff(&index, &source, changed_only)?;
        }

        Commands::Info { index, file } => {
            let idx = index::ArchivumIndex::read(&index)?;
            if let Some(entry) = idx.entries.iter().find(|e| e.path == file) {
                println!("{}", "─".repeat(50).dimmed());
                println!("{} {}", "File:".cyan().bold(), entry.path.display());
                println!("{} {}", "Type:".cyan(), format!("{:?}", entry.entry_type).green());
                println!("{} {}", "Size:".cyan(), utils::human(entry.size).yellow());
                println!(
                    "{} {}",
                    "SHA-256:".cyan(),
                    entry.sha256.as_deref().unwrap_or("—").yellow()
                );
                println!(
                    "{} {}",
                    "Tar part:".cyan(),
                    format!("data.part{:03}", entry.tar_part).yellow()
                );
                if let Some(m) = entry.mtime {
                    println!("{} {}", "Modified:".cyan(), utils::fmt_time(m).yellow());
                }
                if let Some(mode) = entry.unix_mode {
                    println!("{} {:o}", "Mode:".cyan(), mode);
                }
                println!("{}", "─".repeat(50).dimmed());
            } else {
                anyhow::bail!("File not found in archive: {}", file.display());
            }
        }

        Commands::Extract { index, file, output } => {
            let idx = index::ArchivumIndex::read(&index)?;
            let base = index.parent().unwrap_or(std::path::Path::new("."));
            restore::extract_single(&idx, base, &file, output.as_deref())?;
        }
    }

    Ok(())
}
