
#[macro_use]
extern crate clap;
use clap::{Arg, App};

use std::io::{self, Read};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::error::Error;

use std::collections::HashSet;
use std::collections::HashMap;

extern crate petgraph;
use petgraph::Graph;
use petgraph::prelude::NodeIndex;

extern crate walkdir;
use walkdir::WalkDir;

#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

mod path_utils;

mod dot_writer;

mod file_node;
use file_node::FileNode;

// -----------------------------------------------------------------------------

#[derive(Debug)]
enum IncludeStatus {
    Relative, // e.g. <vector>
    Absolute, // e.g. /usr/include/c++/4.8/vector
    FailedLookup, // Failed to resolve an absolute file path.
}

#[derive(Debug)]
struct Include {
    path: PathBuf,
    is_system_include: bool,
    status: IncludeStatus,
}

impl Include {
    fn new_relative(name: &str, is_sys: bool) -> Include {
        Include {
            path: PathBuf::from(name),
            is_system_include: is_sys,
            status: IncludeStatus::Relative,
        }
    }

    fn as_absolute(&self, absolute_path: &Path) -> Include {
        Include {
            path: PathBuf::from(absolute_path),
            is_system_include: self.is_system_include,
            status: IncludeStatus::Absolute,
        }
    }

    fn as_failed_lookup(&self) -> Include {
        Include {
            path: PathBuf::from(&self.path),
            is_system_include: self.is_system_include,
            status: IncludeStatus::FailedLookup,
        }
    }
}

// ----------------------------------------------------------------------------

// Convert a relative include path (e.g. <Windows.h>) into an absolute path.
fn find_absolute_include_path(include: &Include,
                              parent_file: &Path,
                              system_search_paths: &[PathBuf])
                              -> Include {

    let local_dir = parent_file.parent().unwrap(); // strip the file name

    let normalized_path = path_utils::normalize_path_separators(&include.path);

    match path_utils::convert_to_absolute_path(&normalized_path, local_dir, system_search_paths) {
        None => {
            println!("In file {:?}", parent_file);
            println!("Unable to locate include {:?}", &include.path);
            println!("");
            include.as_failed_lookup()
        }
        Some(path_buf) => include.as_absolute(path_buf.as_path()),
    }
}

// -----------------------------------------------------------------------------

