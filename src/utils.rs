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

use chrono::{DateTime, Utc};

pub fn human(b: u64) -> String {
    use humansize::{BINARY, format_size};
    format_size(b, BINARY)
}

pub fn fmt_time(unix: u64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(unix as i64, 0).unwrap_or_default();
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

pub fn now() -> u64 {
    use std::time::*;
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn print_banner(out: &crate::output::OutputCtx) {
    use colored::Colorize;
    out.println(
        &format!(
            " ▲ Archivum v{}  - deterministic archive system ",
            env!("CARGO_PKG_VERSION")
        )
        .black()
        .on_cyan()
        .bold()
        .to_string(),
    );
    out.println("");
}
