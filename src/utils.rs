use chrono::{DateTime, Utc};

pub fn human(b: u64) -> String {
    use humansize::{format_size, BINARY};
    format_size(b, BINARY)
}

pub fn fmt_time(unix: u64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(unix as i64, 0)
        .unwrap_or_default();
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

pub fn now() -> u64 {
    use std::time::*;
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn print_banner() {
    use colored::Colorize;
    println!(
        "{}",
        format!(
            " ▲ Archivum v{}  — deterministic archive system ",
            env!("CARGO_PKG_VERSION")
        )
        .black()
        .on_cyan()
        .bold()
    );
    println!();
}
