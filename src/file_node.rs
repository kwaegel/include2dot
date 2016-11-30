
use std::fmt;
use std::path::PathBuf;
use std::ffi::OsStr;

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub struct FileNode {
    pub path: PathBuf, // Can use is_absolute() and is_relative() to check status.
    pub is_system: bool,
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
