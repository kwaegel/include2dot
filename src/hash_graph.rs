
use std::fmt::Display;
use std::hash::Hash;
use std::collections::{HashMap, VecDeque};

extern crate petgraph;
use petgraph::Graph;
use petgraph::prelude::NodeIndex;

#[derive(Debug)]
pub struct HashGraph<T: Eq+PartialEq+Hash+Clone> {
    pub graph: Graph<T, bool>,
    indices: HashMap<T, NodeIndex>,
}

impl<T: Eq+PartialEq+Hash+Clone+Display> HashGraph<T> {

    pub fn new() -> HashGraph<T> {
        HashGraph::<T>{graph: Graph::<T, bool>::new(),
            indices: HashMap::<T, NodeIndex>::new()}
    }

    // Return a sub-graph of all files included by the target node.
    pub fn filter_included_by(&self, root_node: NodeIndex) -> HashGraph<T> {

        let mut subgraph = HashGraph::new();

        // Run a breadth-first traversal on the graph edges, starting at [root_node].
        let mut queue = VecDeque::new();
        queue.push_back(root_node);

        while let Some(node_idx) = queue.pop_front() {
            for neighbor_idx in self.graph.neighbors_directed(node_idx, petgraph::Outgoing) {
                if !subgraph.contains_node(&self.graph[neighbor_idx]) {
                    subgraph.add_edge(self.graph[node_idx].clone(),
                                      self.graph[neighbor_idx].clone());
                    queue.push_back(neighbor_idx);
                }
            }
        }

        subgraph
    }

    // Return a sub-graph of all files that include the target node.
    pub fn filter_that_includes(&self, root_node: NodeIndex) -> HashGraph<T> {
        // Run a breadth first search with inverted edges.

        let mut subgraph = HashGraph::new();

        // Run a breadth-first traversal on the graph edges, starting at [root_node].
        let mut queue = VecDeque::new();
        queue.push_back(root_node);

        while let Some(node_idx) = queue.pop_front() {
            for neighbor_idx in self.graph.neighbors_directed(node_idx, petgraph::Incoming) {
                if !subgraph.contains_node(&self.graph[neighbor_idx]) {
                    subgraph.add_edge(self.graph[neighbor_idx].clone(),
                                      self.graph[node_idx].clone());
                    queue.push_back(neighbor_idx);
                }
            }
        }

        subgraph
    }

    // Return a sub-graph of all files that are related to the target node,
    // both included-by and that-include. This effectively creates an hourglass
    // shape centered around the target node.
    pub fn filter_bidirectional(&self, root_node: NodeIndex) -> HashGraph<T> {
        // Run a breadth first search with inverted edges.

        let mut subgraph = HashGraph::new();

        {
            // First, add all the nodes that are recursively included by the root.
            let mut queue = VecDeque::new();
            queue.push_back(root_node);

            while let Some(node_idx) = queue.pop_front() {
                for neighbor_idx in self.graph.neighbors_directed(node_idx, petgraph::Outgoing) {
                    if !subgraph.contains_node(&self.graph[neighbor_idx]) {
                        subgraph.add_edge(self.graph[node_idx].clone(),
                                          self.graph[neighbor_idx].clone());
                        queue.push_back(neighbor_idx);
                    }
                }
            }
        }

        {
            // Second, add all the nodes that recursively include the root node.
            // Run a breadth-first traversal on the graph edges, starting at [root_node].
            let mut queue = VecDeque::new();
            queue.push_back(root_node);

            while let Some(node_idx) = queue.pop_front() {
                for neighbor_idx in self.graph.neighbors_directed(node_idx, petgraph::Incoming) {
                    if !subgraph.contains_node(&self.graph[neighbor_idx]) {
                        subgraph.add_edge(self.graph[neighbor_idx].clone(),
                                          self.graph[node_idx].clone());
                        queue.push_back(neighbor_idx);
                    }
                }
            }
        }

        subgraph
    }

    // Return a list of all node indices satisfying a predicate.
    pub fn find<F>(&self, pred: F) -> Vec<NodeIndex>
        where F: Fn(&T) -> bool {

        self.indices.iter()
            .filter(|&(node,_)| pred(node))
            .map(|(_,index)| *index)
            .collect::<Vec<_>>()
    }

    pub fn add_edge(&mut self, src_node: T, dst_node: T) {
        let src_node_idx = self.require_node(src_node);
        let dst_node_idx = self.require_node(dst_node);
        self.graph.add_edge(src_node_idx, dst_node_idx, true);
    }

    pub fn contains_node(&self, node: &T) -> bool {
        self.indices.contains_key(node)
    }

    // Insert node if it does not exist yet.
    fn require_node(&mut self, node: T) -> NodeIndex {
        if self.indices.contains_key(&node) {
            // Should never panic after the contains_key() check.
            self.indices[&node]
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