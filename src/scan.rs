use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntryType {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub struct ScanEntry {
    pub relative_path: PathBuf,
    pub entry_type: EntryType,
    pub size: u64,
    pub mtime: Option<u64>,
    pub unix_mode: Option<u32>,
}

pub fn scan_directory(root: &Path) -> io::Result<Vec<ScanEntry>> {
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

fn walk(root: &Path, path: &Path, out: &mut Vec<ScanEntry>) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    let rel = path.strip_prefix(root).unwrap_or(path).to_path_buf();

    #[cfg(unix)]
    let (mtime, mode) = (Some(meta.mtime() as u64), Some(meta.mode()));
    #[cfg(not(unix))]
    let (mtime, mode) = (None, None);

    if meta.is_dir() {
        out.push(ScanEntry {
            relative_path: rel.clone(),
            entry_type: EntryType::Directory,
            size: 0,
            mtime,
            unix_mode: mode,
        });
        for e in fs::read_dir(path)? {
            walk(root, &e?.path(), out)?;
        }
    } else if meta.is_file() {
        out.push(ScanEntry {
            relative_path: rel,
            entry_type: EntryType::File,
            size: meta.len(),
            mtime,
            unix_mode: mode,
        });
    }
    Ok(())
}
