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
//! Compression algorithm support: none, gzip, bzip2, lz4, zstd.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompressionAlgo {
    #[default]
    None,
    Gzip,
    Bzip2,
    Lz4,
    Zstd,
}

impl CompressionAlgo {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "none" | "raw" => Ok(Self::None),
            "gzip" | "gz" => Ok(Self::Gzip),
            "bzip2" | "bz2" => Ok(Self::Bzip2),
            "lz4" => Ok(Self::Lz4),
            "zstd" | "zst" => Ok(Self::Zstd),
            other => bail!(
                "Unknown compression: '{}'. Use: none, gzip, bzip2, lz4, zstd",
                other
            ),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Gzip => "gzip",
            Self::Bzip2 => "bzip2",
            Self::Lz4 => "lz4",
            Self::Zstd => "zstd",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::None => ".tar",
            Self::Gzip => ".tar.gz",
            Self::Bzip2 => ".tar.bz2",
            Self::Lz4 => ".tar.lz4",
            Self::Zstd => ".tar.zst",
        }
    }

    /// Wrap a file writer with this compression.
    pub fn wrap_writer(&self, file: File, zstd_level: i32) -> Result<Box<dyn Write>> {
        match self {
            Self::None => Ok(Box::new(BufWriter::new(file))),
            Self::Gzip => {
                use flate2::{write::GzEncoder, Compression};
                Ok(Box::new(GzEncoder::new(file, Compression::default())))
            }
            Self::Bzip2 => {
                use bzip2::write::BzEncoder;
                use bzip2::Compression;
                Ok(Box::new(BzEncoder::new(file, Compression::default())))
            }
            Self::Lz4 => {
                use lz4_flex::frame::FrameEncoder;
                Ok(Box::new(Lz4Writer(Some(FrameEncoder::new(file)))))
            }
            Self::Zstd => {
                let level = zstd_level.clamp(1, 22);
                let enc = zstd::Encoder::new(file, level)?;
                Ok(Box::new(enc.auto_finish()))
            }
        }
    }

    /// Wrap a file reader with this decompression.
    pub fn wrap_reader(&self, path: &Path) -> Result<Box<dyn Read>> {
        let file = File::open(path)?;
        match self {
            Self::None => Ok(Box::new(BufReader::new(file))),
            Self::Gzip => {
                use flate2::read::GzDecoder;
                Ok(Box::new(GzDecoder::new(file)))
            }
            Self::Bzip2 => {
                use bzip2::read::BzDecoder;
                Ok(Box::new(BzDecoder::new(file)))
            }
            Self::Lz4 => {
                use lz4_flex::frame::FrameDecoder;
                Ok(Box::new(FrameDecoder::new(file)))
            }
            Self::Zstd => Ok(Box::new(zstd::Decoder::new(file)?)),
        }
    }
}

// ─── Lz4 wrapper that auto-finishes on drop ─────────────────────────────────

struct Lz4Writer<W: Write + 'static>(Option<lz4_flex::frame::FrameEncoder<W>>);

impl<W: Write + 'static> Write for Lz4Writer<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.as_mut().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.as_mut().unwrap().flush()
    }
}

impl<W: Write + 'static> Drop for Lz4Writer<W> {
    fn drop(&mut self) {
        if let Some(enc) = self.0.take() {
            let _: std::result::Result<_, _> = enc.finish();
        }
    }
}
