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
//! ~/.config/archivum/config.toml — user-controlled defaults and preferences.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

// ─── Config structs ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub defaults: DefaultsConfig,
    pub create: CreateConfig,
    pub restore: RestoreConfig,
    pub update: UpdateConfig,
    pub output: OutputConfig,
    pub prune: PruneConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Default compression algorithm: none | gzip | bzip2 | lz4 | zstd
    pub compress: String,
    /// Zstd compression level (1–22)
    pub zstd_level: i32,
    /// Max size per archive part in GB
    pub split_gb: f64,
    /// Max files per part (0 = disabled, use split_gb)
    pub split_files: usize,
    /// Parallel checksum threads
    pub threads: usize,
    /// Enable colored terminal output
    pub color: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConfig {
    /// Default glob exclude patterns
    pub exclude: Vec<String>,
    /// Enable deduplication by SHA-256
    pub dedup: bool,
    /// Optional notes/description stored in the index
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreConfig {
    /// Overwrite existing files on restore
    pub force: bool,
    /// Restore Unix file permissions
    pub restore_permissions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Use full SHA-256 comparison (not just mtime+size) to detect changes
    pub checksum_diff: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output JSON instead of human-readable text
    pub json: bool,
    /// Suppress non-error output
    pub quiet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneConfig {
    /// Always keep at least this many archives
    pub keep_last: usize,
    /// Delete archives older than N days (0 = disabled)
    pub max_age_days: u64,
}

// ─── Defaults ──────────────────────────────────────────────────────────────

impl Default for Config {
    fn default() -> Self {
        Self {
            defaults: DefaultsConfig {
                compress: "zstd".into(),
                zstd_level: 3,
                split_gb: 4.0,
                split_files: 0,
                threads: 4,
                color: true,
            },
            create: CreateConfig {
                exclude: vec![
                    ".DS_Store".into(),
                    "Thumbs.db".into(),
                    "*.tmp".into(),
                    "*.swp".into(),
                ],
                dedup: false,
                notes: String::new(),
            },
            restore: RestoreConfig {
                force: false,
                restore_permissions: true,
            },
            update: UpdateConfig {
                checksum_diff: true,
            },
            output: OutputConfig {
                json: false,
                quiet: false,
            },
            prune: PruneConfig {
                keep_last: 3,
                max_age_days: 30,
            },
        }
    }
}

// ─── Load / Save ───────────────────────────────────────────────────────────

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("archivum").join("config.toml"))
}

impl Config {
    /// Load config from disk, creating defaults if missing.
    pub fn load() -> Self {
        if let Some(path) = config_path() {
            if path.exists() {
                match Self::load_from(&path) {
                    Ok(cfg) => return cfg,
                    Err(e) => {
                        eprintln!(
                            "{} Could not parse config at {}: {}",
                            "warning:".yellow(),
                            path.display(),
                            e
                        );
                    }
                }
            }
        }
        Config::default()
    }

