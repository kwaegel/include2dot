
use std::fmt;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub struct FileNode {
    pub path: PathBuf, // Can use is_absolute() and is_relative() to check status.
    pub is_system: bool,
}

impl FileNode {
    pub fn new(name: &str, is_sys: bool) -> FileNode {
        FileNode {
            path: PathBuf::from(name),
            is_system: is_sys,
        }
    }

    pub fn from_path(path: &Path, is_sys: bool) -> FileNode {
        FileNode {
            path: PathBuf::from(path),
            is_system: is_sys,
        }
    }
}

impl fmt::Display for FileNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{:?}",
               self.path
                   .as_path()
                   .file_name()
                   .unwrap_or(OsStr::new("[error getting filename]")))
    }
}
