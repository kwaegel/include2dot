
extern crate walkdir;
use walkdir::DirEntry;

use std::path::{Path, PathBuf};

// ----------------------------------------------------------------------------

pub fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

// ----------------------------------------------------------------------------

// Normalize a string to use system-specific path separators.
// E.g. '/' to '\' on Windows.
pub fn normalize_path_separators(path: &Path) -> PathBuf {

    match path.to_str() {
        Some(string) => {
            let mut composite_path = PathBuf::new();
            for seg in string.split("/") {
                composite_path.push(seg);
            }
            composite_path
        }
        None => path.to_path_buf()
    }
}

// ----------------------------------------------------------------------------

// Convert a relative include path (e.g. <Windows.h>) into an absolute path.
pub fn convert_to_absolute_path(relative_path: &Path,
                                local_search_path: &Path,
                                system_search_paths: &[PathBuf])
                                -> Option<PathBuf> {

    // Search relative to the local directory first.
    let full_path = local_search_path.join(&relative_path);
    if full_path.exists() {
        return Some(full_path);
    }

    // Then search system include paths
    for search_prefix in system_search_paths {
        let full_path = search_prefix.join(&relative_path);
        if full_path.exists() {
            return Some(full_path);
        }
    }

    // Note: path.join() currenly breaks because the relative pathing in C++ uses Unix seperators.
    // Need to normalize seperators.
    //println!("Failed to get absolute path for {:?}", relative_path);
    //println!("Local joined path: {:?}", full_path);
    //include.as_failed_lookup()
    return None;
}

// ----------------------------------------------------------------------------