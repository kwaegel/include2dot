
extern crate clap;
use clap::{Arg, App};

use std::io::{self, Read};
use std::fs::File;
use std::path::Path;
use std::ffi::OsStr;

use std::collections::HashSet;
use std::collections::HashMap;

extern crate walkdir;
use walkdir::{DirEntry, WalkDir, WalkDirIterator};

#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}


// Return a list of #include statements found in the file
fn scan_for_includes(file: &Path) -> Result<Vec<String>, io::Error> {
    let mut f = try!(File::open(file));
    let mut text = String::new();
    try!(f.read_to_string(&mut text));

    let mut includes = Vec::new();

    // Use a regex to search for '#include ...' lines.
    // The (...) capture group isolates just the text, not the "" or <> symbols.
    lazy_static! {
        static ref RE: Regex = Regex::new(r##"#include [<|"](.*)[>|"]"##).unwrap();
    }

    for cap in RE.captures_iter(&text) {
        match cap.at(1) {
            Some(include_str) => includes.push(include_str.to_owned()),
            None => {}
        }
    }

    Ok(includes)
}


fn main() {
    // TODO: accept extra include paths.
    let args = App::new("IncludeGraph-rs")
        .version("0.1.0")
        .about("Generates an include graph from a tree of C++ files")
//        .arg(Arg::with_name("output_format")
//            .help("The output format to produce: PDF, DOT, PNG")
//            .value_name("OUTPUT")
//            .short("o"))
        .arg(Arg::with_name("source")
            .help("The source directory to scan")
            .value_name("PATH")
            .required(true)
            .multiple(false)
            .index(1))
        .get_matches();


    let root_dir_string = match args.value_of("source") {
        Some(path) => path,
        None => panic!("Unable to parse directory argument."),
    };

    println!("Scanning path: {}", root_dir_string);
    let root_dir = Path::new(root_dir_string);

    if !root_dir.exists() {
        println!("Unable to access directory: {}", root_dir.display());
    }

    // Restrict the file extensions to search.
    let mut extensions = HashSet::new();
    extensions.insert(OsStr::new("h"));
    extensions.insert(OsStr::new("cpp"));
    extensions.insert(OsStr::new("hpp"));

    let mut files: HashSet<String> = HashSet::new();


    let walker = WalkDir::new(root_dir).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = entry.unwrap(); // What error values could there be?
        let path = entry.path();

        // If the file extension matches our set, queue full path for processing.
        match path.extension() {
            Some(ext) => {
                if extensions.contains(ext) {
                    files.insert(path.to_str().unwrap().to_owned());
                }
            }
            None => {}
        }
    }

    let mut include_table = HashMap::new();

    for path in files {
        let includes_result = scan_for_includes(Path::new(&path));
        // println!("{:?}", includes_result);
        match includes_result {
            Ok(includes) => {
                include_table.insert(path.clone(), includes);
            }
            Err(err) => {
                println!("Unable to process file {}: {}", path, err);
            }
        };
    }

    for (file, includes) in include_table {
        println!("{}", file);
        for inc in includes {
            println!("  {}", inc);
        }
    }
}
