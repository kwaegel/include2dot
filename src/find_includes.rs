
use std::io::{self, Read};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::ffi::OsString;
use std::error::Error;
use std::collections::HashSet;

use walkdir::WalkDir;
use regex::Regex;
use itertools::Itertools;

use file_node::FileNode;
use hash_graph::HashGraph;
use super::*;

// ----------------------------------------------------------------------------

// Convert a relative include path (e.g. <Windows.h>) into an absolute path.
fn find_absolute_include_path(include: &FileNode,
                              parent_file: &Path,
                              system_search_paths: &[PathBuf])
                              -> FileNode {

    let local_dir = parent_file.parent().unwrap(); // strip the file name

    let normalized_path = path_utils::normalize_path_separators(&include.path);

    match path_utils::convert_to_absolute_path(&normalized_path, local_dir, system_search_paths) {
        None => {

            println!("Unable to locate {:?}", &include.path);
            println!("Included from file {:?}\n", parent_file);
            include.clone()
        }
        Some(path_buf) => FileNode::from_path(&path_buf, include.is_system),
    }
}

// -----------------------------------------------------------------------------

// Return a list of #include statements found in the file
fn scan_file_for_includes(file: &Path) -> Result<Vec<FileNode>, io::Error> {
    let mut f = File::open(file)?;
    let mut text = String::new();
    f.read_to_string(&mut text)?;

    let mut includes = Vec::new();

    // Use a regex to search for '#include ...' lines.
    // The second (...) capture group isolates just the text, not the "" or <> symbols.
    lazy_static! {
    // Notes:
    // (?m:^[[:blank:]]*) => empty space at line start, multi-line mode, non-capturing group.
        static ref RE: Regex =
        Regex::new(r##"(?m:^[[:blank:]]*)#[[:blank:]]*include[[:blank:]]*([<"])(.*)[>"]"##).unwrap();
    }

    // cap.at(1) is an angle brace or double quote, to determine user or system include.
    // cap.at(2) is the include file name.
    for cap in RE.captures_iter(&text) {
        let is_system_include = cap.get(1).map_or(false, |sym| sym.as_str() == "<");

        if let Some(include_name) = cap.get(2) {
            includes.push(FileNode::new(include_name.as_str(), is_system_include));
        }
    }

    // println!("Found {} includes in {}", includes.len(), &file.display());

    Ok(includes)
}

// -----------------------------------------------------------------------------

// Core include searching loop
pub fn find_includes_in_tree(root_dir: &Path,
                             search_paths: &[PathBuf],
                             extensions: &HashSet<OsString>,
                             parse_user_includes: bool,
                             parse_system_includes: bool,
                             exclude_regex: &Option<Regex>)
                             -> HashGraph<FileNode> {
    // Collect all the files to scan in a HashSet
    // Note: is_hidden() is currently hiding paths that start with './', so don't use it yet.
    let input_queue = WalkDir::new(root_dir).into_iter()
        //.filter_entry(|e| !path_utils::is_hidden(e))
        .filter_map(|entry| match entry {
            Err(what) => {println!("Error reading directory: {}", what.description()); None},
            Ok(val) => Some(val),
        })
        .map(|entry| PathBuf::from(entry.path()))
        .filter(|path| path.extension().map_or(false, |ext| extensions.contains(ext)))
        .filter(|path| !path_utils::filename_matches_regex(exclude_regex, path))
        .collect::<HashSet<_>>();

    // Graph of all the tracked files
    let mut hash_graph = HashGraph::<FileNode>::new();

    for path_buf in input_queue {
        let parent_file = path_buf.as_path();
        let includes_result = scan_file_for_includes(parent_file);
        match includes_result {
            Ok(includes) => {

                // Convert relative includes to absolute includes
                includes.iter()
                    .filter(|inc| {
                        (!inc.is_system && parse_user_includes) ||
                        (inc.is_system && parse_system_includes)
                    })
                    .filter(|inc| !path_utils::name_matches_regex(exclude_regex, &inc.path))
                    .map(|inc| find_absolute_include_path(inc, parent_file, search_paths))
                    .foreach(|inc| {
                        // Add an edge to the graph
                        let src_node = FileNode::from_path(parent_file, false);
                        let dst_node = FileNode::from_path(&inc.path, inc.is_system);
                        hash_graph.add_edge(src_node, dst_node);
                    });
            }
            Err(err) => {
                println!("Unable to process file {:?}: {}", parent_file, err);
            }
        }
    }

    hash_graph
}

// -----------------------------------------------------------------------------

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn parse_simple() {
        let testdata_dir = env::current_dir().unwrap().join("testdata").join("simple");

        let mut search_paths = Vec::new();
        search_paths.push(PathBuf::from(&testdata_dir));

        let mut extensions = HashSet::new();
        extensions.insert(OsString::from("h"));
        extensions.insert(OsString::from("cpp"));

        let hash_graph = find_includes_in_tree(&testdata_dir,
                                               &search_paths,
                                               &extensions,
                                               true,
                                               false,
                                               &None);

        assert_eq!(hash_graph.graph.node_count(), 4);
    }

    #[test]
    fn parse_user_includes() {
        let testdata_dir = env::current_dir().unwrap().join("testdata").join("complex");

        let mut search_paths = Vec::new();
        search_paths.push(PathBuf::from(&testdata_dir));

        let mut extensions = HashSet::new();
        extensions.insert(OsString::from("h"));
        extensions.insert(OsString::from("cpp"));

        let hash_graph = find_includes_in_tree(&testdata_dir,
                                               &search_paths,
                                               &extensions,
                                               true,
                                               false,
                                               &None);

        assert_eq!(hash_graph.graph.node_count(), 7);
    }

    #[test]
    fn parse_all_includes() {
        let testdata_dir = env::current_dir().unwrap().join("testdata").join("complex");

        let mut search_paths = Vec::new();
        search_paths.push(PathBuf::from(&testdata_dir));

        let mut extensions = HashSet::new();
        extensions.insert(OsString::from("h"));
        extensions.insert(OsString::from("cpp"));

        let hash_graph =
            find_includes_in_tree(&testdata_dir, &search_paths, &extensions, true, true, &None);

        assert_eq!(hash_graph.graph.node_count(), 12);
    }

    #[test]
    fn filter_included_by_subgraph() {
        let testdata_dir = env::current_dir().unwrap().join("testdata").join("complex");

        let mut search_paths = Vec::new();
        search_paths.push(PathBuf::from(&testdata_dir));

        let mut extensions = HashSet::new();
        extensions.insert(OsString::from("h"));
        extensions.insert(OsString::from("cpp"));

        let graph =
            find_includes_in_tree(&testdata_dir, &search_paths, &extensions, true, true, &None);

        let idx_list = graph.find(|n| n.path.file_name().unwrap() == "test_1.cpp");
        assert_eq!(idx_list.len(), 1);

        let root_idx = idx_list[0];
        assert_eq!(graph.graph[root_idx].path.file_name().unwrap(),
                   "test_1.cpp");

        let subgraph = graph.filter_included_by(root_idx);
        assert_eq!(subgraph.graph.node_count(), 5);

        // Verify the nodes are correct
        assert!(graph.find(|n| n.path.file_name().unwrap() == "test_1.cpp").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "set").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "map").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "inc_1.h").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "vector").len() == 1);
    }

    #[test]
    fn filter_that_includes_subgraph() {
        let testdata_dir = env::current_dir().unwrap().join("testdata").join("complex");

        let mut search_paths = Vec::new();
        search_paths.push(PathBuf::from(&testdata_dir));

        let mut extensions = HashSet::new();
        extensions.insert(OsString::from("h"));
        extensions.insert(OsString::from("cpp"));

        let graph =
            find_includes_in_tree(&testdata_dir, &search_paths, &extensions, true, true, &None);

        let idx_list = graph.find(|n| n.path.file_name().unwrap() == "inc_1.h");
        assert_eq!(idx_list.len(), 1);

        let root_idx = idx_list[0];
        assert_eq!(graph.graph[root_idx].path.file_name().unwrap(), "inc_1.h");

        let subgraph = graph.filter_that_includes(root_idx);
        assert_eq!(subgraph.graph.node_count(), 4);

        // Verify the nodes are correct
        assert!(graph.find(|n| n.path.file_name().unwrap() == "inc_1.h").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "test_1.cpp").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "b.cpp").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "a.cpp").len() == 1);
    }

    #[test]
    fn filter_related_to_subgraph() {
        let testdata_dir = env::current_dir().unwrap().join("testdata").join("complex");

        let mut search_paths = Vec::new();
        search_paths.push(PathBuf::from(&testdata_dir));

        let mut extensions = HashSet::new();
        extensions.insert(OsString::from("h"));
        extensions.insert(OsString::from("cpp"));

        let graph =
            find_includes_in_tree(&testdata_dir, &search_paths, &extensions, true, true, &None);

        let idx_list = graph.find(|n| n.path.file_name().unwrap() == "inc_1.h");
        assert_eq!(idx_list.len(), 1);

        let root_idx = idx_list[0];
        assert_eq!(graph.graph[root_idx].path.file_name().unwrap(), "inc_1.h");

        let subgraph = graph.filter_bidirectional(root_idx);
        assert_eq!(subgraph.graph.node_count(), 5);

        // Verify the nodes are correct
        assert!(graph.find(|n| n.path.file_name().unwrap() == "inc_1.h").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "test_1.cpp").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "b.cpp").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "a.cpp").len() == 1);
        assert!(graph.find(|n| n.path.file_name().unwrap() == "vector").len() == 1);
    }
}
