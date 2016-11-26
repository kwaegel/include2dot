
extern crate walkdir;
use walkdir::DirEntry;

use std::env;
use std::error::Error;
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
    else {
        println!("Unable to locate {:?}", full_path);
    }

    // Then search system include paths
    for search_prefix in system_search_paths {
        let full_path = search_prefix.join(&relative_path);
        if full_path.exists() {
            return Some(full_path);
        }
        else {
            println!("Unable to locate {:?}", full_path);
        }
    }
    return None;
}

// ----------------------------------------------------------------------------

#[test]
fn test_relative_path() {
    let project_dir = env::current_dir().unwrap();
    println!("{:?}", project_dir);

    let example_dir = project_dir.join("example_tree");
    println!("{:?}", example_dir);

    // Check if file exists
    let file_a = example_dir.join("inc_1.h");
    println!("{:?}", file_a);
    assert!(file_a.exists());

    let file_b = example_dir.join("subdir").join("..").join("inc_1.h");
    println!("{:?}", file_b);
    assert!(file_b.exists());

    // This fails for some reason. Unable to parse the relative path after calling canonicalize()?
    let file_b = example_dir.canonicalize().unwrap().join("subdir").join("..").join("inc_1.h");
    println!("{:?}", file_b);
    assert!(file_b.exists());

//    let file_a_canonical = file_a.canonicalize().unwrap();
//    println!("{:?}", file_a_canonical);
//    assert!(file_a_canonical.exists());
//
//    // Check if relative file exists
//    let relative_file_a = example_dir.join("subdir")
//        .join("..")
//        .join("inc_1.h");
//    println!("{:?}", relative_file_a);
//    assert!(relative_file_a.exists());
//
//    let relative_file_a_canonical = file_a.canonicalize().unwrap();
//    println!("{:?}", relative_file_a_canonical);
//    assert!(relative_file_a_canonical.exists());
//
//    // Check fixed relative path path
//    let file_b_relative = PathBuf::from("C:\\Users\\Ky\\dev\\rust_include_graph\\example_tree\\subdir\\..\\inc_1.h");
//    println!("{:?}", file_b_relative);
//    assert!(file_b_relative.exists());
//
//    // Check UNC path
//    let file_b = PathBuf::from("\\\\?\\C:\\Users\\Ky\\dev\\rust_include_graph\\example_tree\\subdir\\..\\inc_1.h");
//    println!("{:?}", file_b);
//    assert!(file_b.exists());
}