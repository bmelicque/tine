use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
};

use swc_common::FileName;

use crate::{ast, parser::parser::ParseError, type_checker};

#[derive(Debug)]
pub struct Module {
    pub name: Rc<FileName>,
    pub ast: ast::Program,
    pub context: Option<type_checker::ModuleMetadata>,
    pub errors: Vec<ParseError>,
}

pub type Edge = (Rc<FileName>, Rc<FileName>);

#[derive(Debug)]
pub struct ModuleGraph {
    nodes: HashMap<Rc<FileName>, Rc<RefCell<Module>>>,
    edges: HashSet<Edge>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashSet::new(),
        }
    }

    pub fn add_module(&mut self, module: Module) {
        self.nodes
            .insert(module.name.clone(), Rc::new(RefCell::new(module)));
    }

    pub fn add_edge(&mut self, parent: &Module, child: &Module) {
        self.edges.insert((parent.name.clone(), child.name.clone()));
    }

    pub fn get_module(&self, name: &FileName) -> Option<Rc<RefCell<Module>>> {
        self.nodes.get(name).map(|m| m.clone())
    }

    pub fn use_errors<F>(&self, predicate: F)
    where
        F: Fn(&ParseError),
    {
        for module in self.nodes.values() {
            for error in &module.borrow().errors {
                predicate(error);
            }
        }
    }

    /// Try a topological sort of the nodes contained in the graph.
    ///
    /// On failure, return a HashSet of all edges contained in at least one cycle.
    pub fn try_sorted_vec(&self) -> Result<Vec<Rc<RefCell<Module>>>, HashSet<Edge>> {
        let mut sorted = Vec::<Rc<RefCell<Module>>>::with_capacity(self.nodes.len());

        let mut queue = VecDeque::<Rc<FileName>>::new();
        for (name, module) in &self.nodes {
            if self.edges.iter().find(|e| *e.1 == **name).is_none() {
                queue.push_back(module.borrow().name.clone());
            }
        }

        let mut edges = self.edges.clone();
        while let Some(node) = queue.pop_front() {
            sorted.push(self.nodes.get(&node).unwrap().clone());

            let edges_to_remove: Vec<_> = edges
                .iter()
                .filter(|edge| edge.0 == node)
                .cloned()
                .collect();
            for edge in edges_to_remove {
                edges.remove(&edge);
                if edges.iter().find(|e| *e.1 == *edge.1).is_none() {
                    queue.push_back(edge.1);
                }
            }
        }

        if edges.len() > 0 {
            Err(edges)
        } else {
            Ok(sorted)
        }
    }
}
