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
//! `completions` — generate shell completion scripts.

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, shells};
use std::io;

use crate::Cli;

pub fn generate_completions(shell: &str) -> Result<()> {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    let mut stdout = io::stdout();

    match shell.to_lowercase().as_str() {
        "bash" => generate(shells::Bash, &mut cmd, &bin_name, &mut stdout),
        "zsh" => generate(shells::Zsh, &mut cmd, &bin_name, &mut stdout),
        "fish" => generate(shells::Fish, &mut cmd, &bin_name, &mut stdout),
        "powershell" | "pwsh" => {
            generate(shells::PowerShell, &mut cmd, &bin_name, &mut stdout)
        }
        "elvish" => generate(shells::Elvish, &mut cmd, &bin_name, &mut stdout),
        other => {
            anyhow::bail!(
                "Unknown shell '{}'. Supported: bash, zsh, fish, powershell, elvish",
                other
            );
        }
    }

    Ok(())
}
