pub mod diffoscope;

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

pub fn find_binaries(path: &Path) -> impl Iterator<Item = (OsString, PathBuf)> {
    std::fs::read_dir(path).unwrap().filter_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        Some((entry.file_name(), path))
    })
}

pub fn is_binary(name: &OsStr) -> bool {
    name.to_str().map_or(false, |name| {
        name.ends_with(".deb") || name.ends_with(".udeb")
    })
}
