use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use crate::scan::{ScanEntry, EntryType};

pub const INDEX_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexHeader {
    pub version: u32,
    pub created_at_unix: u64,
    pub total_files: u64,
    pub total_dirs: u64,
    pub total_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexEntry {
    pub path: PathBuf,
    pub entry_type: EntryType,
    pub size: u64,
    pub mtime: Option<u64>,
    pub unix_mode: Option<u32>,
    pub tar_part: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchivumIndex {
    pub header: IndexHeader,
    pub entries: Vec<IndexEntry>,
}

impl ArchivumIndex {
    pub fn build(scan: Vec<ScanEntry>) -> Self {
        let mut files = 0;
        let mut dirs = 0;
        let mut size = 0;

        let entries = scan.into_iter().map(|e| {
            match e.entry_type {
                EntryType::File => { files += 1; size += e.size; }
                EntryType::Directory => dirs += 1,
            }

            IndexEntry {
                path: e.relative_path,
                entry_type: e.entry_type,
                size: e.size,
                mtime: e.mtime,
                unix_mode: e.unix_mode,
                tar_part: 0,
            }
        }).collect();

        Self {
            header: IndexHeader {
                version: INDEX_VERSION,
                created_at_unix: now(),
                total_files: files,
                total_dirs: dirs,
                total_size: size,
            },
            entries,
        }
    }

    pub fn write(&self, path: &Path) -> io::Result<()> {
        serde_json::to_writer_pretty(
            BufWriter::new(File::create(path)?),
            self
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    pub fn read(path: &Path) -> io::Result<Self> {
        serde_json::from_reader(
            BufReader::new(File::open(path)?)
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    pub fn print_summary(&self) {
        println!("Files: {}", self.header.total_files);
        println!("Dirs : {}", self.header.total_dirs);
        println!("Size : {}", human(self.header.total_size));
    }
}

fn now() -> u64 {
    use std::time::*;
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn human(b: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    match b {
        b if b >= GB => format!("{:.2} GB", b as f64 / GB as f64),
        b if b >= MB => format!("{:.2} MB", b as f64 / MB as f64),
        b if b >= KB => format!("{:.2} KB", b as f64 / KB as f64),
        b => format!("{b} B"),
    }
}
