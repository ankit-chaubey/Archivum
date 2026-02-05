use std::env;
use std::path::Path;

mod index;
mod scan;
mod tar;
mod restore;

use index::ArchivumIndex;
use scan::scan_directory;
use tar::TarWriter;
use restore::restore;

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() < 2 { help(); return; }

    match a[1].as_str() {
        "create" => {
            let src = Path::new(&a[2]);
            let out = Path::new(&a[3]);
            let gb: u64 = a.get(4).and_then(|x| x.parse().ok()).unwrap_or(4);

            let scan = scan_directory(src).unwrap();
            let mut idx = ArchivumIndex::build(scan);
            std::fs::create_dir_all(out).unwrap();

            let tw = TarWriter::new(out, gb * 1024 * 1024 * 1024).unwrap();
            tw.write_all(src, &mut idx).unwrap();

            idx.write(&out.join("index.arc.json")).unwrap();
            println!("Archive created");
        }

        "list" => {
            ArchivumIndex::read(Path::new(&a[2])).unwrap().print_summary();
        }

        "restore" => {
            restore(&a[2], &a[3]).unwrap();
            println!("Restore complete");
        }

        _ => help(),
    }
}

fn help() {
    println!("archivum create <src> <out> [gb]");
    println!("archivum list <index>");
    println!("archivum restore <index> <target>");
}