// Return a list of #include statements found in the file
fn scan_file_for_includes(file: &Path) -> Result<Vec<Include>, io::Error> {
    let mut f = File::open(file)?;
    let mut text = String::new();
    f.read_to_string(&mut text)?;

    let mut includes = Vec::new();

    // Use a regex to search for '#include ...' lines.
    // The second (...) capture group isolates just the text, not the "" or <> symbols.
    lazy_static! {
        static ref RE: Regex = Regex::new(r##"#include ([<|"])(.*)[>|"]"##).unwrap();
    }

    // cap.at(1) is an angle brace or double quote, to determine user or system include.
    // cap.at(2) is the include file name.
    for cap in RE.captures_iter(&text) {
        let inc_symbol = cap.at(1).unwrap_or("<");
        let is_system_include = inc_symbol == "<";
        match cap.at(2) {
            Some(include_name) => {
                includes.push(Include::new_relative(include_name, is_system_include))
            }
            None => {}
        }
    }

    // println!("Found {} includes in {}", includes.len(), &file.display());

    Ok(includes)
}

// -----------------------------------------------------------------------------

// TODO: implement the command line arguments from the original 'cinclude2dot' project.
// --debug       Display various debug info
// --exclude     Specify a regular expression of filenames to ignore
//              For example, ignore your test harnesses.
// --merge       Granularity of the diagram:
//              file - the default, treats each file as separate
//              module - merges .c/.cc/.cpp/.cxx and .h/.hpp/.hxx pairs
//              directory - merges directories into one node
// --groups      Cluster files or modules into directory groups
// --help        Display this help page.
// --include     Followed by a comma separated list of include search paths.
// --paths       Leaves relative paths in displayed filenames.
// --quotetypes  Select for parsing the files included by strip quotes or angle brackets:
//              both - the default, parse all headers.
//              angle - include only "system" headers included by anglebrackets (<>)
//              quote - include only "user" headers included by strip quotes ("")
// --src         Followed by a path to the source code, defaults to current directory

arg_enum! {
    #[derive(Debug)]
    #[allow(non_camel_case_types)]
    enum MergeType {
        file,
        module,
        directory
    }
}

arg_enum! {
    #[derive(Debug)]
    #[allow(non_camel_case_types)]
    enum QuoteTypes {
        both,
        angle,
        quote
    }
}

fn main() {
    // TODO: accept extra include paths.
    let args = App::new("IncludeGraph-rs")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Generates an include graph from a tree of C++ files")
        .arg(Arg::with_name("debug")
            .long("debug")
            .help("Display extra debug info")
            .takes_value(true))
        .arg(Arg::with_name("exclude")
            .long("exclude")
            .help("Specify a regular expression of filenames to ignore. \nRust/RE2 \
                   syntax.\n\tExample: --exclude=\"test_|noisyFile\"")
            .takes_value(true))
        .arg(Arg::with_name("merge")
            .long("merge")
            .help("Granularity of the diagram: \nfile - the default, treats each file as \
                   separate \nmodule - merges .c/.cc/.cpp/.cxx and .h/.hpp/.hxx pairs \
                   \ndirectory - merges directories into one node\n")
            .possible_values(&MergeType::variants())
            .default_value("file")
            .takes_value(true))
        .arg(Arg::with_name("groups")
            .long("groups")
            .help("Cluster files or modules into directory groups")
            .takes_value(true))
        .arg(Arg::with_name("include")
            .long("include")
            .help("Comma separated list of include search paths.")
            .takes_value(true))
        .arg(Arg::with_name("paths")
            .long("paths")
            .help("Leaves relative paths in displayed filenames.")
            .takes_value(true))
        .arg(Arg::with_name("quotetypes")
            .long("quotetypes")
            .help("Select which type of includes to parse:\nboth - the default, parse all \
                   includes. \nangle - parse only \"system\" includes (<>) \nquote - parse only \
                   \"user\" includes (\"\")\n")
            .possible_values(&QuoteTypes::variants())
            .default_value("both")
            .takes_value(true))
        .arg(Arg::with_name("src")
            .help("Path to the source code, defaults to current directory.")
            .required(true)
            .multiple(false)
            .index(1))
        .get_matches();

    let expand_system_includes = false;

    let root_dir_string = match args.value_of("src") {
        Some(path) => path,
        None => panic!("Unable to parse source directory argument."),
    };

    println!("Scanning path: {}", root_dir_string);
    let root_dir = Path::new(root_dir_string);

    if !root_dir.exists() {
        println!("Unable to access directory: {}", root_dir.display());
    }

    // Create a list of default system include paths.
    let mut search_paths = Vec::new();
    if expand_system_includes {
        search_paths.push(PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\include"));
    }

    // Restrict the file extensions to search.
    let mut extensions = HashSet::new();
    extensions.insert(OsStr::new("c"));
    extensions.insert(OsStr::new("cc"));
    extensions.insert(OsStr::new("cpp"));
    extensions.insert(OsStr::new("cxx"));
    extensions.insert(OsStr::new("h"));
    extensions.insert(OsStr::new("hpp"));
    extensions.insert(OsStr::new("hxx"));



    // Regular expression of files to exclude. Skip if exclude string is empty.
    let exclude_regex = args.value_of("exclude")
        .and_then(|ref regex_str| {
            Regex::new(regex_str)
                .map_err(|err| panic!("Unable to parse exclude regex: {}", err.description()))
                .ok() // Converts successful result to Some(), discarding errors.
        });


    // Collect all the files to scan in a HashSet
    // Note: is_hidden() is currently hiding paths that start with './', so don't use it yet.
    let input_queue = WalkDir::new(root_dir).into_iter()
        //.filter_entry(|e| !path_utils::is_hidden(e))
        .filter_map(|entry| entry.ok())                     // This discards errors.
        .map(|entry| PathBuf::from(entry.path()))
        .filter(|ref path| path.extension().map_or(false, |ext| extensions.contains(ext)))
        .filter(|ref path| !path_utils::filename_matches_regex(&exclude_regex, &path))
        .collect::<HashSet<_>>();

    // Graph of all the tracked files
    let mut graph = Graph::<FileNode, bool>::new();
    let mut indices = HashMap::<FileNode, NodeIndex>::new();

    for path_buf in input_queue {
        let parent_file = path_buf.as_path();
        let includes_result = scan_file_for_includes(parent_file);
        match includes_result {
            Ok(includes) => {

                // Convert relative includes to absolute includes
                let user_includes: Vec<_> = includes.iter()
                    .filter(|inc| !inc.is_system_include || expand_system_includes)
                    .filter(|inc| !path_utils::name_matches_regex(&exclude_regex, &inc.path))
                    .map(|inc| find_absolute_include_path(inc, parent_file, &search_paths))
                    .collect();

                for inc in user_includes {
                    // Get an existing NodeIndex from the graph, on create a new node.
                    let src_node = FileNode {
                        path: PathBuf::from(&parent_file),
                        is_system: false,
                    };
                    let dst_node = FileNode {
                        path: PathBuf::from(&inc.path),
                        is_system: false,
                    };

                    // These functions should work, but currently create duplicate nodes.
                    let src_node_idx = indices.entry(src_node.clone())
                        .or_insert_with(|| graph.add_node(src_node))
                        .clone();
                    let dst_node_idx = indices.entry(dst_node.clone())
                        .or_insert_with(|| graph.add_node(dst_node))
                        .clone();

                    // println!("Adding edge {:?} -> {:?}", src_node_idx, dst_node_idx);
                    graph.add_edge(src_node_idx, dst_node_idx, true);
                }

                if !expand_system_includes {
                    let system_includes: Vec<_> = includes.iter()
                        .filter(|inc| inc.is_system_include && !expand_system_includes)
                        .filter(|inc| !path_utils::name_matches_regex(&exclude_regex, &inc.path))
                        .collect();

                    for inc in system_includes {

                        // Get an existing NodeIndex from the graph, on create a new node.
                        let src_node = FileNode {
                            path: PathBuf::from(&parent_file),
                            is_system: false,
                        };
                        let dst_node = FileNode {
                            path: PathBuf::from(&inc.path),
                            is_system: true,
                        };

                        // These functions should work, but currently create duplicate nodes.
                        let src_node_idx = indices.entry(src_node.clone())
                            .or_insert_with(|| graph.add_node(src_node))
                            .clone();
                        let dst_node_idx = indices.entry(dst_node.clone())
                            .or_insert_with(|| graph.add_node(dst_node))
                            .clone();

                        // println!("Adding edge {:?} -> {:?}", src_node_idx, dst_node_idx);
                        graph.add_edge(src_node_idx, dst_node_idx, true);
                    }

                }
            }
            Err(err) => {
                println!("Unable to process file {:?}: {}", parent_file, err);
            }
        };
    }

    // Write the graph to a dot file.
    let _ = dot_writer::write_dot_with_header("./graph.dot", &graph);

    println!("Now run \"dot -Tpdf graph.dot > graph.pdf\" to render the graph.");
}
