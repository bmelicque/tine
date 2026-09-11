use std::collections::{HashMap, HashSet, VecDeque};
use tine_ast::{
    self as ast,
    use_tree::{use_decl_to_paths, ModuleIdentifier},
};
use tine_common::{
    diagnostics::{Diagnostic, DiagnosticKind, DiagnosticLevel},
    module_path::ModulePath,
    sources::Source,
};

use crate::Parser;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphEdge {
    pub dependency: ModuleId,
    pub dependent: ModuleId,
}

pub trait Loader {
    fn load(&self, path: &ModulePath) -> anyhow::Result<String>;
}

pub struct ParserLoader;

impl Loader for ParserLoader {
    fn load<'a>(&'a self, path: &ModulePath) -> anyhow::Result<String> {
        match path {
            ModulePath::Real(buf) => std::fs::read_to_string(buf).map_err(|e| anyhow::anyhow!(e)),
            ModulePath::Virtual(_) => Ok("".to_string()),
        }
    }
}

pub type ModuleId = usize;

pub fn parse_project(entry_point: ModulePath, loader: Option<Box<dyn Loader>>) -> ProjectParser {
    let loader = loader.unwrap_or(Box::new(ParserLoader));
    let mut parser = ProjectParser::new(loader);
    let _ = parser.parse_project(entry_point);
    parser
}

pub struct ProjectParser {
    entry_point: ModulePath,
    loader: Box<dyn Loader>,

    pub ids: HashMap<ModulePath, ModuleId>,
    pub names: Vec<ModulePath>,

    pub sources: HashMap<ModuleId, Source>,
    pub ast: HashMap<ModuleId, ast::Program>,
    pub diagnostics: HashMap<ModuleId, Vec<Diagnostic>>,

    edges: HashSet<GraphEdge>,
}

impl ProjectParser {
    pub fn new(loader: Box<dyn Loader>) -> Self {
        Self {
            entry_point: ModulePath::default(),
            loader,

            ids: HashMap::new(),
            names: Vec::new(),

            sources: HashMap::new(),
            ast: HashMap::new(),
            diagnostics: HashMap::new(),

            edges: HashSet::new(),
        }
    }

    fn add_module(&mut self, path: ModulePath) -> ModuleId {
        let id: ModuleId = self.names.len() + 1;
        self.names.push(path.clone());
        self.ids.insert(path, id);
        id
    }

    fn add_edge(&mut self, dependency: ModuleId, dependent: ModuleId) {
        self.edges.insert(GraphEdge {
            dependency,
            dependent,
        });
    }

    pub fn get_id(&self, path: &ModulePath) -> Option<ModuleId> {
        self.ids.get(path).copied()
    }

    pub fn name(&self, id: ModuleId) -> &ModulePath {
        &self.names[id - 1]
    }

    pub fn consume_ast(&mut self, id: ModuleId) -> ast::Program {
        self.ast.remove(&id).unwrap()
    }

    /// Try a topological sort of the nodes contained in the graph.
    ///
    /// On failure, return a HashSet of all edges contained in at least one cycle.
    pub fn try_sorted_vec(&self) -> Result<Vec<ModuleId>, HashSet<GraphEdge>> {
        let mut sorted = Vec::<ModuleId>::with_capacity(self.names.len());

        let mut queue = VecDeque::<ModuleId>::new();
        // List all nodes without dependencies
        for id in 1..self.names.len() + 1 {
            if self.edges.iter().all(|e| e.dependent != id) {
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
                    .find(|e| e.dependent == removed.dependent)
                    .is_none();
                if dependent_met_prerequisites && !queue.contains(&removed.dependent) {
                    queue.push_back(removed.dependent);
                }
            }
        }

        if edges.is_empty() {
            Ok(sorted)
        } else {
            Err(edges)
        }
    }

    pub fn parse_project(&mut self, entry_point: ModulePath) -> anyhow::Result<()> {
        assert!(matches!(entry_point, ModulePath::Real(_)));
        self.entry_point = entry_point.clone();
        self.parse_module(&entry_point)?;
        Ok(())
    }

    fn parse_module(&mut self, path: &ModulePath) -> anyhow::Result<ModuleId> {
        let module_id = match path {
            ModulePath::Real(_) => self.parse_real_module(path)?,
            ModulePath::Virtual(c) => self.parse_virtual_module(c)?,
        };
        let file_names = get_dependencies(&self.entry_point, &self.ast[&module_id]);

        for dependency_name in file_names {
            self.parse_dependency(&dependency_name, module_id);
        }

        Ok(module_id)
    }

