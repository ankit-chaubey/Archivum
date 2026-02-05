use std::fs::{self, File, OpenOptions};
use std::io::{self, copy};
use std::path::Path;
use tar::Archive;

use crate::index::ArchivumIndex;
use crate::scan::EntryType;

pub fn restore(index_path: &str, target_dir: &str) -> io::Result<()> {
    let index_path = Path::new(index_path);
    let base_dir = index_path.parent().unwrap_or(Path::new("."));
    let index = ArchivumIndex::read(index_path)?;
    let target = Path::new(target_dir);

    // Create directories first
    for entry in &index.entries {
        if entry.entry_type == EntryType::Directory {
            fs::create_dir_all(target.join(&entry.path))?;
        }
    }

    // Restore files
    for entry in &index.entries {
        if entry.entry_type != EntryType::File {
            continue;
        }

        let tar_path = base_dir.join(format!("data.part{:03}.tar", entry.tar_part));
        let file = File::open(&tar_path)?;
        let mut archive = Archive::new(file);

        for item in archive.entries()? {
            let mut item = item?;
            if item.path()? == entry.path {
                let out_path = target.join(&entry.path);
                if let Some(p) = out_path.parent() {
                    fs::create_dir_all(p)?;
                }

                let mut out = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(out_path)?;

                copy(&mut item, &mut out)?;
                break;
            }
        }
    }

    Ok(())
}
