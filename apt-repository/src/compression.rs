//! Compression support for APT repository files.

use crate::Result;
use std::io::{Read, Write};

/// Supported compression formats for APT repository files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// No compression.
    None,
    /// Gzip compression.
    Gzip,
    /// Bzip2 compression.
    Bzip2,
}

impl Compression {
    /// Get the file extension for this compression format.
    pub fn extension(&self) -> &'static str {
        match self {
            Compression::None => "",
            Compression::Gzip => ".gz",
            Compression::Bzip2 => ".bz2",
        }
    }

    /// Get the MIME type for this compression format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Compression::None => "text/plain",
            Compression::Gzip => "application/gzip",
            Compression::Bzip2 => "application/x-bzip2",
        }
    }

    /// Create a compressor that implements Write.
    pub fn writer<W: Write + 'static>(self, writer: W) -> Result<Box<dyn Write>> {
        match self {
            Compression::None => Ok(Box::new(writer)),
            Compression::Gzip => {
                let encoder = flate2::write::GzEncoder::new(writer, flate2::Compression::default());
                Ok(Box::new(encoder))
            }
            Compression::Bzip2 => {
                let encoder = bzip2::write::BzEncoder::new(writer, bzip2::Compression::default());
                Ok(Box::new(encoder))
            }
        }
    }

    /// Create a decompressor that implements Read.
    pub fn reader<R: Read + 'static>(self, reader: R) -> Result<Box<dyn Read>> {
        match self {
            Compression::None => Ok(Box::new(reader)),
            Compression::Gzip => {
                let decoder = flate2::read::GzDecoder::new(reader);
                Ok(Box::new(decoder))
            }
            Compression::Bzip2 => {
                let decoder = bzip2::read::BzDecoder::new(reader);
                Ok(Box::new(decoder))
            }
        }
    }

    /// Compress data using this compression format.
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Compression::None => Ok(data.to_vec()),
            Compression::Gzip => {
                let mut compressed = Vec::new();
                let mut encoder =
                    flate2::write::GzEncoder::new(&mut compressed, flate2::Compression::default());
                encoder.write_all(data)?;
                encoder.finish()?;
                Ok(compressed)
            }
            Compression::Bzip2 => {
                let mut compressed = Vec::new();
                let mut encoder =
                    bzip2::write::BzEncoder::new(&mut compressed, bzip2::Compression::default());
                encoder.write_all(data)?;
                encoder.finish()?;
                Ok(compressed)
            }
        }
    }

    /// Decompress data using this compression format.
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Compression::None => Ok(data.to_vec()),
            Compression::Gzip => {
                let mut decompressed = Vec::new();
                let mut decoder = flate2::read::GzDecoder::new(data);
                decoder.read_to_end(&mut decompressed)?;
                Ok(decompressed)
            }
            Compression::Bzip2 => {
                let mut decompressed = Vec::new();
                let mut decoder = bzip2::read::BzDecoder::new(data);
                decoder.read_to_end(&mut decompressed)?;
                Ok(decompressed)
            }
        }
    }

    /// Get all supported compression formats.
    pub fn all() -> &'static [Compression] {
        &[Compression::None, Compression::Gzip, Compression::Bzip2]
    }
}

impl std::fmt::Display for Compression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Compression::None => write!(f, "none"),
            Compression::Gzip => write!(f, "gzip"),
            Compression::Bzip2 => write!(f, "bzip2"),
        }
    }
}

/// A writer that supports multiple compression formats simultaneously.
pub struct MultiCompressionWriter<W: Write> {
    writers: Vec<(Compression, Box<dyn Write>)>,
    _phantom: std::marker::PhantomData<W>,
}

impl<W: Write + Clone + 'static> MultiCompressionWriter<W> {
    /// Create a new multi-compression writer.
    pub fn new(base_writer: W, compressions: &[Compression]) -> Result<Self> {
        let mut writers = Vec::new();

        for &compression in compressions {
            let writer = base_writer.clone();
            let compressed_writer = compression.writer(writer)?;
            writers.push((compression, compressed_writer));
        }

        Ok(Self {
            writers,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Write data to all compression formats.
    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        for (_, writer) in &mut self.writers {
            writer.write_all(data)?;
        }
        Ok(())
    }

    /// Flush all writers.
    pub fn flush(&mut self) -> Result<()> {
        for (_, writer) in &mut self.writers {
            writer.flush()?;
        }
        Ok(())
    }

    /// Finish all compression writers.
    pub fn finish(mut self) -> Result<()> {
        self.flush()?;

        // Finish compression for encoders that need it
        for (compression, mut writer) in self.writers {
            match compression {
                Compression::Gzip => {
                    // The GzEncoder will finish when dropped
                }
                Compression::Bzip2 => {
                    // The BzEncoder will finish when dropped
                }
                Compression::None => {}
            }
            writer.flush()?;
        }

        Ok(())
    }
}

// We can't implement Write for MultiCompressionWriter because we need to handle
// multiple writers, and the Write trait doesn't allow for proper error handling
// in all cases we need.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_extensions() {
        assert_eq!(Compression::None.extension(), "");
        assert_eq!(Compression::Gzip.extension(), ".gz");
        assert_eq!(Compression::Bzip2.extension(), ".bz2");
    }

    #[test]
    fn test_compression_mime_types() {
        assert_eq!(Compression::None.mime_type(), "text/plain");
        assert_eq!(Compression::Gzip.mime_type(), "application/gzip");
        assert_eq!(Compression::Bzip2.mime_type(), "application/x-bzip2");
    }

    #[test]
    fn test_no_compression() -> Result<()> {
        let data = b"hello world";
        let compressed = Compression::None.compress(data)?;
        assert_eq!(compressed, data);

        let decompressed = Compression::None.decompress(&compressed)?;
        assert_eq!(decompressed, data);

        Ok(())
    }

    #[test]
    fn test_gzip_compression() -> Result<()> {
        let data = b"hello world";
        let compressed = Compression::Gzip.compress(data)?;
        assert_ne!(compressed, data);
        assert!(compressed.len() > 0);

        let decompressed = Compression::Gzip.decompress(&compressed)?;
        assert_eq!(decompressed, data);

        Ok(())
    }

    #[test]
    fn test_bzip2_compression() -> Result<()> {
        let data = b"hello world";
        let compressed = Compression::Bzip2.compress(data)?;
        assert_ne!(compressed, data);
        assert!(compressed.len() > 0);

        let decompressed = Compression::Bzip2.decompress(&compressed)?;
        assert_eq!(decompressed, data);

        Ok(())
    }

    #[test]
    fn test_all_compressions() {
        let compressions = Compression::all();
        assert_eq!(compressions.len(), 3);
        assert!(compressions.contains(&Compression::None));
        assert!(compressions.contains(&Compression::Gzip));
        assert!(compressions.contains(&Compression::Bzip2));
    }
}
