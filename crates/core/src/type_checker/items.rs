use crate::{
    analyzer::ModuleId,
    ast::{self, UseTree},
    common::{use_decl_to_paths, ModuleImports},
    ir,
    type_checker::TypeChecker,
    DiagnosticKind, Location, SymbolRef,
};

impl TypeChecker<'_> {
    pub fn visit_item(&mut self, node: ast::Item) -> Vec<ir::Statement> {
        match node {
            ast::Item::Invalid(_) => vec![],
            ast::Item::Statement(s) => self.visit_statement(s),
            ast::Item::UseDeclaration(u) => self
                .visit_use_declaration(u)
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }

    fn visit_use_declaration(&mut self, node: ast::UseDeclaration) -> Vec<ir::UseDeclaration> {
        if node.relative_count == 0 {
            match self.visit_use_virtual_module(node) {
                Some(decl) => vec![decl],
                None => vec![],
            }
        } else {
            self.visit_use_real_modules(node)
        }
    }

    fn visit_use_virtual_module(
        &mut self,
        node: ast::UseDeclaration,
    ) -> Option<ir::UseDeclaration> {
        debug_assert_eq!(node.relative_count, 0);
        let module_name = node.tree.path[0].as_str();
        let Some(module_id) = self.session.get_module_id(module_name.into()) else {
            let error = DiagnosticKind::CannotFindModule {
                name: module_name.to_string(),
            };
            self.error(error, node.loc);
            return None;
        };
        let subtree = ast::UseTree {
            path: node.tree.path.iter().skip(1).cloned().collect(),
            sub_trees: node.tree.sub_trees.clone(),
        };
        let symbols = if subtree.path.len() > 0 {
            match self.visit_imported_name(module_id, &subtree) {
                Some(symbol) => vec![symbol],
                None => vec![],
            }
        } else {
            subtree
                .sub_trees
                .into_iter()
                .filter_map(|tree| self.visit_imported_name(module_id, &tree))
                .collect()
        };

        let path = self.session.read_module(module_id).name.clone();
        Some(ir::UseDeclaration {
            loc: node.loc,
            module: module_id,
            path,
            symbols,
        })
    }

    fn visit_use_real_modules(&mut self, node: ast::UseDeclaration) -> Vec<ir::UseDeclaration> {
        debug_assert_ne!(node.relative_count, 0);
        let base_path = self.get_file_name();
        let imports = use_decl_to_paths(&base_path, &node);

        imports
            .into_iter()
            .filter_map(|import| self.visit_module_imports(import, node.loc))
            .collect()
    }

    /// Visit all the import trees from a given real module.
    ///
    /// For example, `use Module.(a, b.c)` will visit import trees `a` and `b.c` from module `Module`
    fn visit_module_imports(
        &mut self,
        imports: ModuleImports,
        loc: Location,
    ) -> Option<ir::UseDeclaration> {
        let module_name = imports.module_name;
        let Some(module_id) = self.session.get_module_id(module_name.clone()) else {
            panic!("Cannot find module '{}' within parsed modules", module_name)
        };
        let symbols = imports
            .import_tree
            .into_iter()
            .map(|subtree| self.visit_imported_name(module_id, &subtree))
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<Option<Vec<_>>>()?;

        Some(ir::UseDeclaration {
            module: module_id,
            path: module_name,
            loc,
            symbols,
        })
    }

    /// Visit an imported element.
    ///
    /// Subvalue imports (like `use Module.value.subvalue`) are not permitted (yet?).
    fn visit_imported_name(&mut self, module: ModuleId, tree: &UseTree) -> Option<SymbolRef> {
        let path_element = &tree.path[0];
        let name = path_element.as_str();
        let Some(symbol) = self.session.find_export(module, name) else {
            let error = DiagnosticKind::UnknownMember {
                member: name.to_string(),
            };
            self.error(error, path_element.loc());
            return None;
        };
        symbol.borrow().access.read(path_element.loc());
        self.ctx.import(symbol.clone());
        if tree.path.len() > 1 || tree.sub_trees.len() > 0 {
            self.error(DiagnosticKind::UnexpectedModuleTree, tree.loc());
            return None;
        }

        Some(symbol)
    }
}
