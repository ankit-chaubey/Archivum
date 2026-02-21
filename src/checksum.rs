use anyhow::Result;
use colored::Colorize;
use hex::encode;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::index::ArchivumIndex;
use crate::scan::EntryType;

pub fn compute_checksums(root: &Path, idx: &mut ArchivumIndex, num_threads: usize) -> Result<()> {
    let total: u64 = idx
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::File)
        .map(|e| e.size)
        .sum();

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} Checksums  [{bar:40.cyan/blue}] {bytes}/{total_bytes}  {elapsed}",
        )
        .unwrap()
        .progress_chars("ââââââââ "),
    );

    let work: Vec<(usize, std::path::PathBuf, u64)> = idx
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.entry_type == EntryType::File)
        .map(|(i, e)| (i, root.join(&e.path), e.size))
        .collect();

    let pb_arc = Arc::new(pb);
    let results: Arc<Mutex<Vec<(usize, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let work_arc = Arc::new(Mutex::new(work.into_iter()));

    let mut handles = vec![];
    for _ in 0..num_threads.max(1) {
        let work_arc = Arc::clone(&work_arc);
        let results = Arc::clone(&results);
        let thread_pb = Arc::clone(&pb_arc);

        let handle = thread::spawn(move || -> Result<()> {
            loop {
                let item = {
                    let mut w = work_arc.lock().unwrap();
                    w.next()
                };
                match item {
                    None => break,
                    Some((idx_pos, path, size)) => {
                        let hash = hash_file(&path)?;
                        results.lock().unwrap().push((idx_pos, hash));
                        thread_pb.inc(size);
                    }
                }
            }
            Ok(())
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap()?;
    }

    pb_arc.finish_with_message("checksums done".green().to_string());

    let res = results.lock().unwrap();
    for (i, hash) in res.iter() {
        idx.entries[*i].sha256 = Some(hash.clone());
    }

    Ok(())
}

pub fn hash_file(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(encode(hasher.finalize()))
}