    fn parse_dependency(&mut self, dep: &ModuleIdentifier, parent_id: ModuleId) {
        if let Some(&dependency_id) = self.ids.get(&dep.path) {
            self.add_edge(dependency_id, parent_id.clone());
            return;
        }
        let Ok(dependency) = self.parse_module(&dep.path) else {
            let diags = self.diagnostics.entry(parent_id).or_default();
            let kind = DiagnosticKind::CannotFindModule {
                name: dep.path.to_string(),
            };
            diags.push(Diagnostic {
                level: DiagnosticLevel::Error,
                loc: dep.loc,
                kind,
            });
            return;
        };
        self.add_edge(dependency, parent_id);
    }

    fn parse_real_module(&mut self, path: &ModulePath) -> anyhow::Result<ModuleId> {
        let src = self.loader.load(path)?;
        let id = self.add_module(path.clone());

        let source = Source::new(&src);
        self.sources.insert(id, source);

        let result = Parser::new(id, &src).parse();
        self.ast.insert(id, result.node);
        if !result.diagnostics.is_empty() {
            self.diagnostics.insert(id, result.diagnostics);
        }

        Ok(id)
    }

    fn parse_virtual_module(&mut self, name: &String) -> anyhow::Result<ModuleId> {
        let module = match name.as_str() {
            "dom" | "signals" => ModulePath::Virtual(name.to_string()),
            name => anyhow::bail!("Cannot find module '{}'", name),
        };
        let id = self.add_module(module);
        self.ast.insert(id, ast::Program::dummy());
        Ok(id)
    }
}

/// Extract the paths of the modules used/imported within the given AST.
fn get_dependencies(root_path: &ModulePath, ast: &ast::Program) -> Vec<ModuleIdentifier> {
    let mut file_names: Vec<ModuleIdentifier> = ast
        .items
        .iter()
        .filter_map(|item| item.as_use_declaration_ref())
        .flat_map(|decl| use_decl_to_paths(root_path, decl))
        .map(|imports| imports.module_name)
        .collect();
    file_names.sort_by_key(|n| n.path.clone());
    file_names.dedup_by(|a, b| a.path == b.path);
    file_names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(ModuleId, ModuleId)]) -> ProjectParser {
        let mut graph = ProjectParser::new(Box::new(ParserLoader));

        for &(dependency, dependent) in edges {
            graph.edges.insert(GraphEdge {
                dependency,
                dependent,
            });
        }

        graph
    }

    #[test]
    fn empty_graph() {
        let graph = graph(&[]);

        assert_eq!(graph.try_sorted_vec(), Ok(vec![]));
    }

    #[test]
    fn single_node() {
        let mut graph = graph(&[]);
        graph.names.push("A".into());

        assert_eq!(graph.try_sorted_vec(), Ok(vec![1]));
    }

    #[test]
    fn independent_nodes() {
        let mut graph = graph(&[]);
        graph.names.extend(["A".into(), "B".into(), "C".into()]);

        assert_eq!(graph.try_sorted_vec(), Ok(vec![1, 2, 3]));
    }

    #[test]
    fn simple_chain() {
        // 1 -> 2 -> 3
        let mut graph = graph(&[(1, 2), (2, 3)]);
        graph.names.extend(["A".into(), "B".into(), "C".into()]);

        assert_eq!(graph.try_sorted_vec(), Ok(vec![1, 2, 3]));
    }

    #[test]
    fn branching() {
        //     ┌-> 2
        // 1 --┤
        //     └-> 3
        let mut graph = graph(&[(1, 2), (1, 3)]);
        graph.names.extend(["A".into(), "B".into(), "C".into()]);

        assert!(matches!(graph.try_sorted_vec(), Ok(_)));
    }

    #[test]
    fn convergence() {
        // 1 -> 3
        // 2 -> 3
        let mut graph = graph(&[(1, 3), (2, 3)]);
        graph.names.extend(["A".into(), "B".into(), "C".into()]);

        assert_eq!(graph.try_sorted_vec(), Ok(vec![1, 2, 3]));
    }

    #[test]
    fn simple_cycle() {
        // 1 -> 2 -> 1
        let mut graph = graph(&[(1, 2), (2, 1)]);
        graph.names.extend(["A".into(), "B".into()]);

        let result = graph.try_sorted_vec();

        assert_eq!(
            result,
            Err(HashSet::from([
                GraphEdge {
                    dependency: 1,
                    dependent: 2,
                },
                GraphEdge {
                    dependency: 2,
                    dependent: 1,
                },
            ]))
        );
    }

    #[test]
    fn three_node_cycle() {
        // 1 -> 2 -> 3 -> 1
        let mut graph = graph(&[(1, 2), (2, 3), (3, 1)]);
        graph.names.extend(["A".into(), "B".into(), "C".into()]);

        let result = graph.try_sorted_vec();

        assert_eq!(
            result,
            Err(HashSet::from([
                GraphEdge {
                    dependency: 1,
                    dependent: 2,
                },
                GraphEdge {
                    dependency: 2,
                    dependent: 3,
                },
                GraphEdge {
                    dependency: 3,
                    dependent: 1,
                },
            ]))
        );
    }
}
