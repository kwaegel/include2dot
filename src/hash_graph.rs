
use std::hash::Hash;
use std::collections::HashMap;

extern crate petgraph;
use petgraph::Graph;
use petgraph::prelude::NodeIndex;

pub struct HashGraph<T: Eq+PartialEq+Hash+Clone> {
    pub graph: Graph<T, bool>,
    indices: HashMap<T, NodeIndex>,
}

impl<T: Eq+PartialEq+Hash+Clone> HashGraph<T> {

    pub fn new() -> HashGraph<T> {
        HashGraph::<T>{graph: Graph::<T, bool>::new(),
            indices: HashMap::<T, NodeIndex>::new()}
    }

    pub fn add_edge(&mut self, src_node: T, dst_node: T) {
        let src_node_idx = self.require_node(src_node);
        let dst_node_idx = self.require_node(dst_node);
        self.graph.add_edge(src_node_idx, dst_node_idx, true);
    }

    // Insert node if it does not exist yet.
    fn require_node(&mut self, node: T) -> NodeIndex {
        if self.indices.contains_key(&node) {
            // Should never panic after the contains_key() check.
            *self.indices.get(&node).unwrap()
        } else {
            let new_idx = self.graph.add_node(node.clone());
            self.indices.insert(node, new_idx);
            new_idx
        }

        // I would prefer this syntax, but borrow of &self prevents it.
        // May require using a Cell<> around the graph or HashMap.
//        *self.indices.entry(node.clone())
//            .or_insert_with(|| self.graph.add_node(node))
    }
}