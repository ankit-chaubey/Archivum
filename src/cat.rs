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
//! `cat` — stream a single file from an archive to stdout.

use anyhow::{Context, Result};
use std::io::{self, copy};
use std::path::Path;

use crate::index::ArchivumIndex;
use crate::scan::EntryType;

pub fn cat(index_path: &Path, file: &Path) -> Result<()> {
    let idx = ArchivumIndex::read(index_path)?;
    let index_dir = index_path.parent().unwrap_or(Path::new("."));

    let entry = idx
        .entries
        .iter()
        .find(|e| e.path == file)
        .with_context(|| format!("File not found in archive: {}", file.display()))?;

    if entry.entry_type != EntryType::File {
        anyhow::bail!("Entry is not a regular file: {}", file.display());
    }

    // For deduped files, read from the original
    let (target_path, target_entry) = if let Some(ref orig) = entry.dedup_of {
        let orig_entry = idx
            .entries
            .iter()
            .find(|e| &e.path == orig)
            .with_context(|| format!("Dedup origin not found: {}", orig.display()))?;
        (orig.as_path(), orig_entry)
    } else {
        (file, entry)
    };

    let part_path = target_entry.part_path(index_dir, &idx.header);
    let reader = idx.header.compression.wrap_reader(&part_path)?;
    let mut archive = tar::Archive::new(reader);

    let mut stdout = io::stdout();
    for item in archive.entries()? {
        let mut item = item?;
        if item.path()? == target_path {
            copy(&mut item, &mut stdout)?;
            return Ok(());
        }
    }

    anyhow::bail!("File not found inside tar: {}", file.display());
}
