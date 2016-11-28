
extern crate petgraph;
use petgraph::Graph;
use petgraph::visit::NodeIndexable;
use petgraph::visit::EdgeRef;

use std::io::{self, Write};
use std::path::Path;
use std::fs::File;

use file_node::FileNode;

// The simple dot writer in petgraph is not sufficient, so implement one here.
pub fn write_dot_with_header(filename: &str, graph: &Graph<FileNode, bool>) -> Result<(), io::Error> {

    let out_path = Path::new(filename);
    let mut dotfile = File::create(&out_path)?;

    // Define a directed graph graph type
    writeln!(&mut dotfile, "digraph {{")?;

    // Write layout header. This is the part that petgraph can't do yet.
    writeln!(&mut dotfile, "    overlap=scale;")?;
    writeln!(&mut dotfile, "    size=\"80,100\";")?;
    writeln!(&mut dotfile, "    ratio=\"compress\";")?;
    //writeln!(&mut dotfile, "    ratio=\"fill\";")?;
    writeln!(&mut dotfile, "    fontsize=\"16\";")?;
    writeln!(&mut dotfile, "    fontname=\"Helvetica\";")?;
    writeln!(&mut dotfile, "    clusterrank=\"local\";")?;

    // Write nodes with labels
    // Format:
    //     6 [label="\"vector\""]
    for node_idx in graph.node_indices() {
        let integer_idx = graph.to_index(node_idx);
        let ref node_ref = graph[node_idx];
        //println!("    {} [label={}]", integer_idx, node_ref);
        writeln!(&mut dotfile, "    {} [label={}]", integer_idx, node_ref)?;
    }

    // Write edges
    // Format:
    //     1 -> 2
    for edge in graph.edge_references() {
        let src_idx = graph.to_index(edge.source());
        let dst_idx = graph.to_index(edge.target());
        //println!("    {} -> {}", src_idx, dst_idx);
        writeln!(&mut dotfile, "    {} -> {}", src_idx, dst_idx)?;
    }

    // Close graph
    writeln!(&mut dotfile, "}}")?;

    Ok(())
}