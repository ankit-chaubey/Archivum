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
//! Output context — respects --quiet, --json, and --log-file flags.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;

/// Shared output context passed through all commands.
#[derive(Clone)]
pub struct OutputCtx {
    pub json: bool,
    pub quiet: bool,
    pub dry_run: bool,
    log: Option<Arc<Mutex<File>>>,
}

impl OutputCtx {
    pub fn new(json: bool, quiet: bool, dry_run: bool, log_file: Option<&Path>) -> Result<Self> {
        let log = if let Some(path) = log_file {
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| anyhow::anyhow!("Cannot open log file {}: {}", path.display(), e))?;
            Some(Arc::new(Mutex::new(f)))
        } else {
            None
        };
        Ok(Self {
            json,
            quiet,
            dry_run,
            log,
        })
    }

    /// Print a line to stdout (unless quiet), and also to log file (no ANSI).
    pub fn println(&self, line: &str) {
        if !self.quiet {
            println!("{}", line);
        }
        self.write_log(line);
    }

    /// Always print to stderr + log file.
    pub fn eprintln(&self, line: &str) {
        eprintln!("{}", line);
        self.write_log(&format!("ERROR: {}", line));
    }

    /// Print a "dry-run would do X" message.
    pub fn dry(&self, line: &str) {
        if !self.quiet {
            println!("[dry-run] {}", line);
        }
        self.write_log(&format!("[dry-run] {}", line));
    }

    fn write_log(&self, line: &str) {
        if let Some(log) = &self.log {
            let plain = strip_ansi(line);
            let mut f = log.lock().unwrap();
            let _ = writeln!(f, "{}", plain);
        }
    }

    /// Print a raw string to stdout regardless of quiet mode (for JSON / cat output).
    pub fn raw(&self, s: &str) {
        print!("{}", s);
    }
}

/// Remove ANSI escape sequences for clean log output.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // consume until 'm'
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}
