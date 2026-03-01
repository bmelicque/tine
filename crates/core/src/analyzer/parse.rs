use anyhow::bail;

use crate::{
    analyzer::{
        modules::{Module, ModuleId, ModulePath},
        session::Session,
        Source,
    },
    ast::Program,
    common::use_decl_to_paths,
    parser::{ParseResult, Parser},
};

impl Session {
    /// Parse a package starting from the given entry point, which should be
    /// the program's main entry.
    /// Entry point should be a real, canonical file path.
    pub fn parse_project(&mut self, entry_point: ModulePath) -> anyhow::Result<()> {
        assert!(matches!(entry_point, ModulePath::Real(_)));
        self.parse_module(&entry_point)?;
        Ok(())
    }

    fn parse_module(&mut self, path: &ModulePath) -> anyhow::Result<ModuleId> {
        let (module_id, result) = match path {
            ModulePath::Real(_) => self.parse_real_module(path)?,
            ModulePath::Virtual(c) => self.parse_virtual_module(c)?,
        };
        let file_names = get_dependencies(&self.entry_point, &result.node);

        self.parsed.insert(module_id, result.node);
        self.diagnostics.insert(module_id, result.diagnostics);

        for dependency_name in file_names {
            self.parse_dependency(&dependency_name, module_id);
        }

        Ok(module_id)
    }

    fn parse_dependency(&mut self, dependency_name: &ModulePath, parent_id: ModuleId) {
        if let Some(dependency_id) = self.module_graph.find_id(&dependency_name) {
            self.module_graph.add_edge(dependency_id, parent_id.clone());
            return;
        }
        let Ok(dependency) = self.parse_module(&dependency_name) else {
            todo!()
        };
        self.module_graph.add_edge(dependency, parent_id);
    }

    fn parse_real_module(&mut self, path: &ModulePath) -> anyhow::Result<(ModuleId, ParseResult)> {
        let src = self.loader.load(path)?;
        let module_id = self.module_graph.next_id();
        let result = Parser::new(module_id, &src).parse();
        let module = Module {
            name: path.to_owned(),
            src: Source::new(&src),
        };
        self.module_graph.add_module(module);

        Ok((module_id, result))
    }

    fn parse_virtual_module(&mut self, name: &String) -> anyhow::Result<(ModuleId, ParseResult)> {
        let src = Source::new("");
        let module = match name.as_str() {
            "dom" | "signals" => Module {
                src,
                name: ModulePath::Virtual(name.to_string()),
            },
            name => bail!("Cannot find module '{}'", name),
        };
        let module_id = self.module_graph.add_module(module);
        let result = ParseResult {
            node: Program::dummy(),
            diagnostics: vec![],
        };
        Ok((module_id, result))
    }
}

fn get_dependencies(root_path: &ModulePath, ast: &Program) -> Vec<ModulePath> {
    let mut file_names: Vec<ModulePath> = ast
        .items
        .iter()
        .filter_map(|item| item.as_use_declaration_ref())
        .flat_map(|decl| use_decl_to_paths(root_path, decl))
        .map(|imports| imports.module_name)
        .collect();
    file_names.sort();
    file_names.dedup();
    file_names
}