    fn load_from(path: &PathBuf) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&text).with_context(|| format!("Invalid TOML in {}", path.display()))?;
        Ok(cfg)
    }

    /// Write config to disk.
    pub fn save(&self) -> Result<()> {
        if let Some(path) = config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Cannot create config dir {}", parent.display()))?;
            }
            let text = toml::to_string_pretty(self).context("Failed to serialize config")?;
            fs::write(&path, text)
                .with_context(|| format!("Cannot write config to {}", path.display()))?;
            println!(
                "{} {}",
                "Config saved to:".green().bold(),
                path.display().to_string().yellow()
            );
        } else {
            anyhow::bail!("Cannot determine config directory for this OS");
        }
        Ok(())
    }

    /// Interactive setup — prompts user for each field.
    pub fn setup_interactive() -> Result<()> {
        let mut cfg = Config::load();

        println!("{}", "─".repeat(60).dimmed());
        println!("{}", "  Archivum Interactive Setup".cyan().bold());
        println!(
            "  {}",
            "Press Enter to keep current value shown in [brackets]".dimmed()
        );
        println!("{}", "─".repeat(60).dimmed());

        // ── Compression
        cfg.defaults.compress = prompt(
            "Default compression (none/gzip/bzip2/lz4/zstd)",
            &cfg.defaults.compress,
        )?;

        // ── Zstd level
        if cfg.defaults.compress == "zstd" {
            let level_str = prompt(
                "Zstd compression level (1-22, higher=smaller/slower)",
                &cfg.defaults.zstd_level.to_string(),
            )?;
            if let Ok(v) = level_str.parse::<i32>() {
                cfg.defaults.zstd_level = v.clamp(1, 22);
            }
        }

        // ── Split size
        let split_str = prompt(
            "Max archive part size in GB",
            &cfg.defaults.split_gb.to_string(),
        )?;
        if let Ok(v) = split_str.parse::<f64>() {
            cfg.defaults.split_gb = v;
        }

        // ── Threads
        let threads_str = prompt(
            "Parallel checksum threads",
            &cfg.defaults.threads.to_string(),
        )?;
        if let Ok(v) = threads_str.parse::<usize>() {
            cfg.defaults.threads = v.max(1);
        }

        // ── Dedup
        let dedup_str = prompt(
            "Enable deduplication by default (true/false)",
            &cfg.create.dedup.to_string(),
        )?;
        cfg.create.dedup = dedup_str.eq_ignore_ascii_case("true") || dedup_str == "1";

        // ── Exclude patterns
        println!(
            "
  {} (current: {})",
            "Default exclude patterns (comma-separated globs):".cyan(),
            cfg.create.exclude.join(", ").yellow()
        );
        let excl = prompt("Exclude patterns", &cfg.create.exclude.join(","))?;
        if !excl.trim().is_empty() {
            cfg.create.exclude = excl.split(',').map(|s| s.trim().to_string()).collect();
        }

        // ── Restore permissions
        let perm_str = prompt(
            "Restore Unix permissions by default (true/false)",
            &cfg.restore.restore_permissions.to_string(),
        )?;
        cfg.restore.restore_permissions = perm_str.eq_ignore_ascii_case("true") || perm_str == "1";

        // ── Prune keep
        let keep_str = prompt(
            "Minimum archives to keep during prune",
            &cfg.prune.keep_last.to_string(),
        )?;
        if let Ok(v) = keep_str.parse::<usize>() {
            cfg.prune.keep_last = v.max(1);
        }

        // ── Prune age
        let age_str = prompt(
            "Max age (days) for prune (0 = disable age pruning)",
            &cfg.prune.max_age_days.to_string(),
        )?;
        if let Ok(v) = age_str.parse::<u64>() {
            cfg.prune.max_age_days = v;
        }

        println!();
        cfg.save()?;
        println!("{}", "  Setup complete!".green().bold());
        println!("{}", "─".repeat(60).dimmed());
        Ok(())
    }

    /// Print current config in a readable table.
    pub fn print(&self) {
        println!("{}", "─".repeat(60).dimmed());
        println!("{}", "  Current Configuration".cyan().bold());
        println!("{}", "─".repeat(60).dimmed());

        let p = config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<unknown>".into());
        println!("  {} {}", "Config file:".dimmed(), p.yellow());
        println!();

        println!("  [defaults]");
        println!("    compress      = {}", self.defaults.compress.green());
        println!(
            "    zstd_level    = {}",
            self.defaults.zstd_level.to_string().yellow()
        );
        println!(
            "    split_gb      = {}",
            self.defaults.split_gb.to_string().yellow()
        );
        println!(
            "    split_files   = {}",
            self.defaults.split_files.to_string().yellow()
        );
        println!(
            "    threads       = {}",
            self.defaults.threads.to_string().yellow()
        );
        println!(
            "    color         = {}",
            self.defaults.color.to_string().yellow()
        );

        println!();
        println!("  [create]");
        println!(
            "    dedup         = {}",
            self.create.dedup.to_string().yellow()
        );
        println!("    notes         = {:?}", self.create.notes);
        println!("    exclude       = {:?}", self.create.exclude);

        println!();
        println!("  [restore]");
        println!(
            "    force                = {}",
            self.restore.force.to_string().yellow()
        );
        println!(
            "    restore_permissions  = {}",
            self.restore.restore_permissions.to_string().yellow()
        );

        println!();
        println!("  [update]");
        println!(
            "    checksum_diff = {}",
            self.update.checksum_diff.to_string().yellow()
        );

        println!();
        println!("  [prune]");
        println!(
            "    keep_last    = {}",
            self.prune.keep_last.to_string().yellow()
        );
        println!(
            "    max_age_days = {}",
            self.prune.max_age_days.to_string().yellow()
        );

        println!("{}", "─".repeat(60).dimmed());
    }
}

// ─── Prompt helper ─────────────────────────────────────────────────────────

fn prompt(label: &str, current: &str) -> Result<String> {
    print!("  {} [{}]: ", label.cyan(), current.yellow());
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        Ok(current.to_string())
    } else {
        Ok(trimmed)
    }
}
