
extern crate clap;
use clap::{Arg, App};

use std::ops::Index;
use std::fmt;
use std::io::{self, Read, Write};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::error::Error;

use std::collections::HashSet;
use std::collections::HashMap;

extern crate petgraph;
use petgraph::Graph;
use petgraph::visit::EdgeRef;
use petgraph::prelude::NodeIndex;
use petgraph::dot::{Dot, Config};

extern crate walkdir;
use walkdir::{WalkDir, WalkDirIterator};

#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

mod path_utils;

// -----------------------------------------------------------------------------

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
struct FileNode {
    path: PathBuf, // Can use is_absolute() and is_relative() to check status.
    is_system: bool,
}

impl fmt::Display for FileNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.path.as_path()
            .file_name()
            .unwrap_or(OsStr::new("[error getting filename]")))
    }
}

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
                              //local_search_path: &Path,
                              system_search_paths: &[PathBuf])
                              -> Include {

    let local_dir = parent_file.parent().unwrap(); // strip the file name

    let normalized_path = path_utils::normalize_path_separators(&include.path);

    match path_utils::convert_to_absolute_path(&normalized_path,
                                               local_dir,
                                               system_search_paths) {
        None => {
            println!("In file {:?}", parent_file);
            println!("Unable to locate include {:?}", &include.path);
            //println!("with normalized path {:?}", normalized_path);
            println!("");
            include.as_failed_lookup()
        },
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

    //println!("Found {} includes in {}", includes.len(), &file.display());

    Ok(includes)
}

// -----------------------------------------------------------------------------

fn main() {
    // TODO: accept extra include paths.
    // TODO: accept regex exclusion filters (e.g. stdafx.h)
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

    let expand_system_includes = false;

    let root_dir_string = match args.value_of("source") {
        Some(path) => path,
        None => panic!("Unable to parse directory argument."),
    };

    //println!("Scanning path: {}", root_dir_string);
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


    let mut input_queue: HashSet<PathBuf> = HashSet::new();

    // Collect all the files to scan
    let walker = WalkDir::new(root_dir).into_iter();
    // Note: is_hidden() is currently hiding paths that start with './', and should be fixed.
    //for entry in walker.filter_entry(|e| !path_utils::is_hidden(e)) {
    for entry in walker {
        match entry {
            Ok(entry) => {
                let path_buf = PathBuf::from(entry.path());

                // If the file extension matches our set, queue full path for processing.
                let has_matching_ext = path_buf.extension().map_or(false, |ext| extensions.contains(ext));
                if has_matching_ext {
                    //println!("Found file {}", &path_buf.display());
                    input_queue.insert(path_buf);
                }
            },
            Err(what) => {
                println!("Error reading file: {}", what.description());
            },
        }
    }



    // Graph of all the tracked files
    let mut graph = Graph::<FileNode, bool>::new();
    let mut indices = HashMap::<FileNode, NodeIndex>::new();

    for path_buf in input_queue {
        let parent_file = path_buf.as_path();
        let includes_result = scan_file_for_includes(parent_file);
        match includes_result {
            Ok(includes) => {

                //println!("Processing file {}", parent_file.display());

                // Convert relative includes to absolute includes
                let absolute_includes: Vec<_> = includes.iter()
                    .filter(|inc| !inc.is_system_include || expand_system_includes )
                    .map(|inc| find_absolute_include_path(inc, parent_file, &search_paths))
                    .collect();

                //println!("Found {} absolute includes", absolute_includes.len());

                for inc in absolute_includes {

                    // Get an existing NodeIndex from the graph, on create a new node.
                    let src_node = FileNode{path: PathBuf::from(&parent_file), is_system: false};
                    let dst_node = FileNode{path: PathBuf::from(&inc.path), is_system: false};

                    // These functions should work, but currently create duplicate nodes.
                    let src_node_idx = indices.entry(src_node.clone())
                        .or_insert_with(|| graph.add_node(src_node))
                        .clone();
                    let dst_node_idx = indices.entry(dst_node.clone())
                        .or_insert_with(|| graph.add_node(dst_node))
                        .clone();

                    //println!("Adding edge {:?} -> {:?}", src_node_idx, dst_node_idx);
                    graph.add_edge(src_node_idx, dst_node_idx, true);
                }

                if !expand_system_includes {
                    let system_includes: Vec<_> = includes.iter()
                        .filter(|inc| inc.is_system_include && !expand_system_includes)
                        .collect();

                    for inc in system_includes {

                        // Get an existing NodeIndex from the graph, on create a new node.
                        let src_node = FileNode{path: PathBuf::from(&parent_file), is_system: false};
                        let dst_node = FileNode{path: PathBuf::from(&inc.path), is_system: true};

                        // These functions should work, but currently create duplicate nodes.
                        let src_node_idx = indices.entry(src_node.clone())
                            .or_insert_with(|| graph.add_node(src_node))
                            .clone();
                        let dst_node_idx = indices.entry(dst_node.clone())
                            .or_insert_with(|| graph.add_node(dst_node))
                            .clone();

                        //println!("Adding edge {:?} -> {:?}", src_node_idx, dst_node_idx);
                        graph.add_edge(src_node_idx, dst_node_idx, true);
                    }

                }
            }
            Err(err) => {
                println!("Unable to process file {:?}: {}", parent_file, err);
            }
        };
    }

    // Print all the includes
//    for node_idx in graph.node_indices() {
//        for edge in graph.edges(node_idx) {
//            let src_str = graph.index(edge.source());
//            let dst_str = graph.index(edge.target());
//            println!("{:?} -> {:?}", src_str, dst_str);
//        }
//    }

    // Write output to file graph.dot
    let out_path = Path::new("./graph.dot");
    let out_path_display = out_path.display();

    let mut dotfile = match File::create(&out_path) {
        Ok(file) => file,
        Err(why) => panic!("couldn't create {}: {}", out_path_display, why.description()),
    };

    // TODO: print the dot file with the following header to avoid max-width issues.
    //    overlap=scale;
    //    size="80,100";
    //    ratio="fill";
    //    fontsize="16";
    //    fontname="Helvetica";
    //    clusterrank="local";
    writeln!(&mut dotfile,
             "{}",
             Dot::with_config(&graph, &[Config::EdgeNoLabel]));

    println!("Wrote graph to {}", out_path_display);
    println!("Run 'dot -Tpdf graph.dot > graph.pdf' to create a graph.");
}
