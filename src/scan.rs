use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone)]
pub struct ScanEntry {
    pub relative_path: PathBuf,
    pub entry_type: EntryType,
    pub size: u64,
    pub mtime: Option<u64>,
    pub unix_mode: Option<u32>,
    pub symlink_target: Option<PathBuf>,
}

pub fn scan_directory(root: &Path, excludes: &[String]) -> Result<Vec<ScanEntry>> {
    let excludeset = build_globset(excludes)?;
    let mut out = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let rel = path.strip_prefix(root)?.to_path_buf();

        if rel.as_os_str().is_empty() {
            continue;
        }

        if excludeset.is_match(&rel) {
            continue;
        }

        let meta = fs::symlink_metadata(path)?;

        #[cfg(unix)]
        let (mtime, mode) = (Some(meta.mtime() as u64), Some(meta.mode()));
        #[cfg(not(unix))]
        let (mtime, mode) = {
            let m = meta.modified().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs())
            });
            (m, None)
        };

        if meta.is_symlink() {
            let target = fs::read_link(path).ok();
            out.push(ScanEntry {
                relative_path: rel,
                entry_type: EntryType::Symlink,
                size: 0,
                mtime,
                unix_mode: mode,
                symlink_target: target,
            });
        } else if meta.is_dir() {
            out.push(ScanEntry {
                relative_path: rel,
                entry_type: EntryType::Directory,
                size: 0,
                mtime,
                unix_mode: mode,
                symlink_target: None,
            });
        } else if meta.is_file() {
            out.push(ScanEntry {
                relative_path: rel,
                entry_type: EntryType::File,
                size: meta.len(),
                mtime,
                unix_mode: mode,
                symlink_target: None,
            });
        }
    }

    Ok(out)
}

fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(Glob::new(p)?);
    }
    Ok(builder.build()?)
}
