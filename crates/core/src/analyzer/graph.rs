use std::collections::{HashSet, VecDeque};

use crate::analyzer::modules::{Module, ModuleId, ModulePath};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphEdge {
    pub dependency: ModuleId,
    pub dependent: ModuleId,
}

#[derive(Debug)]
pub struct ModuleSort {
    pub sorted: Vec<ModuleId>,
    pub unsorted: HashSet<GraphEdge>,
}

#[derive(Debug)]
pub struct ModuleGraph {
    pub(super) nodes: Vec<Module>,
    pub(crate) edges: HashSet<GraphEdge>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: HashSet::new(),
        }
    }

    pub fn next_id(&self) -> ModuleId {
        self.nodes.len()
    }

    pub fn add_module(&mut self, module: Module) -> ModuleId {
        let id = self.nodes.len();
        self.nodes.push(module);
        id
    }

    pub fn add_edge(&mut self, dependency: ModuleId, dependent: ModuleId) {
        self.edges.insert(GraphEdge {
            dependency,
            dependent,
        });
    }

    pub fn find_id(&self, name: &ModulePath) -> Option<ModuleId> {
        self.nodes.iter().position(|m| m.name == *name)
    }

    /// Try a topological sort of the nodes contained in the graph.
    ///
    /// On failure, return a HashSet of all edges contained in at least one cycle.
    pub fn try_sorted_vec(&self) -> ModuleSort {
        let mut sorted = Vec::<ModuleId>::with_capacity(self.nodes.len());

        let mut queue = VecDeque::<ModuleId>::new();
        // List all nodes without dependencies
        for id in 0..self.nodes.len() {
            if self.edges.iter().find(|e| e.dependency == id).is_none() {
                queue.push_back(id);
            }
        }

        let mut edges = self.edges.clone();
        while let Some(id) = queue.pop_front() {
            sorted.push(id);

            // Remove every edge that has the current node as a dependency
            let edges_to_remove: Vec<_> = edges
                .iter()
                .filter(|edge| edge.dependency == id)
                .cloned()
                .collect();

            // For every removed edge, check if the dependant is now without
            // dependency; add it to list if it is.
            for removed in edges_to_remove {
                edges.remove(&removed);
                let dependent_met_prerequisites = edges
                    .iter()
                    .find(|e| e.dependency == removed.dependent)
                    .is_none();
                if dependent_met_prerequisites {
                    queue.push_back(removed.dependent);
                }
            }
        }

        ModuleSort {
            sorted,
            unsorted: edges,
        }
    }
}
