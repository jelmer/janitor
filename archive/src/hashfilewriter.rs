use md5::Context as Md5;
use sha2::{Digest, Sha1, Sha256, Sha512};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// A writer that writes a file to disk and calculates the hashes of the file.
pub struct HashedFileWriter {
    release: HashMap<String, Vec<HashMap<String, String>>>,
    base: PathBuf,
    path: PathBuf,
    tmpf: NamedTempFile,
    size: u64,
}

impl HashedFileWriter {
    /// Create a new `HashedFileWriter` that writes to the given path.
    pub fn new(
        release: HashMap<String, Vec<HashMap<String, String>>>,
        base: &str,
        path: &str,
    ) -> io::Result<Self> {
        let base_path = Path::new(base).to_path_buf();
        let full_path = base_path.join(path);
        let tmpf = NamedTempFile::new()?;
        Ok(HashedFileWriter {
            release,
            base: base_path,
            path: full_path,
            tmpf,
            size: 0,
        })
    }

    pub fn done(&mut self) -> io::Result<()> {
        self.tmpf.flush()?;
        let mut hashes: HashMap<&str, Box<dyn Digest>> = HashMap::new();
        hashes.insert("MD5Sum", Box::new(Md5::new()));
        hashes.insert("SHA1", Box::new(Sha1::new()));
        hashes.insert("SHA256", Box::new(Sha256::new()));
        hashes.insert("SHA512", Box::new(Sha512::new()));

        let mut f = File::open(self.tmpf.path())?;
        let mut buffer = [0; io::DEFAULT_BUFFER_SIZE];

        loop {
            let n = f.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            for h in hashes.values_mut() {
                h.update(&buffer[..n]);
            }
            self.size += n as u64;
        }

        let (d, n) = self
            .path
            .parent()
            .unwrap()
            .to_path_buf()
            .split_at(self.path.file_name().unwrap().to_str().unwrap().len());

        for (hn, v) in &hashes {
            let hash_path = self
                .base
                .join(&d)
                .join("by-hash")
                .join(hn)
                .join(&format!("{:x}", v.clone().finalize()));
            fs::create_dir_all(hash_path.parent().unwrap())?;
            fs::copy(self.tmpf.path(), &hash_path)?;
            let mut release_entry = HashMap::new();
            release_entry.insert(
                hn.to_lowercase().to_string(),
                format!("{:x}", v.clone().finalize()),
            );
            release_entry.insert("size".to_string(), self.size.to_string());
            release_entry.insert("name".to_string(), self.path.to_str().unwrap().to_string());
            self.release
                .entry(hn.to_string())
                .or_insert(Vec::new())
                .push(release_entry);
            assert_eq!(self.size, fs::metadata(&hash_path)?.len());
        }

        Ok(())
    }
}

impl Write for HashedFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tmpf.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tmpf.flush()
    }
}

impl Drop for HashedFileWriter {
    fn drop(&mut self) {
        let dest_path = self.base.join(&self.path);
        let _ = fs::rename(self.tmpf.path(), &dest_path);
        
        // Validate file size, but don't panic in Drop if validation fails
        if let Ok(metadata) = fs::metadata(&dest_path) {
            if self.size != metadata.len() {
                eprintln!(
                    "Warning: File size mismatch for {}: expected {}, got {}",
                    dest_path.display(),
                    self.size,
                    metadata.len()
                );
            }
        } else {
            eprintln!("Warning: Failed to validate file size for {}", dest_path.display());
        }
    }
}
