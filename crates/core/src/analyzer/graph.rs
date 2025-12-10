use std::collections::{HashSet, VecDeque};

use crate::{ast, parser::parser::ParseError};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModulePath {
    /// A file path to a file in the project.
    ///
    /// This is expected to be an absolute, canonical path.
    Real(std::path::PathBuf),
    /// The name of another module, usually from the standard library or a
    /// project dependency
    Virtual(String),
}
impl From<std::path::PathBuf> for ModulePath {
    fn from(value: std::path::PathBuf) -> Self {
        Self::Real(value)
    }
}
impl From<String> for ModulePath {
    fn from(value: String) -> Self {
        Self::Virtual(value)
    }
}
impl From<&str> for ModulePath {
    fn from(value: &str) -> Self {
        Self::Virtual(value.to_string())
    }
}
impl std::fmt::Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModulePath::Real(p) => write!(f, "{}", p.display()),
            ModulePath::Virtual(c) => write!(f, "{}", c),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedModule {
    pub name: ModulePath,
    pub ast: ast::Program,
    pub errors: Vec<ParseError>,
}
pub type ModuleId = usize;

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
    nodes: Vec<ParsedModule>,
    pub(crate) edges: HashSet<GraphEdge>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: HashSet::new(),
        }
    }

    pub fn add_module(&mut self, module: ParsedModule) -> ModuleId {
        self.nodes.push(module);
        self.nodes.len() - 1
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

    pub fn errors<'a>(&'a self) -> impl Iterator<Item = &'a ParseError> {
        self.nodes.iter().flat_map(|m| &m.errors)
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

    pub fn into_ordered_nodes(self, order: &[ModuleId]) -> Vec<ParsedModule> {
        let mut nodes = self.nodes.into_iter().map(Some).collect::<Vec<_>>();

        order
            .iter()
            .map(|&id| nodes[id as usize].take().expect("duplicate or invalid id"))
            .collect()
    }
}
