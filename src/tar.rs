/*
 * Copyright 2026 Ankit Chaubey <ankitchaubey.dev@gmail.com>
 * github.com/ankit-chaubey
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use tar::Builder;

use crate::index::ArchivumIndex;
use crate::scan::EntryType;

const TAR_BLOCK: u64 = 512;

pub struct TarWriter {
    out_dir: PathBuf,
    split_bytes: u64,
    current_part: u32,
    current_size: u64,
    builder: Builder<File>,
}

impl TarWriter {
    pub fn new(out_dir: &Path, split_bytes: u64) -> io::Result<Self> {
        let file = create_part(out_dir, 0)?;
        Ok(Self {
            out_dir: out_dir.to_path_buf(),
            split_bytes,
            current_part: 0,
            current_size: 0,
            builder: Builder::new(file),
        })
    }

    pub fn write_all(mut self, root: &Path, index: &mut ArchivumIndex) -> io::Result<()> {
        for entry in index.entries.iter_mut() {
            if entry.entry_type != EntryType::File {
                continue;
            }

            let full = root.join(&entry.path);
            let required =
                TAR_BLOCK + ((entry.size + TAR_BLOCK - 1) / TAR_BLOCK) * TAR_BLOCK;

            if self.current_size + required > self.split_bytes {
                self.rotate()?;
            }

            let mut file = File::open(full)?;
            self.builder.append_file(&entry.path, &mut file)?;

            entry.tar_part = self.current_part;
            self.current_size += required;
        }

        self.builder.finish()?;
        Ok(())
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.builder.finish()?;
        self.current_part += 1;
        self.current_size = 0;
        let file = create_part(&self.out_dir, self.current_part)?;
        self.builder = Builder::new(file);
        Ok(())
    }
}

fn create_part(out_dir: &Path, part: u32) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out_dir.join(format!("data.part{:03}.tar", part)))
}
