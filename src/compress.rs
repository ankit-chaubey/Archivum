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
    Zstd,
}

impl CompressionAlgo {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "none" | "raw" => Ok(Self::None),
            "gzip" | "gz" => Ok(Self::Gzip),
            "zstd" | "zst" => Ok(Self::Zstd),
            other => bail!("Unknown compression: '{}'. Use: none, gzip, zstd", other),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Gzip => "gzip",
            Self::Zstd => "zstd",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::None => ".tar",
            Self::Gzip => ".tar.gz",
            Self::Zstd => ".tar.zst",
        }
    }

    /// Wrap a File in a compressing writer
    pub fn wrap_writer(&self, file: File) -> Result<Box<dyn Write>> {
        match self {
            Self::None => Ok(Box::new(BufWriter::new(file))),
            Self::Gzip => {
                use flate2::{write::GzEncoder, Compression};
                Ok(Box::new(GzEncoder::new(file, Compression::default())))
            }
            Self::Zstd => {
                // zstd::Encoder::auto_finish() returns an AutoFinishEncoder which impl Write
                let enc = zstd::Encoder::new(file, 3)?;
                Ok(Box::new(enc.auto_finish()))
            }
        }
    }

    /// Wrap a file at the given path in a decompressing reader
    pub fn wrap_reader(&self, path: &Path) -> Result<Box<dyn Read>> {
        let file = File::open(path)?;
        match self {
            Self::None => Ok(Box::new(BufReader::new(file))),
            Self::Gzip => {
                use flate2::read::GzDecoder;
                Ok(Box::new(GzDecoder::new(file)))
            }
            Self::Zstd => Ok(Box::new(zstd::Decoder::new(file)?)),
        }
    }
}

