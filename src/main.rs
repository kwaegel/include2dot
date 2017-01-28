
#[macro_use]
extern crate clap;
use clap::{Arg, App};

use std::path::PathBuf;
use std::ffi::OsString;
use std::error::Error;
use std::env;
use std::collections::HashSet;

use std::process::Command;
use std::fs::File;
use std::io::Write;

extern crate petgraph;
extern crate walkdir;
extern crate itertools;


#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

mod path_utils;
mod dot_writer;
mod file_node;
mod hash_graph;

mod find_includes;
use find_includes::find_includes_in_tree;


// -----------------------------------------------------------------------------

// TODO: implement the command line arguments from the original 'cinclude2dot' project.
// Output from "cinclude2dot.pl --help":
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
//        .arg(Arg::with_name("debug")
//            .long("debug")
//            .help("Display extra debug info")
//            .takes_value(true))
        .arg(Arg::with_name("exclude")
            .long("exclude")
            .help("Specify a regular expression of filenames to ignore. \nRust/RE2 \
                   syntax.\n\tExample: --exclude=\"test_|noisyFile\"")
            .takes_value(true))
//        .arg(Arg::with_name("merge")
//            .long("merge")
//            .help("Granularity of the diagram: \nfile - the default, treats each file as \
//                   separate \nmodule - merges .c/.cc/.cpp/.cxx and .h/.hpp/.hxx pairs \
//                   \ndirectory - merges directories into one node\n")
//            .possible_values(&MergeType::variants())
//            .default_value("file")
//            .takes_value(true))
//        .arg(Arg::with_name("groups")
//            .long("groups")
//            .help("Cluster files or modules into directory groups")
//            .takes_value(true))
        .arg(Arg::with_name("include")
            .long("include")
            .help("Space separated list of include search paths. (e.g. ./*/include)")
            .multiple(true)
            .takes_value(true))
//        .arg(Arg::with_name("paths")
//            .long("paths")
//            .help("Leaves relative paths in displayed filenames.")
//            .takes_value(true))
        .arg(Arg::with_name("quotetypes")
            .long("quotetypes")
            .help("Select which type of includes to parse:\nboth - parse all \
                   includes. \nangle - parse only \"system\" includes (<>) \nquote - parse only \
                   \"user\" includes (\"\")\n")
            .possible_values(&QuoteTypes::variants())
            .default_value("quote")
            .multiple(false)
            .takes_value(true))
        .arg(Arg::with_name("src")
            .long("src")
            .help("Path to the source code, defaults to current directory.")
            .multiple(false)
            .takes_value(true))
        .get_matches();

    let root_dir = match args.value_of("src") {
        Some(path) => PathBuf::from(path),
        None => env::current_dir().unwrap()
    };

    if !root_dir.exists() {
        panic!("Unable to access directory: {}", root_dir.display());
    }
    //println!("Scanning directory: {}", root_dir.display());

    // Collect a list of include paths to search.
    let mut search_paths = Vec::new();
    if let Some(values) = args.values_of("include") {
        for string in values {
            println!("Using include: {}", &string);
            search_paths.push(PathBuf::from(string));
        }
    }

    // Add a list of default system include paths.
    search_paths.push(PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\include"));

    // Collect the type of includes to scan (<> vs "")
    let quote_types = args.value_of("quotetypes").unwrap_or("both");
    let (parse_user_includes, parse_system_includes) = match quote_types {
        "angle" => (false, true),
        "quote" => (true, false),
        _ => (true, true), // both
    };

    // Restrict the file extensions to search.
    let mut extensions = HashSet::new();
    extensions.insert(OsString::from("c"));
    extensions.insert(OsString::from("cc"));
    extensions.insert(OsString::from("cpp"));
    extensions.insert(OsString::from("cxx"));
    extensions.insert(OsString::from("h"));
    extensions.insert(OsString::from("hpp"));
    extensions.insert(OsString::from("hxx"));


    // Regular expression of files to exclude. Skip if exclude string is empty.
    let exclude_regex = args.value_of("exclude")
        .and_then(|regex_str| {
            Regex::new(regex_str)
                .map_err(|err| panic!("Unable to parse exclude regex: {}", err.description()))
                .ok() // Converts successful result to Some(), discarding errors.
        });

    let hash_graph = find_includes_in_tree(&root_dir,
                                           &search_paths,
                                           &extensions,
                                           parse_user_includes,
                                           parse_system_includes,
                                           &exclude_regex);
    
    // Write the graph to a dot file.
    let _ = dot_writer::write_dot_with_header("./graph.dot", &hash_graph.graph);

    // Print summary stats
    println!("Generated graph with {} nodes and {} edges.",
             &hash_graph.graph.node_count(), &hash_graph.graph.edge_count());

    // Create a sub-process to generate a PDF of the graph
    let graphviz_output_result = Command::new("dot")
        .arg("-Tpdf")
        .arg("graph.dot")
        .output();

    if let Ok(graphviz_output) = graphviz_output_result {
        if graphviz_output.status.success() {
            let mut pdf_file = File::create("graph.pdf").expect("Failed to create output PDF");
            pdf_file.write_all(&graphviz_output.stdout).expect("Failed to write to graph.pdf");
        }
    } else {
        println!("Unable to find graphviz. Is it installed?");
        println!("Run \"dot -Tpdf graph.dot > graph.pdf\" to render the graph.");
    }


}
