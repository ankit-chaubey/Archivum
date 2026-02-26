// ─────────────────────────────────────────────────────────────────────────────
// Archivum v0.2.0
// Copyright 2026 Ankit Chaubey <ankitchaubey.dev@gmail.com>
// github.com/ankit-chaubey
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// All rights reserved 2026.
// ─────────────────────────────────────────────────────────────────────────────
mod cat;
mod checksum;
mod completions;
mod compress;
mod config;
mod diff;
mod index;
mod merge;
mod output;
mod prune;
mod repair;
mod restore;
mod scan;
mod search;
mod stats;
mod tar_writer;
mod update;
mod utils;
mod verify;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use compress::CompressionAlgo;
use config::Config;
use output::OutputCtx;

// ─── CLI definition ─────────────────────────────────────────────────────────

/// Archivum — deterministic, split, checksummed, compressed archive system.
/// Also available as `arc` (same binary, both names).
#[derive(Parser)]
#[command(
    name = "archivum",
    version = env!("CARGO_PKG_VERSION"),
    author = "Ankit Chaubey <ankitchaubey.dev@gmail.com>",
    about = "Deterministic, split, checksummed, compressed archive system",
    after_help = concat!(
        "EXAMPLES:
",
        "  archivum create ./photos ./backup
",
        "  arc create ./photos ./backup --compress zstd --split-gb 2
",
        "  archivum list ./backup/index.arc.json
",
        "  archivum restore ./backup/index.arc.json ./restored
",
        "  archivum verify ./backup/index.arc.json
",
        "  archivum diff ./backup/index.arc.json ./photos
",
        "  archivum update ./backup/index.arc.json ./photos ./backup2
",
        "  archivum stats ./backup/index.arc.json
",
        "  archivum search ./backup/index.arc.json '*.jpg'
",
        "  archivum cat ./backup/index.arc.json photos/img.jpg > img.jpg
",
        "  archivum completions bash >> ~/.bashrc
",
        "  archivum setup
",
        "
CONFIG: ~/.config/archivum/config.toml (run `archivum setup` to configure)"
    )
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output JSON instead of human-readable text
    #[arg(long, global = true)]
    json: bool,

    /// Suppress all output except errors
    #[arg(long, global = true, short = 'q')]
    quiet: bool,

    /// Show what would happen without doing it
    #[arg(long, global = true, short = 'n')]
    dry_run: bool,

    /// Append all output to this log file
    #[arg(long, global = true, value_name = "PATH")]
    log_file: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new archive from a source directory
    Create {
        #[arg(value_name = "SOURCE")]
        source: PathBuf,
        #[arg(value_name = "OUTPUT")]
        output: PathBuf,
        /// Max size per archive part in GB (config default: 4.0)
        #[arg(long, value_name = "GB")]
        split_gb: Option<f64>,
        /// Max files per archive part (0 = disabled)
        #[arg(long, value_name = "N", default_value = "0")]
        split_files: usize,
        /// Compression algorithm: none | gzip | bzip2 | lz4 | zstd
        #[arg(long, value_name = "ALGO")]
        compress: Option<String>,
        /// Zstd compression level (1–22)
        #[arg(long, value_name = "LEVEL")]
        zstd_level: Option<i32>,
        /// Exclude glob patterns (repeatable)
        #[arg(long, value_name = "PATTERN")]
        exclude: Vec<String>,
        /// Parallel checksum threads (config default: 4)
        #[arg(long, value_name = "N")]
        threads: Option<usize>,
        /// Deduplicate files with identical SHA-256
        #[arg(long)]
        dedup: bool,
        /// Optional description stored in the index
        #[arg(long, value_name = "TEXT")]
        notes: Option<String>,
    },

    /// List contents and statistics of an archive
    List {
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
        #[arg(value_name = "INDEX")]
        index: PathBuf,
        #[arg(value_name = "TARGET")]
        target: PathBuf,
        /// Only restore files matching this glob
        #[arg(long, value_name = "PATTERN")]
        filter: Option<String>,
        /// Overwrite existing files
        #[arg(long, short)]
        force: bool,
        /// Restore Unix permissions
        #[arg(long)]
        restore_permissions: bool,
    },

    /// Verify archive integrity (checksums + structure)
    Verify {
        #[arg(value_name = "INDEX")]
        index: PathBuf,
        /// Continue on errors instead of stopping
        #[arg(long, short = 'c')]
        continue_on_error: bool,
    },

    /// Compare archive against source directory (drift detection)
    Diff {
        #[arg(value_name = "INDEX")]
        index: PathBuf,
        #[arg(value_name = "SOURCE")]
        source: PathBuf,
        /// Show only changed files
        #[arg(long)]
        changed_only: bool,
        /// Use SHA-256 to detect changes (not just mtime+size)
        #[arg(long)]
        checksum: bool,
    },

    /// Print detailed info about a specific file in the archive
    Info {
        #[arg(value_name = "INDEX")]
        index: PathBuf,
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Extract a single file from the archive to a path
    Extract {
        #[arg(value_name = "INDEX")]
        index: PathBuf,
        #[arg(value_name = "FILE")]
        file: PathBuf,
        #[arg(long, value_name = "OUTPUT")]
        output: Option<PathBuf>,
    },

    /// Stream a single file from the archive to stdout
    Cat {
        #[arg(value_name = "INDEX")]
        index: PathBuf,
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Search the archive index by glob or substring
    Search {
        #[arg(value_name = "INDEX")]
        index: PathBuf,
        /// Glob or substring pattern
        #[arg(value_name = "PATTERN")]
        pattern: String,
    },

    /// Show detailed statistics for an archive
    Stats {
        #[arg(value_name = "INDEX")]
        index: PathBuf,
    },

    /// Incremental update: re-archive only changed/new files
    Update {
        /// Existing archive index to update from
        #[arg(value_name = "OLD_INDEX")]
        old_index: PathBuf,
        /// Source directory to archive
        #[arg(value_name = "SOURCE")]
        source: PathBuf,
        /// Output directory for new delta parts
        #[arg(value_name = "OUTPUT")]
        output: PathBuf,
        #[arg(long, value_name = "GB")]
        split_gb: Option<f64>,
        #[arg(long, value_name = "N", default_value = "0")]
        split_files: usize,
        #[arg(long, value_name = "ALGO")]
        compress: Option<String>,
        #[arg(long, value_name = "LEVEL")]
        zstd_level: Option<i32>,
        #[arg(long, value_name = "PATTERN")]
        exclude: Vec<String>,
        #[arg(long, value_name = "N")]
        threads: Option<usize>,
        /// Use SHA-256 comparison to detect changes
        #[arg(long)]
        checksum: bool,
    },

    /// Prune old archives in a directory
    Prune {
        /// Directory containing archive subdirectories
        #[arg(value_name = "DIR")]
        dir: PathBuf,
        /// Always keep at least N archives (config default: 3)
        #[arg(long, value_name = "N")]
        keep: Option<usize>,
        /// Delete archives older than N days (0 = any age; config default: 30)
        #[arg(long, value_name = "DAYS")]
        max_age: Option<u64>,
    },

    /// Merge multiple archives into one
    Merge {
        /// Two or more index.arc.json paths to merge
        #[arg(value_name = "INDEX", num_args = 2..)]
        indexes: Vec<PathBuf>,
        /// Output directory for merged archive
        #[arg(long, value_name = "OUTPUT", required = true)]
        output: PathBuf,
        #[arg(long, value_name = "GB")]
        split_gb: Option<f64>,
        #[arg(long, value_name = "ALGO")]
        compress: Option<String>,
        #[arg(long, value_name = "LEVEL")]
        zstd_level: Option<i32>,
    },

    /// Rebuild a missing index.arc.json from existing tar parts
    Repair {
        /// Directory containing the archive parts
        #[arg(value_name = "DIR")]
        dir: PathBuf,
        /// Compression algorithm of the parts
        #[arg(long, value_name = "ALGO", default_value = "zstd")]
        compression: String,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate for: bash | zsh | fish | powershell | elvish
        #[arg(value_name = "SHELL")]
        shell: String,
    },

    /// Interactive configuration setup
    Setup,

    /// Print current configuration
    Config,
}

// ─── Entry point ─────────────────────────────────────────────────────────────

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
    let cfg = Config::load();

    let out = OutputCtx::new(
        cli.json || cfg.output.json,
        cli.quiet || cfg.output.quiet,
        cli.dry_run,
        cli.log_file.as_deref(),
    )?;

    match cli.command {
        // ── Create ──────────────────────────────────────────────────────────
        Commands::Create {
            source,
            output,
            split_gb,
            split_files,
            compress,
            zstd_level,
            mut exclude,
            threads,
            dedup,
            notes,
        } => {
            utils::print_banner(&out);

            let compress_str = compress.as_deref().unwrap_or(&cfg.defaults.compress);
            let algo = CompressionAlgo::parse(compress_str)
                .with_context(|| format!("Unknown compression algorithm: '{compress_str}'"))?;
            let zstd_lvl = zstd_level.unwrap_or(cfg.defaults.zstd_level);
            let split =
                (split_gb.unwrap_or(cfg.defaults.split_gb) * 1024.0 * 1024.0 * 1024.0) as u64;
            let split_f = if split_files > 0 {
                split_files
            } else {
                cfg.defaults.split_files
            };
            let thread_count = threads.unwrap_or(cfg.defaults.threads);
            let do_dedup = dedup || cfg.create.dedup;

            // Merge config excludes
            let mut all_excludes = cfg.create.exclude.clone();
            all_excludes.append(&mut exclude);

            out.println(&format!(
                "{} {} → {}",
                "Creating archive:".cyan().bold(),
                source.display().to_string().yellow(),
                output.display().to_string().yellow()
            ));
            out.println(&format!(
                "  split={:.1} GB  split-files={}  compress={}  zstd-level={}  dedup={}  threads={}",
                split_gb.unwrap_or(cfg.defaults.split_gb),
                split_f,
                algo.name().green(),
                zstd_lvl,
                do_dedup,
                thread_count
            ));
            out.println("");

            if out.dry_run {
                out.dry(&format!("would scan: {}", source.display()));
                out.dry(&format!("would create: {}", output.display()));
                return Ok(());
            }

            if !source.exists() {
                anyhow::bail!("Source directory does not exist: {}", source.display());
            }

            let scan = scan::scan_directory(&source, &all_excludes)
                .with_context(|| format!("Failed to scan {}", source.display()))?;

            let mut idx = index::ArchivumIndex::build(scan, algo.clone(), zstd_lvl);

            if let Some(n) = notes {
                idx.header.notes = n;
            } else if !cfg.create.notes.is_empty() {
                idx.header.notes = cfg.create.notes.clone();
            }

            std::fs::create_dir_all(&output)
                .with_context(|| format!("Failed to create output dir {}", output.display()))?;

            checksum::compute_checksums(&source, &mut idx, thread_count)?;

            // If dedup NOT requested, clear dedup_of fields
            if !do_dedup {
                for e in idx.entries.iter_mut() {
                    e.dedup_of = None;
                }
            }

            tar_writer::write_archive(&source, &output, &mut idx, split, split_f, &algo, zstd_lvl)?;

            let index_path = output.join("index.arc.json");
            idx.write(&index_path)?;

            let deduped = idx.entries.iter().filter(|e| e.dedup_of.is_some()).count();

            out.println("");
            out.println(&"─".repeat(60).dimmed().to_string());
            out.println(&"  Archive created successfully!".green().bold().to_string());
            out.println(&format!(
                "  Files : {}  |  Dirs : {}  |  Parts : {}{}",
                idx.header.total_files.to_string().yellow(),
                idx.header.total_dirs.to_string().yellow(),
                idx.header.total_parts.to_string().yellow(),
                if deduped > 0 {
                    format!("  |  Deduped: {}", deduped.to_string().cyan())
                } else {
                    String::new()
                }
            ));
            out.println(&format!(
                "  Size  : {}  |  Compression: {}",
                utils::human(idx.header.total_size).cyan(),
                algo.name().green()
            ));
            out.println(&format!(
                "  Index : {}",
                index_path.display().to_string().cyan()
            ));
            out.println(&"─".repeat(60).dimmed().to_string());
        }

        // ── List ────────────────────────────────────────────────────────────
        Commands::List {
            index,
            verbose,
            filter,
        } => {
            let idx = index::ArchivumIndex::read(&index)
                .with_context(|| format!("Failed to read index: {}", index.display()))?;
            if out.json {
                idx.print_summary_json()?;
            } else {
                idx.print_summary(verbose, filter.as_deref(), &out)?;
            }
        }

        // ── Restore ─────────────────────────────────────────────────────────
        Commands::Restore {
            index,
            target,
            filter,
            force,
            restore_permissions,
        } => {
            utils::print_banner(&out);
            let do_force = force || cfg.restore.force;
            let do_perm = restore_permissions || cfg.restore.restore_permissions;
            restore::restore(&index, &target, filter.as_deref(), do_force, do_perm, &out)?;
        }

        // ── Verify ──────────────────────────────────────────────────────────
        Commands::Verify {
            index,
            continue_on_error,
        } => {
            utils::print_banner(&out);
            verify::verify(&index, continue_on_error, &out)?;
        }

        // ── Diff ────────────────────────────────────────────────────────────
        Commands::Diff {
            index,
            source,
            changed_only,
            checksum,
        } => {
            let use_cs = checksum || cfg.update.checksum_diff;
            diff::diff(&index, &source, changed_only, use_cs, &out)?;
        }

        // ── Info ────────────────────────────────────────────────────────────
        Commands::Info { index, file } => {
            let idx = index::ArchivumIndex::read(&index)?;
            if let Some(entry) = idx.entries.iter().find(|e| e.path == file) {
                if out.json {
                    let j = serde_json::json!({
                        "path": entry.path,
                        "type": format!("{:?}", entry.entry_type),
                        "size": entry.size,
                        "sha256": entry.sha256,
                        "tar_part": entry.tar_part,
                        "mtime": entry.mtime,
                        "unix_mode": entry.unix_mode,
                        "dedup_of": entry.dedup_of
                    });
                    println!("{}", serde_json::to_string_pretty(&j).unwrap());
                } else {
                    println!("{}", "─".repeat(50).dimmed());
                    println!("{} {}", "File:".cyan().bold(), entry.path.display());
                    println!(
                        "{} {}",
                        "Type:".cyan(),
                        format!("{:?}", entry.entry_type).green()
                    );
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
                    if let Some(ref orig) = entry.dedup_of {
                        println!(
                            "{} {}",
                            "Dedup of:".cyan(),
                            orig.display().to_string().yellow()
                        );
                    }
                    println!("{}", "─".repeat(50).dimmed());
                }
            } else {
                anyhow::bail!("File not found in archive: {}", file.display());
            }
        }

        // ── Extract ─────────────────────────────────────────────────────────
        Commands::Extract {
            index,
            file,
            output,
        } => {
            let idx = index::ArchivumIndex::read(&index)?;
            let base = index.parent().unwrap_or(std::path::Path::new("."));
            restore::extract_single(&idx, base, &file, output.as_deref(), &out)?;
        }

        // ── Cat ─────────────────────────────────────────────────────────────
        Commands::Cat { index, file } => {
            cat::cat(&index, &file)?;
        }

        // ── Search ──────────────────────────────────────────────────────────
        Commands::Search { index, pattern } => {
            search::search(&index, &pattern, &out)?;
        }

        // ── Stats ────────────────────────────────────────────────────────────
        Commands::Stats { index } => {
            stats::stats(&index, &out)?;
        }

        // ── Update ──────────────────────────────────────────────────────────
        Commands::Update {
            old_index,
            source,
            output,
            split_gb,
            split_files,
            compress,
            zstd_level,
            mut exclude,
            threads,
            checksum,
        } => {
            utils::print_banner(&out);
            let compress_str = compress.as_deref().unwrap_or(&cfg.defaults.compress);
            let algo = CompressionAlgo::parse(compress_str)?;
            let zstd_lvl = zstd_level.unwrap_or(cfg.defaults.zstd_level);
            let split =
                (split_gb.unwrap_or(cfg.defaults.split_gb) * 1024.0 * 1024.0 * 1024.0) as u64;
            let split_f = if split_files > 0 {
                split_files
            } else {
                cfg.defaults.split_files
            };
            let thread_count = threads.unwrap_or(cfg.defaults.threads);
            let use_cs = checksum || cfg.update.checksum_diff;
            let mut all_excludes = cfg.create.exclude.clone();
            all_excludes.append(&mut exclude);

            update::update(
                &old_index,
                &source,
                &output,
                split,
                split_f,
                &algo,
                zstd_lvl,
                thread_count,
                &all_excludes,
                use_cs,
                &out,
            )?;
        }

        // ── Prune ───────────────────────────────────────────────────────────
        Commands::Prune { dir, keep, max_age } => {
            let keep_n = keep.unwrap_or(cfg.prune.keep_last);
            let age = max_age.unwrap_or(cfg.prune.max_age_days);
            prune::prune(&dir, keep_n, age, &out)?;
        }

        // ── Merge ───────────────────────────────────────────────────────────
        Commands::Merge {
            indexes,
            output,
            split_gb,
            compress,
            zstd_level,
        } => {
            let compress_str = compress.as_deref().unwrap_or(&cfg.defaults.compress);
            let algo = CompressionAlgo::parse(compress_str)?;
            let zstd_lvl = zstd_level.unwrap_or(cfg.defaults.zstd_level);
            let split =
                (split_gb.unwrap_or(cfg.defaults.split_gb) * 1024.0 * 1024.0 * 1024.0) as u64;
            merge::merge(&indexes, &output, split, &algo, zstd_lvl, &out)?;
        }

        // ── Repair ──────────────────────────────────────────────────────────
        Commands::Repair { dir, compression } => {
            utils::print_banner(&out);
            repair::repair(&dir, &compression, &out)?;
        }

        // ── Completions ─────────────────────────────────────────────────────
        Commands::Completions { shell } => {
            completions::generate_completions(&shell)?;
        }

        // ── Setup ───────────────────────────────────────────────────────────
        Commands::Setup => {
            Config::setup_interactive()?;
        }

        // ── Config ──────────────────────────────────────────────────────────
        Commands::Config => {
            cfg.print();
            if let Some(p) = config::config_path() {
                if !p.exists() {
                    println!();
                    println!(
                        "  {} Config file does not exist yet. Run {} to create it.",
                        "Note:".yellow(),
                        "archivum setup".cyan()
                    );
                }
            }
        }
    }

    Ok(())
}
